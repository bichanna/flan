use std::collections::HashMap;

use crate::compiler::opcode::Position;
use crate::compiler::value::Value;
use crate::compiler::Compiler;

pub struct VM<'a> {
    /// The bytecode to be run
    bytecode: &'a Vec<u8>,
    /// The constants pool, holds all constants in a program
    values: &'a Vec<Value>,
    /// Position information only used when runtime errors occur
    positions: &'a HashMap<usize, Position>,

    filename: &'a str,
    source: &'a String,
}

impl<'a> VM<'a> {
    pub fn new(filename: &'a str, source: &'a String, compiler: &'a Compiler) -> Self {
        Self {
            bytecode: &compiler.bytecode,
            values: &compiler.values,
            positions: &compiler.positions,
            filename,
            source,
        }
    }

    pub fn run(&mut self) {}
}
