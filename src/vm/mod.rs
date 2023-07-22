use std::collections::HashMap;
use std::mem::replace;
use std::sync::Arc;

use num_traits::FromPrimitive;

use crate::as_t;
use crate::compiler::opcode::OpCode;
use crate::compiler::util::{from_little_endian, MemorySlice};
use crate::error::Positions;

use self::value::*;

pub mod value;

macro_rules! read_byte {
    ($self: expr) => {{
        $self.ip = unsafe { $self.ip.add(1) };
        unsafe { *$self.ip }
    }};
}

macro_rules! read_2bytes {
    ($self: expr) => {
        from_little_endian([read_byte!($self), read_byte!($self)])
    };
}

macro_rules! try_push {
    ($self: expr, $val: expr) => {
        match $val {
            Ok(v) => $self.push(v),
            Err(_msg) => {} // TODO: report an error
        }
    };
}

macro_rules! binary_op {
    ($self: expr, $op: tt) => {
        let right = $self.pop();
        let left = $self.pop();
        try_push!($self, left $op right);
    };
}

struct VM {
    /// Constants
    constants: Vec<Value>,
    /// Positions for error reporting
    positions: Positions,
    /// Instruction pointer, holds the current instruction being executed
    ip: *const u8,
    /// Dynamically sized stack
    stack: Vec<Value>,
    /// All global variables are stored in here
    globals: HashMap<String, Value>,
    /// Path index
    path_idx: usize,
}

impl VM {
    pub fn execute(path_idx: usize, mem_slice: MemorySlice) {
        let mut vm = VM {
            constants: mem_slice.constants,
            positions: mem_slice.positions,
            ip: mem_slice.bytecode.as_ptr(),
            stack: Vec::with_capacity(u8::MAX as usize),
            globals: HashMap::with_capacity(12),
            path_idx,
        };
        vm._execute();
    }

    fn _execute(&mut self) {
        let mut inst: OpCode = FromPrimitive::from_u8(unsafe { *self.ip }).unwrap();
        loop {
            match inst {
                OpCode::Return => break,

                OpCode::LoadConst => {
                    let val = self.read_const(false);
                    self.push(val);
                }

                OpCode::LoadLongConst => {
                    let val = self.read_const(true);
                    self.push(val);
                }

                OpCode::Negate => {
                    let val = self.pop();
                    if let Ok(num) = -val {
                        self.push(num);
                    } else {
                        // TODO: report an error
                    }
                }

                OpCode::NegateBool => {
                    let val = self.pop();
                    if let Ok(num) = !val {
                        self.push(num);
                    } else {
                        // TODO: report an error
                    }
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

                OpCode::Rem => {
                    binary_op!(self, %);
                }

                OpCode::Pop => {
                    self.pop();
                }

                OpCode::PopN => {
                    let n = read_byte!(self) as i32;
                    self.popn(n);
                }

                OpCode::InitList => {
                    let len = read_2bytes!(self) as usize;
                    let mut list: Vec<Value> = Vec::with_capacity(len);
                    // adding elements to the list
                    (0..len).for_each(|_| list.push(self.pop()));
                    list.reverse();
                    self.push(FList::new(list));
                }

                OpCode::InitObj => {
                    let len = read_2bytes!(self) as usize;
                    let mut obj: HashMap<Arc<str>, Value> = HashMap::with_capacity(len);
                    (0..len).for_each(|_| {
                        // getting the value
                        let val = self.pop();
                        // getting the key
                        let key = self.pop();
                        if let Some(key) = as_t!(key, FVar) {
                            obj.insert(key.0.clone(), val);
                        } else {
                            // TODO: report error
                        }
                    });
                    self.push(FObj::new(obj));
                }

                OpCode::PopExceptLast => {
                    let last = self.pop();
                    self.pop();
                    self.push(last);
                }

                OpCode::PopExceptLastN => {
                    let last = self.pop();
                    let n = read_byte!(self) as i32;
                    self.popn(n);
                    self.push(last);
                }

                OpCode::Jump => {
                    let jump = read_2bytes!(self);
                    unsafe { self.ip.add(jump as usize) };
                }

                OpCode::JumpIfFalse => {
                    let jump = read_2bytes!(self);
                    if !self.pop().truthy() {
                        unsafe { self.ip.add(jump as usize) };
                    }
                }

                OpCode::Equal => {}

                _ => {}
            }

            inst = FromPrimitive::from_u8(read_byte!(self)).unwrap();
        }
    }

    /// Returns a Value from `constants`
    fn read_const(&mut self, long: bool) -> Value {
        let idx = if long {
            read_2bytes!(self) as usize
        } else {
            read_byte!(self) as usize
        };

        replace(&mut self.constants[idx], FEmpty::new())
    }

    /// Pops a `Value` off from `stack`
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    /// Pops `Value`'s `n` times off `stack`
    fn popn(&mut self, n: i32) {
        (0..n).for_each(|_| {
            self.stack.pop();
        });
    }

    /// Pushes a `Value` onto `stack`
    fn push(&mut self, val: Value) {
        self.stack.push(val);

        // growing the stack
        if self.stack.capacity() == self.stack.len() {
            self.stack.reserve(self.stack.len() / 3);
        }
    }
}
