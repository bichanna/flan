pub mod debug;
pub mod opcode;
pub mod value;

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use self::opcode::{OpCode, Position};
use self::value::Value;
use crate::frontend::ast::Expr;

#[derive(Clone, PartialEq)]
pub struct Compiler<'a> {
    /// The name of this Compiler, used for debugging
    name: &'static str,
    /// The AST nodes that are to be compiled to bytecode
    exprs: &'a Vec<Expr>,
    /// The compiled bytecode
    bytecode: Vec<u8>,
    /// For simplicity's sake, we'll put all constants in here
    values: Vec<Value>,
    /// Position information used for runtime errors
    positions: HashMap<usize, Position>,
}

impl<'a> Compiler<'a> {
    pub fn new<'b>(name: &'static str, exprs: &'a Vec<Expr>) -> Self {
        Self {
            name,
            exprs,
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

    /// Add a constant to the values vector and adds the index to the bytecode vector
    pub fn write_constant(&mut self, value: Value, pos: Position) {
        self.values.push(value);
        if self.values.len() > 255 {
            // use OP_LCONSTANT
            let byte = OpCode::ConstantLong as u8;
            self.write_byte(byte, pos);

            // convert the constant index into two u8's and writes the bytes to the bytecode vector
            let mut bytes = [0u8; 2];
            LittleEndian::write_u16(&mut bytes, (self.values.len() - 1) as u16);
            for byte in bytes {
                self.write_byte(byte, pos);
            }
        } else {
            // use OP_CONSTANT
            let byte = OpCode::Constant as u8;
            self.write_byte(byte, pos);

            self.write_byte((self.values.len() - 1) as u8, pos)
        }
    }

    /// Writes a byte to the bytecode vector
    fn write_byte(&mut self, byte: u8, pos: Position) {
        self.bytecode.push(byte);
        // self.positions.insert(self.bytecode.len() - 1, pos);
        self.positions.entry(self.bytecode.len() - 1).or_insert(pos);
    }
}
