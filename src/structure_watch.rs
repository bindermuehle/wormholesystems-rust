//! Watching the structures members share with their maps, see
//! [processes.md](../docs/processes.md#structure-watch).
//!
//! ESI tells nobody anything, so this asks. For every character whose user shares
//! structures with a map it reads two things with that character's own token: the in-game
//! notifications they received (attacks on anything, and EVE's low-fuel warnings), and
//! their corporation's structure list with its fuel timers. What comes back is stored
//! against the character who could read it, the maps they share with are told, and those
//! maps' structure alerts are offered it.
//!
//! Unlike the presence poller this runs whether or not anyone has the app open: a
//! structure is shot when nobody is looking.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::alerts::Runtime;
use crate::db::PgTokenStore;
use crate::entities::{self, EntityKind};
use crate::esi::{CorporationStructure, EsiClient, EsiError, Notification, Scope, Sso, TokenCache};
use crate::maps::structures::StructureEventKind;
use crate::realtime::MapEvent;
use crate::server_status::ServerWatch;

/// How often to look for something due. The waits below decide what is.
const TICK: Duration = Duration::from_secs(60);

/// ESI caches a character's notifications for ten minutes, and allows only a handful of
/// reads in fifteen, so asking sooner buys nothing and costs the budget.
const NOTIFICATIONS_EVERY: Duration = Duration::from_secs(10 * 60 + 5);
/// ESI caches a corporation's structure list for an hour.
const STRUCTURES_EVERY: Duration = Duration::from_secs(60 * 60 + 5);
/// After a failure that may pass: ESI had a bad minute, or said to back off.
const RETRY: Duration = Duration::from_mins(5);
/// After ESI refused the character outright. The structure list takes the Station Manager
/// role, which is given by hand in game, so it is asked again a few times a day.
const REFUSED: Duration = Duration::from_hours(6);

/// A notification older than this is not stored: the card shows a week.
const STORE_DAYS: i64 = 7;
/// A notification older than this is stored but no longer news. The list ESI returns
/// reaches back weeks, and the first read of a character must not replay it into Discord.
const NEWS_HOURS: i64 = 2;

/// NPC corporations (ids below this) own no structures, and every character in one would
/// otherwise be read.
const FIRST_PLAYER_CORPORATION: i64 = 2_000_000;

/// A character whose user shares structures with at least one map, and what they granted.
#[derive(Debug, Clone)]
pub struct Source {
    pub character_id: i64,
    pub corporation_id: i64,
    /// The maps this character's structures are shared with.
    pub map_ids: Vec<i64>,
    pub notifications: bool,
    pub structures: bool,
}

/// When each character is next worth asking, per route. In memory only: after a restart
/// everyone is asked once, and ESI answers from its cache.
#[derive(Debug, Default)]
struct Due {
    notifications: Mutex<HashMap<i64, Instant>>,
    structures: Mutex<HashMap<i64, Instant>>,
}

impl Due {
    fn is_due(slots: &Mutex<HashMap<i64, Instant>>, character_id: i64) -> bool {
        slots
            .lock()
            .expect("structure watch lock")
            .get(&character_id)
            .is_none_or(|at| *at <= Instant::now())
    }

    fn wait(slots: &Mutex<HashMap<i64, Instant>>, character_id: i64, wait: Duration) {
        slots
            .lock()
            .expect("structure watch lock")
            .insert(character_id, Instant::now() + wait);
    }

    /// Forget characters who no longer share anything.
    fn retain(&self, sources: &[Source]) {
        for slots in [&self.notifications, &self.structures] {
            slots
                .lock()
                .expect("structure watch lock")
                .retain(|id, _| sources.iter().any(|s| s.character_id == *id));
        }
    }
}

/// Everything a pass needs.
#[derive(Debug)]
pub struct Watch {
    pub pool: PgPool,
    pub sso: Arc<Sso>,
    pub tokens: Arc<TokenCache>,
    pub esi: EsiClient,
    /// Absent when the alert runtime could not start; the card still fills.
    pub alerts: Option<Arc<Runtime>>,
    due: Due,
}

impl Watch {
    pub fn new(
        pool: PgPool,
        sso: Arc<Sso>,
        tokens: Arc<TokenCache>,
        esi: EsiClient,
        alerts: Option<Arc<Runtime>>,
    ) -> Watch {
        Watch {
            pool,
            sso,
            tokens,
            esi,
            alerts,
            due: Due::default(),
        }
    }

    /// One pass: read whatever is due for every sharing character.
    pub async fn pass(&self) -> sqlx::Result<()> {
        let sources = sources(&self.pool).await?;
        self.due.retain(&sources);
        // A character seen for the first time takes its place in its corporation's
        // rotation, instead of being read along with everyone else.
        for (character_id, wait) in stagger(&sources, NOTIFICATIONS_EVERY) {
            let mut slots = self.due.notifications.lock().expect("structure watch lock");
            slots
                .entry(character_id)
                .or_insert_with(|| Instant::now() + wait);
        }
        for source in &sources {
            // The list first: it is what names the structure an attack is about.
            if source.structures && Due::is_due(&self.due.structures, source.character_id) {
                let wait = self.read_structures(source).await;
                Due::wait(&self.due.structures, source.character_id, wait);
            }
            if source.notifications && Due::is_due(&self.due.notifications, source.character_id) {
                let wait = self.read_notifications(source).await;
                Due::wait(&self.due.notifications, source.character_id, wait);
            }
        }
        Ok(())
    }

    /// The character's access token, if it still carries `scope`.
    async fn token(&self, character_id: i64, scope: Scope) -> Option<String> {
        let store = PgTokenStore::new(self.pool.clone());
        match self
            .tokens
            .token(&self.sso, &store, character_id, &[scope])
            .await
        {
            Ok(token) if token.has_scope(scope) => Some(token.access_token),
            Ok(_) => None,
            Err(err) => {
                tracing::debug!("structure watch: no token for {character_id}: {err}");
                None
            }
        }
    }

    /// Read a character's notifications and take the ones about structures. Returns how
    /// long to leave them alone.
    async fn read_notifications(&self, source: &Source) -> Duration {
        let id = source.character_id;
        let Some(token) = self.token(id, Scope::ReadNotifications).await else {
            return RETRY;
        };
        let notifications = match self.esi.character_notifications(&token, id).await {
            Ok(notifications) => notifications,
            Err(EsiError::Api {
                status: 401 | 403, ..
            }) => return REFUSED,
            Err(err) => {
                tracing::debug!("structure watch: notifications of {id}: {err}");
                return RETRY;
            }
        };
        let now = Utc::now();
        let events: Vec<NewEvent> = notifications
            .iter()
            .filter(|n| n.timestamp > now - chrono::Duration::days(STORE_DAYS))
            .filter_map(|n| NewEvent::from_notification(n, source.corporation_id))
            .collect();
        match self.take_events(source, &events, now).await {
            Ok(()) => NOTIFICATIONS_EVERY,
            Err(err) => {
                tracing::error!("structure watch: storing events of {id}: {err}");
                RETRY
            }
        }
    }

    /// Store what a character received, tell their maps, and offer the news to the alerts.
    async fn take_events(
        &self,
        source: &Source,
        events: &[NewEvent],
        now: DateTime<Utc>,
    ) -> sqlx::Result<()> {
        let mut received = false;
        let mut news = Vec::new();
        for event in events {
            let (event_id, first_receipt) =
                store_event(&self.pool, event, source.character_id).await?;
            received |= first_receipt;
            // News once per character: a second read of the same list is not.
            if first_receipt && event.occurred_at > now - chrono::Duration::hours(NEWS_HOURS) {
                news.push((event_id, event));
            }
        }
        if !received {
            return Ok(());
        }
        if !news.is_empty() {
            self.name_aggressors(news.iter().map(|(_, event)| *event))
                .await;
        }
        let ids: Vec<i64> = news.iter().map(|(id, _)| *id).collect();
        for &map_id in &source.map_ids {
            crate::realtime::hub().publish(MapEvent::StructuresChanged { map_id });
            if let Some(alerts) = &self.alerts {
                let out = alerts.structure_events(&self.pool, map_id, &ids).await;
                alerts.enqueue(&self.pool, out).await;
            }
        }
        Ok(())
    }

    /// Fetch the names of whoever is shooting, so the message can say who. Best effort: an
    /// attack is news with or without the name.
    async fn name_aggressors(&self, events: impl Iterator<Item = &NewEvent>) {
        let (mut characters, mut corporations, mut alliances) = (Vec::new(), Vec::new(), vec![]);
        let mut owners = Vec::new();
        for event in events {
            characters.extend(event.aggressor_character_id);
            corporations.extend(event.aggressor_corporation_id);
            alliances.extend(event.aggressor_alliance_id);
            owners.push(event.corporation_id);
        }
        corporations.extend(owners);
        entities::ensure_character_names(&self.pool, &self.esi, &characters).await;
        entities::ensure(
            &self.pool,
            &self.esi,
            EntityKind::Corporation,
            &corporations,
        )
        .await;
        entities::ensure(&self.pool, &self.esi, EntityKind::Alliance, &alliances).await;
    }

    /// Read the corporation's structure list as this character. Returns how long to leave
    /// them alone.
    async fn read_structures(&self, source: &Source) -> Duration {
        let (id, corporation_id) = (source.character_id, source.corporation_id);
        let Some(token) = self.token(id, Scope::ReadCorporationStructures).await else {
            return RETRY;
        };
        let structures = match self
            .esi
            .corporation_structures(&token, corporation_id)
            .await
        {
            Ok(structures) => structures,
            // Not a Station Manager (or no longer one), so they vouch for nothing.
            Err(EsiError::Api {
                status: 401 | 403, ..
            }) => {
                if let Err(err) = forget_reader(&self.pool, id).await {
                    tracing::error!("structure watch: forgetting reader {id}: {err}");
                }
                for &map_id in &source.map_ids {
                    crate::realtime::hub().publish(MapEvent::StructuresChanged { map_id });
                }
                return REFUSED;
            }
            Err(err) => {
                tracing::debug!("structure watch: structures of {corporation_id}: {err}");
                return RETRY;
            }
        };
        if let Err(err) = store_structures(&self.pool, id, corporation_id, &structures).await {
            tracing::error!("structure watch: storing structures of {corporation_id}: {err}");
            return RETRY;
        }
        entities::ensure(
            &self.pool,
            &self.esi,
            EntityKind::Corporation,
            &[corporation_id],
        )
        .await;
        for &map_id in &source.map_ids {
            crate::realtime::hub().publish(MapEvent::StructuresChanged { map_id });
            if let Some(alerts) = &self.alerts {
                let out = alerts.structure_fuel(&self.pool, map_id).await;
                alerts.enqueue(&self.pool, out).await;
            }
        }
        STRUCTURES_EVERY
    }
}

/// Spawn the watch. Returns immediately; the loop runs for the process lifetime.
pub fn start(watch: Watch, server: ServerWatch) {
    let watch = Arc::new(watch);
    crate::background::spawn_every("structure watch", TICK, move || {
        let (watch, server) = (Arc::clone(&watch), server.clone());
        async move {
            if server.should_poll() {
                watch.pass().await?;
            }
            Ok::<_, sqlx::Error>(())
        }
    });
}

/// When each character first reads its notifications, counted from now: the characters of
/// one corporation spread evenly over `every`, the first of them at once.
///
/// ESI caches a character's notifications for ten minutes, per character. Read together,
/// three directors of one corporation see an attack ten minutes late at worst, all three
/// of them; read in turn, one of them sees it within about three. Each keeps the place it
/// starts with, since it is next due a fixed time after it was last read.
pub fn stagger(sources: &[Source], every: Duration) -> Vec<(i64, Duration)> {
    let mut corporations: HashMap<i64, Vec<i64>> = HashMap::new();
    for source in sources.iter().filter(|s| s.notifications) {
        corporations
            .entry(source.corporation_id)
            .or_default()
            .push(source.character_id);
    }
    corporations
        .into_values()
        .flat_map(|mut characters| {
            characters.sort_unstable();
            let count = u32::try_from(characters.len()).unwrap_or(u32::MAX);
            characters
                .into_iter()
                .zip(0u32..)
                .map(move |(id, turn)| (id, every * turn / count))
        })
        .collect()
}

/// Every character worth reading: their user shares structures with a map they still have
/// a grant on, they are in a player corporation, and their token carries one of the two
/// permissions and has not been refused.
pub async fn sources(pool: &PgPool) -> sqlx::Result<Vec<Source>> {
    let rows = sqlx::query!(
        r#"select sh.character_id as "character_id!", sh.corporation_id as "corporation_id!",
                  array_agg(distinct sh.map_id) as "map_ids!",
                  coalesce(bool_or(sc.name = $1), false) as "notifications!",
                  coalesce(bool_or(sc.name = $2), false) as "structures!"
           from map_structure_sharers sh
           join esi_tokens t on t.character_id = sh.character_id and t.rejected_at is null
           join esi_token_scopes ts on ts.token_id = t.id
           join esi_scopes sc on sc.id = ts.scope_id and sc.name in ($1, $2)
           where sh.corporation_id >= $3
           group by sh.character_id, sh.corporation_id
           order by sh.character_id"#,
        Scope::ReadNotifications.as_str(),
        Scope::ReadCorporationStructures.as_str(),
        FIRST_PLAYER_CORPORATION,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Source {
            character_id: r.character_id,
            corporation_id: r.corporation_id,
            map_ids: r.map_ids,
            notifications: r.notifications,
            structures: r.structures,
        })
        .collect())
}

/// A notification about a structure, read into columns.
#[derive(Debug, Clone, PartialEq)]
pub struct NewEvent {
    pub corporation_id: i64,
    pub kind: StructureEventKind,
    pub notification_type: String,
    pub structure_id: Option<i64>,
    pub moon_id: Option<i64>,
    pub planet_id: Option<i64>,
    pub type_id: Option<i64>,
    pub solar_system_id: Option<i64>,
    pub aggressor_character_id: Option<i64>,
    pub aggressor_corporation_id: Option<i64>,
    pub aggressor_alliance_id: Option<i64>,
    pub aggressor_corporation: Option<String>,
    pub shield: Option<f32>,
    pub armor: Option<f32>,
    pub hull: Option<f32>,
    pub timer_ends_at: Option<DateTime<Utc>>,
    pub occurred_at: DateTime<Utc>,
}

/// EVE counts time in 100 ns ticks.
const TICKS_PER_SECOND: i64 = 10_000_000;
/// Seconds from 1601, where Windows file times start, to 1970.
const FILETIME_EPOCH_OFFSET: i64 = 11_644_473_600;

impl NewEvent {
    /// `None` for the notifications that are not about a structure, which is most of them.
    pub fn from_notification(n: &Notification, corporation_id: i64) -> Option<NewEvent> {
        let kind = StructureEventKind::of_notification(&n.kind)?;
        let attack = kind.is_hostile();
        // Upwell notifications give percentages, towers and customs offices fractions.
        let health = |percent: &str, fractions: &[&str]| -> Option<f32> {
            let value = match n.number(&[percent]) {
                Some(percent) => percent,
                None => n.number(fractions)? * 100.0,
            };
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "a percentage, clamped to 0-100 first"
            )]
            Some(value.clamp(0.0, 100.0) as f32)
        };
        // A reinforcement timer is either how long is left or the moment it ends.
        let timer_ends_at = n
            .id(&["timeLeft"])
            .map(|ticks| n.timestamp + chrono::Duration::seconds(ticks / TICKS_PER_SECOND))
            .or_else(|| {
                let filetime = n.id(&["reinforceExitTime"])?;
                DateTime::from_timestamp(filetime / TICKS_PER_SECOND - FILETIME_EPOCH_OFFSET, 0)
            });
        Some(NewEvent {
            corporation_id,
            kind,
            notification_type: n.kind.clone(),
            structure_id: n.id(&["structureID", "skyhookID", "itemID"]),
            moon_id: n.id(&["moonID"]),
            planet_id: n.id(&["planetID"]),
            type_id: n.id(&["structureTypeID", "typeID"]),
            solar_system_id: n.id(&["solarsystemID", "solarSystemID"]),
            // On anything but an attack these keys, where present, name the owner.
            aggressor_character_id: attack.then(|| n.id(&["charID", "aggressorID"])).flatten(),
            aggressor_corporation_id: attack.then(|| n.id(&["aggressorCorpID"])).flatten(),
            aggressor_alliance_id: attack
                .then(|| n.id(&["aggressorAllianceID", "allianceID"]))
                .flatten(),
            aggressor_corporation: attack
                .then(|| n.field(&["corpName"]).map(str::to_string))
                .flatten(),
            shield: health("shieldPercentage", &["shieldValue", "shieldLevel"]),
            armor: health("armorPercentage", &["armorValue"]),
            hull: health("hullPercentage", &["hullValue"]),
            timer_ends_at,
            occurred_at: n.timestamp,
        })
    }

    /// The same for every character who received it: whose, what, about which and when.
    /// Not the notification's own id, which nothing promises is shared between recipients.
    pub fn dedup_key(&self) -> String {
        let about = self
            .structure_id
            .or(self.moon_id)
            .or(self.planet_id)
            .unwrap_or(0);
        format!(
            "{}:{}:{about}:{}",
            self.corporation_id,
            self.notification_type,
            self.occurred_at.timestamp()
        )
    }
}

/// Store an event unless its corporation already has it, and note that `character_id`
/// received it. Returns its id, and whether this is the first we hear of it from them.
pub async fn store_event(
    pool: &PgPool,
    event: &NewEvent,
    character_id: i64,
) -> sqlx::Result<(i64, bool)> {
    let event_id = sqlx::query_scalar!(
        // The no-op update is there so `returning` hands back the id of one already stored.
        "insert into structure_events
             (corporation_id, kind, notification_type, structure_id, moon_id, planet_id,
              type_id, solar_system_id, aggressor_character_id, aggressor_corporation_id,
              aggressor_alliance_id, aggressor_corporation, shield, armor, hull,
              timer_ends_at, occurred_at, dedup_key)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                 $18)
         on conflict (dedup_key) do update set dedup_key = excluded.dedup_key
         returning id",
        event.corporation_id,
        event.kind.as_str(),
        event.notification_type,
        event.structure_id,
        event.moon_id,
        event.planet_id,
        event.type_id,
        event.solar_system_id,
        event.aggressor_character_id,
        event.aggressor_corporation_id,
        event.aggressor_alliance_id,
        event.aggressor_corporation,
        event.shield,
        event.armor,
        event.hull,
        event.timer_ends_at,
        event.occurred_at,
        event.dedup_key(),
    )
    .fetch_one(pool)
    .await?;
    let first_receipt = sqlx::query!(
        "insert into structure_event_receipts (structure_event_id, character_id)
         values ($1, $2) on conflict do nothing",
        event_id,
        character_id,
    )
    .execute(pool)
    .await?
    .rows_affected()
        == 1;
    Ok((event_id, first_receipt))
}

/// Replace what we hold of a corporation's structures with what ESI just listed, and note
/// that `character_id` could read it. One transaction, so nobody sees half a list.
pub async fn store_structures(
    pool: &PgPool,
    character_id: i64,
    corporation_id: i64,
    structures: &[CorporationStructure],
) -> sqlx::Result<()> {
    let ids: Vec<i64> = structures.iter().map(|s| s.structure_id).collect();
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "delete from corporation_structures
         where corporation_id = $1 and structure_id <> all($2)",
        corporation_id,
        &ids,
    )
    .execute(&mut *tx)
    .await?;
    for structure in structures {
        sqlx::query!(
            "insert into corporation_structures
                 (structure_id, corporation_id, name, type_id, solar_system_id, state,
                  fuel_expires, state_timer_end)
             values ($1, $2, $3, $4, $5, $6, $7, $8)
             on conflict (structure_id) do update set
                 corporation_id = excluded.corporation_id, name = excluded.name,
                 type_id = excluded.type_id, solar_system_id = excluded.solar_system_id,
                 state = excluded.state, fuel_expires = excluded.fuel_expires,
                 state_timer_end = excluded.state_timer_end, updated_at = now()",
            structure.structure_id,
            corporation_id,
            structure.name,
            structure.type_id,
            structure.system_id,
            structure.state,
            structure.fuel_expires,
            structure.state_timer_end,
        )
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query!(
        "insert into corporation_structure_readers (character_id, corporation_id)
         values ($1, $2)
         on conflict (character_id) do update
         set corporation_id = excluded.corporation_id, read_at = now()",
        character_id,
        corporation_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

/// ESI refused the character the list, so they no longer vouch for it.
pub async fn forget_reader(pool: &PgPool, character_id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "delete from corporation_structure_readers where character_id = $1",
        character_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORP: i64 = 98_000_001;

    fn notification(kind: &str, text: &str) -> Notification {
        Notification {
            notification_id: 1,
            kind: kind.into(),
            timestamp: DateTime::from_timestamp(1_800_000_000, 0).expect("a valid timestamp"),
            text: Some(text.into()),
        }
    }

    #[test]
    fn an_upwell_attack_names_the_structure_and_who_is_shooting() {
        let n = notification(
            "StructureUnderAttack",
            "allianceID: 99005338\nallianceName: Pandemic Horde\narmorPercentage: 100.0\n\
             charID: 2112625428\ncorpName: Horde Vanguard.\nhullPercentage: 100.0\n\
             shieldPercentage: 94.88\nsolarsystemID: 30004608\n\
             structureID: &id001 1026660410904\nstructureShowInfoData:\n- showinfo\n- 35833\n\
             - *id001\nstructureTypeID: 35833\n",
        );
        let event = NewEvent::from_notification(&n, CORP).expect("a structure notification");
        assert_eq!(event.kind, StructureEventKind::UnderAttack);
        assert_eq!(event.structure_id, Some(1026660410904));
        assert_eq!(event.type_id, Some(35833));
        assert_eq!(event.solar_system_id, Some(30004608));
        assert_eq!(event.aggressor_character_id, Some(2112625428));
        assert_eq!(event.aggressor_alliance_id, Some(99005338));
        assert_eq!(
            event.aggressor_corporation.as_deref(),
            Some("Horde Vanguard.")
        );
        assert_eq!(event.shield, Some(94.88));
        assert_eq!(
            event.dedup_key(),
            "98000001:StructureUnderAttack:1026660410904:1800000000"
        );
    }

    /// A tower is known by its moon, reports fractions, and names its attacker by id.
    #[test]
    fn a_tower_attack_is_read_from_its_own_spelling() {
        let n = notification(
            "TowerAlertMsg",
            "aggressorAllianceID: 99000001\naggressorCorpID: 98000002\naggressorID: 90000001\n\
             armorValue: 1.0\nhullValue: 1.0\nmoonID: 40009081\nshieldValue: 0.5\n\
             solarSystemID: 30000142\ntypeID: 16213\n",
        );
        let event = NewEvent::from_notification(&n, CORP).expect("a structure notification");
        assert_eq!(event.kind, StructureEventKind::UnderAttack);
        assert_eq!((event.structure_id, event.moon_id), (None, Some(40009081)));
        assert_eq!(event.solar_system_id, Some(30000142));
        assert_eq!(event.aggressor_character_id, Some(90000001));
        assert_eq!(event.aggressor_corporation_id, Some(98000002));
        assert_eq!(event.shield, Some(50.0));
    }

    /// The owner's ids on a fuel warning are not somebody shooting it.
    #[test]
    fn a_fuel_warning_has_no_aggressor() {
        let n = notification(
            "TowerResourceAlertMsg",
            "allianceID: 99000001\ncorpID: 98000001\nmoonID: 40009081\n\
             solarSystemID: 30000142\ntypeID: 16213\nwants:\n- quantity: 120\n  typeID: 4051\n",
        );
        let event = NewEvent::from_notification(&n, CORP).expect("a structure notification");
        assert_eq!(event.kind, StructureEventKind::FuelLow);
        assert_eq!(event.aggressor_alliance_id, None);
        // The fuel type nested under `wants` is not the tower's.
        assert_eq!(event.type_id, Some(16213));
    }

    #[test]
    fn a_reinforcement_timer_is_read_either_way_it_is_given() {
        let left = notification(
            "StructureLostShields",
            "solarsystemID: 30004608\nstructureID: 1026660410904\nstructureTypeID: 35833\n\
             timeLeft: 864000000000\n",
        );
        let event = NewEvent::from_notification(&left, CORP).expect("a structure notification");
        assert_eq!(
            event.timer_ends_at,
            Some(left.timestamp + chrono::Duration::days(1))
        );
        // 2026-01-15 08:00:00 UTC as a Windows file time.
        let exit = notification(
            "OrbitalReinforced",
            "planetID: 40009077\nreinforceExitTime: 134129376000000000\n\
             solarSystemID: 30000142\ntypeID: 2233\n",
        );
        let event = NewEvent::from_notification(&exit, CORP).expect("a structure notification");
        assert_eq!(event.kind, StructureEventKind::Reinforced);
        assert_eq!(
            event.timer_ends_at.map(|t| t.to_rfc3339()),
            Some("2026-01-15T08:00:00+00:00".to_string())
        );
    }

    #[test]
    fn a_corporations_characters_read_in_turn_and_corporations_do_not_wait_on_each_other() {
        let source = |character_id, corporation_id, notifications| Source {
            character_id,
            corporation_id,
            map_ids: vec![1],
            notifications,
            structures: true,
        };
        let mut turns = stagger(
            &[
                source(3, CORP, true),
                source(1, CORP, true),
                source(2, CORP, true),
                // Another corporation's only character, and one that reads no notifications.
                source(4, CORP + 1, true),
                source(5, CORP, false),
            ],
            Duration::from_secs(600),
        );
        turns.sort_unstable();
        assert_eq!(
            turns,
            vec![
                (1, Duration::ZERO),
                (2, Duration::from_secs(200)),
                (3, Duration::from_secs(400)),
                (4, Duration::ZERO),
            ]
        );
    }

    #[test]
    fn anything_else_a_character_receives_is_ignored() {
        let n = notification("CorpAppNewMsg", "applicationText: hi\ncharID: 90000001\n");
        assert_eq!(NewEvent::from_notification(&n, CORP), None);
    }
}
