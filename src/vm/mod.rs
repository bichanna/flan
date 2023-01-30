use std::collections::HashMap;
use std::ptr;

use byteorder::{ByteOrder, LittleEndian};

use crate::compiler::opcode::{OpCode, Position};
use crate::compiler::value::Value;

// Constants
const STACK_MAX: usize = 256;

macro_rules! read_byte {
    ($self: expr) => {{
        $self.ip = unsafe { $self.ip.add(1) };
        unsafe { *$self.ip }
    }};
}

pub struct VM<'a> {
    /// The bytecode to be run
    bytecode: &'a Vec<u8>,
    /// The constants pool, holds all constants in a program
    values: &'a Vec<Value>,
    /// Position information only used when runtime errors occur
    positions: &'a HashMap<usize, Position>,
    /// The file name of the source code
    filename: &'a str,
    /// Source code
    source: &'a String,
    /// Instruction pointer, holds the current instruction being executed
    ip: *const u8,
    /// This stack can be safely accessed without bound checking
    stack: Box<[Value; STACK_MAX]>,
    stack_top: *mut Value,
}

impl<'a> VM<'a> {
    pub fn new(
        filename: &'a str,
        source: &'a String,
        bytecode: &'a Vec<u8>,
        values: &'a Vec<Value>,
        positions: &'a HashMap<usize, Position>,
    ) -> Self {
        Self {
            bytecode,
            values,
            positions,
            ip: bytecode[0] as *const u8,
            filename,
            source,
            stack: Box::new([Value::Null; STACK_MAX]),
            stack_top: ptr::null_mut(),
        }
    }

    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();
        self.stack_top = &mut self.stack[0] as *mut Value;

        loop {
            match instruction {
                OpCode::Return => {
                    println!("{}", self.pop().print());
                }
                OpCode::Constant => {
                    let value = self.read_constant(false);
                    self.push(value);
                }
                OpCode::ConstantLong => {
                    let value = self.read_constant(true);
                    self.push(value);
                }
            }

            instruction = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
        }
    }

    /// Pushes a Value onto the stack
    fn push(&mut self, mut value: Value) {
        self.stack_top = &mut value as *mut Value;
        self.stack_top = unsafe { self.stack_top.add(1) };
    }

    /// Pops a Value from the stack
    fn pop(&mut self) -> Value {
        self.stack_top = unsafe { self.stack_top.sub(1) };
        unsafe { *self.stack_top }
    }

    /// Reads a Value and returns it
    fn read_constant(&mut self, long: bool) -> Value {
        if long {
            let bytes = [read_byte!(self), read_byte!(self)];
            let constant = LittleEndian::read_u16(&bytes) as usize;
            self.values[constant].clone()
        } else {
            self.values[read_byte!(self) as usize].clone()
        }
    }
}
