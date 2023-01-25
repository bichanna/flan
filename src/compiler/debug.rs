use super::chunk::{Chunk, OpCode};

impl OpCode {
    pub fn u8_to_opcode(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Return),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== {} ===", self.name)?;

        let mut offset: usize = 0;
        while offset < self.opcodes.len() {
            offset = self.disasemble_instruction(offset);
        }

        Ok(())
    }
}

impl Chunk {
    /// Disassembles one instruction
    fn disasemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        let instruction = OpCode::u8_to_opcode(self.opcodes[offset]);
        if let Some(instruction) = instruction {
            match instruction {
                OpCode::Return => Chunk::debug_print_simple_instruction("OP_RETURN", offset),
            }
        } else {
            println!("Unknown opcode {:?}", instruction);
            offset + 1
        }
    }

    fn debug_print_simple_instruction(name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }
}
