use byteorder::{ByteOrder, LittleEndian};

use super::opcode::{pos_str, OpCode};
use super::Compiler;

#[macro_export]
macro_rules! compile {
    ($source:expr, $expected:expr) => {
        let source = String::from($source);
        // for tokenizing
        let (ts, tr) = crossbeam_channel::unbounded();
        // for parsing
        let (ps, pr) = crossbeam_channel::unbounded();

        let mut compiler = Compiler::new(&source, "input", "test", FuncType::TopLevel, &pr);

        std::thread::scope(|s| {
            s.spawn(|| {
                Lexer::new(&source, "input", &ts);
            });

            s.spawn(|| {
                Parser::new(&source, "input", &tr, &ps);
            });
        });

        compiler.compile();

        assert_eq!(compiler.function.bytecode, $expected);
    };
}

impl<'a> Compiler<'a> {
    pub fn debug(&mut self) {
        println!("=== {} ===", self.name);

        let mut offset: usize = 0;
        while offset < self.function.bytecode.len() {
            offset = self.disasemble_instruction(offset);
        }
    }
}

impl<'a> Compiler<'a> {
    /// Disassembles one instruction
    fn disasemble_instruction(&mut self, mut offset: usize) -> usize {
        print!("{:04} ", offset);

        if offset > 0 && self.positions.get(&offset) == self.positions.get(&(offset - 1)) {
            print!("     | ");
        } else {
            print!("{:>6} ", pos_str(self.positions.get(&offset).unwrap()));
        }

        let instruction = OpCode::u8_to_opcode(self.function.bytecode[offset]);
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
                OpCode::DefineGlobal => {
                    self.debug_print_simple_instruction("OP_DEFINE_GLOBAL", offset)
                }
                OpCode::GetGlobal => self.debug_print_simple_instruction("OP_GET_GLOBAL", offset),
                OpCode::SetGlobal => self.debug_print_simple_instruction("OP_SET_GLOBAL", offset),
                OpCode::Pop => self.debug_print_simple_instruction("OP_POP", offset),
                OpCode::PopExceptLast => {
                    self.debug_print_simple_instruction("OP_POP_EXCEPT_LAST", offset)
                }
                OpCode::PopN => self.debug_print_length_instruction("OP_POPN", offset),
                OpCode::PopExceptLastN => {
                    self.debug_print_length_instruction("OP_POP_EXCEPT_LASTN", offset)
                }
                OpCode::GetLocal => self.debug_print_length_instruction("OP_GET_LOCAL", offset),
                OpCode::SetLocalVar => {
                    self.debug_print_length_instruction("OP_SET_LOCAL_VAR", offset)
                }
                OpCode::SetLocalList => {
                    self.debug_print_set_local_list(&mut offset);
                    offset
                }
                OpCode::SetLocalObj => {
                    self.debug_print_set_local_obj(&mut offset);
                    offset
                }
                OpCode::DefineLocal => {
                    self.debug_print_simple_instruction("OP_DEFINE_LOCAL", offset)
                }
                OpCode::InitList => {
                    self.debug_print_long_length_instruction("OP_INIT_LIST", offset)
                }
                OpCode::InitObj => self.debug_print_long_length_instruction("OP_INIT_OBJ", offset),
                OpCode::Match => self.debug_print_simple_instruction("OP_MATCH", offset),
                OpCode::Jump => self.debug_print_long_length_instruction("OP_JUMP", offset),
                OpCode::Load1 => self.debug_print_simple_instruction("OP_LOAD1", offset),
                OpCode::Load2 => self.debug_print_simple_instruction("OP_LOAD2", offset),
                OpCode::Load3 => self.debug_print_simple_instruction("OP_LOAD3", offset),
                OpCode::LoadU8 => self.debug_print_length_instruction("OP_LOAD_U8", offset),
                OpCode::LoadTrue => self.debug_print_simple_instruction("OP_LOAD_TRUE", offset),
                OpCode::LoadFalse => self.debug_print_simple_instruction("OP_LOAD_FALSE", offset),
                OpCode::LoadEmpty => self.debug_print_simple_instruction("OP_LOAD_EMPTY", offset),
                OpCode::LoadNull => self.debug_print_simple_instruction("OP_LOAD_NULL", offset),
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

    fn debug_print_constant_instruction(&mut self, name: &str, offset: usize) -> usize {
        let constant = self.function.bytecode[offset + 1];
        println!(
            "{:-16} {:>4} '{:#?}'",
            name,
            constant,
            self.function.values[constant as usize].print()
        );
        offset + 2
    }

    fn debug_print_lconstant_instruction(&mut self, name: &str, offset: usize) -> usize {
        let constant = LittleEndian::read_u16(&[
            self.function.bytecode[offset + 1],
            self.function.bytecode[offset + 2],
        ]);
        println!(
            "{:-16} {:>4} '{:#?}'",
            name,
            constant,
            self.function.values[constant as usize].print()
        );
        offset + 3
    }

    fn debug_print_length_instruction(&mut self, name: &str, offset: usize) -> usize {
        let length = self.function.bytecode[offset + 1] as usize;
        println!("{:-16} length: {}", name, length);
        offset + 2
    }

    fn debug_print_long_length_instruction(&mut self, name: &str, offset: usize) -> usize {
        let bytes = [
            self.function.bytecode[offset + 1],
            self.function.bytecode[offset + 2],
        ];
        let length = LittleEndian::read_u16(&bytes) as usize;
        println!("{:-16} length: {}", name, length);
        offset + 3
    }

    fn debug_print_set_local_list(&mut self, offset: &mut usize) {
        let length = self.function.bytecode[*offset + 1] as usize;
        println!("{:-16} length: {}", "OP_SET_LOCAL_LIST", length);
        *offset += 2;
        for _ in 0..length {
            *offset = self.disasemble_instruction(*offset);
            println!("      u8arg: {}", self.function.bytecode[*offset + 1]);
            *offset += 1;
        }
    }

    fn debug_print_set_local_obj(&mut self, offset: &mut usize) {
        let length = self.function.bytecode[*offset + 1] as usize;
        println!("{:-16} length: {}", "OP_SET_LOCAL_OBJ", length);
        *offset += 2;
        for _ in 0..length {
            *offset = self.disasemble_instruction(*offset);
            *offset = self.disasemble_instruction(*offset);
            println!("      u8arg: {}", self.function.bytecode[*offset + 1]);
            *offset += 1;
        }
    }
}
