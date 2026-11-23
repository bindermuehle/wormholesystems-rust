//! "Tell me when one of ours is shot, or about to run dry."
//!
//! Two alerts over the structures a map's members share with it (see
//! [`crate::maps::structures`]). An attack alert repeats what EVE told a member in game:
//! a structure, tower, customs office or skyhook under fire, reinforced or lost. A fuel
//! alert fires a set number of days before an Upwell structure's fuel runs out, and
//! repeats EVE's own low-fuel warnings for the things that have no timer to read.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::delivery::{Embed, Field, Image};
use super::{Alert, AlertKind, Outgoing};
use crate::maps::structures::{MapStructure, StructureEvent, StructureEventKind};
use crate::util::security::ccp_round_security;

/// Somebody is shooting it.
const ATTACK_COLOR: u32 = 0xd7_30_00;
/// Nobody is, yet.
const FUEL_COLOR: u32 = 0xd7_77_00;
/// Nothing is wrong: it anchored, came online or was refuelled.
const ROUTINE_COLOR: u32 = 0x5b_8d_b8;

/// Offer events to a map's structure alerts, and claim the ones that fire. An attack goes
/// to the attack alerts and a fuel warning to the fuel alerts.
pub async fn events(
    pool: &PgPool,
    alerts: &[Alert],
    events: &[StructureEvent],
    public_url: &str,
) -> Vec<Outgoing> {
    let mut out = Vec::new();
    for event in events {
        for alert in alerts.iter().filter(|a| wants(a, event.kind)) {
            let key = format!("structure_event:{}", event.id);
            if !super::claim(pool, alert.id, &key).await {
                continue;
            }
            out.push(Outgoing {
                alert: alert.clone(),
                key,
                embed: event_embed(event, public_url, alert.map_id),
            });
        }
    }
    out
}

/// Whether an alert speaks about this kind of event. What is done to a structure goes to
/// the attack alerts and what its fuel does to the fuel alerts; within that, an alert
/// speaks about the events it was given, or the usual ones when it was given none.
fn wants(alert: &Alert, kind: StructureEventKind) -> bool {
    let category = if kind.is_fuel() {
        AlertKind::StructureFuel
    } else {
        AlertKind::StructureAttack
    };
    alert.kind == category
        && if alert.structure_events.is_empty() {
            kind.by_default()
        } else {
            alert.structure_events.contains(&kind)
        }
}

/// Offer a map's structures to its fuel alerts. One fires once per fill: the key carries
/// when the fuel runs out, so topping it up makes it a new occasion.
pub async fn fuel(
    pool: &PgPool,
    alerts: &[Alert],
    structures: &[MapStructure],
    now: DateTime<Utc>,
    public_url: &str,
) -> Vec<Outgoing> {
    let mut out = Vec::new();
    for alert in alerts.iter().filter(|a| a.kind == AlertKind::StructureFuel) {
        let Some(days) = alert.fuel_days else {
            continue;
        };
        for structure in structures {
            let Some(expires) = structure.fuel_expires else {
                continue;
            };
            if !runs_out_within(expires, now, days) {
                continue;
            }
            let key = format!("fuel:{}:{}", structure.structure_id, expires.timestamp());
            if !super::claim(pool, alert.id, &key).await {
                continue;
            }
            out.push(Outgoing {
                alert: alert.clone(),
                key,
                embed: fuel_embed(structure, expires, now, public_url, alert.map_id),
            });
        }
    }
    out
}

/// Whether the fuel lasts no longer than `days` from `now`. One that already ran out is
/// not news any more: EVE said so at the time, and the alert for that is the event.
fn runs_out_within(expires: DateTime<Utc>, now: DateTime<Utc>, days: i32) -> bool {
    expires > now && expires - now <= chrono::Duration::days(i64::from(days))
}

fn system_field(name: Option<&str>, security: Option<f64>) -> Option<Field> {
    let name = name?;
    Some(Field::inline(
        "System",
        match security {
            Some(security) => format!("{name} ({:.1})", ccp_round_security(security)),
            None => name.to_string(),
        },
    ))
}

fn type_icon(type_id: Option<i64>) -> Option<Image> {
    type_id.map(|id| Image {
        url: format!("https://images.evetech.net/types/{id}/icon?size=64"),
    })
}

/// A moment the way Discord shows it in the reader's own time zone, with how far off it is.
fn discord_time(at: DateTime<Utc>) -> String {
    let unix = at.timestamp();
    format!("<t:{unix}:f> (<t:{unix}:R>)")
}

fn event_embed(event: &StructureEvent, public_url: &str, map_id: i64) -> Embed {
    let label = event.label();
    let happened = match event.kind {
        StructureEventKind::UnderAttack => "is under attack",
        StructureEventKind::LostShields => "lost its shields",
        StructureEventKind::LostArmor => "lost its armor",
        StructureEventKind::Reinforced => "was reinforced",
        StructureEventKind::Destroyed => "was destroyed",
        StructureEventKind::FuelLow => "is running out of fuel",
        StructureEventKind::LowPower => "ran out of fuel",
        StructureEventKind::ReagentsLow => "is running out of reagents",
        StructureEventKind::Unanchoring => "is being unanchored",
        StructureEventKind::Anchoring => "started anchoring",
        StructureEventKind::Online => "came online",
        StructureEventKind::HighPower => "is back at full power",
        StructureEventKind::ServicesOffline => "had its services go offline",
    };
    let place = event
        .system_name
        .as_deref()
        .map(|system| format!(" in **{system}**"))
        .unwrap_or_default();
    let by = match (
        event.aggressor.as_deref(),
        event.aggressor_corporation.as_deref(),
    ) {
        (Some(pilot), Some(corp)) => format!(" by **{pilot}** ({corp})"),
        (Some(name), None) | (None, Some(name)) => format!(" by **{name}**"),
        (None, None) => String::new(),
    };

    let mut fields = Vec::new();
    fields.extend(system_field(event.system_name.as_deref(), event.security));
    if let Some(owner) = event.corporation_name.as_deref() {
        fields.push(Field::inline("Owner", owner));
    }
    // Named apart from its type only when it has a name of its own.
    if let (Some(_), Some(kind)) = (event.structure_name.as_deref(), event.type_name.as_deref()) {
        fields.push(Field::inline("Type", kind));
    }
    if let Some(alliance) = event.aggressor_alliance.as_deref() {
        fields.push(Field::inline("Attacker's alliance", alliance));
    }
    let health: Vec<String> = [
        ("shield", event.shield),
        ("armor", event.armor),
        ("hull", event.hull),
    ]
    .into_iter()
    .filter_map(|(layer, left)| Some(format!("{:.0}% {layer}", left?)))
    .collect();
    if !health.is_empty() {
        fields.push(Field::inline("Health", health.join(", ")));
    }
    if let Some(ends) = event.timer_ends_at {
        fields.push(Field::full_width("Timer ends", discord_time(ends)));
    }

    Embed {
        title: format!("{label} {happened}"),
        url: Some(format!("{public_url}/maps/{map_id}")),
        description: Some(format!("**{label}**{place} {happened}{by}.")),
        color: match event.kind {
            StructureEventKind::Anchoring
            | StructureEventKind::Online
            | StructureEventKind::HighPower => ROUTINE_COLOR,
            kind if kind.is_fuel() => FUEL_COLOR,
            _ => ATTACK_COLOR,
        },
        fields,
        thumbnail: type_icon(event.type_id),
        footer: None,
        timestamp: Some(event.occurred_at.to_rfc3339()),
    }
}

/// "3 days", "1 day", "5 hours": how long the fuel lasts, rounded down.
fn time_left(expires: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let left = expires - now;
    let (count, unit) = match left.num_days() {
        0 => (left.num_hours().max(1), "hour"),
        days => (days, "day"),
    };
    format!("{count} {unit}{}", if count == 1 { "" } else { "s" })
}

fn fuel_embed(
    structure: &MapStructure,
    expires: DateTime<Utc>,
    now: DateTime<Utc>,
    public_url: &str,
    map_id: i64,
) -> Embed {
    let label = structure.label();
    let left = time_left(expires, now);
    let place = structure
        .system_name
        .as_deref()
        .map(|system| format!(" in **{system}**"))
        .unwrap_or_default();

    let mut fields = Vec::new();
    fields.extend(system_field(
        structure.system_name.as_deref(),
        structure.security,
    ));
    if let Some(owner) = structure.corporation_name.as_deref() {
        fields.push(Field::inline("Owner", owner));
    }
    if let (Some(_), Some(kind)) = (structure.name.as_deref(), structure.type_name.as_deref()) {
        fields.push(Field::inline("Type", kind));
    }
    fields.push(Field::full_width("Fuel runs out", discord_time(expires)));

    Embed {
        title: format!("{label} runs out of fuel in {left}"),
        url: Some(format!("{public_url}/maps/{map_id}")),
        description: Some(format!("**{label}**{place} has fuel for {left}.")),
        color: FUEL_COLOR,
        fields,
        thumbnail: type_icon(structure.type_id),
        footer: None,
        timestamp: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: StructureEventKind) -> StructureEvent {
        StructureEvent {
            id: 7,
            kind,
            structure_name: Some("J123456 - Home".into()),
            type_id: Some(35832),
            type_name: Some("Astrahus".into()),
            solar_system_id: Some(31000001),
            system_name: Some("J123456".into()),
            security: Some(-1.0),
            region: None,
            region_id: None,
            constellation_id: None,
            wormhole_class_id: Some(1),
            corporation_id: 98000001,
            corporation_name: Some("Hole Control".into()),
            aggressor: Some("Bad Guy".into()),
            aggressor_corporation: Some("Evictors".into()),
            aggressor_alliance: None,
            shield: Some(94.88),
            armor: Some(100.0),
            hull: Some(100.0),
            timer_ends_at: None,
            occurred_at: Utc::now(),
        }
    }

    fn alert(kind: AlertKind, events: &[StructureEventKind]) -> Alert {
        Alert {
            id: 1,
            map_id: 4,
            created_by_user_id: None,
            name: "Test".into(),
            kind,
            delivery: crate::alerts::AlertDelivery::Webhook,
            webhook_url: None,
            discord_guild_id: None,
            discord_channel_id: None,
            discord_role_id: None,
            mention: crate::alerts::AlertMention::None,
            target_solar_system_id: None,
            origin_solar_system_id: None,
            max_jumps: 0,
            ship_type: None,
            jdc_level: None,
            fuel_days: None,
            structure_events: events.to_vec(),
            filters: Vec::new(),
            filter_match: crate::alerts::filters::Match::Any,
            is_active: true,
        }
    }

    #[test]
    fn an_alert_speaks_about_its_usual_events_until_it_is_given_a_choice() {
        use StructureEventKind as K;
        let attack = alert(AlertKind::StructureAttack, &[]);
        assert!(wants(&attack, K::UnderAttack) && wants(&attack, K::Unanchoring));
        // Routine, so only for whoever asks; and fuel is the other alert's.
        assert!(!wants(&attack, K::Online) && !wants(&attack, K::FuelLow));

        let chosen = alert(AlertKind::StructureAttack, &[K::LostArmor, K::Online]);
        assert!(wants(&chosen, K::LostArmor) && wants(&chosen, K::Online));
        assert!(!wants(&chosen, K::UnderAttack));

        let fuel = alert(AlertKind::StructureFuel, &[]);
        assert!(wants(&fuel, K::FuelLow) && wants(&fuel, K::LowPower));
        assert!(!wants(&fuel, K::HighPower) && !wants(&fuel, K::UnderAttack));
    }

    #[test]
    fn an_attack_says_what_where_and_who() {
        let embed = event_embed(
            &event(StructureEventKind::UnderAttack),
            "https://ws.test",
            4,
        );
        assert_eq!(embed.title, "J123456 - Home is under attack");
        assert_eq!(
            embed.description.as_deref(),
            Some("**J123456 - Home** in **J123456** is under attack by **Bad Guy** (Evictors).")
        );
        assert_eq!(embed.color, ATTACK_COLOR);
        assert!(
            embed
                .fields
                .iter()
                .any(|f| f.value == "95% shield, 100% armor, 100% hull")
        );
        assert!(
            embed
                .fields
                .iter()
                .any(|f| f.name == "Type" && f.value == "Astrahus")
        );
        crate::alerts::delivery::assert_links_absolute(&embed);
    }

    /// A tower has no name of its own, so its type stands in and is not repeated.
    #[test]
    fn a_structure_without_a_name_goes_by_its_type() {
        let mut tower = event(StructureEventKind::FuelLow);
        tower.structure_name = None;
        tower.type_name = Some("Caldari Control Tower".into());
        tower.aggressor = None;
        tower.aggressor_corporation = None;
        let embed = event_embed(&tower, "https://ws.test", 4);
        assert_eq!(embed.title, "Caldari Control Tower is running out of fuel");
        assert_eq!(embed.color, FUEL_COLOR);
        assert!(!embed.fields.iter().any(|f| f.name == "Type"));
    }

    #[test]
    fn fuel_fires_inside_the_window_and_not_once_it_is_out() {
        let now = Utc::now();
        let days = |n| now + chrono::Duration::days(n);
        assert!(runs_out_within(days(2), now, 3));
        assert!(runs_out_within(days(3), now, 3));
        assert!(!runs_out_within(days(4), now, 3));
        assert!(!runs_out_within(days(-1), now, 3));
    }

    #[test]
    fn time_left_rounds_down_and_never_says_zero() {
        let now = Utc::now();
        assert_eq!(time_left(now + chrono::Duration::hours(80), now), "3 days");
        assert_eq!(time_left(now + chrono::Duration::hours(30), now), "1 day");
        assert_eq!(time_left(now + chrono::Duration::hours(5), now), "5 hours");
        assert_eq!(
            time_left(now + chrono::Duration::minutes(20), now),
            "1 hour"
        );
    }
}
