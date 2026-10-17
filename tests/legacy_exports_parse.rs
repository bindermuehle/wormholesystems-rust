//! Does the real ECHO production export parse? Scratch check against the files in
//! ~/wh-exports, skipped when they are not there. Not for CI.
use wormholesystems::maps::transfer::{SectionSet, parse_export};

#[test]
fn legacy_exports_parse() {
    let dir = std::path::Path::new(env!("HOME")).join("wh-exports");
    if !dir.exists() {
        eprintln!("no ~/wh-exports, skipping");
        return;
    }
    let wanted = SectionSet {
        settings: true,
        access: true,
        solarsystems: true,
        connections: false,
        signatures: false,
        routes: false,
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for path in files {
        let content = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match parse_export(&content, wanted, true) {
            Ok(p) => println!("OK    {name}  map={}", p.map_name),
            Err(e) => println!("FAIL  {name}  {e}"),
        }
    }
}
