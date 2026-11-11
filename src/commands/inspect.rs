use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use crate::commands::{OutputFormat, format_bytes};
use crate::diagnostics::Severity;
use crate::manifest::Manifest;
use crate::resolver::Resolver;

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Path to the build.zig.zon manifest
    #[arg(
        short = 'm',
        long = "manifest-path",
        alias = "manifest",
        default_value = "build.zig.zon"
    )]
    pub manifest_path: PathBuf,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

impl InspectArgs {
    pub fn run(&self) -> Result<()> {
        let manifest = Manifest::load_from_path(&self.manifest_path).with_context(|| {
            format!(
                "Failed to load manifest at {}",
                self.manifest_path.display()
            )
        })?;

        let resolver = Resolver::new(&manifest);
        let resolved = resolver.resolve()?;

        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&resolved)?);
            }
            OutputFormat::Text => {
                println!("Manifest: {}", resolved.manifest_path);
                println!("Package root: {}", resolved.package_root);
                if let Some(ref name) = resolved.name {
                    print!("Package: {}", name);
                    if let Some(ref ver) = resolved.version {
                        print!(" ({})", ver);
                    }
                    println!();
                }
                if let Some(ref min_zig) = resolved.minimum_zig_version {
                    println!("Minimum Zig version: {}", min_zig);
                }
                if let Some(ref fp) = resolved.fingerprint {
                    println!("Fingerprint: {}", fp);
                }
                println!("Package files: {}", resolved.total_file_count);
                println!("Total size: {}", format_bytes(resolved.total_size_bytes));

                println!("\nIncluded paths:");
                if resolved.entries.is_empty() {
                    println!("  (none)");
                } else {
                    for entry in &resolved.entries {
                        let path_display = if entry.raw.is_empty() {
                            "\"\" (package root)"
                        } else {
                            &entry.raw
                        };

                        if !entry.exists {
                            println!("  {} [missing]", path_display);
                        } else if entry.is_escape {
                            println!("  {} [escapes root]", path_display);
                        } else {
                            println!(
                                "  {} ({} file{}, {})",
                                path_display,
                                entry.file_count,
                                if entry.file_count == 1 { "" } else { "s" },
                                format_bytes(entry.total_bytes)
                            );
                        }
                    }
                }

                if !resolved.diagnostics.is_empty() {
                    println!("\nFindings:");
                    for diag in &resolved.diagnostics {
                        let prefix = match diag.severity {
                            Severity::Error => "  error  ",
                            Severity::Warning => "  warning",
                            Severity::Notice => "  notice ",
                        };
                        println!("{prefix} {}", diag.message);
                    }
                }
            }
        }

        Ok(())
    }
}
