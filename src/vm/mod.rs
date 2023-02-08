pub mod object;
pub mod value;

use std::collections::HashMap;
use std::ptr;

use byteorder::{ByteOrder, LittleEndian};

use self::object::ObjectType;
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
            globals: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();
        self.stack_top = &mut self.stack[0] as *mut Value;

        loop {
            match instruction {
                OpCode::Return => {
                    println!("{}", self.pop().print());
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
                OpCode::DefineGlobalVar => {
                    self.define_global();
                }
            }

            instruction = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
        }

        for value in self.values {
            match value {
                Value::Object(obj) => obj.free(),
                _ => {}
            }
        }
    }

    /// Defines globals
    fn define_global(&mut self) {
        let right = self.pop();
        let left = self.pop();
        match left {
            Value::Object(left_obj) => match left_obj.obj_type {
                ObjectType::Identifier => {
                    // used for plain variable
                    // TODO: check for variables already defined and return error
                    self.globals
                        .insert(unsafe { (*(*left_obj.obj).string).clone() }, right);

                    self.push(right);
                }
                ObjectType::List => match right {
                    Value::Object(right_obj) => match right_obj.obj_type {
                        ObjectType::List => {
                            let left = unsafe { (*(*left_obj.obj).list).clone() };
                            let right = unsafe { (*(*right_obj.obj).list).clone() };

                            // check if both of them are the same length
                            if left.len() != right.len() {
                                // TODO: report error
                            }

                            for (i, id) in left.into_iter().enumerate() {
                                match *id {
                                    Value::Empty => continue,
                                    Value::Object(obj) => match obj.obj_type {
                                        ObjectType::Atom => {
                                            // TODO: check for variables already
                                            // defined and return error
                                            self.globals.insert(
                                                unsafe { (*(*obj.obj).string).clone() },
                                                *right[i],
                                            );
                                        }
                                        _ => {
                                            // TODO: report error
                                        }
                                    },
                                    _ => {
                                        // TODO: report error
                                    }
                                }
                            }
                        }
                        _ => {
                            // TODO: report error
                        }
                    },
                    _ => {
                        // TODO: report error
                    }
                },
                ObjectType::Object => match right {
                    Value::Object(right_obj) => match right_obj.obj_type {
                        ObjectType::Object => {
                            let left = unsafe { (*(*left_obj.obj).object).clone() };
                            let right = unsafe { (*(*right_obj.obj).object).clone() };

                            for (key, value) in left.into_iter() {
                                if key == "_" {
                                    continue;
                                }

                                match &*value {
                                    Value::Object(obj) => match obj.obj_type {
                                        ObjectType::Identifier => {
                                            match right.get(&key) {
                                                Some(to_be_assigned) => {
                                                    // TODO: check if the variable is already
                                                    // defined or not
                                                    self.globals.insert(
                                                        unsafe { (*(*obj.obj).string).clone() },
                                                        **to_be_assigned,
                                                    );
                                                }
                                                None => {
                                                    // TODO: report error
                                                }
                                            }
                                        }
                                        _ => {
                                            // TODO: report error
                                        }
                                    },
                                    _ => {
                                        // TODO: report error
                                    }
                                }
                            }
                        }
                        _ => {
                            // TODO: report error
                        }
                    },
                    _ => {
                        // TODO: report error
                    }
                },
                _ => {
                    // can't happen, nothing happens
                }
            },
            _ => {
                // TODO: report error
            }
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
            self.values[constant]
        } else {
            self.values[read_byte!(self) as usize]
        }
    }
}
