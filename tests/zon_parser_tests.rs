use std::path::PathBuf;
use zig_lean::manifest::Manifest;
use zig_lean::manifest::zon_parser::{Lexer, TokenKind};

#[test]
fn test_lexer_tokens() {
    let input = r#"
    .{
        .name = "my_pkg",
        .version = "1.0.0",
        .fingerprint = 0x1234abcd,
        .paths = .{
            "build.zig",
            "src",
        },
    }
    "#;

    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("tokenization should succeed");
    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_escapes_and_multiline() {
    let input = r#"
    .{
        .name = "hello\nworld",
        .description = \\first line
                       \\second line
        ,
    }
    "#;

    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("tokenization should succeed");
    let strings: Vec<String> = tokens
        .into_iter()
        .filter_map(|t| match t.kind {
            TokenKind::StringLiteral(s) => Some(s),
            _ => None,
        })
        .collect();

    assert_eq!(strings[0], "hello\nworld");
    assert_eq!(strings[1], "first line\nsecond line");
}

#[test]
fn test_parse_standard_manifest() {
    let source = r#"
    .{
        // Package name
        .name = "sample-package",
        .version = "0.2.0",
        .minimum_zig_version = "0.14.0",
        .fingerprint = 0xdeadbeef12345678,
        .paths = .{
            "build.zig",
            "build.zig.zon",
            "src",
            "LICENSE",
            "README.md",
        },
    }
    "#;

    let manifest =
        Manifest::parse_source(source, PathBuf::from("build.zig.zon"), PathBuf::from("."))
            .expect("manifest parsing should succeed");

    assert_eq!(manifest.name.as_deref(), Some("sample-package"));
    assert_eq!(manifest.version.as_deref(), Some("0.2.0"));
    assert_eq!(manifest.minimum_zig_version.as_deref(), Some("0.14.0"));
    assert_eq!(manifest.fingerprint.as_deref(), Some("0xdeadbeef12345678"));
    assert_eq!(manifest.paths.len(), 5);
    assert_eq!(manifest.paths[0].value, "build.zig");
    assert_eq!(manifest.paths[1].value, "build.zig.zon");
    assert_eq!(manifest.paths[2].value, "src");
}

#[test]
fn test_parse_manifest_with_enum_and_special_identifiers() {
    let source = r#"
    .{
        .name = .@"special-name",
        .version = "1.0.0",
        .paths = .{
            "",
        },
    }
    "#;

    let manifest =
        Manifest::parse_source(source, PathBuf::from("build.zig.zon"), PathBuf::from("."))
            .expect("manifest parsing should succeed");

    assert_eq!(manifest.name.as_deref(), Some("special-name"));
    assert_eq!(manifest.paths.len(), 1);
    assert_eq!(manifest.paths[0].value, "");
}

#[test]
fn test_parse_manifest_missing_paths() {
    let source = r#"
    .{
        .name = "incomplete",
        .version = "0.1.0",
    }
    "#;

    let manifest =
        Manifest::parse_source(source, PathBuf::from("build.zig.zon"), PathBuf::from("."))
            .expect("manifest parsing should succeed");

    assert!(!manifest.has_paths);
    assert!(manifest.paths.is_empty());
}

#[test]
fn test_parse_manifest_syntax_error() {
    let source = r#"
    .{
        .name = "unclosed string
    }
    "#;

    let result = Manifest::parse_source(source, PathBuf::from("build.zig.zon"), PathBuf::from("."));

    assert!(result.is_err());
}
