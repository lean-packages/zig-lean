use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

use crate::commands::OutputFormat;
use crate::manifest::Manifest;
use crate::manifest::editor::{apply_paths_update, write_manifest_atomic};
use crate::proposal::ProposalEngine;

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Path to the build.zig.zon manifest
    #[arg(
        short = 'm',
        long = "manifest-path",
        alias = "manifest",
        default_value = "build.zig.zon"
    )]
    pub manifest_path: PathBuf,

    /// Write changes to manifest (default is dry-run)
    #[arg(short = 'w', long)]
    pub write: bool,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Debug, Serialize)]
struct UpdateReport {
    manifest_path: String,
    written: bool,
    changed: bool,
    current_paths: Vec<String>,
    proposed_paths: Vec<String>,
    diff: String,
}

impl UpdateArgs {
    pub fn run(&self) -> Result<()> {
        let manifest = Manifest::load_from_path(&self.manifest_path).with_context(|| {
            format!(
                "Failed to load manifest at {}",
                self.manifest_path.display()
            )
        })?;

        let engine = ProposalEngine::new(&manifest);
        let proposed_paths = engine.generate_proposed_paths()?;
        let current_paths: Vec<String> = manifest.paths.iter().map(|p| p.value.clone()).collect();

        let changed = current_paths != proposed_paths;
        let diff = if changed {
            engine.generate_diff(&proposed_paths)?
        } else {
            String::new()
        };

        if self.write && changed {
            let updated_source = apply_paths_update(&manifest, &proposed_paths)?;
            write_manifest_atomic(&manifest.path, &updated_source)?;
        }

        match self.format {
            OutputFormat::Json => {
                let report = UpdateReport {
                    manifest_path: manifest.path.display().to_string(),
                    written: self.write && changed,
                    changed,
                    current_paths,
                    proposed_paths,
                    diff,
                };
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            OutputFormat::Text => {
                if !changed {
                    println!(
                        "Manifest .paths in {} is already optimal. No changes needed.",
                        manifest.path.display()
                    );
                    return Ok(());
                }

                if !self.write {
                    println!("Dry-run mode (use --write to apply changes).\n");
                } else {
                    println!("Updated {} successfully.\n", manifest.path.display());
                }

                println!("Proposed .paths:");
                for path in &proposed_paths {
                    println!("  \"{}\"", path);
                }

                if !diff.is_empty() {
                    println!("\nDiff:\n{}", diff);
                }
            }
        }

        Ok(())
    }
}
