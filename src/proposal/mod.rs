use anyhow::Result;
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::Manifest;
use crate::manifest::editor::apply_paths_update;
use crate::resolver::{is_development_or_generated_artifact, normalize_path_str};

pub struct ProposalEngine<'a> {
    manifest: &'a Manifest,
}

impl<'a> ProposalEngine<'a> {
    pub fn new(manifest: &'a Manifest) -> Self {
        Self { manifest }
    }

    pub fn generate_proposed_paths(&self) -> Result<Vec<String>> {
        let package_root = &self.manifest.package_root;
        let mut candidates = BTreeSet::new();

        // 1. Discover project files (via git ls-files if available, else filesystem scan)
        let discovered_files = self.discover_project_files(package_root);

        // 2. Filter out development / build artifacts
        let mut clean_files = BTreeSet::new();
        for file in &discovered_files {
            if is_development_or_generated_artifact(file).is_none() {
                clean_files.insert(file.clone());
            }
        }

        // 3. Always include essential files if they exist in clean_files
        for essential in &["build.zig", "build.zig.zon"] {
            if clean_files.contains(*essential) {
                candidates.insert(essential.to_string());
            }
        }

        // 4. Check for common top-level source directories (e.g. src)
        // If "src" directory exists and all files under "src/" are clean_files, include "src"
        let top_dirs = self.find_top_level_directories(package_root);
        for dir in top_dirs {
            let prefix = format!("{dir}/");
            let files_in_dir: Vec<&String> = discovered_files
                .iter()
                .filter(|f| f.starts_with(&prefix))
                .collect();

            let all_clean = files_in_dir.iter().all(|f| {
                clean_files.contains(*f) && is_development_or_generated_artifact(f).is_none()
            });

            // If dir is "src" or contains files, and is not a dev artifact (like .github, tests, examples)
            if is_development_or_generated_artifact(&dir).is_none()
                && (dir == "src"
                    || (all_clean
                        && !files_in_dir.is_empty()
                        && (dir == "include" || dir == "lib")))
            {
                candidates.insert(dir.clone());
                // Remove individual files under this dir
                clean_files.retain(|f| !f.starts_with(&prefix));
            }
        }

        // 5. Consider existing paths in the manifest:
        // If an existing entry is a valid directory or file and not a broad root or transient artifact, keep it
        for entry in &self.manifest.paths {
            let norm = normalize_path_str(&entry.value);
            if norm.is_empty() || norm == "." {
                continue; // Skip broad root
            }
            if is_development_or_generated_artifact(&norm).is_some() {
                continue; // Skip development artifact
            }
            let target = package_root.join(&norm);
            if target.exists() {
                if target.is_dir() {
                    candidates.insert(norm.clone());
                    let prefix = format!("{norm}/");
                    clean_files.retain(|f| !f.starts_with(&prefix));
                } else if target.is_file() {
                    candidates.insert(norm);
                }
            }
        }

        // 6. Include LICENSE and README files if present
        for file in &clean_files {
            let lower = file.to_lowercase();
            if lower.starts_with("license")
                || lower.starts_with("readme")
                || lower.starts_with("copying")
                || lower.starts_with("unlicense")
            {
                candidates.insert(file.clone());
            }
        }

        // 7. Include remaining clean source files
        for file in clean_files {
            if file.ends_with(".zig")
                || file.ends_with(".c")
                || file.ends_with(".h")
                || file.ends_with(".cpp")
            {
                candidates.insert(file);
            }
        }

        // Remove redundant entries (e.g. if "src" is present, remove "src/foo.zig")
        let mut final_paths: Vec<String> = Vec::new();
        for path in &candidates {
            let mut is_redundant = false;
            for other in &candidates {
                if path != other && !other.is_empty() {
                    let parent_prefix = format!("{other}/");
                    if path.starts_with(&parent_prefix) {
                        is_redundant = true;
                        break;
                    }
                }
            }
            if !is_redundant {
                final_paths.push(path.clone());
            }
        }

        // Sort deterministically with standard Zig convention
        final_paths.sort_by(|a, b| {
            let rank_a = sort_rank(a);
            let rank_b = sort_rank(b);
            if rank_a != rank_b {
                rank_a.cmp(&rank_b)
            } else {
                a.cmp(b)
            }
        });

        Ok(final_paths)
    }

    pub fn generate_diff(&self, proposed_paths: &[String]) -> Result<String> {
        let proposed_source = apply_paths_update(self.manifest, proposed_paths)?;
        let diff = TextDiff::from_lines(&self.manifest.source, &proposed_source);

        let mut out = String::new();
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            out.push_str(sign);
            out.push_str(change.value());
        }

        Ok(out)
    }

    fn discover_project_files(&self, root: &Path) -> BTreeSet<String> {
        // Try git ls-files first
        if let Ok(output) = Command::new("git")
            .arg("ls-files")
            .current_dir(root)
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut files = BTreeSet::new();
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.insert(normalize_path_str(trimmed));
                }
            }
            if !files.is_empty() {
                return files;
            }
        }

        // Fallback: filesystem scan
        let mut files = BTreeSet::new();
        let mut visited = HashSet::new();
        if let Ok(canon_root) = root.canonicalize() {
            self.scan_dir(root, root, &canon_root, &mut visited, &mut files);
        }
        files
    }

    fn scan_dir(
        &self,
        dir: &Path,
        root: &Path,
        canon_root: &Path,
        visited: &mut HashSet<PathBuf>,
        results: &mut BTreeSet<String>,
    ) {
        if let Ok(canon) = dir.canonicalize()
            && (!canon.starts_with(canon_root) || !visited.insert(canon))
        {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    let rel_str = normalize_path_str(&rel.to_string_lossy());
                    results.insert(rel_str);
                }
            } else if path.is_dir() {
                self.scan_dir(&path, root, canon_root, visited, results);
            }
        }
    }

    fn find_top_level_directories(&self, root: &Path) -> Vec<String> {
        let mut dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_dir()
                    && let Ok(name) = entry.file_name().into_string()
                {
                    dirs.push(name);
                }
            }
        }
        dirs.sort();
        dirs
    }
}

fn sort_rank(name: &str) -> usize {
    match name {
        "build.zig" => 1,
        "build.zig.zon" => 2,
        "src" => 3,
        _ if name.to_lowercase().starts_with("license") => 10,
        _ if name.to_lowercase().starts_with("readme") => 11,
        _ => 5,
    }
}
