pub mod debug;
pub mod opcode;
pub mod value;

use std::collections::HashMap;

use self::opcode::{OpCode, Position};
use self::value::Value;

#[derive(Clone, PartialEq)]
pub struct Compiler {
    /// The name of this Compiler, used for debugging
    pub name: &'static str,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,
    /// Position information used for runtime errors
    pub positions: HashMap<usize, Position>,
}

impl Compiler {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            bytecode: vec![],
            positions: HashMap::new(),
            values: vec![],
        }
    }

    /// Writes an opcode to the bytecode vector
    pub fn write_opcode(&mut self, opcode: OpCode, pos: Position) {
        let byte = opcode as u8;
        self.write_byte(byte, pos);
    }

    /// Writes a byte to the bytecode vector
    pub fn write_byte(&mut self, byte: u8, pos: Position) {
        self.bytecode.push(byte);
        self.positions.insert(self.bytecode.len() - 1, pos);
    }

    /// Add a constant to the values vector and adds the index to the bytecode vector
    pub fn write_constant(&mut self, value: Value, pos: Position) {
        self.values.push(value);
        self.write_byte((self.values.len() - 1) as u8, pos)
    }
}
