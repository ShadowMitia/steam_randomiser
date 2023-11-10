use clap::Parser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Randomly picks an installed game from your Steam library and launches it.
#[derive(Parser, Debug)]
#[clap(about,
    version = VERSION
)]
#[command(name = "completion-derive")]
pub struct Opts {
    /// Shows program output (game launched, errors...)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Runs the program but doesn't actually launch the game.
    #[clap(short, long)]
    pub dry_run: bool,
}
