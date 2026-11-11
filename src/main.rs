use clap::Parser;
use zig_lean::commands::Commands;

#[derive(Parser)]
#[command(name = "zig-lean", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Validate and optimize Zig package build.zig.zon manifests")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.command.run()
}
