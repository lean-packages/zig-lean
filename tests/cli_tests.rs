use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Validate and optimize Zig package build.zig.zon manifests",
        ));
}

#[test]
fn test_cli_lint_clean_manifest() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"build.zig\",\n        \"build.zig.zon\",\n    },\n}\n").unwrap();

    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.current_dir(root)
        .arg("lint")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All .paths entries are valid and lean.",
        ));
}

#[test]
fn test_cli_lint_invalid_manifest() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"missing_file.zig\",\n    },\n}\n").unwrap();

    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.current_dir(root)
        .arg("lint")
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"));
}

#[test]
fn test_cli_lint_json() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"test\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"build.zig\",\n        \"build.zig.zon\",\n    },\n}\n").unwrap();

    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.current_dir(root)
        .args(["lint", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\": true"));
}

#[test]
fn test_cli_inspect_text_and_json() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"sample_pkg\",\n    .version = \"1.2.3\",\n    .paths = .{\n        \"build.zig\",\n        \"build.zig.zon\",\n    },\n}\n").unwrap();

    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.current_dir(root)
        .arg("inspect")
        .assert()
        .success()
        .stdout(predicate::str::contains("Package: sample_pkg (1.2.3)"))
        .stdout(predicate::str::contains("Package files: 2"));

    let mut cmd_json = Command::cargo_bin("zig-lean").unwrap();
    cmd_json
        .current_dir(root)
        .args(["inspect", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"sample_pkg\""))
        .stdout(predicate::str::contains("\"total_file_count\": 2"));
}

#[test]
fn test_cli_update_dry_run_and_write() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join("build.zig"), "const std = @import(\"std\");\n").unwrap();
    fs::write(root.join("build.zig.zon"), ".{\n    .name = \"my_pkg\",\n    .version = \"0.1.0\",\n    .paths = .{\n        \"\",\n    },\n}\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.zig"), "pub fn main() void {}\n").unwrap();

    // Dry-run
    let mut cmd = Command::cargo_bin("zig-lean").unwrap();
    cmd.current_dir(root)
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run mode"))
        .stdout(predicate::str::contains("Proposed .paths:"))
        .stdout(predicate::str::contains("\"src\""));

    // File should still be unchanged
    let content_before = fs::read_to_string(root.join("build.zig.zon")).unwrap();
    assert!(content_before.contains("\"\""));

    // Write mode
    let mut cmd_write = Command::cargo_bin("zig-lean").unwrap();
    cmd_write
        .current_dir(root)
        .args(["update", "--write"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    // File should now contain updated paths
    let content_after = fs::read_to_string(root.join("build.zig.zon")).unwrap();
    assert!(!content_after.contains("\"\""));
    assert!(content_after.contains("\"build.zig\""));
    assert!(content_after.contains("\"build.zig.zon\""));
    assert!(content_after.contains("\"src\""));
}
