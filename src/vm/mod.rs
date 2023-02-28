pub mod destruct;
pub mod object;
pub mod value;

use std::collections::HashMap;
use std::ptr;

use byteorder::{ByteOrder, LittleEndian};

use self::destruct::Assignee;
use self::value::Value;
use crate::compiler::opcode::{OpCode, Position};

// Constants
const STACK_MAX: usize = 256;

macro_rules! read_byte {
    ($self: expr) => {{
        $self.ip = unsafe { $self.ip.add(1) };
        unsafe { *$self.ip }
    }};
}

macro_rules! push_or_err {
    ($self: expr, $value: expr) => {
        match $value {
            Ok(v) => $self.push(v),
            Err(_msg) => {
                // TODO: Report error
            }
        }
    };
}

macro_rules! binary_op {
    ($self: expr, $op: tt) => {
        let right = $self.pop();
        let left = $self.pop();
        push_or_err!($self, left $op right);
    };
}

pub struct VM<'a> {
    /// The bytecode to be run
    bytecode: &'a Vec<u8>,
    /// The constants pool, holds all constants in a program
    values: &'a Vec<Value>,
    /// The destructs pool, holds all assignees in a program (will be removed)
    destructs: &'a Vec<Assignee>,
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
    /// All global variables
    globals: HashMap<String, Value>,
    /// Stack for Assignees (will be removed)
    destruct_stack: Vec<Assignee>,
}

impl<'a> VM<'a> {
    pub fn new(
        filename: &'a str,
        source: &'a String,
        bytecode: &'a Vec<u8>,
        values: &'a Vec<Value>,
        destructs: &'a Vec<Assignee>,
        positions: &'a HashMap<usize, Position>,
    ) -> Self {
        Self {
            bytecode,
            values,
            destructs,
            positions,
            ip: bytecode.as_ptr(),
            filename,
            source,
            stack: Box::new([Value::Null; STACK_MAX]),
            stack_top: ptr::null_mut(),
            globals: HashMap::new(),
            destruct_stack: Vec::new(),
        }
    }

    /// The heart of the VM
    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();
        self.stack_top = &mut self.stack[0] as *mut Value;

        loop {
            match instruction {
                OpCode::Return => {
                    break;
                }
                OpCode::Constant => {
                    let value = self.read_constant(false);
                    self.push(value);
                }
                OpCode::ConstantLong => {
                    let value = self.read_constant(true);
                    self.push(value);
                }
                OpCode::Destruct => {
                    let assignee = self.read_destruct(false);
                    self.push_destruct(assignee);
                }
                OpCode::LDestruct => {
                    let assignee = self.read_destruct(true);
                    self.push_destruct(assignee);
                }
                OpCode::Negate => {
                    push_or_err!(self, -self.pop());
                }
                OpCode::Add => {
                    binary_op!(self, +);
                }
                OpCode::Sub => {
                    binary_op!(self, -);
                }
                OpCode::Mult => {
                    binary_op!(self, *);
                }
                OpCode::Div => {
                    binary_op!(self, /);
                }
                OpCode::Mod => {
                    binary_op!(self, %);
                }
                OpCode::DefineGlobalVar => {}
                OpCode::GetGlobalVar => {}
                OpCode::SetGlobalVar => {}
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::PopN => {
                    let n = read_byte!(self);
                    self.popn(n);
                }
                OpCode::DefineLocal => {}
                OpCode::GetLocal => {}
                OpCode::SetLocalVar => {}
                OpCode::SetLocalList => {}
                OpCode::SetLocalObj => {}
            }

            instruction = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
        }

        // rudementary garbage collection
        for value in self.values {
            match value {
                Value::Object(obj) => obj.free(),
                _ => {}
            }
        }
    }

    /// Pushes a Value onto the stack
    fn push(&mut self, value: Value) {
        unsafe { *self.stack_top = value }
        self.stack_top = unsafe { self.stack_top.add(1) };
    }

    /// Pushes an Assignee onto the destructs stack
    fn push_destruct(&mut self, destruct: Assignee) {
        self.destruct_stack.push(destruct);
    }

    /// Pops a Value from the stack
    fn pop(&mut self) -> Value {
        self.popn(1);
        unsafe { *self.stack_top }
    }

    /// Pops n times from the stack
    fn popn(&mut self, n: u8) {
        self.stack_top = unsafe { self.stack_top.sub(n as usize) };
    }

    /// Pops an Assignee from the destructs stack
    fn pop_destruct(&mut self) -> Assignee {
        self.destruct_stack.pop().unwrap()
    }

    fn read_2bytes(&mut self) -> u16 {
        let bytes = [read_byte!(self), read_byte!(self)];
        LittleEndian::read_u16(&bytes)
    }

    /// Reads a Value and returns it
    fn read_constant(&mut self, long: bool) -> Value {
        if long {
            let constant = self.read_2bytes();
            self.values[constant as usize]
        } else {
            self.values[read_byte!(self) as usize]
        }
    }

    /// Reads an Assignee and returns it
    fn read_destruct(&mut self, long: bool) -> Assignee {
        if long {
            let constant = self.read_2bytes();
            self.destructs[constant as usize].clone()
        } else {
            self.destructs[read_byte!(self) as usize].clone()
        }
    }

    /// Peeks a Value from the stack
    fn peek(&mut self, n: usize) -> *mut Value {
        unsafe { self.stack_top.sub(n + 1) }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary() {
        let bytecode: Vec<u8> = vec![1, 0, 1, 1, 4, 12, 0];
        let values: Vec<Value> = vec![Value::Int(1), Value::Int(1)];
        let destructs: Vec<Assignee> = vec![];
        let positions = HashMap::new();
        let source = "1 + 1".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &destructs, &positions);
        vm.run();

        assert_eq!(unsafe { *vm.stack_top }, Value::Int(2));
    }

    #[test]
    fn test_unary() {
        let bytecode: Vec<u8> = vec![1, 0, 3, 12, 0];
        let values: Vec<Value> = vec![Value::Bool(false)];
        let destructs: Vec<Assignee> = vec![];
        let positions = HashMap::new();
        let source = "not false".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &destructs, &positions);
        vm.run();

        assert_eq!(unsafe { *vm.stack_top }, Value::Bool(true));
    }
}
