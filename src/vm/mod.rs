pub mod object;
pub mod value;

use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::ptr;

use byteorder::{ByteOrder, LittleEndian};

use self::object::RawObject;
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
    pub stack_top: *mut Value,
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
            ip: bytecode.as_ptr(),
            filename,
            source,
            stack: Box::new([Value::Null; STACK_MAX]),
            stack_top: ptr::null_mut(),
            globals: HashMap::new(),
        }
    }

    /// The heart of the VM
    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();
        self.stack_top = &mut self.stack[0] as *mut Value;

        loop {
            if self.execute_once(instruction) {
                break;
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

    fn execute_once(&mut self, instruction: OpCode) -> bool {
        let mut br = false;
        match instruction {
            OpCode::Return => {
                br = true;
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
            OpCode::Pop => {
                self.popn(1);
            }
            OpCode::PopN => {
                let n = read_byte!(self);
                self.popn(n);
            }
            OpCode::DefineGlobal => {
                self.define_or_set_global(true);
            }
            OpCode::SetGlobal => {
                self.define_or_set_global(false);
            }
            OpCode::GetGlobal => {
                match self.pop() {
                    Value::Object(obj) => match obj {
                        RawObject::Atom(var_name) => {
                            let var_name = unsafe { (**var_name).clone() };
                            match self.globals.get(&var_name) {
                                Some(v) => self.push(*v),
                                None => {} // TODO: report error
                            }
                        }
                        _ => todo!(), // does not happen
                    },
                    _ => todo!(), // does not happen
                }
            }
            OpCode::DefineLocal => {}
            OpCode::GetLocal => {}
            OpCode::SetLocalVar => {}
            OpCode::SetLocalList => {}
            OpCode::SetLocalObj => {}
            OpCode::InitList => {
                let length = self.read_2bytes() as usize;
                let mut list: ManuallyDrop<Vec<Box<Value>>> = ManuallyDrop::new(Vec::new());
                for _ in 0..length {
                    let inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                    self.execute_once(inst);
                    let element = self.pop();
                    list.push(Box::new(element));
                }
                self.push(Value::Object(RawObject::List(
                    &mut list as *mut ManuallyDrop<Vec<Box<Value>>>,
                )));
            }
            OpCode::InitObj => {
                let length = self.read_2bytes() as usize;
                let mut map: ManuallyDrop<HashMap<String, Box<Value>>> =
                    ManuallyDrop::new(HashMap::new());
                for _ in 0..length {
                    // get key
                    let mut inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                    self.execute_once(inst);
                    let key = self.pop();
                    // get value
                    inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                    self.execute_once(inst);
                    let value = self.pop();
                    match key {
                        Value::Object(obj) => match obj {
                            RawObject::Atom(v) => {
                                let key = unsafe { (**v).clone() };
                                map.insert(key, Box::new(value));
                            }
                            _ => todo!(), // does not happen
                        },
                        _ => {} // TODO: report error
                    }
                    self.push(Value::Object(RawObject::Object(
                        &mut map as *mut ManuallyDrop<HashMap<String, Box<Value>>>,
                    )));
                }
            }
        }
        br
    }

    fn define_or_set_global(&mut self, define: bool) {
        let right = self.pop();
        let left = self.pop();
        match left {
            Value::Object(left_obj) => match left_obj {
                RawObject::Atom(v) => {
                    let var_name = unsafe { (**v).clone() };
                    if define {
                        self.define_global(var_name, right);
                    } else {
                        self.set_global(var_name, right);
                    }
                }
                RawObject::List(list) => {
                    let left = unsafe { (**list).clone() };
                    match right {
                        Value::Object(right_obj) => match right_obj {
                            RawObject::List(list) => {
                                let right = unsafe { (**list).clone() };
                                if right.len() != left.len() {
                                    // TODO: report error
                                }
                                for (l, r) in left.into_iter().zip(right.into_iter()) {
                                    match *l {
                                        Value::Object(left_obj) => match left_obj {
                                            RawObject::Atom(v) => {
                                                let var_name = unsafe { (**v).clone() };
                                                if define {
                                                    self.define_global(var_name, *r);
                                                } else {
                                                    self.set_global(var_name, *r);
                                                }
                                            }
                                            _ => todo!(), // does not happen
                                        },
                                        Value::Empty => continue,
                                        _ => todo!(), // does not happen
                                    }
                                }
                            }
                            _ => {} // TODO: report error
                        },
                        _ => {} // TODO: report error
                    }
                }
                RawObject::Object(map) => {
                    let assignee = unsafe { (**map).clone() };
                    match right {
                        Value::Object(right_obj) => match right_obj {
                            RawObject::Object(map) => {
                                let right = unsafe { (**map).clone() };
                                for (k, assignee) in assignee.into_iter() {
                                    match right.get(&k) {
                                        Some(v) => match *assignee {
                                            Value::Object(assignee_obj) => match assignee_obj {
                                                RawObject::Atom(assignee) => {
                                                    let var_name = unsafe { (**assignee).clone() };
                                                    if define {
                                                        self.define_global(var_name, **v);
                                                    } else {
                                                        self.set_global(var_name, **v);
                                                    }
                                                }
                                                _ => todo!(), // does not happen
                                            },
                                            _ => todo!(), // does not happen
                                        },
                                        None => {} // TODO: report error
                                    }
                                }
                            }
                            _ => {} // TODO: report error
                        },
                        _ => {} // TODO: report error
                    }
                }
                _ => {} // TODO: report error
            },
            _ => {} // TODO: report error
        }
    }

    /// Pushes a Value onto the stack
    fn push(&mut self, value: Value) {
        unsafe { *self.stack_top = value }
        self.stack_top = unsafe { self.stack_top.add(1) };
    }

    /// Pops a Value from the stack
    fn pop(&mut self) -> Value {
        self.popn(1);
        unsafe { *self.stack_top }
    }

    /// Pops n times from the stack
    fn popn(&mut self, n: u8) {
        self.stack_top = unsafe { self.stack_top.sub(n as usize) };
        // TODO: actually pops Values
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

    /// Defines a global variable
    fn define_global(&mut self, name: String, value: Value) {
        if self.globals.contains_key(&name) {
            // TODO: report error
        } else {
            self.globals.insert(name, value);
        }
    }

    /// Sets a Value to a global variable
    fn set_global(&mut self, name: String, value: Value) {
        if self.globals.contains_key(&name) {
            self.globals.insert(name, value);
        } else {
            // TODO: report error
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
        let bytecode: Vec<u8> = vec![1, 0, 1, 1, 4, 1, 2, 5, 1, 3, 6, 12, 0];
        let values: Vec<Value> = vec![Value::Int(2), Value::Int(1), Value::Int(1), Value::Int(3)];
        let positions = HashMap::new();
        let source = "(2 + 1 - 1) * 3".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(unsafe { *vm.stack_top }, Value::Int(6));
    }

    #[test]
    fn test_unary() {
        let bytecode: Vec<u8> = vec![1, 0, 3, 12, 0];
        let values: Vec<Value> = vec![Value::Bool(false)];
        let positions = HashMap::new();
        let source = "not false".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(unsafe { *vm.stack_top }, Value::Bool(true));
    }

    //  #[test]
    //  fn test_global() {
    //      let bytecode: Vec<u8> = vec![
    //          1, 0, 1, 1, 9, 12, 19, 2, 0, 1, 2, 1, 3, 19, 2, 0, 1, 4, 1, 5, 9, 12, 20, 1, 0, 1, 6,
    //          1, 7, 20, 1, 0, 1, 8, 1, 9, 9, 12, 1, 10, 10, 1, 11, 10, 4, 1, 12, 10, 4, 12, 0,
    //      ];
    //      let values: Vec<Value> = vec![
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "a".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Int(1),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "b".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Empty,
    //          Value::Int(2),
    //          Value::Int(3),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "c".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "c".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "c".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Int(4),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "a".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "b".to_string() as *mut String,
    //              },
    //          }),
    //          Value::Object(Object {
    //              obj_type: ObjectType::Atom,
    //              obj: &mut ObjectUnion {
    //                  string: &mut "c".to_string() as *mut String,
    //              },
    //          }),
    //      ];
    //      let positions = HashMap::new();
    //      let source = r#"a := 1 [b, _] := [2, 3] {c: c} := {c: 4} a+b+c"#.to_string();

    //      let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
    //      vm.run();

    //      assert_eq!(unsafe { *vm.stack_top }, Value::Int(7));
    //  }
}
