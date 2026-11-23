//! Building the chain from a jump.
//!
//! When a tracked character moves through a wormhole, the client offers the signatures in
//! the system it left and posts the one that was used. That single command places the new
//! system, connects it and links the signature, so a mis-picked signature is one undo
//! rather than three separate ones to unpick.
//!
//! Separate from [`super::jumps::record_transit`], which records the same movement for mass
//! accounting whether or not anyone is looking at the map. The two meet at
//! [`super::jumps::claim_pending_tx`]: a transit recorded before the hole was mapped is
//! claimed by the connection this creates.

use serde::{Deserialize, Serialize};

use super::command::{CommandOutput, Effect, MapCommand, Sequence, Tx};
use super::connection::{AddConnection, SetConnectionStatus, connect};
use super::error::{MapError, Result};
use super::naming::bookmark::{is_abyssal_class, is_wormhole_class};
use super::restore::RemoveRestored;
use super::signatures::{LinkSignature, UnlinkSignature, UpdateSignature};
use super::solar_system::{AddSystem, SetAlias};
use super::{ConnectionType, MassStatus, SignatureGroup, TimeStatus, WormholeSize};
use crate::realtime::MapEvent;
use crate::util::security::ccp_round_security;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct TrackJump {
    pub map_id: i64,
    /// The placement the character jumped *from*; it anchors the new system.
    pub from_map_solar_system_id: i64,
    pub to_solar_system_id: i64,
    /// Where to put the new system. The client owns the layout, so it picks the spot.
    pub x: f64,
    pub y: f64,
    /// The signature that turned out to be this hole. `None` = jumped an unscanned one.
    #[serde(default)]
    #[ts(optional)]
    pub signature_pk: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub alias: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub size: Option<WormholeSize>,
    #[serde(default)]
    #[ts(optional)]
    pub mass_status: Option<MassStatus>,
    #[serde(default)]
    #[ts(optional)]
    pub time_status: Option<TimeStatus>,
}

/// Record a jump: place the system if it is new, connect it, and link the signature.
/// Member+.
pub async fn track_jump(pool: &sqlx::PgPool, actor: super::Actor, cmd: TrackJump) -> Result<()> {
    super::command::run(pool, actor, cmd).await
}

pub(super) async fn apply_track_jump(tx: &mut Tx<'_>, cmd: TrackJump) -> Result<Effect> {
    let from_system = check_origin(tx, &cmd).await?;

    // Noted before anything touches it, so undo can put the signature back as it was.
    let before = match cmd.signature_pk {
        Some(pk) => Some(signature_state(tx, cmd.map_id, pk).await?),
        None => None,
    };

    // The connection this jump went through, if the map has it. One that another signature
    // in this system already is, is not it: the pilot named a different hole, and two holes
    // can lead to the same system, so that jump draws its own edge below.
    let existing = sqlx::query_scalar!(
        r#"select c.id from map_connections c
           join map_solar_systems f on f.id = c.from_system
           join map_solar_systems t on t.id = c.to_system
           where c.map_id = $1
             and ((f.solar_system_id = $2 and t.solar_system_id = $3)
                  or (f.solar_system_id = $3 and t.solar_system_id = $2))
             and not exists (
                 select 1 from signatures s
                 where s.map_id = c.map_id and s.connection_id = c.id
                   and s.solar_system_id = $2 and s.id <> $4)
           order by exists (
                 select 1 from signatures s where s.id = $4 and s.connection_id = c.id) desc,
               c.id
           limit 1"#,
        cmd.map_id,
        from_system,
        cmd.to_solar_system_id,
        cmd.signature_pk,
    )
    .fetch_optional(&mut **tx)
    .await?;

    // Already mapped: the only news is which signature it turned out to be.
    if let Some(connection_id) = existing {
        return link_existing(tx, &cmd, from_system, connection_id, before.as_ref()).await;
    }

    let (target_name, placed) = target_of(tx, &cmd).await?;

    let (to_placement, added, alias_undo) = place_target(tx, &cmd, placed).await?;

    let size = match cmd.size {
        Some(size) => Some(size),
        None => size_between(tx, from_system, cmd.to_solar_system_id).await?,
    };
    let effect = connect(
        tx,
        AddConnection {
            map_id: cmd.map_id,
            from_system: cmd.from_map_solar_system_id,
            to_system: to_placement,
            kind: ConnectionType::Wormhole,
            size,
        },
    )
    .await?;
    let connection = effect.output.take::<super::MapConnection>()?;
    let connection_id = connection.id;

    set_status(tx, &cmd, connection_id).await?;

    if let Some(signature_pk) = cmd.signature_pk {
        link(tx, cmd.map_id, signature_pk, connection_id).await?;
    }

    // Undo walks it back: the signature first, so clearing the link does not fight the
    // connection's own delete, then the edge and the system it brought with it, then the
    // name it gave a system that was already there.
    let mut steps = undo_signature(cmd.map_id, before.as_ref());
    steps.push(MapCommand::RemoveRestored(RemoveRestored {
        map_id: cmd.map_id,
        system_ids: added.into_iter().collect(),
        connection_ids: vec![connection_id],
        signature_ids: Vec::new(),
    }));
    steps.extend(alias_undo);
    let inverse = MapCommand::Sequence(Sequence {
        map_id: cmd.map_id,
        steps,
    });

    let label = match added {
        Some(_) => format!("jumped into {target_name}"),
        None => format!("connected {target_name} by jumping it"),
    };
    let mut events = Vec::new();
    if let Some(id) = added {
        events.push(MapEvent::SystemAdded {
            map_id: cmd.map_id,
            map_solar_system_id: id,
        });
    } else {
        events.push(MapEvent::SystemDetailsChanged {
            map_id: cmd.map_id,
            map_solar_system_id: to_placement,
        });
    }
    events.push(MapEvent::ConnectionChanged {
        map_id: cmd.map_id,
        connection_id,
    });
    if cmd.signature_pk.is_some() {
        events.push(MapEvent::SignatureChanged {
            map_id: cmd.map_id,
            solar_system_id: from_system,
        });
    }
    Ok(Effect::new(
        "tracking.jumped",
        label,
        CommandOutput::Connection(Box::new(connection)),
    )
    .undo_with(inverse)
    .emit_all(events))
}

/// Put the mass and lifetime the pilot saw on the new connection, if they said.
async fn set_status(tx: &mut Tx<'_>, cmd: &TrackJump, connection_id: i64) -> Result<()> {
    if cmd.mass_status.is_some() || cmd.time_status.is_some() {
        super::connection::apply_set_connection_status(
            tx,
            SetConnectionStatus {
                map_id: cmd.map_id,
                connection_id,
                kind: None,
                mass_status: cmd.mass_status.map(Some),
                time_status: cmd.time_status.map(Some),
                size: None,
                preserve_mass: None,
            },
        )
        .await?;
    }
    Ok(())
}

/// The system the jump left, once it is known to be a real jump: a different system, and
/// not one a stargate reaches.
async fn check_origin(tx: &mut Tx<'_>, cmd: &TrackJump) -> Result<i64> {
    // The origin is where the character was, so it is always a real system; a ghost there
    // means the placement changed under the jump.
    let from_system = sqlx::query_scalar!(
        "select solar_system_id from map_solar_systems where id = $1 and map_id = $2",
        cmd.from_map_solar_system_id,
        cmd.map_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .flatten()
    .ok_or(MapError::NotFound)?;

    if from_system == cmd.to_solar_system_id {
        return Err(MapError::Validation("a jump must change system".into()));
    }

    // Gate travel never builds a wormhole. The client checks too, off a cached stargate
    // graph, but this is the copy that decides.
    let is_gate = sqlx::query_scalar!(
        "select exists(
             select 1 from stargates
             where solar_system_id = $1 and destination_system_id = $2
         )",
        from_system,
        cmd.to_solar_system_id,
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if is_gate {
        return Err(MapError::Conflict(
            "those systems are connected by a stargate".into(),
        ));
    }
    Ok(from_system)
}

/// One end of a hole, as far as its size can be read off it.
struct End {
    solar_system_id: i64,
    class: Option<i32>,
    security: f64,
}

/// The size a hole between two systems has to be, when where it leads settles it: nothing
/// bigger than a frigate reaches a C13, nothing bigger than a battlecruiser reaches a C1,
/// and the same goes for Thera to highsec and Turnur to wormhole space. The pilot's own
/// pick and the signature's type come first; this is for the jump that said neither.
/// Mirrors `heuristicSize` in the frontend, which does the same for a hand-drawn line.
fn size_from_ends(a: &End, b: &End) -> Option<WormholeSize> {
    const TURNUR: i64 = 30002086;
    const THERA_CLASS: i32 = 12;
    const HIGHSEC_CLASS: i32 = 7;

    let classes = [a.class, b.class];
    if classes.contains(&Some(13)) {
        return Some(WormholeSize::Small);
    }
    if classes.contains(&Some(1)) {
        return Some(WormholeSize::Medium);
    }
    let highsec = |e: &End| e.class == Some(HIGHSEC_CLASS) || ccp_round_security(e.security) >= 0.5;
    let thera = |e: &End| e.class == Some(THERA_CLASS);
    let turnur = |e: &End| e.solar_system_id == TURNUR;
    let wormhole = |e: &End| is_wormhole_class(e.class);
    let either =
        |x: &dyn Fn(&End) -> bool, y: &dyn Fn(&End) -> bool| (x(a) && y(b)) || (x(b) && y(a));
    (either(&thera, &highsec) || either(&turnur, &wormhole)).then_some(WormholeSize::Medium)
}

async fn size_between(tx: &mut Tx<'_>, from: i64, to: i64) -> Result<Option<WormholeSize>> {
    let rows = sqlx::query!(
        "select id, wormhole_class_id, security_status from solar_systems where id = any($1)",
        &[from, to],
    )
    .fetch_all(&mut **tx)
    .await?;
    let ends: Vec<End> = rows
        .into_iter()
        .map(|r| End {
            solar_system_id: r.id,
            class: r.wormhole_class_id,
            security: r.security_status,
        })
        .collect();
    Ok(match ends.as_slice() {
        [a, b] => size_from_ends(a, b),
        _ => None,
    })
}

/// A jump through a connection the map already has: link the signature it came through.
async fn link_existing(
    tx: &mut Tx<'_>,
    cmd: &TrackJump,
    from_system: i64,
    connection_id: i64,
    before: Option<&SignatureState>,
) -> Result<Effect> {
    let Some(signature_pk) = cmd.signature_pk else {
        return Err(MapError::Conflict(
            "that connection is already on the map".into(),
        ));
    };
    link(tx, cmd.map_id, signature_pk, connection_id).await?;
    Ok(Effect::new(
        "tracking.linked",
        "matched a jump to a signature",
        CommandOutput::None,
    )
    .undo_with(MapCommand::Sequence(Sequence {
        map_id: cmd.map_id,
        steps: undo_signature(cmd.map_id, before),
    }))
    .emit(MapEvent::SignatureChanged {
        map_id: cmd.map_id,
        solar_system_id: from_system,
    })
    .emit(MapEvent::ConnectionChanged {
        map_id: cmd.map_id,
        connection_id,
    }))
}

/// The destination's name and its placement, if the map has one. Refuses abyssal space and
/// a destination the map keeps off.
async fn target_of(tx: &mut Tx<'_>, cmd: &TrackJump) -> Result<(String, Option<i64>)> {
    let target = sqlx::query!(
        "select name, wormhole_class_id from solar_systems where id = $1",
        cmd.to_solar_system_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        MapError::Validation(format!("unknown solar system {}", cmd.to_solar_system_id))
    })?;
    let target_name = target.name;
    // A filament, not a hole. The clients drop these themselves; this is the copy that
    // decides.
    if is_abyssal_class(target.wormhole_class_id) {
        return Err(MapError::Validation(format!(
            "{target_name} is abyssal space"
        )));
    }

    let placed = sqlx::query_scalar!(
        "select id from map_solar_systems where map_id = $1 and solar_system_id = $2",
        cmd.map_id,
        cmd.to_solar_system_id,
    )
    .fetch_optional(&mut **tx)
    .await?;

    // The client checks too, but this is the copy that decides: a pilot passing through
    // a trade hub the map keeps off does not put the hub on the chain. Already placed, it
    // is on the map by somebody's choice and stays reachable.
    if placed.is_none()
        && super::ignored::is_ignored_tx(tx, cmd.map_id, cmd.to_solar_system_id).await?
    {
        return Err(MapError::Validation(format!(
            "{target_name} is on this map's ignore list"
        )));
    }
    Ok((target_name, placed))
}

/// The destination's placement, placing it if it is new. Returns the placement, the id
/// again when this added it, and the undo of any rename.
async fn place_target(
    tx: &mut Tx<'_>,
    cmd: &TrackJump,
    placed: Option<i64>,
) -> Result<(i64, Option<i64>, Option<MapCommand>)> {
    // A system reached by another route is linked to, not duplicated. Renaming it on the
    // way is part of the jump, so undoing the jump puts the old name back.
    if let Some(id) = placed {
        let mut alias_undo = None;
        if let Some(alias) = cmd.alias.clone() {
            let renamed = super::solar_system::apply_set_alias(
                tx,
                SetAlias {
                    map_id: cmd.map_id,
                    map_solar_system_id: id,
                    alias: Some(alias),
                },
            )
            .await?;
            alias_undo = renamed.inverse;
        }
        return Ok((id, None, alias_undo));
    }
    let effect = super::solar_system::apply_add_system(
        tx,
        AddSystem {
            map_id: cmd.map_id,
            solar_system_id: cmd.to_solar_system_id,
            x: cmd.x,
            y: cmd.y,
            alias: cmd.alias.clone(),
        },
    )
    .await?;
    let system = effect.output.take::<super::MapSolarSystem>()?;
    Ok((system.id, Some(system.id), None))
}

/// What the signature looked like before the jump touched it.
#[derive(Debug, Clone)]
pub(super) struct SignatureState {
    pk: i64,
    group: SignatureGroup,
    signature_type_id: Option<i64>,
    connection_id: Option<i64>,
}

pub(super) async fn signature_state(
    tx: &mut Tx<'_>,
    map_id: i64,
    pk: i64,
) -> Result<SignatureState> {
    let row = sqlx::query!(
        r#"select "group", signature_type_id, connection_id
           from signatures where id = $1 and map_id = $2"#,
        pk,
        map_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(MapError::NotFound)?;
    Ok(SignatureState {
        pk,
        group: row.group,
        signature_type_id: row.signature_type_id,
        connection_id: row.connection_id,
    })
}

/// Put the signature back to how it was scanned. Deliberately not `RestoreSignatures`: that
/// undoes a deletion, so its own inverse deletes the rows again and redoing a jump would
/// take the signature with it.
pub(super) fn undo_signature(map_id: i64, before: Option<&SignatureState>) -> Vec<MapCommand> {
    let Some(before) = before else {
        return Vec::new();
    };
    // Group first: dropping out of `wormhole` clears the link on its way past, so the
    // link/unlink has to be the last word on it.
    let mut steps = vec![MapCommand::UpdateSignature(UpdateSignature {
        map_id,
        signature_pk: before.pk,
        signature_id: None,
        group: Some(before.group),
        signature_type_id: Some(before.signature_type_id),
        name: None,
        size: None,
        mass_status: None,
        time_status: None,
    })];
    steps.push(match before.connection_id {
        Some(connection_id) => MapCommand::LinkSignature(LinkSignature {
            map_id,
            signature_pk: before.pk,
            connection_id,
        }),
        None => MapCommand::UnlinkSignature(UnlinkSignature {
            map_id,
            signature_pk: before.pk,
        }),
    });
    steps
}

/// Link a signature to the connection, promoting it to a wormhole first. A jumped signature
/// is usually still `unknown`: the id was scanned but never classified.
pub(super) async fn link(
    tx: &mut Tx<'_>,
    map_id: i64,
    signature_pk: i64,
    connection_id: i64,
) -> Result<()> {
    sqlx::query!(
        r#"update signatures set "group" = $1, updated_at = now()
           where id = $2 and map_id = $3 and "group" = 'unknown'"#,
        SignatureGroup::Wormhole,
        signature_pk,
        map_id,
    )
    .execute(&mut **tx)
    .await?;

    super::signatures::apply_link_signature(
        tx,
        LinkSignature {
            map_id,
            signature_pk,
            connection_id,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end(solar_system_id: i64, class: Option<i32>, security: f64) -> End {
        End {
            solar_system_id,
            class,
            security,
        }
    }

    #[test]
    fn thera_to_highsec_and_turnur_to_wormhole_space_are_medium() {
        let thera = end(31000005, Some(12), -0.99);
        let turnur = end(30002086, Some(8), 0.4);
        let highsec = end(30000142, Some(7), 0.95);
        let lowsec = end(30002087, Some(8), 0.3);
        let c5 = end(31002000, Some(5), -1.0);

        assert_eq!(size_from_ends(&thera, &highsec), Some(WormholeSize::Medium));
        assert_eq!(size_from_ends(&highsec, &thera), Some(WormholeSize::Medium));
        assert_eq!(size_from_ends(&thera, &lowsec), None);
        assert_eq!(size_from_ends(&turnur, &c5), Some(WormholeSize::Medium));
        assert_eq!(size_from_ends(&turnur, &lowsec), None);
        assert_eq!(size_from_ends(&c5, &highsec), None);
    }
}
