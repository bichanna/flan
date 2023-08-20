use cli::{parse_args, run_file};

mod cli;
mod compiler;
mod debug;
mod error;
mod lexer;
mod parser;
mod util;
mod vm;

#[macro_use]
extern crate num_derive;
extern crate num_traits;

fn main() {
    let config = parse_args();
    if let Some(path) = config.input {
        run_file(path);
    }
}
