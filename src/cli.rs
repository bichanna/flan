use std::path::PathBuf;

use clap::Parser;

use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser as FParser;
use crate::vm::gc::heap::Heap;
use crate::vm::VM;

static FLAN_VERSION: &str = "0.0.0";

#[derive(Parser)]
#[command(author = "Nobuharu Shimazu <nobu.bichanna@gmail.com>")]
#[command(version = FLAN_VERSION)]
#[command(about = "A simple, intuitive, expression-oriented programming language", long_about = None)]
pub struct Cli {
    /// Input file
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,
}

/// Parses passed arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Runs the given file
pub fn run_file(path: PathBuf) {
    // tokenizing
    let tokens = Lexer::tokenize(path);
    let tok_num = tokens.len();
    // parsing
    let exprs = FParser::parse(tokens);
    // creating the heap
    let heap = Heap::new();
    // compiling
    let (mem_slice, mut heap) = Compiler::compile(exprs, heap, tok_num);
    // executing
    VM::execute(mem_slice, &mut heap);
}
