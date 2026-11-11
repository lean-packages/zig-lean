use std::path::PathBuf;
use zig_lean::manifest::Manifest;
use zig_lean::manifest::editor::{apply_paths_update, format_paths_tuple};

#[test]
fn test_format_paths_tuple() {
    let paths = vec!["build.zig".to_string(), "src".to_string()];
    let formatted = format_paths_tuple(&paths, "    ");
    assert_eq!(
        formatted,
        ".{\n        \"build.zig\",\n        \"src\",\n    }"
    );
}

#[test]
fn test_apply_paths_update_preserves_comments() {
    let source = r#".{
    // Top-level comment
    .name = "my_pkg",
    .version = "1.0.0",
    // Paths comment
    .paths = .{
        "",
    },
    // End comment
}
"#;

    let manifest =
        Manifest::parse_source(source, PathBuf::from("build.zig.zon"), PathBuf::from("."))
            .expect("parsing should succeed");

    let new_paths = vec![
        "build.zig".to_string(),
        "build.zig.zon".to_string(),
        "src".to_string(),
    ];
    let updated = apply_paths_update(&manifest, &new_paths).expect("update should succeed");

    assert!(updated.contains("// Top-level comment"));
    assert!(updated.contains(".name = \"my_pkg\","));
    assert!(updated.contains("// End comment"));
    assert!(updated.contains("\"build.zig\","));
    assert!(updated.contains("\"build.zig.zon\","));
    assert!(updated.contains("\"src\","));
    assert!(!updated.contains("\"\""));
}
