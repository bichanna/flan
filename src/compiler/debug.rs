use super::opcode::OpCode;
use super::Compiler;

impl std::fmt::Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== {} ===", self.name)?;

        let mut offset: usize = 0;
        while offset < self.bytecode.len() {
            offset = self.disasemble_instruction(offset);
        }

        Ok(())
    }
}

impl Compiler {
    /// Disassembles one instruction
    fn disasemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        let instruction = OpCode::u8_to_opcode(self.bytecode[offset]);
        if let Some(instruction) = instruction {
            match instruction {
                OpCode::Return => self.debug_print_simple_instruction("OP_RETURN", offset),
                OpCode::Constant => self.debug_print_constant_instruction("OP_CONSTANT", offset),
            }
        } else {
            println!("Unknown opcode {:?}", instruction);
            offset + 1
        }
    }

    fn debug_print_simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn debug_print_constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant = self.bytecode[offset + 1];
        println!(
            "{:-16} {:>4} '{:#?}'",
            name,
            constant,
            self.values[constant as usize].print()
        );
        offset + 2
    }
}
