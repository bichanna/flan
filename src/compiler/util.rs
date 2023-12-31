use std::collections::HashMap;

use super::opcode::OpCode;
use crate::error::{Position, Positions};
use crate::vm::value::Value;

use crate::num_traits::ToPrimitive;

/// Holds information for the VM
pub struct MemorySlice {
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// For simplicity's sake, almost all constants are stored in here
    pub constants: Vec<Value>,
    /// Line and column numbers for reporting errors later
    pub positions: Positions,
}

/// Encodes a u16 value in little-endian byte order
pub fn to_little_endian(val: u16) -> [u8; 2] {
    [(val & 0xFF) as u8, ((val >> 8) & 0xFF) as u8]
}

/// Decodes a little-endian byte representation to a u16
pub fn from_little_endian(bytes: [u8; 2]) -> u16 {
    let (b1, b2) = (bytes[0], bytes[1]);
    ((b2 as u16 & 0xFF) << 8) | (b1 as u16 & 0xFF)
}

/// Encodes a u32 value in little-endian byte order
pub fn to_little_endian_u32(val: u32) -> [u8; 4] {
    [
        (val & 0xFF) as u8,
        ((val >> 8) & 0xFF) as u8,
        ((val >> 16) & 0xFF) as u8,
        ((val >> 24) & 0xFF) as u8,
    ]
}

/// Decodes a little-endian byte representation to a u32
pub fn from_little_endian_u32(bytes: [u8; 4]) -> u32 {
    (bytes[0] as u32)
        | ((bytes[1] as u32) << 8)
        | ((bytes[2] as u32) << 16)
        | ((bytes[3] as u32) << 24)
}

impl MemorySlice {
    pub fn new(tok_num: usize) -> Self {
        Self {
            bytecode: Vec::with_capacity(tok_num / 3),
            constants: Vec::with_capacity(tok_num / 10),
            positions: HashMap::new(),
        }
    }

    /// Adds a constant to the constant pool and adds the index to the bytecode list
    pub fn add_const(&mut self, val: Value, pos: Position) {
        self.constants.push(val);
        // check whether to LoadLongConst or or just LoadConst
        if self.constants.len() > u8::MAX as usize {
            self.write_opcode(OpCode::LoadLongConst, pos);
            to_little_endian((self.constants.len() - 1) as u16)
                .into_iter()
                .for_each(|b| self.write_byte(b, pos));
        } else {
            self.write_opcode(OpCode::LoadConst, pos);
            self.write_byte((self.constants.len() - 1) as u8, pos);
        }
    }

    /// Appends an opcode to `bytecode`
    pub fn write_opcode(&mut self, opcode: OpCode, pos: Position) {
        let byte = opcode.to_u8().unwrap();
        self.write_byte(byte, pos);
    }

    /// Writes a byte to `bytecode`
    pub fn write_byte(&mut self, b: u8, pos: Position) {
        self.bytecode.push(b);
        self.positions.insert(self.bytecode.len() - 1, pos);
    }

    /// Writes multple bytes to `bytecode`
    pub fn write_bytes(&mut self, bytes: &[u8], pos: Position) {
        bytes.iter().for_each(|b| self.write_byte(*b, pos));
    }

    /// Writes a byte to `bytecode` at the specified index
    pub fn write_byte_with_index(&mut self, index: usize, b: u8) {
        self.bytecode[index] = b;
    }
}

#[cfg(test)]
mod test {
    use crate::vm::value::FInt;

    use super::*;

    #[test]
    fn encode() {
        let slice = to_little_endian(1000);
        assert_eq!(slice, [0xE8, 0x03]);
        let slice = to_little_endian_u32(100000);
        assert_eq!(slice, [0xA0, 0x86, 0x1, 0x0]);
    }

    #[test]
    fn decode() {
        let val = from_little_endian([0xE8, 0x03]);
        assert_eq!(val, 1000);
        let val = from_little_endian_u32([0xA0, 0x86, 0x1, 0x0]);
        assert_eq!(val, 100000);
    }

    #[test]
    fn memory_slice() {
        let pos = (1, 1);
        let mut mem_slice: MemorySlice = MemorySlice::new(5);
        mem_slice.write_byte(1, pos);
        mem_slice.write_opcode(OpCode::Halt, pos);
        mem_slice.write_byte_with_index(0, 100);
        mem_slice.add_const(Box::new(FInt(100)), pos);
        mem_slice.write_bytes(&[1, 2, 3], pos);

        assert_eq!(mem_slice.bytecode, vec![100, 0, 1, 0, 1, 2, 3]);
        assert_eq!(mem_slice.constants.len(), 1);
    }
}
