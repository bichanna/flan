use byteorder::{ByteOrder, LittleEndian};

use super::opcode::{pos_str, OpCode};
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

        if offset > 0 && self.positions.get(&offset) == self.positions.get(&(offset - 1)) {
            print!("     | ");
        } else {
            print!("{:>6} ", pos_str(self.positions.get(&offset).unwrap()));
        }

        let instruction = OpCode::u8_to_opcode(self.bytecode[offset]);
        if let Some(instruction) = instruction {
            match instruction {
                OpCode::Return => self.debug_print_simple_instruction("OP_RETURN", offset),
                OpCode::Constant => self.debug_print_constant_instruction("OP_CONSTANT", offset),
                OpCode::ConstantLong => {
                    self.debug_print_lconstant_instruction("OP_LCONSTANT", offset)
                }
                OpCode::Negate => self.debug_print_simple_instruction("OP_NEGATE", offset),
                OpCode::Add => self.debug_print_simple_instruction("OP_ADD", offset),
                OpCode::Sub => self.debug_print_simple_instruction("OP_SUB", offset),
                OpCode::Mult => self.debug_print_simple_instruction("OP_MULT", offset),
                OpCode::Div => self.debug_print_simple_instruction("OP_DIV", offset),
                OpCode::Mod => self.debug_print_simple_instruction("OP_MOD", offset),
                OpCode::DefineGlobalVar => {
                    self.debug_print_simple_instruction("OP_DEFINE_GLOBAL", offset)
                }
                OpCode::GetGlobalVar => {
                    self.debug_print_simple_instruction("OP_GET_GLOBAL", offset)
                }
                OpCode::SetGlobalVar => {
                    self.debug_print_simple_instruction("OP_SET_GLOBAL", offset)
                }
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

    fn debug_print_lconstant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant =
            LittleEndian::read_u16(&[self.bytecode[offset + 1], self.bytecode[offset + 2]]);
        println!(
            "{:-16} {:>4} '{:#?}'",
            name,
            constant,
            self.values[constant as usize].print()
        );
        offset + 3
    }
}
