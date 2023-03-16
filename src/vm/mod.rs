pub mod value;

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use self::value::Value;
use crate::compiler::opcode::{OpCode, Position};

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
    stack: Vec<Value>,
    /// All global variables
    pub globals: HashMap<String, (Value, bool)>, // if bool is true, then it's a public variable
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
            stack: vec![],
            globals: HashMap::new(),
        }
    }

    /// The heart of the VM
    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();
        println!("bytecode: {:?}", self.bytecode);

        loop {
            if self.execute_once(instruction) {
                break;
            }
            instruction = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
        }
    }

    fn execute_once(&mut self, instruction: OpCode) -> bool {
        let mut br = false;
        println!("inst: {:#?}", instruction);
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
                let v = self.stack.pop().unwrap();
                self.stack.pop();
                self.push(v);
            }
            OpCode::PopExceptLastN => {
                let v = self.stack.pop().unwrap();
                let n = read_byte!(self) as usize;
                for _ in 0..n {
                    self.stack.pop();
                }
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
                    Value::Atom(var_name) => {
                        let var_name = var_name.as_str().to_string();
                        match self.globals.get(&var_name) {
                            Some(v) => self.push(v.0.clone()),
                            None => {} // TODO: report error
                        }
                    }
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
                self.push(list.into());
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
                        Value::Atom(v) => {
                            let key = v.as_str().to_string();
                            map.insert(key, Box::new(value));
                        }
                        _ => todo!(), // does not happen
                    }
                }
                self.push(map.clone().into());
            }
        }
        br
    }

    fn define_or_set_global(&mut self, define: bool) {
        let right = self.pop();
        let left = self.pop();
        let mut public = false;
        if define {
            public = if read_byte!(self) == 1 { true } else { false };
        }
        match left {
            Value::Atom(v) => {
                let var_name = v.as_str().to_string();
                if define {
                    self.define_global(var_name, right, public);
                } else {
                    self.set_global(var_name, right);
                }
            }
            Value::List(list) => {
                let left = list.borrow();
                match right {
                    Value::List(list) => {
                        let right = list.borrow();
                        if right.len() != left.len() {
                            // TODO: report error
                        }
                        for (l, r) in left.clone().into_iter().zip(right.clone().into_iter()) {
                            match *l {
                                Value::Atom(v) => {
                                    let var_name = v.as_str().to_string();
                                    if define {
                                        self.define_global(var_name, *r, public);
                                    } else {
                                        self.set_global(var_name, *r);
                                    }
                                }
                                Value::Empty => continue,
                                _ => todo!(), // does not happen
                            }
                        }
                    }
                    _ => {} // TODO: report error
                }
            }
            Value::Object(map) => {
                let assignee = map.borrow();
                match right {
                    Value::Object(map) => {
                        let right = map.borrow();
                        for (k, assignee) in assignee.clone().into_iter() {
                            match right.get(&k) {
                                Some(v) => match *assignee {
                                    Value::Atom(assignee) => {
                                        let var_name = assignee.as_str().to_string();
                                        if define {
                                            self.define_global(var_name, (**v).clone(), public);
                                        } else {
                                            self.set_global(var_name, (**v).clone());
                                        }
                                    }
                                    _ => todo!(), // does not happen
                                },
                                None => {} // TODO: report error
                            }
                        }
                    }
                    _ => {} // TODO: report error
                }
            }
            _ => {} // TODO: report error
        }
    }

    /// Pushes a Value onto the stack
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pops a Value from the stack
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    /// Pops n times from the stack
    fn popn(&mut self, n: u8) {
        for _ in 0..n {
            self.stack.pop();
        }
    }

    fn read_2bytes(&mut self) -> u16 {
        let bytes = [read_byte!(self), read_byte!(self)];
        LittleEndian::read_u16(&bytes)
    }

    /// Reads a Value and returns it
    fn read_constant(&mut self, long: bool) -> Value {
        if long {
            let constant = self.read_2bytes();
            self.values[constant as usize].clone()
        } else {
            self.values[read_byte!(self) as usize].clone()
        }
    }

    /// Defines a global variable
    fn define_global(&mut self, name: String, value: Value, public: bool) {
        if self.globals.contains_key(&name) {
            // TODO: report error
        } else {
            self.globals.insert(name, (value, public));
        }
    }

    /// Sets a Value to a global variable
    fn set_global(&mut self, name: String, value: Value) {
        match self.globals.get(&name) {
            Some(v) => {
                self.globals.insert(name, (value, v.1));
            }
            None => {
                // TODO: report error
            }
        }
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
    }

    #[test]
    fn test_unary() {
        let bytecode: Vec<u8> = vec![1, 0, 3, 12, 0];
        let values: Vec<Value> = vec![Value::Bool(false)];
        let positions = HashMap::new();
        let source = "not false".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();
    }

    #[test]
    fn test_global() {
        let bytecode: Vec<u8> = vec![
            1, 0, 1, 1, 9, 0, 12, 19, 2, 0, 1, 2, 1, 3, 19, 2, 0, 1, 4, 1, 5, 9, 0, 12, 20, 1, 0,
            1, 6, 1, 7, 20, 1, 0, 1, 8, 1, 9, 9, 0, 12, 1, 10, 1, 11, 10, 1, 12, 10, 4, 1, 13, 10,
            4, 1, 14, 10, 4, 9, 0, 12, 0,
        ];
        let values: Vec<Value> = vec![
            "a".into(),
            Value::Int(1),
            "b".into(),
            "c".into(),
            Value::Int(2),
            Value::Int(3),
            "d".into(),
            "d".into(),
            "d".into(),
            Value::Int(4),
            "e".into(),
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
        ];
        let positions = HashMap::new();
        let source = "a := 1 [b, c] := [2, 3] {d: d} := {d: 4} e := a+b+c+d".to_string();

        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(vm.globals.get("e"), Some(&(Value::Int(10), false)));
    }
}
