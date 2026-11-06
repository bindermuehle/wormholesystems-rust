//! The echo strategy's own endpoints, over HTTP: the names the signature list shows, and
//! naming a hole's ghost once its type is picked. Ours, not upstream's. Home is a C5 with
//! no statics, so the first hole off it takes branch `a`.

use crate::common;

use axum::http::StatusCode;
use common::app::{app, get, request_json, session_cookie};
use common::catalog::{C247, seed_catalog, seed_wormhole_system};
use common::map::{ghost_for, place, scan};
use common::{World, member_with_role, world};
use serde_json::json;
use sqlx::PgPool;
use wormholesystems::allowlist::AffiliationAllowlist;
use wormholesystems::maps::Role;
use wormholesystems::maps::map::{CreateMap, create_map};

const C5: i64 = 31000001;

struct EchoMap {
    w: World,
    cookie: String,
    home: i64,
    /// A scan in home whose type leads to a C3, and the ghost the map raised for it.
    sig: i64,
    ghost: i64,
}

async fn echo_map(pool: &PgPool, strategy: &str) -> EchoMap {
    let w = world(pool).await;
    seed_catalog(pool).await;
    seed_wormhole_system(pool, C5, 5).await;
    on_map(pool, w, strategy).await
}

/// Another map of the same owner's. `world` makes the owner, so it runs once per test.
async fn another_map(pool: &PgPool, w: &World, strategy: &str) -> EchoMap {
    let map = create_map(
        pool,
        w.owner,
        &AffiliationAllowlist::default(),
        CreateMap {
            name: "Elsewhere".into(),
            description: None,
        },
    )
    .await
    .expect("create_map");
    on_map(
        pool,
        World {
            owner: w.owner,
            map_id: map.id,
        },
        strategy,
    )
    .await
}

async fn on_map(pool: &PgPool, w: World, strategy: &str) -> EchoMap {
    sqlx::query("update maps set naming_strategy = $2::naming_strategy where id = $1")
        .bind(w.map_id)
        .bind(strategy)
        .execute(pool)
        .await
        .expect("set up the map");
    let home = place(pool, w.owner, w.map_id, C5).await;
    sqlx::query("update map_solar_systems set is_home = true where id = $1")
        .bind(home)
        .execute(pool)
        .await
        .expect("set up the map");
    let sig = scan(pool, w.owner, w.map_id, C5, "ABC-123").await;
    sqlx::query("update signatures set signature_type_id = $2 where id = $1")
        .bind(sig)
        .bind(C247)
        .execute(pool)
        .await
        .expect("set up the map");
    let ghost = ghost_for(pool, w.map_id, sig).await;
    let cookie = session_cookie(pool, w.owner).await;
    EchoMap {
        w,
        cookie,
        home,
        sig,
        ghost,
    }
}

async fn alias_of(pool: &PgPool, node: i64) -> Option<String> {
    sqlx::query_scalar("select alias from map_solar_systems where id = $1")
        .bind(node)
        .fetch_one(pool)
        .await
        .expect("read the alias")
}

#[tokio::test]
async fn the_rows_suggest_a_name_until_the_ghost_carries_it() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "echo").await;
    let rows = format!("/api/maps/{}/naming/echo/rows/{}", m.w.map_id, m.home);

    let before = get(app(&pool), &rows, Some(&m.cookie)).await;
    assert_eq!(before.status, StatusCode::OK);
    assert_eq!(
        before.body,
        json!([{ "signature": m.sig, "alias": "a3a", "homeward": false, "settled": false }])
    );

    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let named = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&m.cookie),
        json!({ "signature": m.sig }),
    )
    .await;
    assert_eq!(named.body, json!({ "alias": "a3a" }));
    assert_eq!(alias_of(&pool, m.ghost).await.as_deref(), Some("a3a"));

    let after = get(app(&pool), &rows, Some(&m.cookie)).await;
    assert_eq!(after.body[0]["settled"], json!(true));
}

#[tokio::test]
async fn the_picked_type_names_the_ghost_before_it_is_saved() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "echo").await;
    sqlx::query("update signatures set signature_type_id = null where id = $1")
        .bind(m.sig)
        .execute(&pool)
        .await
        .unwrap();
    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let named = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&m.cookie),
        json!({ "signature": m.sig, "signature_type_id": C247 }),
    )
    .await;
    assert_eq!(named.body, json!({ "alias": "a3a" }));
}

#[tokio::test]
async fn a_name_somebody_typed_is_left_alone() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "echo").await;
    sqlx::query("update map_solar_systems set alias = 'Staging' where id = $1")
        .bind(m.ghost)
        .execute(&pool)
        .await
        .unwrap();
    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let named = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&m.cookie),
        json!({ "signature": m.sig }),
    )
    .await;
    assert_eq!(named.body, json!({ "alias": null }));
    assert_eq!(alias_of(&pool, m.ghost).await.as_deref(), Some("Staging"));
}

#[tokio::test]
async fn a_map_on_another_strategy_gets_nothing() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "chain").await;
    let rows = format!("/api/maps/{}/naming/echo/rows/{}", m.w.map_id, m.home);
    assert_eq!(
        get(app(&pool), &rows, Some(&m.cookie)).await.body,
        json!([])
    );
    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let named = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&m.cookie),
        json!({ "signature": m.sig }),
    )
    .await;
    assert_eq!(named.body, json!({ "alias": null }));
    assert_eq!(alias_of(&pool, m.ghost).await, None);
}

#[tokio::test]
async fn what_is_not_on_this_map_is_not_found() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "echo").await;
    let elsewhere = another_map(&pool, &m.w, "echo").await;

    let rows = format!(
        "/api/maps/{}/naming/echo/rows/{}",
        m.w.map_id, elsewhere.home
    );
    let refused = get(app(&pool), &rows, Some(&m.cookie)).await;
    assert_eq!(refused.status, StatusCode::NOT_FOUND);

    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let refused = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&m.cookie),
        json!({ "signature": elsewhere.sig }),
    )
    .await;
    assert_eq!(refused.status, StatusCode::NOT_FOUND);
    assert_eq!(alias_of(&pool, elsewhere.ghost).await, None);
}

#[tokio::test]
async fn a_viewer_reads_the_names_but_cannot_write_one() {
    let (pool, _db) = common::db::fresh().await;
    let m = echo_map(&pool, "echo").await;
    let viewer = member_with_role(&pool, m.w.owner, m.w.map_id, 1002, 2002, Role::Viewer).await;
    let cookie = session_cookie(&pool, viewer).await;

    let rows = format!("/api/maps/{}/naming/echo/rows/{}", m.w.map_id, m.home);
    assert_eq!(
        get(app(&pool), &rows, Some(&cookie)).await.body[0]["alias"],
        json!("a3a")
    );

    let ghost = format!("/api/maps/{}/naming/echo/ghost", m.w.map_id);
    let refused = request_json(
        app(&pool),
        "POST",
        &ghost,
        Some(&cookie),
        json!({ "signature": m.sig }),
    )
    .await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);
    assert_eq!(alias_of(&pool, m.ghost).await, None);

    let stranger = request_json(
        app(&pool),
        "POST",
        &ghost,
        None,
        json!({ "signature": m.sig }),
    )
    .await;
    assert_eq!(stranger.status, StatusCode::UNAUTHORIZED);
}

/// 0048 moves the maps that named themselves with the echo alias scheme onto the echo
/// strategy. The test database is migrated before any map exists, so the update has only
/// ever met an empty table there; this runs it against one.
#[tokio::test]
async fn migration_0048_moves_echo_maps_onto_the_echo_strategy() {
    let (pool, _db) = common::db::fresh().await;
    let echo = world(&pool).await;
    let other = create_map(
        &pool,
        echo.owner,
        &AffiliationAllowlist::default(),
        CreateMap {
            name: "Other".into(),
            description: None,
        },
    )
    .await
    .expect("create_map");
    sqlx::query("update maps set alias_scheme = 'echo', naming_strategy = 'chain' where id = $1")
        .bind(echo.map_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "update maps set alias_scheme = 'alphabetical', naming_strategy = 'chain' where id = $1",
    )
    .bind(other.id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!(
        "../../migrations/0048_echo_maps_use_echo_strategy.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let settings = |id: i64| {
        let pool = pool.clone();
        async move {
            sqlx::query_as::<_, (String, String)>(
                "select naming_strategy::text, alias_scheme::text from maps where id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(
        settings(echo.map_id).await,
        ("echo".to_string(), "numeric".to_string())
    );
    assert_eq!(
        settings(other.id).await,
        ("chain".to_string(), "alphabetical".to_string())
    );
}
