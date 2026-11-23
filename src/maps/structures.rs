//! The structures a map's members share with it: what happened to them lately, and how
//! long their fuel lasts. Kept current by [`crate::structure_watch`].
//!
//! Nothing is the map's own. A row is here because a character who could read it belongs
//! to a user who shares with the map (`map_structure_sharers`), and it stops being here the
//! moment they stop, lose their grant or leave the account.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::error::Result;

super::text_enum! {
    /// What an in-game notification said happened to a structure.
    pub enum StructureEventKind {
        UnderAttack => "under_attack",
        LostShields => "lost_shields",
        LostArmor => "lost_armor",
        /// A customs office or mercenary den went into its reinforcement timer.
        Reinforced => "reinforced",
        Destroyed => "destroyed",
        /// EVE's own warning that the fuel bay is nearly empty.
        FuelLow => "fuel_low",
        /// An Upwell structure ran dry.
        LowPower => "low_power",
        /// A Metenox drill is short of, or out of, reagents.
        ReagentsLow => "reagents_low",
        /// Somebody started taking it down. Not an attack, but the thing a corporation
        /// most wants to hear about when it was not planned.
        Unanchoring => "unanchoring",
        /// One of ours started anchoring.
        Anchoring => "anchoring",
        /// It finished anchoring and came online.
        Online => "online",
        /// Refuelled after running dry.
        HighPower => "high_power",
        /// Its service modules went offline, which they do when the fuel is gone.
        ServicesOffline => "services_offline",
    }
}

impl StructureEventKind {
    /// Which of EVE's notification types we take, and as what. Everything else a character
    /// receives is none of the map's business and is never stored.
    pub fn of_notification(kind: &str) -> Option<StructureEventKind> {
        Some(match kind {
            "StructureUnderAttack"
            | "TowerAlertMsg"
            | "OrbitalAttacked"
            | "SkyhookUnderAttack"
            | "MercenaryDenAttacked" => StructureEventKind::UnderAttack,
            "StructureLostShields" | "SkyhookLostShields" => StructureEventKind::LostShields,
            "StructureLostArmor" => StructureEventKind::LostArmor,
            "OrbitalReinforced" | "MercenaryDenReinforced" => StructureEventKind::Reinforced,
            "StructureDestroyed" | "SkyhookDestroyed" => StructureEventKind::Destroyed,
            "StructureFuelAlert" | "TowerResourceAlertMsg" => StructureEventKind::FuelLow,
            "StructureWentLowPower" => StructureEventKind::LowPower,
            "StructureLowReagentsAlert" | "StructureNoReagentsAlert" => {
                StructureEventKind::ReagentsLow
            }
            "StructureUnanchoring" => StructureEventKind::Unanchoring,
            "StructureAnchoring" => StructureEventKind::Anchoring,
            "StructureOnline" => StructureEventKind::Online,
            "StructureWentHighPower" => StructureEventKind::HighPower,
            "StructureServicesOffline" => StructureEventKind::ServicesOffline,
            _ => return None,
        })
    }

    pub const ALL: [StructureEventKind; 13] = [
        StructureEventKind::UnderAttack,
        StructureEventKind::LostShields,
        StructureEventKind::LostArmor,
        StructureEventKind::Reinforced,
        StructureEventKind::Destroyed,
        StructureEventKind::Unanchoring,
        StructureEventKind::Anchoring,
        StructureEventKind::Online,
        StructureEventKind::FuelLow,
        StructureEventKind::LowPower,
        StructureEventKind::ReagentsLow,
        StructureEventKind::HighPower,
        StructureEventKind::ServicesOffline,
    ];

    /// About fuel, as opposed to what is done to the structure. Decides which of the two
    /// alerts hears of it.
    pub fn is_fuel(self) -> bool {
        matches!(
            self,
            StructureEventKind::FuelLow
                | StructureEventKind::LowPower
                | StructureEventKind::ReagentsLow
                | StructureEventKind::HighPower
                | StructureEventKind::ServicesOffline
        )
    }

    /// Somebody is shooting it, so the notification says who.
    pub fn is_hostile(self) -> bool {
        matches!(
            self,
            StructureEventKind::UnderAttack
                | StructureEventKind::LostShields
                | StructureEventKind::LostArmor
                | StructureEventKind::Reinforced
                | StructureEventKind::Destroyed
        )
    }

    /// Whether an alert that was not told which events it wants speaks about this one.
    /// The rest are good news or routine, there for whoever asks for them.
    pub fn by_default(self) -> bool {
        !matches!(
            self,
            StructureEventKind::Anchoring
                | StructureEventKind::Online
                | StructureEventKind::HighPower
                | StructureEventKind::ServicesOffline
        )
    }
}

/// One thing that happened to a structure, with every id it carries named.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct StructureEvent {
    pub id: i64,
    pub kind: StructureEventKind,
    /// The structure's own name, or the planet a customs office or skyhook sits on. A
    /// tower has neither.
    #[ts(optional)]
    pub structure_name: Option<String>,
    #[ts(optional)]
    pub type_id: Option<i64>,
    #[ts(optional)]
    pub type_name: Option<String>,
    #[ts(optional)]
    pub solar_system_id: Option<i64>,
    #[ts(optional)]
    pub system_name: Option<String>,
    #[ts(optional)]
    pub security: Option<f64>,
    /// With the two ids below, enough for a row to offer the system's menu.
    #[ts(optional)]
    pub region: Option<String>,
    #[ts(optional)]
    pub region_id: Option<i64>,
    #[ts(optional)]
    pub constellation_id: Option<i64>,
    #[ts(optional)]
    pub wormhole_class_id: Option<i64>,
    pub corporation_id: i64,
    #[ts(optional)]
    pub corporation_name: Option<String>,
    /// Who is shooting, as far as the notification says and we could name.
    #[ts(optional)]
    pub aggressor: Option<String>,
    #[ts(optional)]
    pub aggressor_corporation: Option<String>,
    #[ts(optional)]
    pub aggressor_alliance: Option<String>,
    /// Percent left.
    #[ts(optional)]
    pub shield: Option<f32>,
    #[ts(optional)]
    pub armor: Option<f32>,
    #[ts(optional)]
    pub hull: Option<f32>,
    /// When the reinforcement timer this started runs out.
    #[ts(optional)]
    pub timer_ends_at: Option<DateTime<Utc>>,
    pub occurred_at: DateTime<Utc>,
}

impl StructureEvent {
    /// What to call it in a sentence: its name, else its type, else the least we can say.
    pub fn label(&self) -> &str {
        self.structure_name
            .as_deref()
            .or(self.type_name.as_deref())
            .unwrap_or("A structure")
    }
}

/// An Upwell structure a member shares, with its fuel timer.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct MapStructure {
    pub structure_id: i64,
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub type_id: Option<i64>,
    #[ts(optional)]
    pub type_name: Option<String>,
    #[ts(optional)]
    pub solar_system_id: Option<i64>,
    #[ts(optional)]
    pub system_name: Option<String>,
    #[ts(optional)]
    pub security: Option<f64>,
    /// With the two ids below, enough for a row to offer the system's menu.
    #[ts(optional)]
    pub region: Option<String>,
    #[ts(optional)]
    pub region_id: Option<i64>,
    #[ts(optional)]
    pub constellation_id: Option<i64>,
    #[ts(optional)]
    pub wormhole_class_id: Option<i64>,
    pub corporation_id: i64,
    #[ts(optional)]
    pub corporation_name: Option<String>,
    /// ESI's state, e.g. `shield_vulnerable`, `armor_reinforce`.
    pub state: String,
    /// Absent once it has run dry (or for a structure that burns none).
    #[ts(optional)]
    pub fuel_expires: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub state_timer_end: Option<DateTime<Utc>>,
    /// When ESI last listed it. The list is read hourly, so the state can be that old.
    pub updated_at: DateTime<Utc>,
}

impl MapStructure {
    pub fn label(&self) -> &str {
        self.name
            .as_deref()
            .or(self.type_name.as_deref())
            .unwrap_or("A structure")
    }
}

/// What the structures card shows.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct MapStructures {
    /// How many of the map's members share their structures with it, the caller included.
    pub sharers: i64,
    /// The last week, newest first.
    pub events: Vec<StructureEvent>,
    /// Soonest out of fuel first.
    pub structures: Vec<MapStructure>,
}

/// How far back the card reaches.
const EVENT_DAYS: i32 = 7;
const EVENT_LIMIT: i64 = 50;

/// The events a map may see, newest first: the last week's, or only `ids` of them.
pub async fn events(
    pool: &PgPool,
    map_id: i64,
    ids: Option<&[i64]>,
) -> Result<Vec<StructureEvent>> {
    let rows = sqlx::query!(
        r#"select e.id, e.kind, e.type_id, t.name as "type_name?",
                  coalesce(cs.name, s.name, mo.name, pl.name) as "structure_name?",
                  pl.celestial_index as "planet_index?",
                  e.solar_system_id, ss.name as "system_name?",
                  ss.security_status as "security?",
                  rg.name as "region?", ss.region_id as "region_id?",
                  ss.constellation_id as "constellation_id?",
                  ws.wormhole_class_id::bigint as "wormhole_class_id?",
                  e.corporation_id, oc.name as "corporation_name?",
                  ac.name as "aggressor?",
                  coalesce(acorp.name, e.aggressor_corporation) as "aggressor_corporation?",
                  aal.name as "aggressor_alliance?",
                  e.shield, e.armor, e.hull, e.timer_ends_at, e.occurred_at
           from structure_events e
           left join types t on t.id = e.type_id
           left join corporation_structures cs on cs.structure_id = e.structure_id
           left join structures s on s.id = e.structure_id
           left join moons mo on mo.id = e.moon_id
           left join planets pl on pl.id = e.planet_id
           left join solar_systems ss on ss.id = e.solar_system_id
           left join regions rg on rg.id = ss.region_id
           left join wormhole_systems ws on ws.solar_system_id = ss.id
           left join corporations oc on oc.id = e.corporation_id
           left join characters ac on ac.id = e.aggressor_character_id
           left join corporations acorp on acorp.id = e.aggressor_corporation_id
           left join alliances aal on aal.id = e.aggressor_alliance_id
           where e.occurred_at > now() - make_interval(days => $3)
             and ($2::bigint[] is null or e.id = any($2))
             and exists(
                 select 1 from structure_event_receipts rc
                 join map_structure_sharers sh on sh.character_id = rc.character_id
                 where rc.structure_event_id = e.id and sh.map_id = $1)
           order by e.occurred_at desc, e.id desc
           limit $4"#,
        map_id,
        ids,
        EVENT_DAYS,
        EVENT_LIMIT,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let kind = StructureEventKind::from_db(&r.kind)?;
            // The SDE names next to no planets, so a customs office is called what the
            // client calls it: the system and the planet's position.
            let structure_name = r.structure_name.or_else(|| {
                Some(crate::skyhooks::planet_label(
                    r.system_name.as_deref()?,
                    r.planet_index?,
                    None,
                ))
            });
            Some(StructureEvent {
                id: r.id,
                kind,
                structure_name,
                type_id: r.type_id,
                type_name: r.type_name,
                solar_system_id: r.solar_system_id,
                system_name: r.system_name,
                security: r.security,
                region: r.region,
                region_id: r.region_id,
                constellation_id: r.constellation_id,
                wormhole_class_id: r.wormhole_class_id,
                corporation_id: r.corporation_id,
                corporation_name: r.corporation_name,
                aggressor: r.aggressor,
                aggressor_corporation: r.aggressor_corporation,
                aggressor_alliance: r.aggressor_alliance,
                shield: r.shield,
                armor: r.armor,
                hull: r.hull,
                timer_ends_at: r.timer_ends_at,
                occurred_at: r.occurred_at,
            })
        })
        .collect())
}

/// How long a character's read of the list vouches for them. The watch reads hourly; this
/// rides out a bad afternoon at ESI without showing a list its reader lost long ago.
const READER_FRESH_HOURS: i32 = 24;

/// The Upwell structures a map may see, soonest out of fuel first. A corporation's list is
/// visible through a sharing character who is in it and whom ESI let read it.
pub async fn structures(pool: &PgPool, map_id: i64) -> Result<Vec<MapStructure>> {
    Ok(sqlx::query_as!(
        MapStructure,
        r#"select cs.structure_id, cs.name, cs.type_id, t.name as "type_name?",
                  cs.solar_system_id, ss.name as "system_name?",
                  ss.security_status as "security?",
                  rg.name as "region?", ss.region_id as "region_id?",
                  ss.constellation_id as "constellation_id?",
                  ws.wormhole_class_id::bigint as "wormhole_class_id?",
                  cs.corporation_id, oc.name as "corporation_name?",
                  cs.state, cs.fuel_expires, cs.state_timer_end, cs.updated_at
           from corporation_structures cs
           left join types t on t.id = cs.type_id
           left join solar_systems ss on ss.id = cs.solar_system_id
           left join regions rg on rg.id = ss.region_id
           left join wormhole_systems ws on ws.solar_system_id = ss.id
           left join corporations oc on oc.id = cs.corporation_id
           where exists(
               select 1 from corporation_structure_readers rd
               join map_structure_sharers sh on sh.character_id = rd.character_id
               where sh.map_id = $1
                 and rd.corporation_id = cs.corporation_id
                 and sh.corporation_id = cs.corporation_id
                 and rd.read_at > now() - make_interval(hours => $2))
           order by cs.fuel_expires nulls last, cs.name"#,
        map_id,
        READER_FRESH_HOURS,
    )
    .fetch_all(pool)
    .await?)
}

/// Everything the card shows for a map.
pub async fn for_map(pool: &PgPool, map_id: i64) -> Result<MapStructures> {
    let sharers = sqlx::query_scalar!(
        r#"select count(*) as "sharers!" from map_user_settings mus
           join map_user_roles r on r.map_id = mus.map_id and r.user_id = mus.user_id
           where mus.map_id = $1 and mus.structures_shared"#,
        map_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(MapStructures {
        sharers,
        events: events(pool, map_id, None).await?,
        structures: structures(pool, map_id).await?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_structure_notifications_are_taken() {
        use StructureEventKind as K;
        assert_eq!(
            K::of_notification("StructureUnderAttack"),
            Some(K::UnderAttack)
        );
        assert_eq!(K::of_notification("TowerAlertMsg"), Some(K::UnderAttack));
        assert_eq!(K::of_notification("OrbitalReinforced"), Some(K::Reinforced));
        assert_eq!(
            K::of_notification("TowerResourceAlertMsg"),
            Some(K::FuelLow)
        );
        assert_eq!(K::of_notification("CorpAppNewMsg"), None);
        assert_eq!(K::of_notification("KillReportVictim"), None);
        assert_eq!(
            K::of_notification("StructureUnanchoring"),
            Some(K::Unanchoring)
        );
        assert!(K::Unanchoring.by_default() && !K::Unanchoring.is_hostile());
        assert!(!K::Online.by_default() && !K::HighPower.by_default());
        // Every kind is one a text column can hold and hand back.
        for kind in K::ALL {
            assert_eq!(K::from_db(kind.as_str()), Some(kind));
        }
        assert!(K::FuelLow.is_fuel() && K::LowPower.is_fuel() && K::ReagentsLow.is_fuel());
        assert!(!K::UnderAttack.is_fuel() && !K::Destroyed.is_fuel());
    }
}
