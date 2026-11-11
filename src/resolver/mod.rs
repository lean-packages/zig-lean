use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::diagnostics::Diagnostic;
use crate::manifest::{Manifest, PathEntry};

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedEntry {
    pub raw: String,
    pub exists: bool,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_broad: bool,
    pub is_redundant: bool,
    pub redundant_with: Option<String>,
    pub is_escape: bool,
    pub file_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedPackage {
    pub manifest_path: String,
    pub package_root: String,
    pub name: Option<String>,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_zig_version: Option<String>,
    pub entries: Vec<ResolvedEntry>,
    pub files: Vec<ResolvedFile>,
    pub total_file_count: usize,
    pub total_size_bytes: u64,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Resolver<'a> {
    manifest: &'a Manifest,
}

impl<'a> Resolver<'a> {
    pub fn new(manifest: &'a Manifest) -> Self {
        Self { manifest }
    }

    pub fn resolve(&self) -> Result<ResolvedPackage> {
        let package_root = &self.manifest.package_root;
        let canonical_root = package_root
            .canonicalize()
            .unwrap_or_else(|_| package_root.clone());

        let mut diagnostics = Vec::new();

        if !self.manifest.has_paths {
            diagnostics.push(
                Diagnostic::error(
                    "missing-paths-field",
                    "Manifest is missing the required '.paths' field",
                )
                .with_path(self.manifest.path.display().to_string()),
            );
        }

        // Check for duplicates
        let duplicates = self.manifest.duplicate_paths();
        for (_first, dup) in duplicates {
            diagnostics.push(
                Diagnostic::warning(
                    "duplicate-path",
                    format!("Duplicate entry '{}' in .paths", dup.value),
                )
                .with_path(self.manifest.path.display().to_string())
                .with_location(dup.line, dup.column)
                .with_suggestion(format!("Remove duplicate entry at line {}", dup.line)),
            );
        }

        // Check for redundant entries (e.g. "src" and "src/foo.zig")
        let mut clean_paths: Vec<(usize, &PathEntry, String)> = Vec::new();
        for (i, entry) in self.manifest.paths.iter().enumerate() {
            let normalized = normalize_path_str(&entry.value);
            clean_paths.push((i, entry, normalized));
        }

        let mut redundant_entries: BTreeMap<usize, String> = BTreeMap::new();
        for (i, _entry_a, norm_a) in &clean_paths {
            for (j, entry_b, norm_b) in &clean_paths {
                if i == j {
                    continue;
                }
                if norm_b.is_empty() || is_child_path(norm_a, norm_b) {
                    redundant_entries.insert(*i, entry_b.value.clone());
                    break;
                }
            }
        }

        for (idx, parent) in &redundant_entries {
            let entry = &self.manifest.paths[*idx];
            diagnostics.push(
                Diagnostic::warning(
                    "redundant-path",
                    format!(
                        "Entry '{}' is redundant because it is already included by '{}'",
                        entry.value, parent
                    ),
                )
                .with_path(self.manifest.path.display().to_string())
                .with_location(entry.line, entry.column)
                .with_suggestion(format!("Remove redundant entry '{}'", entry.value)),
            );
        }

        // Resolve files for each entry
        let mut resolved_entries = Vec::new();
        let mut all_files_map: BTreeMap<String, u64> = BTreeMap::new();

        for (idx, entry) in self.manifest.paths.iter().enumerate() {
            let val = &entry.value;
            let is_redundant = redundant_entries.contains_key(&idx);
            let redundant_with = redundant_entries.get(&idx).cloned();

            // Check for path escapes or absolute paths
            let path_obj = Path::new(val);
            if path_obj.is_absolute() {
                diagnostics.push(
                    Diagnostic::error(
                        "absolute-path",
                        format!("Entry '{}' in .paths must not be an absolute path", val),
                    )
                    .with_path(self.manifest.path.display().to_string())
                    .with_location(entry.line, entry.column),
                );
                resolved_entries.push(ResolvedEntry {
                    raw: val.clone(),
                    exists: false,
                    is_dir: false,
                    is_file: false,
                    is_broad: false,
                    is_redundant,
                    redundant_with,
                    is_escape: true,
                    file_count: 0,
                    total_bytes: 0,
                });
                continue;
            }

            if has_parent_traversal(path_obj) {
                diagnostics.push(
                    Diagnostic::error(
                        "parent-traversal",
                        format!("Entry '{}' escapes package root via '..' traversal", val),
                    )
                    .with_path(self.manifest.path.display().to_string())
                    .with_location(entry.line, entry.column),
                );
                resolved_entries.push(ResolvedEntry {
                    raw: val.clone(),
                    exists: false,
                    is_dir: false,
                    is_file: false,
                    is_broad: false,
                    is_redundant,
                    redundant_with,
                    is_escape: true,
                    file_count: 0,
                    total_bytes: 0,
                });
                continue;
            }

            let is_broad = val.is_empty() || val == ".";
            if is_broad {
                diagnostics.push(
                    Diagnostic::warning(
                        "broad-root-inclusion",
                        format!(
                            "Entry '{}' includes the entire package root recursively",
                            val
                        ),
                    )
                    .with_path(self.manifest.path.display().to_string())
                    .with_location(entry.line, entry.column)
                    .with_suggestion(
                        "List specific source directories and files instead of the entire root",
                    ),
                );
            }

            let target_path = if is_broad {
                package_root.to_path_buf()
            } else {
                package_root.join(val)
            };

            if !target_path.exists() {
                diagnostics.push(
                    Diagnostic::error(
                        "missing-path",
                        format!("Path '{}' listed in .paths does not exist on disk", val),
                    )
                    .with_path(self.manifest.path.display().to_string())
                    .with_location(entry.line, entry.column),
                );
                resolved_entries.push(ResolvedEntry {
                    raw: val.clone(),
                    exists: false,
                    is_dir: false,
                    is_file: false,
                    is_broad,
                    is_redundant,
                    redundant_with,
                    is_escape: false,
                    file_count: 0,
                    total_bytes: 0,
                });
                continue;
            }

            // Verify symlink destination if symlink
            if target_path.is_symlink()
                && let Ok(canon) = target_path.canonicalize()
                && !canon.starts_with(&canonical_root)
            {
                diagnostics.push(
                    Diagnostic::error(
                        "symlink-escape",
                        format!("Symlink '{}' resolves outside package root", val),
                    )
                    .with_path(self.manifest.path.display().to_string())
                    .with_location(entry.line, entry.column),
                );
                resolved_entries.push(ResolvedEntry {
                    raw: val.clone(),
                    exists: true,
                    is_dir: false,
                    is_file: false,
                    is_broad,
                    is_redundant,
                    redundant_with,
                    is_escape: true,
                    file_count: 0,
                    total_bytes: 0,
                });
                continue;
            }

            // Traverse file or directory
            let mut entry_files = Vec::new();
            let mut entry_bytes = 0u64;
            let is_dir = target_path.is_dir();
            let is_file = target_path.is_file();

            if is_file {
                let size = fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0);
                let rel = normalize_path_str(val);
                entry_files.push((rel.clone(), size));
                entry_bytes += size;
                all_files_map.insert(rel, size);
            } else if is_dir {
                let mut visited_dirs = HashSet::new();
                collect_dir_files(
                    &target_path,
                    package_root,
                    &canonical_root,
                    &mut visited_dirs,
                    &mut entry_files,
                );
                for (rel, size) in &entry_files {
                    entry_bytes += size;
                    all_files_map.insert(rel.clone(), *size);
                }
            }

            resolved_entries.push(ResolvedEntry {
                raw: val.clone(),
                exists: true,
                is_dir,
                is_file,
                is_broad,
                is_redundant,
                redundant_with,
                is_escape: false,
                file_count: entry_files.len(),
                total_bytes: entry_bytes,
            });
        }

        // Check for included non-release / development artifacts
        for rel_path in all_files_map.keys() {
            if let Some(reason) = is_development_or_generated_artifact(rel_path) {
                diagnostics.push(
                    Diagnostic::notice(
                        "development-artifact-included",
                        format!(
                            "Package includes likely non-release artifact '{}' ({reason})",
                            rel_path
                        ),
                    )
                    .with_path(rel_path.clone()),
                );
            }
        }

        // Check for missing essential files in package root
        let build_zig = package_root.join("build.zig");
        if build_zig.exists() && !all_files_map.contains_key("build.zig") {
            diagnostics.push(
                Diagnostic::notice(
                    "missing-build-zig",
                    "'build.zig' exists in package root but is not included in .paths",
                )
                .with_path(self.manifest.path.display().to_string())
                .with_suggestion("Add \"build.zig\" to .paths so consumers can build this package"),
            );
        }

        let build_zig_zon = package_root.join("build.zig.zon");
        if build_zig_zon.exists() && !all_files_map.contains_key("build.zig.zon") {
            diagnostics.push(
                Diagnostic::notice(
                    "missing-build-zig-zon",
                    "'build.zig.zon' exists in package root but is not included in .paths",
                )
                .with_path(self.manifest.path.display().to_string())
                .with_suggestion(
                    "Add \"build.zig.zon\" to .paths so package manager can identify the package",
                ),
            );
        }

        let total_file_count = all_files_map.len();
        let total_size_bytes = all_files_map.values().sum();

        let files: Vec<ResolvedFile> = all_files_map
            .into_iter()
            .map(|(path, size_bytes)| ResolvedFile { path, size_bytes })
            .collect();

        Ok(ResolvedPackage {
            manifest_path: self.manifest.path.display().to_string(),
            package_root: self.manifest.package_root.display().to_string(),
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            fingerprint: self.manifest.fingerprint.clone(),
            minimum_zig_version: self.manifest.minimum_zig_version.clone(),
            entries: resolved_entries,
            files,
            total_file_count,
            total_size_bytes,
            diagnostics,
        })
    }
}

fn collect_dir_files(
    dir: &Path,
    root: &Path,
    canonical_root: &Path,
    visited: &mut HashSet<PathBuf>,
    results: &mut Vec<(String, u64)>,
) {
    if let Ok(canon) = dir.canonicalize()
        && (!canon.starts_with(canonical_root) || !visited.insert(canon))
    {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink()
            && let Ok(canon) = path.canonicalize()
            && !canon.starts_with(canonical_root)
        {
            continue;
        }

        if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = normalize_path_str(&rel.to_string_lossy());
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                results.push((rel_str, size));
            }
        } else if path.is_dir() {
            collect_dir_files(&path, root, canonical_root, visited, results);
        }
    }
}

pub fn normalize_path_str(p: &str) -> String {
    let mut s = p.trim().replace('\\', "/");
    while s.starts_with("./") {
        s = s[2..].to_string();
    }
    if s.ends_with('/') {
        s.pop();
    }
    if s == "." { String::new() } else { s }
}

fn has_parent_traversal(p: &Path) -> bool {
    for comp in p.components() {
        if matches!(comp, Component::ParentDir) {
            return true;
        }
    }
    false
}

fn is_child_path(child: &str, parent: &str) -> bool {
    if parent.is_empty() {
        return !child.is_empty();
    }
    if child == parent {
        return false;
    }
    let parent_prefix = format!("{parent}/");
    child.starts_with(&parent_prefix)
}

pub fn is_development_or_generated_artifact(rel_path: &str) -> Option<&'static str> {
    let parts: Vec<&str> = rel_path.split('/').collect();
    let file_name = parts.last().copied().unwrap_or("");

    // VCS and Git
    if parts.contains(&".git") || rel_path.starts_with(".git/") {
        return Some("VCS directory");
    }
    if file_name == ".gitignore" || file_name == ".gitattributes" || file_name == ".gitmodules" {
        return Some("Git metadata");
    }
    if parts.contains(&".github") || rel_path.starts_with(".github/") {
        return Some("CI / GitHub metadata");
    }

    // Zig build caches and outputs
    if parts.contains(&"zig-cache")
        || parts.contains(&".zig-cache")
        || parts.contains(&"zig-out")
        || file_name == "build.zig.zon.lock"
    {
        return Some("Zig cache / build output");
    }

    // General build artifacts & target folders
    if parts.contains(&"target") || parts.contains(&".target") || parts.contains(&"node_modules") {
        return Some("Build / dependency directory");
    }

    // IDE / Editor configurations
    if parts.contains(&".idea")
        || parts.contains(&".vscode")
        || parts.contains(&".vs")
        || file_name == ".editorconfig"
        || file_name.ends_with(".iml")
    {
        return Some("Editor / IDE configuration");
    }

    // Task runners and CI configs
    if file_name == "Justfile"
        || file_name == "justfile"
        || file_name == "Makefile"
        || file_name == "release.toml"
        || file_name == "rust-toolchain.toml"
        || file_name == "PROMPTS.md"
        || file_name == ".travis.yml"
    {
        return Some("Development tooling config");
    }

    None
}
