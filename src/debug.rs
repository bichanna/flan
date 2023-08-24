use std::collections::HashMap;

use crate::compiler::opcode::OpCode;
use crate::compiler::util::{from_little_endian, from_little_endian_u32, MemorySlice};
use crate::error::Position;
use crate::vm::value::Value;

use crate::num_traits::FromPrimitive;

/// Used for debugging compiled bytecode
pub struct Debug<'a> {
    /// Pointer to the bytecode list
    bytecode: &'a Vec<u8>,
    /// Positions
    positions: &'a HashMap<usize, Position>,
    /// Constants
    consts: &'a Vec<Value>,
    /// Index
    pub offset: usize,
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
            print!("{:>8} ", pos);
        }

        match FromPrimitive::from_u8(self.bytecode[offset]).unwrap() {
            OpCode::Halt => self.simple_instruction("Halt"),
            OpCode::Const => self.simple_instruction("Const"),
            OpCode::LoadConst => self.const_instruction("LoadConst"),
            OpCode::LoadLongConst => self.lconst_instruction("LoadLongConst"),
            OpCode::Negate => self.simple_instruction("Negate"),
            OpCode::NegateBool => self.simple_instruction("NegateBool"),
            OpCode::Add => self.simple_instruction("Add"),
            OpCode::Sub => self.simple_instruction("Sub"),
            OpCode::Mult => self.simple_instruction("Mul"),
            OpCode::Div => self.simple_instruction("Div"),
            OpCode::Rem => self.simple_instruction("Rem"),
            OpCode::Pop => self.simple_instruction("Pop"),
            OpCode::PopN => self.single_arg_instruction("PopN"),
            OpCode::InitTup => self.single_arg_instruction("InitTup"),
            OpCode::InitList => self.init_list_instruction(),
            OpCode::InitObj => self.init_obj_instruction(),
            OpCode::PopExceptLast => self.simple_instruction("PopExceptLast"),
            OpCode::PopExceptLastN => self.single_arg_instruction("PopExceptLastN"),
            OpCode::LongJump => self.long_jump_instruction(),
            OpCode::Jump => self.jump_instruction("Jump"),
            OpCode::JumpIfFalse => self.jump_instruction("JumpIfFalse"),
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
            OpCode::LoadNil => self.simple_instruction("LoadNil"),
            OpCode::DefGlobal => self.single_arg_instruction("DefGlobal"),
            OpCode::SetGlobal => self.simple_instruction("SetGlobal"),
            OpCode::GetGlobal => self.simple_instruction("GetGlobal"),
            OpCode::DefLocal => self.simple_instruction("DefLocal"),
            OpCode::GetLocal => self.single_arg_instruction("GetLocal"),
            OpCode::SetLocalVar => self.single_arg_instruction("SetLocalVar"),
            OpCode::SetLocalTup => self.set_local_list_or_tup_instruction("SetLocalTup"),
            OpCode::SetLocalList => self.set_local_list_or_tup_instruction("SetLocalList"),
            OpCode::SetLocalObj => self.set_local_obj_instruction(),
            OpCode::Match => self.match_instruction(),
            OpCode::CallFn => self.single_arg_instruction("CallFn"),
            OpCode::GetProperty => self.simple_instruction("GetProperty"),
            OpCode::SetProperty => self.simple_instruction("SetProperty"),
            OpCode::WrapClosure => self.simple_instruction("WrapClosure"),
            OpCode::RetFn => self.simple_instruction("RetFn"),
        }
    }

    fn simple_instruction(&mut self, name: &'static str) {
        println!("{}", name);
        self.offset += 1;
    }

    fn const_instruction(&mut self, name: &'static str) {
        let idx = self.bytecode[self.offset + 1];
        println!("{:-16} {:>6} {}", name, idx, self.consts[idx as usize]);
        self.offset += 2;
    }

    fn lconst_instruction(&mut self, name: &'static str) {
        let idx = from_little_endian([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
        ]);
        println!("{:-16} {:>6} {}", name, idx, self.consts[idx as usize]);
        self.offset += 3;
    }

    fn single_arg_instruction(&mut self, name: &'static str) {
        let next_b = self.bytecode[self.offset + 1];
        println!("{:-16} {:>6}", name, next_b);
        self.offset += 2;
    }

    fn init_list_instruction(&mut self) {
        let next_b = self.bytecode[self.offset + 1];
        let mutable = self.bytecode[self.offset + 2] == 1;
        println!("{:-16} {:>6} {}", "InitList", next_b, mutable);
        self.offset += 3;
    }

    fn init_obj_instruction(&mut self) {
        let next_b = self.bytecode[self.offset + 1];
        let mutable = self.bytecode[self.offset + 2] == 1;
        println!("{:-16} {:>6} {}", "InitObj", next_b, mutable);
        self.offset += 3;
    }

    fn jump_instruction(&mut self, name: &'static str) {
        let jump = from_little_endian([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
        ]);
        self.offset += 3;
        let idx = self.offset + jump as usize;
        let opcode: OpCode = FromPrimitive::from_u8(self.bytecode[idx]).unwrap();
        println!("{:-16} {:>6} => {:?}", name, idx, opcode);
    }

    fn long_jump_instruction(&mut self) {
        let jump = from_little_endian_u32([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
            self.bytecode[self.offset + 3],
            self.bytecode[self.offset + 4],
        ]);
        self.offset += 5;
        let idx = self.offset + jump as usize;
        let opcode: OpCode = FromPrimitive::from_u8(self.bytecode[idx]).unwrap();
        println!("{:-16} {:>6} => {:?}", "LongJump", idx, opcode);
    }

    fn match_instruction(&mut self) {
        let len = from_little_endian_u32([
            self.bytecode[self.offset + 1],
            self.bytecode[self.offset + 2],
            self.bytecode[self.offset + 3],
            self.bytecode[self.offset + 4],
        ]);
        let has_next = self.bytecode[self.offset + 5] == 1;
        self.offset += 6;
        let idx = if has_next {
            self.offset + len as usize - 1 + 5
        } else {
            self.offset + len as usize - 1
        };
        let opcode: OpCode = FromPrimitive::from_u8(self.bytecode[idx]).unwrap();
        println!("{:-16} {:>6} => {:?} {}", "Match", idx, opcode, has_next);
    }

    fn set_local_list_or_tup_instruction(&mut self, name: &'static str) {
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
        println!("{:-16} {:>6} {}", name, len, idxs);
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
        println!("{:-16} {:>6} {}", "SetLocalObj", len, idxs);
        self.offset += len + 2;
    }
}
