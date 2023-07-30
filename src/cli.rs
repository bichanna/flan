use std::path::PathBuf;

use clap::{arg, value_parser, Command};

static FLAN_VERSION: &str = "0.0.0";

pub struct Config {
    pub input: PathBuf,
}

pub fn parse_args() -> Config {
    let matches = Command::new("flan")
        .version(FLAN_VERSION)
        .author("Nobuharu Shimazu <nobu.bichanna@gmail.com>")
        .about("A simple, functional, dynamically and strongly typed scripting language")
        .arg(
            arg!([INPUT] "File to be executed")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let input = matches.get_one::<PathBuf>("INPUT").unwrap().clone();

    Config { input }
}
