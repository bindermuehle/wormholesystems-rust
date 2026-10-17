//! One-off: import the legacy production exports in ~/wh-exports/echo as the local user.
//! Scratch tooling, not CI — it writes to whatever DATABASE_URL points at and skips when
//! the folder is missing. Run with: cargo test --test import_legacy_exports -- --nocapture
use wormholesystems::allowlist::AffiliationAllowlist;
use wormholesystems::maps::transfer::{SectionSet, import_map_as_new, parse_export};
use wormholesystems::maps::Actor;

#[tokio::test]
async fn import_legacy_exports() {
    let dir = std::path::Path::new(env!("HOME")).join("wh-exports/echo");
    if !dir.exists() {
        eprintln!("no ~/wh-exports/echo, skipping");
        return;
    }
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();

    // The single local account: whoever signed in through SSO.
    let row: (i64, i64) = sqlx::query_as(
        "select u.id, c.id from users u join characters c on c.user_id = u.id order by u.id limit 1",
    )
    .fetch_one(&pool)
    .await
    .expect("no user — sign in at http://localhost:5173 first");
    let actor = Actor { user_id: row.0, character_id: row.1 };

    // What the migration is for: the map, who may see it, the occupier details and the
    // home system. Signatures are deliberately left out — holes roll, the data is stale
    // the moment it moves.
    let wanted = SectionSet {
        settings: true,
        access: true,
        solarsystems: true,
        connections: true,
        signatures: false,
        routes: true,
    };
    let creators = AffiliationAllowlist::new([]);

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();

    let (mut ok, mut failed) = (0, 0);
    for path in files {
        let content = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let parsed = match parse_export(&content, wanted, true) {
            Ok(p) => p,
            Err(e) => {
                println!("PARSE FAIL {name}: {e}");
                failed += 1;
                continue;
            }
        };
        match import_map_as_new(&pool, actor, &creators, parsed, None).await {
            Ok(map) => {
                println!("imported  {:<14} id={}", map.name, map.id);
                ok += 1;
            }
            Err(e) => {
                println!("IMPORT FAIL {name}: {e}");
                failed += 1;
            }
        }
    }
    println!("\nimported {ok}, failed {failed}");
    assert_eq!(failed, 0, "{failed} file(s) did not import");
}
