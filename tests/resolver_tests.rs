use std::fs;
use tempfile::tempdir;
use zig_lean::diagnostics::Severity;
use zig_lean::manifest::Manifest;
use zig_lean::resolver::Resolver;

#[test]
fn test_resolve_clean_package() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "const std = @import(\"std\");\n").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"build.zig\",\n        \"build.zig.zon\",\n        \"src\",\n    },\n}\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.zig"), "pub fn main() void {}\n").unwrap();

    let manifest = Manifest::load_from_path(root.join("build.zig.zon")).unwrap();
    let resolver = Resolver::new(&manifest);
    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.total_file_count, 3);
    assert_eq!(resolved.entries.len(), 3);
    let errors: Vec<_> = resolved
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty());
}

#[test]
fn test_resolve_missing_and_redundant_paths() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"build.zig\",\n        \"src\",\n        \"src/main.zig\",\n        \"non_existent\",\n        \"build.zig\",\n    },\n}\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.zig"), "").unwrap();

    let manifest = Manifest::load_from_path(root.join("build.zig.zon")).unwrap();
    let resolver = Resolver::new(&manifest);
    let resolved = resolver.resolve().unwrap();

    let codes: Vec<&str> = resolved
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert!(codes.contains(&"missing-path"));
    assert!(codes.contains(&"duplicate-path"));
    assert!(codes.contains(&"redundant-path"));
}

#[test]
fn test_resolve_escape_and_broad_paths() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"\",\n        \"../escaped\",\n    },\n}\n").unwrap();

    let manifest = Manifest::load_from_path(root.join("build.zig.zon")).unwrap();
    let resolver = Resolver::new(&manifest);
    let resolved = resolver.resolve().unwrap();

    let codes: Vec<&str> = resolved
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert!(codes.contains(&"broad-root-inclusion"));
    assert!(codes.contains(&"parent-traversal"));
}
