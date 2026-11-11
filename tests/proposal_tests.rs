use std::fs;
use tempfile::tempdir;
use zig_lean::manifest::Manifest;
use zig_lean::proposal::ProposalEngine;

#[test]
fn test_proposal_engine_derives_lean_paths() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "const std = @import(\"std\");\n").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"\",\n    },\n}\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.zig"), "pub fn main() void {}\n").unwrap();
    fs::write(root.join("README.md"), "# Test\n").unwrap();
    fs::write(root.join("LICENSE"), "MIT\n").unwrap();
    fs::write(root.join("Justfile"), "all:\n").unwrap();

    let manifest = Manifest::load_from_path(root.join("build.zig.zon")).unwrap();
    let engine = ProposalEngine::new(&manifest);
    let proposed = engine.generate_proposed_paths().unwrap();

    assert!(proposed.contains(&"build.zig".to_string()));
    assert!(proposed.contains(&"build.zig.zon".to_string()));
    assert!(proposed.contains(&"src".to_string()));
    assert!(proposed.contains(&"LICENSE".to_string()));
    assert!(proposed.contains(&"README.md".to_string()));
    // Development artifact Justfile must NOT be included
    assert!(!proposed.contains(&"Justfile".to_string()));
    // Broad root "" must NOT be included
    assert!(!proposed.contains(&"".to_string()));
}
