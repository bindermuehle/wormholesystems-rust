//! The structure watch: what a sharing character's notifications and structure list put
//! on a map, what reaches Discord, and that none of it happens without the per-map switch.

use crate::common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use common::app::{app, session_cookie};
use common::{member_with_role, world};
use serde_json::{Value, json};
use sqlx::PgPool;
use wormholesystems::alerts::Runtime;
use wormholesystems::db::PgTokenStore;
use wormholesystems::esi::sso::{Sso, SsoConfig};
use wormholesystems::esi::{EsiClient, Token, TokenCache, TokenStore};
use wormholesystems::maps::structures::{StructureEventKind, for_map};
use wormholesystems::maps::{Actor, Role};
use wormholesystems::structure_watch::{Watch, sources};

/// A player corporation: the watch skips NPC ones, which is where the test world lives.
const CORP: i64 = 98_000_001;
const STRUCTURE: i64 = 1_026_660_410_904;
const NOTIFICATIONS: &str = "esi-characters.read_notifications.v1";
const STRUCTURES: &str = "esi-corporations.read_structures.v1";

type Received = Arc<Mutex<Vec<Value>>>;

/// What the stubbed ESI answers with.
#[derive(Clone)]
struct Esi {
    notifications: Value,
    structures: Value,
    /// A character with no roles in game: an empty inbox, and a 403 for the list.
    plain_member: Option<i64>,
}

impl Esi {
    fn answering(notifications: Value, structures: Value) -> Esi {
        Esi {
            notifications,
            structures,
            plain_member: None,
        }
    }

    fn is_plain_member(&self, headers: &HeaderMap) -> bool {
        let bearer = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        self.plain_member
            .is_some_and(|id| bearer == Some(format!("Bearer access-{id}").as_str()))
    }
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let base = format!(
        "http://{}",
        listener.local_addr().expect("read listener address")
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test app");
    });
    base
}

async fn esi(answers: Esi) -> EsiClient {
    async fn notifications(State(esi): State<Esi>, headers: HeaderMap) -> Json<Value> {
        if esi.is_plain_member(&headers) {
            return Json(json!([]));
        }
        Json(esi.notifications)
    }
    async fn structures(
        State(esi): State<Esi>,
        headers: HeaderMap,
    ) -> Result<Json<Value>, StatusCode> {
        if esi.is_plain_member(&headers) {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(Json(esi.structures))
    }
    let base = serve(
        Router::new()
            .route("/characters/{id}/notifications", get(notifications))
            .route("/corporations/{id}/structures", get(structures))
            .with_state(answers),
    )
    .await;
    EsiClient::with_config(reqwest::Client::new(), base, "2026-06-09")
}

/// A stand-in Discord webhook that keeps what it is sent.
async fn inbox() -> (String, Received) {
    async fn receive(State(messages): State<Received>, body: String) -> StatusCode {
        let message: Value = serde_json::from_str(&body).expect("parse JSON");
        messages.lock().expect("mutex poisoned").push(message);
        StatusCode::NO_CONTENT
    }
    let messages: Received = Arc::default();
    let base = serve(
        Router::new()
            .route("/webhook", post(receive))
            .with_state(Arc::clone(&messages)),
    )
    .await;
    (format!("{base}/webhook"), messages)
}

fn titles(messages: &Received) -> Vec<String> {
    let mut titles: Vec<String> = messages
        .lock()
        .expect("mutex poisoned")
        .iter()
        .map(|m| m["embeds"][0]["title"].as_str().unwrap_or("").to_string())
        .collect();
    titles.sort();
    titles
}

/// Delivery is a worker of its own, so the messages arrive a moment after the pass.
async fn wait_for(messages: &Received, count: usize) {
    for _ in 0..100 {
        if messages.lock().expect("mutex poisoned").len() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Put the character in the player corporation and give them a token with `scopes`.
async fn director(pool: &PgPool, actor: Actor, scopes: &[&str]) {
    sqlx::query(
        "insert into corporations (id, name, ticker) values ($1, 'Hole Control', 'HOLE')
         on conflict (id) do nothing",
    )
    .bind(CORP)
    .execute(pool)
    .await
    .expect("insert corporations");
    sqlx::query("update characters set corporation_id = $2 where id = $1")
        .bind(actor.character_id)
        .bind(CORP)
        .execute(pool)
        .await
        .expect("update characters");
    PgTokenStore::new(pool.clone())
        .save(
            actor.character_id,
            &Token {
                access_token: format!("access-{}", actor.character_id),
                refresh_token: "refresh".into(),
                expires_at: SystemTime::now() + Duration::from_mins(20),
                scopes: scopes.iter().map(ToString::to_string).collect(),
            },
        )
        .await
        .expect("save token");
}

/// Turn the per-map switch on for `actor`.
async fn share(pool: &PgPool, map_id: i64, actor: Actor) {
    set_shared(pool, map_id, actor, "true").await;
}

async fn unshare(pool: &PgPool, map_id: i64, actor: Actor) {
    set_shared(pool, map_id, actor, "false").await;
}

async fn set_shared(pool: &PgPool, map_id: i64, actor: Actor, shared: &str) {
    sqlx::query(
        "insert into map_user_settings (map_id, user_id, structures_shared)
         values ($1, $2, $3::boolean)
         on conflict (map_id, user_id) do update set structures_shared = excluded.structures_shared",
    )
    .bind(map_id)
    .bind(actor.user_id)
    .bind(shared)
    .execute(pool)
    .await
    .expect("insert map_user_settings");
}

async fn alert(pool: &PgPool, map_id: i64, url: &str, kind: &str, fuel_days: Option<i32>) {
    let webhook: i64 = sqlx::query_scalar(
        "insert into map_webhooks (map_id, name, url) values ($1, 'Test channel', $2)
         on conflict (map_id, name) do update set url = excluded.url
         returning id",
    )
    .bind(map_id)
    .bind(url)
    .fetch_one(pool)
    .await
    .expect("insert map_webhooks");
    sqlx::query(
        "insert into map_alerts (map_id, name, kind, delivery, map_webhook_id, max_jumps, fuel_days)
         values ($1, $2, $2, 'webhook', $3, 0, $4)",
    )
    .bind(map_id)
    .bind(kind)
    .bind(webhook)
    .bind(fuel_days)
    .execute(pool)
    .await
    .expect("insert map_alerts");
}

fn attack(minutes_ago: i64) -> Value {
    json!({
        "notification_id": 1000 + minutes_ago,
        "sender_id": 1000137,
        "sender_type": "corporation",
        "type": "StructureUnderAttack",
        "timestamp": (Utc::now() - chrono::Duration::minutes(minutes_ago)).to_rfc3339(),
        "text": format!(
            "armorPercentage: 100.0\ncorpName: Evictors\nhullPercentage: 100.0\n\
             shieldPercentage: 80.0\nsolarsystemID: {}\nstructureID: &id001 {STRUCTURE}\n\
             structureShowInfoData:\n- showinfo\n- 35832\n- *id001\nstructureTypeID: 35832\n",
            common::SYS_A
        ),
    })
}

/// One attack just now, one three days ago, and something that is none of our business.
fn notifications() -> Value {
    json!([
        attack(5),
        attack(3 * 24 * 60),
        {
            "notification_id": 1, "sender_id": 90000001, "sender_type": "character",
            "type": "CorpAppNewMsg", "timestamp": Utc::now().to_rfc3339(),
            "text": "applicationText: hi\ncharID: 90000001\n",
        },
    ])
}

fn structures(fuel_days: i64) -> Value {
    json!([{
        "structure_id": STRUCTURE,
        "corporation_id": CORP,
        "name": "Jita - Home",
        "type_id": 35832,
        "system_id": common::SYS_A,
        "profile_id": 1,
        "state": "shield_vulnerable",
        "fuel_expires": (Utc::now() + chrono::Duration::days(fuel_days)).to_rfc3339(),
    }])
}

fn sso() -> Arc<Sso> {
    Arc::new(Sso::stub(
        reqwest::Client::new(),
        SsoConfig {
            client_id: "test".into(),
            client_secret: "test".into(),
            redirect_uri: "http://localhost/auth/callback".into(),
            scopes: Vec::new(),
        },
    ))
}

async fn watch(pool: &PgPool, answers: Esi, alerts: Option<Arc<Runtime>>) -> Watch {
    Watch::new(
        pool.clone(),
        sso(),
        Arc::new(TokenCache::new()),
        esi(answers).await,
        alerts,
    )
}

#[tokio::test]
async fn a_pass_fills_the_card_and_tells_discord_what_is_news() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    director(&pool, w.owner, &[NOTIFICATIONS, STRUCTURES]).await;
    share(&pool, w.map_id, w.owner).await;
    let (url, messages) = inbox().await;
    alert(&pool, w.map_id, &url, "structure_attack", None).await;
    alert(&pool, w.map_id, &url, "structure_fuel", Some(3)).await;
    let runtime = Arc::new(
        Runtime::load(&pool, None, "https://map.example.com".into())
            .await
            .unwrap(),
    );
    let answers = Esi::answering(notifications(), structures(2));

    watch(&pool, answers.clone(), Some(Arc::clone(&runtime)))
        .await
        .pass()
        .await
        .unwrap();
    wait_for(&messages, 2).await;

    let card = for_map(&pool, w.map_id).await.unwrap();
    assert_eq!(card.sharers, 1);
    // Both attacks are on the card, newest first, and the application is not.
    assert_eq!(card.events.len(), 2);
    assert!(card.events[0].occurred_at > card.events[1].occurred_at);
    let event = &card.events[0];
    assert_eq!(event.kind, StructureEventKind::UnderAttack);
    assert_eq!(event.structure_name.as_deref(), Some("Jita - Home"));
    assert_eq!(event.corporation_name.as_deref(), Some("Hole Control"));
    assert_eq!(event.aggressor_corporation.as_deref(), Some("Evictors"));
    assert_eq!(event.shield, Some(80.0));
    assert_eq!(card.structures.len(), 1);
    assert_eq!(card.structures[0].name.as_deref(), Some("Jita - Home"));

    // Only the attack from just now is news; the one from three days ago is history.
    assert_eq!(
        titles(&messages),
        vec![
            "Jita - Home is under attack".to_string(),
            "Jita - Home runs out of fuel in 1 day".to_string(),
        ]
    );

    // A restart reads the same lists again and says nothing twice.
    watch(&pool, answers, Some(runtime))
        .await
        .pass()
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(titles(&messages).len(), 2);
    assert_eq!(for_map(&pool, w.map_id).await.unwrap().events.len(), 2);
}

#[tokio::test]
async fn fuel_that_lasts_longer_than_the_alert_asks_is_not_news() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    director(&pool, w.owner, &[STRUCTURES]).await;
    share(&pool, w.map_id, w.owner).await;
    let (url, messages) = inbox().await;
    alert(&pool, w.map_id, &url, "structure_fuel", Some(3)).await;
    let runtime = Arc::new(
        Runtime::load(&pool, None, "https://map.example.com".into())
            .await
            .unwrap(),
    );
    let answers = Esi::answering(json!([]), structures(10));

    watch(&pool, answers, Some(runtime))
        .await
        .pass()
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(for_map(&pool, w.map_id).await.unwrap().structures.len(), 1);
    assert_eq!(titles(&messages), Vec::<String>::new());
}

/// A grant on a map is given, not asked for. Somebody adding a corporation to their map
/// must not get its structures with it.
#[tokio::test]
async fn nothing_is_read_or_shown_without_the_switch() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    director(&pool, w.owner, &[NOTIFICATIONS, STRUCTURES]).await;
    let answers = Esi::answering(notifications(), structures(2));

    assert!(sources(&pool).await.unwrap().is_empty());
    watch(&pool, answers.clone(), None)
        .await
        .pass()
        .await
        .unwrap();
    let card = for_map(&pool, w.map_id).await.unwrap();
    assert_eq!(
        (card.sharers, card.events.len(), card.structures.len()),
        (0, 0, 0)
    );

    // On, it fills; off again, it empties, though the rows are still there.
    share(&pool, w.map_id, w.owner).await;
    watch(&pool, answers, None).await.pass().await.unwrap();
    let card = for_map(&pool, w.map_id).await.unwrap();
    assert_eq!((card.events.len(), card.structures.len()), (2, 1));

    unshare(&pool, w.map_id, w.owner).await;
    let card = for_map(&pool, w.map_id).await.unwrap();
    assert_eq!(
        (card.sharers, card.events.len(), card.structures.len()),
        (0, 0, 0)
    );
}

/// Sharing reaches as far as what the sharer could read: a member without the roles
/// shares nothing, even when a director of the same corporation shares with another map.
#[tokio::test]
async fn a_member_shares_only_what_they_could_read_themselves() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    director(&pool, w.owner, &[NOTIFICATIONS, STRUCTURES]).await;
    share(&pool, w.map_id, w.owner).await;

    // A plain member of the same corporation, on a map of their own.
    let user = common::new_user(&pool).await;
    common::add_character(&pool, user, 1002, 2001, None).await;
    let member = Actor {
        user_id: user,
        character_id: 1002,
    };
    let theirs = wormholesystems::maps::map::create_map(
        &pool,
        member,
        &wormholesystems::allowlist::AffiliationAllowlist::default(),
        wormholesystems::maps::map::CreateMap {
            name: "Spy".into(),
            description: None,
        },
    )
    .await
    .unwrap()
    .id;
    director(&pool, member, &[NOTIFICATIONS, STRUCTURES]).await;
    share(&pool, theirs, member).await;
    // ESI gives them an empty inbox and refuses them the list.
    let answers = Esi {
        plain_member: Some(member.character_id),
        ..Esi::answering(notifications(), structures(2))
    };
    watch(&pool, answers, None).await.pass().await.unwrap();

    let card = for_map(&pool, theirs).await.unwrap();
    assert_eq!(
        (card.sharers, card.events.len(), card.structures.len()),
        (1, 0, 0)
    );
    // The director's own map is none the worse for it.
    let card = for_map(&pool, w.map_id).await.unwrap();
    assert_eq!((card.events.len(), card.structures.len()), (2, 1));
}

#[tokio::test]
async fn the_card_is_for_members_not_viewers() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    let viewer = member_with_role(&pool, w.owner, w.map_id, 1002, 2002, Role::Viewer).await;
    let path = format!("/api/maps/{}/structures", w.map_id);

    let owner = session_cookie(&pool, w.owner).await;
    let read = common::app::get(app(&pool), &path, Some(&owner)).await;
    assert_eq!(read.status, StatusCode::OK);
    assert_eq!(read.body["sharers"], 0);

    let viewer = session_cookie(&pool, viewer).await;
    let refused = common::app::get(app(&pool), &path, Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_switch_is_a_per_map_user_setting() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    let cookie = session_cookie(&pool, w.owner).await;
    let path = format!("/api/maps/{}/settings/user", w.map_id);

    let before = common::app::get(app(&pool), &path, Some(&cookie)).await;
    assert_eq!(before.body["structures_shared"], false);

    let after = common::app::request_json(
        app(&pool),
        "POST",
        &path,
        Some(&cookie),
        json!({ "structures_shared": true }),
    )
    .await;
    assert_eq!(after.status, StatusCode::OK);
    assert_eq!(after.body["structures_shared"], true);
    assert_eq!(for_map(&pool, w.map_id).await.unwrap().sharers, 1);
}

#[tokio::test]
async fn an_alert_keeps_the_events_it_was_given_and_refuses_the_other_kinds() {
    let (pool, _db) = common::db::fresh().await;
    let w = world(&pool).await;
    let cookie = session_cookie(&pool, w.owner).await;
    let webhook = common::app::request_json(
        app(&pool),
        "POST",
        &format!("/api/maps/{}/webhooks", w.map_id),
        Some(&cookie),
        json!({ "name": "Ops", "url": "https://discord.com/api/webhooks/123456/token" }),
    )
    .await
    .body["id"]
        .clone();
    let alert = |events: Value| {
        json!({
            "name": "Defence", "kind": "structure_attack", "delivery": "webhook",
            "map_webhook_id": webhook, "mention": "none", "max_jumps": 0,
            "filters": [], "filter_match": "any", "structure_events": events,
        })
    };
    let path = format!("/api/maps/{}/alerts", w.map_id);

    let made = common::app::request_json(
        app(&pool),
        "POST",
        &path,
        Some(&cookie),
        alert(json!(["lost_armor", "unanchoring"])),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK);
    assert_eq!(
        made.body["structure_events"],
        json!(["lost_armor", "unanchoring"])
    );

    // Low fuel is the fuel alert's to speak about; here it would never fire.
    let refused = common::app::request_json(
        app(&pool),
        "POST",
        &path,
        Some(&cookie),
        alert(json!(["fuel_low"])),
    )
    .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
}
