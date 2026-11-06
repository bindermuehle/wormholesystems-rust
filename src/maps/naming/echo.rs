//! The echo strategy: `<branch><type><slot>`, e.g. `a5s`. Ours, not upstream's.
//!
//! Where the chain extends the parent's alias and so grows with depth, echo stays three
//! characters at any depth and says what a hole leads to:
//!
//! - **branch**: a letter per direct link off home, inherited by everything down that
//!   branch. A new link takes the lowest free letter; a letter is reused once its branch
//!   dies. Home's statics own the first letters whether or not they are scanned yet.
//! - **type**: the destination's class: `1`-`6`, `H`/`L`/`N`/`P` for k-space and Pochven,
//!   `T` for Thera, `D` for any drifter hole and `13` for shattered.
//! - **slot**: the lowest free letter making `<branch><type>` unique, skipping `s`. Home's
//!   own static is the exception and always takes `s`.
//!
//! Shattered is the one two-character type, so its aliases run to four (`a13a`). A slot is
//! always a letter, so `a13a` cannot be mistaken for a C1 in slot `3`.
//!
//! Echo needs more of the map than the graph: which hole is home's static depends on the
//! scans in home and on home's statics. Those are [`EchoFacts`], loaded only for a map that
//! uses echo. Bookmarks are the siblings strategy's: an echo alias says nothing about which
//! way home is, so the map's distances decide it.

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::bookmark::Hole;
use super::siblings::Siblings;
use super::{CLAIMS, Destination, NamingMap, NamingStrategy, Node, Target, load, signature_type};
use crate::maps::access::Reader;
use crate::maps::error::{MapError, Result};
use crate::maps::map::MapNaming;
use crate::maps::solar_system::SetAlias;
use crate::maps::{Actor, NamingStrategyKind};

const BRANCH_LETTERS: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Clone, Copy, Default)]
pub struct Echo;

/// What echo reads that the other strategies do not.
#[derive(Debug, Clone, Default)]
pub struct EchoFacts {
    /// The wormhole codes of home's statics.
    pub home_statics: Vec<String>,
    /// Every wormhole scan on the map.
    pub scans: Vec<Scan>,
}

/// A wormhole signature, as echo reads it.
#[derive(Debug, Clone)]
pub struct Scan {
    /// `signatures.id`.
    pub pk: i64,
    pub solar_system_id: i64,
    /// The in-game id, `ABC-123`.
    pub signature_id: String,
    /// The wormhole code, `H296`, once the type is picked.
    pub code: Option<String>,
    /// The class the type leads to, once the type is picked.
    pub target_class: Option<i32>,
    pub connection_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl EchoFacts {
    /// The scan `signature_id` in `solar_system_id`, by its key.
    pub fn scan_in(&self, solar_system_id: i64, signature_id: &str) -> Option<i64> {
        self.scans
            .iter()
            .find(|s| s.solar_system_id == solar_system_id && s.signature_id == signature_id)
            .map(|s| s.pk)
    }

    fn scan(&self, pk: i64) -> Option<&Scan> {
        self.scans.iter().find(|s| s.pk == pk)
    }
}

impl NamingStrategy for Echo {
    fn alias(&self, map: &NamingMap, origin: &Node, target: &Target) -> Option<String> {
        suggest(map, origin, target, None, &[])
    }

    fn bookmark(
        &self,
        map: &NamingMap,
        destination: &Destination,
        hole: &Hole,
        near: Option<&Node>,
    ) -> String {
        Siblings.bookmark(map, destination, hole, near)
    }

    fn example_aliases(&self, _settings: &MapNaming) -> Vec<String> {
        // Nothing to walk without a map under it: a worked example says more. Home's
        // static, the next hole of that class down its branch, a C3 off the second branch,
        // and a drifter hole.
        ["a5s", "a5a", "b3a", "aDa"].map(String::from).to_vec()
    }
}

/// The alias for a system reached from `origin`, or `None` when echo does not name it: a
/// class it has no character for, a non-home origin with no alias to inherit a branch from,
/// or a branch and type whose 25 slots are taken.
///
/// `skip` is a node to leave out of everything, as if it were not on the map: naming a
/// hole again must not read its own name as taken. `extra` are names already handed out
/// that the map does not carry yet.
fn suggest(
    map: &NamingMap,
    origin: &Node,
    target: &Target,
    skip: Option<i64>,
    extra: &[String],
) -> Option<String> {
    let kind = type_char(target.class)?;
    let home = map.nodes().iter().find(|n| n.is_home);
    let branch = match home {
        Some(home) if home.id == origin.id => {
            let used = home_branch_letters(map, home.id, skip);
            if is_home_static(map, origin, target.signature) {
                // The static takes the lowest free letter, so the primary static reads `a…s`.
                return Some(format!("{}{kind}s", next_branch_letter(&used)));
            }
            let statics = map.echo().home_statics.len().min(BRANCH_LETTERS.len());
            let mut used = used;
            used.extend(BRANCH_LETTERS[..statics].iter().map(|&b| char::from(b)));
            next_branch_letter(&used)
        }
        _ => branch_of(origin.alias.as_deref())?,
    };
    let mut aliases: Vec<String> = map
        .nodes()
        .iter()
        .filter(|n| Some(n.id) != skip)
        .filter_map(|n| n.alias.clone())
        .collect();
    aliases.extend_from_slice(extra);
    let slot = next_slot_letter(branch, &kind, &aliases)?;
    Some(format!("{branch}{kind}{slot}"))
}

/// The echo character for a destination class, or `None` for one it does not name
/// (abyssal C19-C23, and anything with no class).
///
/// All five drifter classes share `D`: the destination is one click away on the map, so
/// telling Sentinel from Vidette by name is not worth four more letters. They share one slot
/// sequence too. Shattered keeps its number, because `S` would read as the static slot.
fn type_char(class: Option<i32>) -> Option<String> {
    let c = match class? {
        n @ 1..=6 => return Some(n.to_string()),
        13 => return Some("13".into()),
        7 => 'H',
        8 => 'L',
        9 => 'N',
        12 => 'T',
        14..=18 => 'D',
        25 => 'P',
        _ => return None,
    };
    Some(c.to_string())
}

/// An alias as written, without the `+` that marks the way home in the signature list.
fn bare(alias: &str) -> &str {
    let alias = alias.trim();
    alias.strip_prefix('+').unwrap_or(alias)
}

/// The branch letter an alias carries: its first character, lower-cased.
fn branch_of(alias: Option<&str>) -> Option<char> {
    bare(alias?).chars().next().map(|c| c.to_ascii_lowercase())
}

/// The branch letters home's links already carry, read off the names of its neighbours.
fn home_branch_letters(map: &NamingMap, home: i64, skip: Option<i64>) -> Vec<char> {
    map.neighbours(home)
        .filter(|n| Some(n.id) != skip)
        .filter_map(|n| branch_of(n.alias.as_deref()))
        .collect()
}

/// The lowest branch letter not in `used`, ignoring case; `z` once all are gone.
fn next_branch_letter(used: &[char]) -> char {
    let taken: HashSet<char> = used.iter().map(char::to_ascii_lowercase).collect();
    BRANCH_LETTERS
        .iter()
        .map(|&b| char::from(b))
        .find(|c| !taken.contains(c))
        .unwrap_or('z')
}

/// The lowest free slot for `<branch><kind>` among `aliases`, never `s`.
fn next_slot_letter(branch: char, kind: &str, aliases: &[String]) -> Option<char> {
    let prefix = format!("{branch}{kind}");
    let used: HashSet<char> = aliases
        .iter()
        .map(|a| bare(a))
        .filter(|a| a.len() > prefix.len() && a.starts_with(&prefix))
        .filter_map(|a| a[prefix.len()..].chars().next())
        .collect();
    BRANCH_LETTERS
        .iter()
        .map(|&b| char::from(b))
        .find(|&c| c != 's' && !used.contains(&c))
}

/// Whether the hole `signature`, scanned in home, is home's static.
///
/// Nothing in the data says which of two H296s in a C5 with an H296 static is the static:
/// a wandering H296 looks the same. The earliest scanned wins, ties going to the lower id,
/// and the scout renames it if that turns out wrong. Two holes sharing the `s` slot would be
/// worse.
fn is_home_static(map: &NamingMap, home: &Node, signature: Option<i64>) -> bool {
    let facts = map.echo();
    let (Some(scan), Some(system)) = (signature.and_then(|pk| facts.scan(pk)), &home.system) else {
        return false;
    };
    let Some(code) = scan.code.as_deref() else {
        return false;
    };
    if scan.solar_system_id != system.solar_system_id
        || !facts.home_statics.iter().any(|s| s == code)
    {
        return false;
    }
    facts
        .scans
        .iter()
        .filter(|s| s.solar_system_id == system.solar_system_id && s.code.as_deref() == Some(code))
        .min_by_key(|s| (s.created_at, s.pk))
        .is_some_and(|first| first.pk == scan.pk)
}

/// The facts for map `map_id`, whose graph is `map`.
pub(super) async fn load_facts(pool: &PgPool, map_id: i64, map: &NamingMap) -> Result<EchoFacts> {
    let home_system = map
        .nodes()
        .iter()
        .find(|n| n.is_home)
        .and_then(|n| n.system.as_ref())
        .map(|s| s.solar_system_id);
    let home_statics = match home_system {
        Some(id) => {
            sqlx::query_scalar!(
                "select wormhole_code from wormhole_system_statics where solar_system_id = $1",
                id,
            )
            .fetch_all(pool)
            .await?
        }
        None => Vec::new(),
    };
    let scans = sqlx::query!(
        r#"select s.id, s.solar_system_id, s.signature_id, st.signature as "code?",
                  st.target_class as "target_class?", s.connection_id, s.created_at
           from signatures s
           left join signature_types st on st.id = s.signature_type_id
           where s.map_id = $1 and s."group" = 'wormhole'"#,
        map_id,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| Scan {
        pk: row.id,
        solar_system_id: row.solar_system_id,
        signature_id: row.signature_id,
        code: row.code,
        target_class: row.target_class,
        connection_id: row.connection_id,
        created_at: row.created_at,
    })
    .collect();
    Ok(EchoFacts {
        home_statics,
        scans,
    })
}

/// What the signature list shows for one wormhole row.
#[derive(Debug, Clone, Serialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct EchoRowAlias {
    /// `signatures.id`.
    pub signature: i64,
    /// The name, without the homeward marker.
    pub alias: String,
    /// This hole is the way back toward home.
    pub homeward: bool,
    /// The far side is on the map and carries this name; otherwise it is a suggestion.
    pub settled: bool,
}

/// The name of every wormhole row scanned in the system at node `system`.
///
/// Worked out for the whole list in one pass, in scan order: each suggestion takes a slot,
/// so two unscanned C5s off one branch are `a5a` and `a5b`, where rows asking one at a time
/// would both get `a5a`. Scan order, not display order, so sorting the list never moves a
/// name. A map that does not use echo gets nothing.
pub async fn rows(
    pool: &PgPool,
    reader: Reader,
    map_id: i64,
    system: i64,
) -> Result<Vec<EchoRowAlias>> {
    let map = load(pool, reader, map_id).await?;
    if map.settings.strategy != NamingStrategyKind::Echo {
        return Ok(Vec::new());
    }
    let origin = map.node(system).ok_or(MapError::NotFound)?;
    Ok(row_aliases(&map, origin))
}

fn row_aliases(map: &NamingMap, origin: &Node) -> Vec<EchoRowAlias> {
    let Some(here) = origin.system.as_ref().map(|s| s.solar_system_id) else {
        return Vec::new();
    };
    let toward_home = parent_toward_home(map, origin.id);
    let homeward = |far: i64| toward_home == Some(far);
    let mut scans: Vec<&Scan> = map
        .echo()
        .scans
        .iter()
        .filter(|s| s.solar_system_id == here)
        .collect();
    scans.sort_by_key(|s| (s.created_at, s.pk));

    let mut out = Vec::new();
    let mut handed_out: Vec<String> = Vec::new();
    for scan in scans {
        let far = scan
            .connection_id
            .and_then(|id| map.edge(id))
            .map(|e| if e.from == origin.id { e.to } else { e.from })
            .and_then(|id| map.node(id));

        // The way back is named after the hole you came in through, which is this system:
        // from `a5s`, the hole home is the `a5s` hole. Naming it after where it leads would
        // say `HOME` on every branch and tell a scout nothing.
        if let (Some(far), Some(alias)) = (far, non_blank(origin.alias.as_deref()))
            && homeward(far.id)
        {
            out.push(EchoRowAlias {
                signature: scan.pk,
                alias: alias.to_string(),
                homeward: true,
                settled: true,
            });
            continue;
        }
        // A destination already named keeps its name.
        if let Some(alias) = far.and_then(|f| non_blank(f.alias.as_deref())) {
            out.push(EchoRowAlias {
                signature: scan.pk,
                alias: alias.to_string(),
                homeward: false,
                settled: true,
            });
            continue;
        }
        // Everything else gets a suggestion, a ghost included: it is linked from the moment
        // it is scanned but has no name until someone gives it one. A real system's class is
        // the truth; a ghost has none, so the type says what it promises.
        let class = far
            .and_then(|f| f.system.as_ref())
            .and_then(|s| s.class)
            .or(scan.target_class);
        let target = Target {
            class,
            security: None,
            signature: Some(scan.pk),
        };
        let Some(alias) = suggest(map, origin, &target, None, &handed_out) else {
            continue;
        };
        out.push(EchoRowAlias {
            signature: scan.pk,
            alias: alias.clone(),
            homeward: far.is_some_and(|f| homeward(f.id)),
            settled: false,
        });
        handed_out.push(alias);
    }
    out
}

/// The neighbour one hop closer to home: `node`'s parent in a breadth-first walk from the
/// map's home system. The hole to it is the way back, which the signature list marks with
/// a `+`. One parent, so a loop in the chain still marks a single hole. `None` with no home,
/// at home, or with no path to it.
fn parent_toward_home(map: &NamingMap, node: i64) -> Option<i64> {
    let home = map.nodes().iter().find(|n| n.is_home)?.id;
    let mut parent: HashMap<i64, i64> = HashMap::new();
    let mut seen: HashSet<i64> = HashSet::from([home]);
    let mut queue = VecDeque::from([home]);
    while let Some(current) = queue.pop_front() {
        if current == node {
            return parent.get(&current).copied();
        }
        // By id, so the answer does not depend on the order the edges were read in.
        let mut next: Vec<i64> = map.neighbours(current).map(|n| n.id).collect();
        next.sort_unstable();
        for id in next {
            if seen.insert(id) {
                parent.insert(id, current);
                queue.push_back(id);
            }
        }
    }
    None
}

/// Name the ghost a scan leads to, now that its type says where.
#[derive(Debug, Clone, Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct NameGhost {
    /// `signatures.id`.
    pub signature: i64,
    /// The type just picked. The pick and this request go out together, so the server may
    /// not have saved the type yet; this says what it is.
    #[serde(default)]
    #[ts(optional)]
    pub signature_type_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS, utoipa::ToSchema)]
#[ts(export)]
pub struct NamedGhost {
    /// The name written, or `None` when nothing was.
    pub alias: Option<String>,
}

/// Give the ghost that `cmd.signature` leads to the name echo has for it, and write it.
///
/// Only an unnamed ghost, or one carrying a name echo wrote, is touched: a name somebody
/// typed was meant. Nothing records which is which, so the shape decides. The ghost itself
/// is left out of the question, so a C6 retyped to another C6 keeps `c6a` instead of
/// reading its own name as taken and moving on.
pub async fn name_ghost(
    pool: &PgPool,
    reader: Reader,
    actor: Actor,
    map_id: i64,
    cmd: NameGhost,
) -> Result<NamedGhost> {
    let _claiming = CLAIMS.lock().await;
    let mut map = load(pool, reader, map_id).await?;
    if map.settings.strategy != NamingStrategyKind::Echo {
        return Ok(NamedGhost { alias: None });
    }
    if let Some(type_id) = cmd.signature_type_id {
        let (code, class) = signature_type(pool, Some(type_id)).await?;
        let mut facts = map.echo().clone();
        let picked = facts
            .scans
            .iter_mut()
            .find(|s| s.pk == cmd.signature)
            .ok_or(MapError::NotFound)?;
        picked.code = code;
        picked.target_class = class;
        map = map.with_echo(facts);
    }
    let scan = map.echo().scan(cmd.signature).ok_or(MapError::NotFound)?;
    let Some(origin) = map.node_of_system(scan.solar_system_id) else {
        return Ok(NamedGhost { alias: None });
    };
    let Some(ghost) = scan
        .connection_id
        .and_then(|id| map.edge(id))
        .map(|e| if e.from == origin.id { e.to } else { e.from })
        .and_then(|id| map.node(id))
        .filter(|n| n.system.is_none())
    else {
        return Ok(NamedGhost { alias: None });
    };
    let existing = non_blank(ghost.alias.as_deref());
    if existing.is_some_and(|a| !is_generated(a)) {
        return Ok(NamedGhost { alias: None });
    }
    let target = Target {
        class: scan.target_class,
        security: None,
        signature: Some(scan.pk),
    };
    let Some(alias) = suggest(&map, origin, &target, Some(ghost.id), &[]) else {
        return Ok(NamedGhost { alias: None });
    };
    if Some(alias.as_str()) == existing {
        return Ok(NamedGhost { alias: Some(alias) });
    }
    crate::maps::run(
        pool,
        actor,
        SetAlias {
            map_id,
            map_solar_system_id: ghost.id,
            alias: Some(alias.clone()),
        },
    )
    .await?;
    Ok(NamedGhost { alias: Some(alias) })
}

fn non_blank(alias: Option<&str>) -> Option<&str> {
    alias.map(str::trim).filter(|a| !a.is_empty())
}

/// Whether `alias` has the shape echo writes: a branch letter, a class of one or two digits
/// or one of `HLNPTD`, and a slot letter.
fn is_generated(alias: &str) -> bool {
    let b = alias.trim().as_bytes();
    let lower = |c: u8| c.is_ascii_lowercase();
    match b {
        [branch, kind, slot] => {
            lower(*branch) && (kind.is_ascii_digit() || b"HLNPTD".contains(kind)) && lower(*slot)
        }
        [branch, d1, d2, slot] => {
            lower(*branch) && d1.is_ascii_digit() && d2.is_ascii_digit() && lower(*slot)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::maps::AliasScheme;
    use crate::maps::naming::{Edge, NodeSystem};

    const HOME_SYSTEM: i64 = 31_000_001;

    fn settings() -> MapNaming {
        MapNaming {
            strategy: NamingStrategyKind::Echo,
            alias_scheme: AliasScheme::Numeric,
            ignored_alias: String::new(),
            bookmark_wormhole: String::new(),
            bookmark_kspace: String::new(),
            bookmark_return: String::new(),
        }
    }

    fn system(id: i64, class: i32) -> NodeSystem {
        NodeSystem {
            solar_system_id: id,
            name: format!("J{id}"),
            region: String::new(),
            class: Some(class),
            security: -1.0,
            occupier: None,
        }
    }

    fn home() -> Node {
        Node {
            id: 1,
            alias: None,
            is_home: true,
            system: Some(system(HOME_SYSTEM, 5)),
        }
    }

    fn node(id: i64, alias: &str) -> Node {
        Node {
            id,
            alias: Some(alias.into()),
            is_home: false,
            system: Some(system(31_000_000 + id, 3)),
        }
    }

    fn ghost(id: i64, alias: Option<&str>) -> Node {
        Node {
            id,
            alias: alias.map(Into::into),
            is_home: false,
            system: None,
        }
    }

    fn edge(id: i64, from: i64, to: i64) -> Edge {
        Edge {
            id,
            from,
            to,
            size: None,
            mass_status: None,
            time_status: None,
        }
    }

    fn scan(pk: i64, code: &str, class: i32, minute: u32, connection: Option<i64>) -> Scan {
        Scan {
            pk,
            solar_system_id: HOME_SYSTEM,
            signature_id: format!("SIG-{pk:03}"),
            code: Some(code.into()),
            target_class: Some(class),
            connection_id: connection,
            created_at: Utc.with_ymd_and_hms(2026, 11, 6, 12, minute, 0).unwrap(),
        }
    }

    fn map(nodes: Vec<Node>, edges: Vec<Edge>, statics: &[&str], scans: Vec<Scan>) -> NamingMap {
        NamingMap::new(settings(), nodes, edges).with_echo(EchoFacts {
            home_statics: statics.iter().map(ToString::to_string).collect(),
            scans,
        })
    }

    fn wormhole(class: i32, signature: Option<i64>) -> Target {
        Target {
            class: Some(class),
            security: None,
            signature,
        }
    }

    fn list(aliases: &[&str]) -> Vec<String> {
        aliases.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn type_chars() {
        assert_eq!(type_char(Some(5)).as_deref(), Some("5"));
        assert_eq!(type_char(Some(7)).as_deref(), Some("H"));
        assert_eq!(type_char(Some(8)).as_deref(), Some("L"));
        assert_eq!(type_char(Some(9)).as_deref(), Some("N"));
        assert_eq!(type_char(Some(25)).as_deref(), Some("P"));
        assert_eq!(type_char(Some(12)).as_deref(), Some("T"));
        for drifter in 14..=18 {
            assert_eq!(type_char(Some(drifter)).as_deref(), Some("D"));
        }
        // Shattered keeps its number: `S` would read as the static slot.
        assert_eq!(type_char(Some(13)).as_deref(), Some("13"));
        for abyssal in 19..=23 {
            assert_eq!(type_char(Some(abyssal)), None);
        }
        assert_eq!(type_char(None), None);
    }

    #[test]
    fn branch_letters_fill_gaps_and_ignore_case() {
        assert_eq!(next_branch_letter(&[]), 'a');
        assert_eq!(next_branch_letter(&['a', 'c']), 'b');
        assert_eq!(next_branch_letter(&['A', 'B']), 'c');
    }

    #[test]
    fn slots_skip_s_and_count_only_their_own_prefix() {
        assert_eq!(next_slot_letter('a', "5", &[]), Some('a'));
        let r_taken: Vec<String> = ('a'..='r').map(|c| format!("a5{c}")).collect();
        assert_eq!(next_slot_letter('a', "5", &r_taken), Some('t'));
        assert_eq!(
            next_slot_letter('a', "5", &list(&["b5a", "a3a"])),
            Some('a')
        );
        assert_eq!(next_slot_letter('a', "5", &list(&["+a5a"])), Some('b'));
        // `a13a` is shattered, not a C1 in slot `3`.
        assert_eq!(next_slot_letter('a', "1", &list(&["a13a"])), Some('a'));
        let all: Vec<String> = ('a'..='z').map(|c| format!("a5{c}")).collect();
        assert_eq!(next_slot_letter('a', "5", &all), None);
    }

    #[test]
    fn home_static_takes_the_s_slot() {
        let m = map(
            vec![home()],
            vec![],
            &["H296"],
            vec![scan(10, "H296", 5, 0, None)],
        );
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(5, Some(10))).as_deref(),
            Some("a5s")
        );
    }

    #[test]
    fn a_wandering_hole_off_home_starts_past_the_static_branches() {
        let m = map(vec![home()], vec![], &["H296"], vec![]);
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(3, None)).as_deref(),
            Some("b3a")
        );
    }

    #[test]
    fn the_first_of_two_holes_with_the_static_code_is_the_static() {
        let scans = vec![scan(10, "H296", 5, 5, None), scan(11, "H296", 5, 1, None)];
        let m = map(vec![home()], vec![], &["H296"], scans);
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(5, Some(11))).as_deref(),
            Some("a5s")
        );
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(5, Some(10))).as_deref(),
            Some("b5a"),
            "the later one is a wandering hole off home"
        );
    }

    #[test]
    fn a_tie_on_scan_time_goes_to_the_lower_id() {
        let scans = vec![scan(11, "H296", 5, 0, None), scan(10, "H296", 5, 0, None)];
        let m = map(vec![home()], vec![], &["H296"], scans);
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(5, Some(10))).as_deref(),
            Some("a5s")
        );
    }

    #[test]
    fn a_hole_whose_code_is_not_a_static_is_not_one() {
        let m = map(
            vec![home()],
            vec![],
            &["H296"],
            vec![scan(10, "V753", 6, 0, None)],
        );
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(6, Some(10))).as_deref(),
            Some("b6a")
        );
    }

    #[test]
    fn branches_come_from_homes_links_whichever_way_the_edge_points() {
        let nodes = vec![home(), node(2, "a5s"), node(3, "b3a")];
        let edges = vec![edge(1, 1, 2), edge(2, 3, 1)];
        let m = map(nodes, edges, &["H296"], vec![]);
        assert_eq!(
            Echo.alias(&m, &home(), &wormhole(4, None)).as_deref(),
            Some("c4a")
        );
    }

    #[test]
    fn a_hole_inherits_its_branch_at_any_depth() {
        let nodes = vec![home(), node(2, "a5s"), node(3, "a3a")];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3)];
        let m = map(nodes, edges, &["H296"], vec![]);
        let deep = m.node(3).unwrap().clone();
        assert_eq!(
            Echo.alias(&m, &deep, &wormhole(3, None)).as_deref(),
            Some("a3b")
        );
        assert_eq!(
            Echo.alias(&m, &deep, &wormhole(13, None)).as_deref(),
            Some("a13a")
        );
        assert_eq!(
            Echo.alias(&m, &deep, &wormhole(16, None)).as_deref(),
            Some("aDa")
        );
    }

    #[test]
    fn nothing_without_a_branch_or_a_class() {
        let m = map(vec![home(), ghost(2, None)], vec![], &[], vec![]);
        let unnamed = m.node(2).unwrap().clone();
        assert_eq!(Echo.alias(&m, &unnamed, &wormhole(5, None)), None);
        assert_eq!(Echo.alias(&m, &home(), &wormhole(20, None)), None);
        // A k-space hop from an unnamed system is travel, not part of the chain.
        let kspace = Target {
            class: Some(7),
            security: Some(0.9),
            signature: None,
        };
        assert_eq!(Echo.alias(&m, &unnamed, &kspace), None);
    }

    #[test]
    fn a_map_without_home_inherits_from_the_origin_alone() {
        let mut m_nodes = vec![node(2, "a5s")];
        m_nodes[0].system = Some(system(HOME_SYSTEM, 5));
        let m = map(m_nodes, vec![], &[], vec![scan(10, "H296", 5, 0, None)]);
        let origin = m.node(2).unwrap().clone();
        assert_eq!(
            Echo.alias(&m, &origin, &wormhole(5, Some(10))).as_deref(),
            Some("a5a")
        );
    }

    #[test]
    fn rows_give_two_holes_of_one_class_different_slots() {
        // A branch is spoken for once a hole off home is linked; until then both of these
        // take `a`, and the slots keep them apart.
        let scans = vec![scan(10, "H296", 5, 0, None), scan(11, "H296", 5, 1, None)];
        let m = map(vec![home(), node(2, "a5a")], vec![], &[], scans);
        let rows = row_aliases(&m, &home());
        let names: Vec<&str> = rows.iter().map(|r| r.alias.as_str()).collect();
        assert_eq!(names, ["a5b", "a5c"]);
        assert!(rows.iter().all(|r| !r.settled && !r.homeward));
    }

    #[test]
    fn rows_give_homes_static_the_s_slot_and_the_next_hole_its_own_branch() {
        let scans = vec![scan(10, "H296", 5, 0, None), scan(11, "H296", 5, 1, None)];
        let m = map(vec![home()], vec![], &["H296"], scans);
        let names: Vec<String> = row_aliases(&m, &home())
            .into_iter()
            .map(|r| r.alias)
            .collect();
        assert_eq!(names, ["a5s", "b5a"]);
    }

    #[test]
    fn rows_report_a_named_destination_as_settled_and_the_way_home_as_homeward() {
        let nodes = vec![home(), node(2, "a5s"), node(3, "a3a")];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3)];
        let mut back = scan(10, "K162", 5, 0, Some(1));
        back.solar_system_id = 31_000_002;
        let mut on = scan(11, "N968", 3, 1, Some(2));
        on.solar_system_id = 31_000_002;
        let m = map(nodes, edges, &["H296"], vec![back, on]);
        let origin = m.node(2).unwrap().clone();
        let rows = row_aliases(&m, &origin);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].alias.as_str(), rows[0].homeward, rows[0].settled),
            ("a5s", true, true),
            "the way home is named after the hole you came in through"
        );
        assert_eq!(
            (rows[1].alias.as_str(), rows[1].homeward, rows[1].settled),
            ("a3a", false, true)
        );
    }

    #[test]
    fn a_loop_back_to_home_marks_one_hole_as_the_way_back() {
        // a4a hangs off both a5s and b3a, which are each one hop from home: two holes lead
        // closer to home, and only one of them is the way back.
        let nodes = vec![home(), node(2, "a5s"), node(3, "b3a"), node(4, "a4a")];
        let edges = vec![edge(1, 1, 2), edge(2, 1, 3), edge(3, 2, 4), edge(4, 3, 4)];
        let mut via_a5s = scan(10, "K162", 5, 0, Some(3));
        via_a5s.solar_system_id = 31_000_004;
        let mut via_b3a = scan(11, "K162", 3, 1, Some(4));
        via_b3a.solar_system_id = 31_000_004;
        let m = map(nodes, edges, &["H296"], vec![via_a5s, via_b3a]);
        let origin = m.node(4).unwrap().clone();
        let rows = row_aliases(&m, &origin);
        let marked: Vec<i64> = rows
            .iter()
            .filter(|r| r.homeward)
            .map(|r| r.signature)
            .collect();
        assert_eq!(
            marked,
            [10],
            "the walk from home reaches a4a through a5s first"
        );
    }

    #[test]
    fn rows_suggest_a_name_for_an_unnamed_ghost() {
        let nodes = vec![home(), ghost(2, None)];
        let m = map(
            nodes,
            vec![edge(1, 1, 2)],
            &["H296"],
            vec![scan(10, "H296", 5, 0, Some(1))],
        );
        let rows = row_aliases(&m, &home());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].alias, "a5s");
        assert!(!rows[0].settled);
    }

    #[test]
    fn rows_skip_a_hole_with_no_type() {
        let mut untyped = scan(10, "H296", 5, 0, None);
        untyped.code = None;
        untyped.target_class = None;
        let m = map(vec![home()], vec![], &["H296"], vec![untyped]);
        assert!(row_aliases(&m, &home()).is_empty());
    }

    #[test]
    fn renaming_a_ghost_does_not_count_its_own_name() {
        let nodes = vec![home(), node(2, "b5a"), ghost(3, Some("c6a"))];
        let edges = vec![edge(1, 1, 2), edge(2, 1, 3)];
        let m = map(
            nodes,
            edges,
            &["H296"],
            vec![scan(10, "V753", 6, 0, Some(2))],
        );
        assert_eq!(
            suggest(&m, &home(), &wormhole(6, Some(10)), Some(3), &[]).as_deref(),
            Some("c6a"),
            "a C6 retyped to another C6 keeps its name"
        );
        assert_eq!(
            suggest(&m, &home(), &wormhole(6, Some(10)), None, &[]).as_deref(),
            Some("d6a"),
            "counting it would move it along"
        );
    }

    #[test]
    fn generated_names_are_told_from_typed_ones() {
        for name in ["a5s", "b4a", "a13a", "aDa", "cHb", "aTa"] {
            assert!(is_generated(name), "{name}");
        }
        for name in ["HOME", "a5", "Staging", "A5a", "a5ab", "1"] {
            assert!(!is_generated(name), "{name}");
        }
    }
}
