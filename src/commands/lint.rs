use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

use crate::commands::OutputFormat;
use crate::diagnostics::{Diagnostic, Severity};
use crate::manifest::Manifest;
use crate::resolver::Resolver;
use crate::zig::ZigToolchain;

#[derive(Args, Debug)]
pub struct LintArgs {
    /// Path to the build.zig.zon manifest
    #[arg(
        short = 'm',
        long = "manifest-path",
        alias = "manifest",
        default_value = "build.zig.zon"
    )]
    pub manifest_path: PathBuf,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Optional path to the zig executable
    #[arg(long = "zig-exe")]
    pub zig_exe: Option<String>,
}

#[derive(Debug, Serialize)]
struct LintReport {
    manifest_path: String,
    valid: bool,
    error_count: usize,
    warning_count: usize,
    notice_count: usize,
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zig_version: Option<String>,
}

impl LintArgs {
    pub fn run(&self) -> Result<()> {
        let manifest = Manifest::load_from_path(&self.manifest_path).with_context(|| {
            format!(
                "Failed to load manifest at {}",
                self.manifest_path.display()
            )
        })?;

        let resolver = Resolver::new(&manifest);
        let resolved = resolver.resolve()?;

        let mut diagnostics = resolved.diagnostics;

        // Optional Zig toolchain check
        let zig_toolchain = ZigToolchain::detect(self.zig_exe.as_deref());
        let zig_version = zig_toolchain.as_ref().map(|z| z.version.clone());

        if let Some(ref zig) = zig_toolchain
            && let Ok(false) = zig.check_fmt(&manifest.path)
        {
            diagnostics.push(
                Diagnostic::notice(
                    "zig-fmt-unformatted",
                    "Manifest does not conform to 'zig fmt' standard formatting",
                )
                .with_path(manifest.path.display().to_string())
                .with_suggestion("Run 'zig fmt build.zig.zon' to reformat"),
            );
        }

        let error_count = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        let notice_count = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Notice)
            .count();

        let is_valid = if self.strict {
            error_count == 0 && warning_count == 0
        } else {
            error_count == 0
        };

        match self.format {
            OutputFormat::Json => {
                let report = LintReport {
                    manifest_path: manifest.path.display().to_string(),
                    valid: is_valid,
                    error_count,
                    warning_count,
                    notice_count,
                    diagnostics,
                    zig_version,
                };
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            OutputFormat::Text => {
                println!("Manifest: {}", manifest.path.display());
                if let Some(ref name) = manifest.name {
                    print!("Package: {}", name);
                    if let Some(ref ver) = manifest.version {
                        print!(" ({})", ver);
                    }
                    println!();
                }

                if diagnostics.is_empty() {
                    println!("\nAll .paths entries are valid and lean.");
                } else {
                    println!(
                        "\nFindings: {} error{}, {} warning{}, {} notice{}",
                        error_count,
                        if error_count == 1 { "" } else { "s" },
                        warning_count,
                        if warning_count == 1 { "" } else { "s" },
                        notice_count,
                        if notice_count == 1 { "" } else { "s" },
                    );

                    for diag in &diagnostics {
                        let prefix = match diag.severity {
                            Severity::Error => "  error  ",
                            Severity::Warning => "  warning",
                            Severity::Notice => "  notice ",
                        };
                        println!("{prefix} {}", diag.message);
                        if let Some(ref path) = diag.path {
                            if let (Some(l), Some(c)) = (diag.line, diag.column) {
                                println!("          at {path}:{l}:{c}");
                            } else if path != &manifest.path.display().to_string() {
                                println!("          at {path}");
                            }
                        }
                        if let Some(ref sugg) = diag.suggestion {
                            println!("          -> {sugg}");
                        }
                    }
                }
            }
        }

        if !is_valid {
            std::process::exit(1);
        }

        Ok(())
    }
}
