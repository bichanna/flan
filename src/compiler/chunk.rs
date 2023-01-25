pub type Position = (usize, usize);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Return,
}

#[derive(Clone, PartialEq)]
pub struct Chunk {
    opcodes: Vec<u8>,
    lines: Vec<(usize, Position)>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            opcodes: vec![],
            lines: vec![],
        }
    }

    /// Adds an opcode to the opcodes vector
    pub fn write_chunk(&mut self, opcode: OpCode) {
        let byte = opcode as u8;
        self.opcodes.push(byte);
    }
}
