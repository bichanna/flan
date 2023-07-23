use std::collections::HashMap;

use crate::compiler::opcode::OpCode;
use crate::compiler::util::{from_little_endian, MemorySlice};
use crate::error::Position;
use crate::vm::value::ValueTrait;

use crate::num_traits::FromPrimitive;

/// Used for debugging compiled bytecode
pub struct Debug<'a> {
    /// Pointer to the bytecode list
    bytecode: &'a Vec<u8>,
    /// Positions
    positions: &'a HashMap<usize, Position>,
    /// Constants
    consts: &'a Vec<Box<dyn ValueTrait>>,
    /// Index
    offset: usize,
}

impl<'a> Debug<'a> {
    pub fn run(name: &'static str, mem_slice: &'a MemorySlice) {
        let mut debugger = Debug {
            bytecode: &mem_slice.bytecode,
            positions: &mem_slice.positions,
            consts: &mem_slice.constants,
            offset: 0,
        };

        println!("==== {} ====", name);

        while debugger.offset < debugger.bytecode.len() {
            debugger.disassemble_instruction();
        }
    }

    pub fn new(mem_slice: &'a MemorySlice) -> Self {
        Debug {
            bytecode: &mem_slice.bytecode,
            positions: &mem_slice.positions,
            consts: &mem_slice.constants,
            offset: 0,
        }
    }

    pub fn disassemble_instruction(&mut self) {
        print!("{:04}", self.offset);

        let offset = self.offset;

        if self.offset > 0 && self.positions[&offset] == self.positions[&(offset - 1)] {
            print!("      |  ");
        } else {
            let pos = self.positions[&offset];
            let pos = format!("({}, {})", pos.0, pos.1);
            print!("{:>8}", pos);
        }

        match FromPrimitive::from_u8(self.bytecode[offset]).unwrap() {
            OpCode::Return => self.simple_instruction("Return"),
            OpCode::LoadConst => self.const_instruction("LoadConst"),
            OpCode::LoadLongConst => self.const_instruction("LoadLongConst"),
            OpCode::Negate => self.simple_instruction("Negate"),
            OpCode::NegateBool => self.simple_instruction("NegateBool"),
            OpCode::Add => self.simple_instruction("Add"),
            OpCode::Sub => self.simple_instruction("Sub"),
            OpCode::Mult => self.simple_instruction("Mul"),
            OpCode::Div => self.simple_instruction("Div"),
            OpCode::Rem => self.simple_instruction("Rem"),
            OpCode::Pop => self.simple_instruction("Pop"),
            OpCode::PopN => self.single_arg_instruction("PopN"),
            OpCode::InitList => self.single_arg_instruction("InitList"),
            OpCode::InitObj => self.single_arg_instruction("InitObj"),
            OpCode::PopExceptLast => self.simple_instruction("PopExceptLast"),
            OpCode::PopExceptLastN => self.single_arg_instruction("PopExceptLastN"),
            OpCode::Jump => self.double_arg_instruction("Jump"),
            OpCode::JumpIfFalse => self.double_arg_instruction("JumpIfFalse"),
            OpCode::Equal => self.simple_instruction("Equal"),
            OpCode::NotEqual => self.simple_instruction("NotEqual"),
            OpCode::GT => self.simple_instruction("GT"),
            OpCode::LT => self.simple_instruction("LT"),
            OpCode::GTEq => self.simple_instruction("GTEq"),
            OpCode::LTEq => self.simple_instruction("LTEq"),
            OpCode::And => self.simple_instruction("And"),
            OpCode::Or => self.simple_instruction("Or"),
            OpCode::LoadInt0 => self.simple_instruction("LoadInt0"),
            OpCode::LoadInt1 => self.simple_instruction("LoadInt1"),
            OpCode::LoadInt2 => self.simple_instruction("LoadInt2"),
            OpCode::LoadInt3 => self.simple_instruction("LoadInt3"),
            OpCode::LoadTrue => self.simple_instruction("LoadTrue"),
            OpCode::LoadFalse => self.simple_instruction("LoadFalse"),
            OpCode::LoadEmpty => self.simple_instruction("LoadEmpty"),
            OpCode::DefGlobal => self.simple_instruction("DefGlobal"),
            OpCode::SetGlobal => self.simple_instruction("SetGlobal"),
            OpCode::GetGlobal => self.simple_instruction("GetGlobal"),
            OpCode::DefLocal => self.simple_instruction("DefLocal"),
            OpCode::GetLocal => self.single_arg_instruction("GetLocal"),
            OpCode::SetLocalVar => self.single_arg_instruction("SetLocalVar"),
            OpCode::SetLocalList => self.set_local_list_instruction(),
            OpCode::SetLocalObj => self.set_local_obj_instruction(),
            OpCode::Match => self.match_instruction(),
            OpCode::Call => todo!(),
            OpCode::Get => self.simple_instruction("Get"),
            OpCode::Set => self.simple_instruction("Set"),
        }
    }

    fn simple_instruction(&mut self, name: &'static str) {
        println!("{}", name);
        self.offset += 1;
    }

    fn const_instruction(&mut self, name: &'static str) {
        let idx = self.bytecode[self.offset + 1];
        println!("{:-16} {:>4} {}", name, idx, self.consts[idx as usize]);
        self.offset += 2;
    }

    fn lconst_instruction(&mut self, name: &'static str) {
        let idx = from_little_endian([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
        ]);
        println!("{:-16} {:>4} {}", name, idx, self.consts[idx as usize]);
        self.offset += 3;
    }

    fn single_arg_instruction(&mut self, name: &'static str) {
        let next_b = self.bytecode[self.offset + 1];
        println!("{:-16} {:>4}", name, next_b);
        self.offset += 2;
    }

    fn double_arg_instruction(&mut self, name: &'static str) {
        let jump = from_little_endian([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
        ]);
        println!("{:-16} {:>4}", name, jump);
        self.offset += 3;
    }

    fn match_instruction(&mut self) {
        let len = from_little_endian([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
        ]);
        println!(
            "{:-16} {:>4} {}",
            "Match",
            len,
            self.bytecode[self.offset + 3] == 1
        );
        self.offset += 4;
    }

    fn set_local_list_instruction(&mut self) {
        let len = self.bytecode[self.offset + 1] as usize;
        self.offset += 1;
        let idxs = vec![
            "{".to_string(),
            (1..(len * 2))
                .step_by(2)
                .map(|i| {
                    format!(
                        "({}, {})",
                        self.bytecode[self.offset + 1],
                        self.bytecode[self.offset + i + 1],
                    )
                })
                .collect::<Vec<String>>()
                .join(", "),
            "}".to_string(),
        ]
        .join(", ");
        println!("{:-16} {:>4} {}", "SetLocalList", len, idxs);
        self.offset += len * 2 + 1;
    }

    fn set_local_obj_instruction(&mut self) {
        let len = self.bytecode[self.offset + 1] as usize;
        let idxs = vec![
            "{".to_string(),
            (1..len)
                .map(|i| format!("{}", self.bytecode[self.offset + i + 1]))
                .collect::<Vec<String>>()
                .join(", "),
            "}".to_string(),
        ]
        .join(", ");
        println!("{:-16} {:>4} {}", "SetLocalObj", len, idxs);
        self.offset += len + 2;
    }
}
