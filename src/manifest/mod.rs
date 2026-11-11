pub mod editor;
pub mod zon_parser;

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use self::zon_parser::{Lexer, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathEntry {
    pub value: String,
    pub span: (usize, usize),
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: PathBuf,
    pub package_root: PathBuf,
    pub source: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub fingerprint: Option<String>,
    pub minimum_zig_version: Option<String>,
    pub paths: Vec<PathEntry>,
    pub has_paths: bool,
    #[allow(dead_code)]
    pub paths_field_span: Option<(usize, usize)>,
    pub paths_value_span: Option<(usize, usize)>,
    pub top_level_span: (usize, usize),
    pub base_indent: String,
}

impl Manifest {
    pub fn load_from_path(manifest_path: impl AsRef<Path>) -> Result<Self> {
        let manifest_path = manifest_path.as_ref();
        let canonical_path = if manifest_path.is_relative() {
            std::env::current_dir()
                .context("Failed to get current working directory")?
                .join(manifest_path)
        } else {
            manifest_path.to_path_buf()
        };

        if !canonical_path.exists() {
            bail!("Manifest file not found: {}", canonical_path.display());
        }

        let package_root = canonical_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let source = fs::read_to_string(&canonical_path).with_context(|| {
            format!("Failed to read manifest file: {}", canonical_path.display())
        })?;

        Self::parse_source(&source, canonical_path, package_root)
    }

    pub fn parse_source(source: &str, path: PathBuf, package_root: PathBuf) -> Result<Self> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;

        // Filter out whitespace and comments for parsing, but keep indices to original tokens
        let non_trivia: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| !matches!(t.kind, TokenKind::Whitespace | TokenKind::Comment))
            .collect();

        if non_trivia.is_empty() {
            bail!("Manifest is empty: {}", path.display());
        }

        let mut idx = 0;
        let mut name = None;
        let mut version = None;
        let mut fingerprint = None;
        let mut minimum_zig_version = None;
        let mut paths = Vec::new();
        let mut has_paths = false;
        let mut paths_field_span = None;
        let mut paths_value_span = None;

        // Expect top-level anonymous struct: `.{` or `{`
        let first_tok = non_trivia[idx].1;
        let top_start = match first_tok.kind {
            TokenKind::Dot => {
                idx += 1;
                if idx >= non_trivia.len() || !matches!(non_trivia[idx].1.kind, TokenKind::LBrace) {
                    bail!(
                        "Expected '{{' after '.' at {}:{}",
                        first_tok.line,
                        first_tok.column
                    );
                }
                idx += 1;
                first_tok.span.0
            }
            TokenKind::LBrace => {
                idx += 1;
                first_tok.span.0
            }
            _ => {
                bail!(
                    "Expected top-level struct '.{{' or '{{' at {}:{}, found {}",
                    first_tok.line,
                    first_tok.column,
                    first_tok.kind
                );
            }
        };

        // Parse struct fields
        while idx < non_trivia.len() {
            let tok = non_trivia[idx].1;
            if matches!(tok.kind, TokenKind::RBrace) {
                break;
            }

            // Field can be `.ident = value` or `ident: value`
            let field_start_byte = tok.span.0;
            let field_name = match &tok.kind {
                TokenKind::DotIdentifier(name) => {
                    idx += 1;
                    name.clone()
                }
                TokenKind::Dot => {
                    idx += 1;
                    if idx >= non_trivia.len() {
                        bail!("Unexpected EOF after '.' at {}:{}", tok.line, tok.column);
                    }
                    match &non_trivia[idx].1.kind {
                        TokenKind::Identifier(name) => {
                            let n = name.clone();
                            idx += 1;
                            n
                        }
                        other => {
                            bail!(
                                "Expected field name after '.' at {}:{}, found {}",
                                tok.line,
                                tok.column,
                                other
                            );
                        }
                    }
                }
                TokenKind::Identifier(name) => {
                    idx += 1;
                    name.clone()
                }
                _ => {
                    bail!(
                        "Expected field identifier at {}:{}, found {}",
                        tok.line,
                        tok.column,
                        tok.kind
                    );
                }
            };

            // Expect '=' or ':'
            if idx >= non_trivia.len() {
                bail!("Unexpected EOF in manifest struct field '{field_name}'");
            }
            let eq_tok = non_trivia[idx].1;
            if !matches!(eq_tok.kind, TokenKind::Equal | TokenKind::Colon) {
                bail!(
                    "Expected '=' after field '.{field_name}' at {}:{}, found {}",
                    eq_tok.line,
                    eq_tok.column,
                    eq_tok.kind
                );
            }
            idx += 1;

            if idx >= non_trivia.len() {
                bail!("Unexpected EOF after '=' for field '.{field_name}'");
            }

            // Value of field
            let val_tok = non_trivia[idx].1;

            if field_name == "paths" {
                has_paths = true;
                // Parse .paths = .{ ... } or { ... }
                let val_open_byte = match val_tok.kind {
                    TokenKind::Dot => {
                        idx += 1;
                        if idx >= non_trivia.len()
                            || !matches!(non_trivia[idx].1.kind, TokenKind::LBrace)
                        {
                            bail!(
                                "Expected '{{' in .paths value at {}:{}",
                                val_tok.line,
                                val_tok.column
                            );
                        }
                        idx += 1;
                        val_tok.span.0
                    }
                    TokenKind::LBrace => {
                        idx += 1;
                        val_tok.span.0
                    }
                    _ => {
                        bail!(
                            "'.paths' must be a tuple/struct '.{{ ... }}', found {} at {}:{}",
                            val_tok.kind,
                            val_tok.line,
                            val_tok.column
                        );
                    }
                };

                // Parse entries inside .paths
                while idx < non_trivia.len() {
                    let entry_tok = non_trivia[idx].1;
                    if matches!(entry_tok.kind, TokenKind::RBrace) {
                        break;
                    }

                    match &entry_tok.kind {
                        TokenKind::StringLiteral(s) => {
                            paths.push(PathEntry {
                                value: s.clone(),
                                span: entry_tok.span,
                                line: entry_tok.line,
                                column: entry_tok.column,
                            });
                            idx += 1;
                        }
                        other => {
                            bail!(
                                "'.paths' entries must be string literals, found {} at {}:{}",
                                other,
                                entry_tok.line,
                                entry_tok.column
                            );
                        }
                    }

                    // Optional comma
                    if idx < non_trivia.len() && matches!(non_trivia[idx].1.kind, TokenKind::Comma)
                    {
                        idx += 1;
                    }
                }

                if idx >= non_trivia.len() || !matches!(non_trivia[idx].1.kind, TokenKind::RBrace) {
                    bail!("Unclosed .paths tuple struct");
                }
                let close_brace_tok = non_trivia[idx].1;
                let paths_val_end_byte = close_brace_tok.span.1;
                idx += 1; // skip '}'

                paths_value_span = Some((val_open_byte, paths_val_end_byte));

                // Optional trailing comma after .paths field
                let mut field_end_byte = paths_val_end_byte;
                if idx < non_trivia.len() && matches!(non_trivia[idx].1.kind, TokenKind::Comma) {
                    field_end_byte = non_trivia[idx].1.span.1;
                    idx += 1;
                }
                paths_field_span = Some((field_start_byte, field_end_byte));
            } else {
                // Parse generic value
                match &val_tok.kind {
                    TokenKind::StringLiteral(s) => {
                        if field_name == "name" {
                            name = Some(s.clone());
                        } else if field_name == "version" {
                            version = Some(s.clone());
                        } else if field_name == "fingerprint" {
                            fingerprint = Some(s.clone());
                        } else if field_name == "minimum_zig_version" {
                            minimum_zig_version = Some(s.clone());
                        }
                        idx += 1;
                    }
                    TokenKind::DotIdentifier(s) | TokenKind::Identifier(s) => {
                        if field_name == "name" {
                            name = Some(s.clone());
                        }
                        idx += 1;
                    }
                    TokenKind::NumberLiteral(s) => {
                        if field_name == "fingerprint" {
                            fingerprint = Some(s.clone());
                        }
                        idx += 1;
                    }
                    TokenKind::Dot => {
                        // Struct or enum
                        idx += 1;
                        if idx < non_trivia.len()
                            && matches!(non_trivia[idx].1.kind, TokenKind::LBrace)
                        {
                            // Sub-struct
                            idx += 1;
                            let mut depth = 1;
                            while idx < non_trivia.len() && depth > 0 {
                                match non_trivia[idx].1.kind {
                                    TokenKind::LBrace => depth += 1,
                                    TokenKind::RBrace => depth -= 1,
                                    _ => {}
                                }
                                idx += 1;
                            }
                        } else if idx < non_trivia.len() {
                            // Enum literal like .my_pkg
                            if let TokenKind::Identifier(s) = &non_trivia[idx].1.kind
                                && field_name == "name"
                            {
                                name = Some(s.clone());
                            }
                            idx += 1;
                        }
                    }
                    TokenKind::LBrace => {
                        // Sub struct
                        idx += 1;
                        let mut depth = 1;
                        while idx < non_trivia.len() && depth > 0 {
                            match non_trivia[idx].1.kind {
                                TokenKind::LBrace => depth += 1,
                                TokenKind::RBrace => depth -= 1,
                                _ => {}
                            }
                            idx += 1;
                        }
                    }
                    _ => {
                        idx += 1;
                    }
                }

                // Optional comma
                if idx < non_trivia.len() && matches!(non_trivia[idx].1.kind, TokenKind::Comma) {
                    idx += 1;
                }
            }
        }

        if idx >= non_trivia.len() || !matches!(non_trivia[idx].1.kind, TokenKind::RBrace) {
            bail!("Unclosed top-level struct in manifest");
        }
        let top_end = non_trivia[idx].1.span.1;

        // Detect base indentation
        let base_indent = Self::detect_base_indent(source, paths_field_span);

        Ok(Self {
            path,
            package_root,
            source: source.to_string(),
            name,
            version,
            fingerprint,
            minimum_zig_version,
            paths,
            has_paths,
            paths_field_span,
            paths_value_span,
            top_level_span: (top_start, top_end),
            base_indent,
        })
    }

    fn detect_base_indent(source: &str, paths_span: Option<(usize, usize)>) -> String {
        if let Some((start, _)) = paths_span {
            let prefix = &source[..start];
            if let Some(last_nl) = prefix.rfind('\n') {
                let line_prefix = &prefix[last_nl + 1..];
                let ws: String = line_prefix
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                if !ws.is_empty() {
                    return ws;
                }
            }
        }
        "    ".to_string()
    }

    pub fn duplicate_paths(&self) -> Vec<(&PathEntry, &PathEntry)> {
        let mut seen: std::collections::HashMap<&str, &PathEntry> =
            std::collections::HashMap::new();
        let mut duplicates = Vec::new();

        for entry in &self.paths {
            if let Some(prev) = seen.get(entry.value.as_str()) {
                duplicates.push((*prev, entry));
            } else {
                seen.insert(&entry.value, entry);
            }
        }

        duplicates
    }
}
