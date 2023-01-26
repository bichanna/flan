use super::{OpCode, Position};
use crate::vm::value::Value;

#[derive(Clone, PartialEq)]
pub struct Chunk {
    /// The name of this Chunk, used for debugging
    pub name: &'static str,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,

    lines: Vec<(usize, Position)>,
}

impl Chunk {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            bytecode: vec![],
            lines: vec![],
            values: vec![],
        }
    }

    /// Adds an opcode to the opcodes vector
    pub fn write_chunk(&mut self, opcode: OpCode) {
        let byte = opcode as u8;
        self.write_byte(byte);
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.bytecode.push(byte);
    }

    /// Add a constant to the values vector and adds the index to the bytecode vector
    pub fn write_constant(&mut self, value: Value) {
        self.values.push(value);
        self.write_byte((self.values.len() - 1) as u8)
    }
}
