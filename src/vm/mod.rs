pub mod object;
pub mod value;

use std::collections::HashMap;
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
    /// All to-be-garbage-collected objects
    pub objects: Vec<RawObject>,
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
            objects: vec![],
        }
    }

    pub fn update(
        &mut self,
        bytecode: &'a Vec<u8>,
        values: &'a Vec<Value>,
        positions: &'a HashMap<usize, Position>,
    ) {
        self.bytecode = bytecode;
        self.ip = self.bytecode.as_ptr();
        self.values = values;
        self.positions = positions;
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

        // rudimentary garbage collection
        for obj in self.objects.clone() {
            obj.free();
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
            OpCode::PopExceptLast => {
                let v = unsafe { *self.stack_top.sub(1) };
                unsafe { self.stack_top.sub(2) };
                self.push(v);
            }
            OpCode::PopExceptLastN => {
                let v = unsafe { *self.stack_top.sub(1) };
                let n = read_byte!(self) as usize;
                unsafe { self.stack_top.sub(n + 1) };
                self.push(v);
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
                            let var_name = unsafe { var_name.read() };
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
                let mut list: Vec<Box<Value>> = Vec::new();
                for _ in 0..length {
                    let inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                    self.execute_once(inst);
                    let element = self.pop();
                    list.push(Box::new(element));
                }
                self.push(Value::Object(RawObject::List(
                    &mut list as *mut Vec<Box<Value>>,
                )));
            }
            OpCode::InitObj => {
                let length = self.read_2bytes() as usize;
                let mut map: HashMap<String, Box<Value>> = HashMap::new();
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
                                let key = unsafe { v.read() };
                                map.insert(key, Box::new(value));
                            }
                            _ => todo!(), // does not happen
                        },
                        _ => {} // TODO: report error
                    }
                    self.push(Value::Object(RawObject::Object(
                        &mut map as *mut HashMap<String, Box<Value>>,
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
                    let var_name = unsafe { v.read() };
                    if define {
                        self.define_global(var_name, right);
                    } else {
                        self.set_global(var_name, right);
                    }
                }
                RawObject::List(list) => {
                    let left = unsafe { list.read() };
                    match right {
                        Value::Object(right_obj) => match right_obj {
                            RawObject::List(list) => {
                                let right = unsafe { list.read() };
                                if right.len() != left.len() {
                                    // TODO: report error
                                }
                                for (l, r) in left.into_iter().zip(right.into_iter()) {
                                    match *l {
                                        Value::Object(left_obj) => match left_obj {
                                            RawObject::Atom(v) => {
                                                let var_name = unsafe { v.read() };
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
                    let assignee = unsafe { map.read() };
                    match right {
                        Value::Object(right_obj) => match right_obj {
                            RawObject::Object(map) => {
                                let right = unsafe { map.read() };
                                for (k, assignee) in assignee.into_iter() {
                                    match right.get(&k) {
                                        Some(v) => match *assignee {
                                            Value::Object(assignee_obj) => match assignee_obj {
                                                RawObject::Atom(assignee) => {
                                                    let var_name = unsafe { assignee.read() };
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

    pub fn new_string(&mut self, mut v: String) -> Value {
        let raw_obj = RawObject::String(&mut v as *mut String);
        self.objects.push(raw_obj);
        Value::Object(raw_obj.clone())
    }

    pub fn new_atom(&mut self, mut v: String) -> Value {
        let raw_obj = RawObject::Atom(&mut v as *const String);
        self.objects.push(raw_obj);
        Value::Object(raw_obj.clone())
    }

    pub fn new_list(&mut self, mut list: Vec<Box<Value>>) -> Value {
        let raw_obj = RawObject::List(&mut list as *mut Vec<Box<Value>>);
        self.objects.push(raw_obj);
        Value::Object(raw_obj.clone())
    }

    pub fn new_obj(&mut self, mut obj: HashMap<String, Box<Value>>) -> Value {
        let raw_obj = RawObject::Object(&mut obj as *mut HashMap<String, Box<Value>>);
        self.objects.push(raw_obj);
        Value::Object(raw_obj.clone())
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
}
