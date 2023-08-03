use std::path::PathBuf;

use clap::Parser;

static FLAN_VERSION: &str = "0.0.0";

#[derive(Parser)]
#[command(author = "Nobuharu Shimazu <nobu.bichanna@gmail.com>")]
#[command(version = FLAN_VERSION)]
#[command(about = "A simple, expression oriented programming language", long_about = None)]
pub struct Cli {
    /// Input file
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,
}

pub fn parse_args() -> Cli {
    let cli = Cli::parse();
    cli
}
