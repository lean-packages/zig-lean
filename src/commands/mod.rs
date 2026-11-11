pub mod inspect;
pub mod lint;
pub mod update;

use anyhow::Result;
use clap::Subcommand;

use self::inspect::InspectArgs;
use self::lint::LintArgs;
use self::update::UpdateArgs;

#[derive(Subcommand)]
pub enum Commands {
    /// Lint and validate build.zig.zon .paths entries
    Lint(LintArgs),
    /// Inspect effective package contents, files, and sizes
    Inspect(InspectArgs),
    /// Propose and safely update build.zig.zon .paths
    Update(UpdateArgs),
}

impl Commands {
    pub fn run(&self) -> Result<()> {
        match self {
            Commands::Lint(args) => args.run(),
            Commands::Inspect(args) => args.run(),
            Commands::Update(args) => args.run(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "human" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "Invalid format: {}. Supported formats: text, json",
                s
            )),
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}
