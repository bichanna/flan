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
    pub stack: Vec<Value>,
    /// All global variables
    pub globals: HashMap<String, (Value, bool)>, // if bool is true, then it's a public variable
    /// Only used for debugging
    last_value: Option<Value>,
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
            stack: Vec::with_capacity(20),
            globals: HashMap::new(),
            last_value: None,
        }
    }

    /// The heart of the VM
    pub fn run(&mut self) {
        let mut instruction = OpCode::u8_to_opcode(unsafe { *self.ip }).unwrap();

        loop {
            if self.execute_once(instruction) {
                break;
            }
            instruction = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
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
                let v = self.stack.pop().unwrap();
                self.stack.pop();
                self.push(v);
            }
            OpCode::PopExceptLastN => {
                let v = self.stack.pop().unwrap();
                let n = read_byte!(self);
                self.popn(n);
                self.push(v);
            }
            OpCode::Jump => {
                let jump = self.read_3bytes();
                unsafe { self.ip.add(jump as usize) };
            }
            OpCode::DefineGlobal => {
                self.define_or_set_global(true);
            }
            OpCode::SetGlobal => {
                self.define_or_set_global(false);
            }
            OpCode::GetGlobal => {
                match self.pop() {
                    Value::Var(var_name) => {
                        let var_name = var_name.as_str().to_string();
                        match self.globals.get(&var_name) {
                            Some(v) => self.push(v.0.clone()),
                            None => {} // TODO: report error
                        }
                    }
                    _ => todo!(), // does not happen
                }
            }
            OpCode::DefineLocal => {
                let right = self.pop();
                let left = self.pop();
                match left {
                    Value::Var(_) => {
                        self.push(right);
                    }
                    Value::List(list) => {
                        let left = list.borrow();
                        match right {
                            Value::List(list) => {
                                let right = list.borrow();
                                if right.len() != left.len() {
                                    // TODO: report error
                                }
                                for (l, r) in
                                    left.clone().into_iter().zip(right.clone().into_iter())
                                {
                                    match *l {
                                        Value::Var(_) => {
                                            self.push(*r);
                                        }
                                        Value::Empty => continue,
                                        _ => todo!(), // does not happen
                                    }
                                }
                            }
                            _ => {} // TODO: report error
                        }
                    }
                    Value::Object(obj) => {
                        let assignee = obj.borrow();
                        match right {
                            Value::Object(map) => {
                                let right = map.borrow();
                                for (k, assignee) in assignee.clone().into_iter() {
                                    match right.get(&k) {
                                        Some(v) => match *assignee {
                                            Value::Var(_) => {
                                                self.push((**v).clone());
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
                    _ => todo!(), // does not happen
                }
            }
            OpCode::GetLocal => {
                let slot = read_byte!(self);
                let value = self.stack[slot as usize].clone();
                self.push(value);
            }
            OpCode::SetLocalVar => {
                let right = self.pop();
                let slot = read_byte!(self) as usize;
                self.stack[slot] = right.clone();
                self.push(right);
            }
            OpCode::SetLocalList => {
                let right = self.pop();
                match right.clone() {
                    Value::List(list) => {
                        for value in list.borrow().clone().into_iter() {
                            let inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                            self.execute_once(inst);
                            let var = self.pop();
                            match var {
                                Value::Var(_) => {
                                    let slot = read_byte!(self) as usize;
                                    self.stack[slot] = *value;
                                }
                                Value::Empty => {
                                    read_byte!(self);
                                    continue;
                                }
                                _ => todo!(), // does not happen
                            }
                        }
                    }
                    _ => {} // TODO: report error
                }
                self.push(right);
            }
            OpCode::SetLocalObj => {
                let right = self.pop();
                let length = self.read_2bytes() as usize;
                match right.clone() {
                    Value::Object(obj) => {
                        let obj = obj.borrow().clone();
                        for _ in 0..length {
                            let inst = OpCode::u8_to_opcode(read_byte!(self)).unwrap();
                            self.execute_once(inst);
                            match self.pop() {
                                Value::Var(key) => {
                                    let key = key.as_str().to_string();
                                    if obj.contains_key(&key) {
                                        let value = obj.get(&key).unwrap();
                                        let slot = read_byte!(self) as usize;
                                        self.stack[slot] = (**value).clone();
                                    }
                                }
                                _ => {} // TODO: report error
                            };
                        }
                    }
                    _ => {} // TODO: report error
                }
                self.push(right);
            }
            OpCode::InitList => {
                let length = self.read_2bytes() as usize;
                let mut list: Vec<Box<Value>> = Vec::new();
                for _ in 0..length {
                    let element = self.pop();
                    list.push(Box::new(element));
                }
                list.reverse();
                self.push(list.into());
            }
            OpCode::InitObj => {
                let length = self.read_2bytes() as usize;
                let mut map: HashMap<String, Box<Value>> = HashMap::new();
                for _ in 0..length {
                    // get value
                    let value = self.pop();
                    // get key
                    let key = self.pop();
                    match key {
                        Value::Var(v) => {
                            let key = v.as_str().to_string();
                            map.insert(key, Box::new(value));
                        }
                        _ => todo!(), // does not happen
                    }
                }
                self.push(map.clone().into());
            }
            OpCode::Match => {
                let target = self.pop();
                let cond = self.pop();
                let next = if read_byte!(self) == 1 { true } else { false };
                let jump = self.read_2bytes() as usize;
                let mut body_run = false;
                match cond.clone() {
                    Value::Empty => {}
                    Value::Null => match target {
                        Value::Null | Value::Empty => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Int(int) => match target {
                        Value::Empty => {}
                        Value::Int(t_int) if int == t_int => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Bool(payload) => match target {
                        Value::Empty => {}
                        Value::Bool(t_bool) if payload == t_bool => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Float(float) => match target {
                        Value::Empty => {}
                        Value::Float(t_float) if float == t_float => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::String(string) => match target {
                        Value::Empty => {}
                        Value::String(t_str) if string == t_str => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Atom(atom) => match target {
                        Value::Empty => {}
                        Value::Atom(t_atom) if atom == t_atom => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::List(list) => match target {
                        Value::Empty => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        Value::List(t_list) => {
                            let list = list.borrow().clone();
                            let t_list = t_list.borrow().clone();
                            if list.len() != t_list.len() {
                                todo!() // TODO: report error
                            }
                            for (l, r) in list.into_iter().zip(t_list.into_iter()) {
                                match *l {
                                    Value::Empty => continue,
                                    Value::Null => match *r {
                                        Value::Null | Value::Empty => continue,
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    },
                                    Value::Int(l_int) => match *r {
                                        Value::Int(r_int) if l_int == r_int => continue,
                                        Value::Empty => continue,
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    },
                                    Value::Float(l_float) => match *r {
                                        Value::Float(r_float) if l_float == r_float => continue,
                                        Value::Empty => continue,
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    },
                                    Value::String(l_str) => match *r {
                                        Value::String(r_str) if l_str == r_str => continue,
                                        Value::Empty => continue,
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    },
                                    Value::Atom(l_atom) => match *r {
                                        Value::Atom(r_atom) if l_atom == r_atom => continue,
                                        Value::Empty => continue,
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    },
                                    Value::Var(var_name) => todo!(), // TODO: fix this later
                                    _ => {
                                        self.jumpf(jump);
                                        body_run = true;
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Object(obj) => match target {
                        Value::Empty => {}
                        Value::Var(var_name) => todo!(), // TODO: fix this later
                        Value::Object(t_obj) => {
                            let obj = obj.borrow().clone();
                            let t_obj = t_obj.borrow().clone();
                            if obj.len() != t_obj.len() {
                                todo!() // TODO: report error
                            }
                            for (l_key, l_val) in obj.into_iter() {
                                if t_obj.contains_key(&l_key) {
                                    let t_val = (**t_obj.get(&l_key).unwrap()).clone();
                                    match *l_val {
                                        Value::Empty => continue,
                                        Value::Null => match t_val {
                                            Value::Empty => continue,
                                            Value::Null => continue,
                                            _ => {
                                                self.jumpf(jump);
                                                body_run = true;
                                                break;
                                            }
                                        },
                                        Value::Int(l_int) => match t_val {
                                            Value::Empty => continue,
                                            Value::Int(r_int) if l_int == r_int => continue,
                                            _ => {
                                                self.jumpf(jump);
                                                body_run = true;
                                                break;
                                            }
                                        },
                                        Value::Float(l_float) => match t_val {
                                            Value::Empty => continue,
                                            Value::Float(r_float) if l_float == r_float => continue,
                                            _ => {
                                                self.jumpf(jump);
                                                body_run = true;
                                                break;
                                            }
                                        },
                                        Value::String(l_str) => match t_val {
                                            Value::Empty => continue,
                                            Value::String(r_str) if l_str == r_str => continue,
                                            _ => {
                                                self.jumpf(jump);
                                                body_run = true;
                                                break;
                                            }
                                        },
                                        Value::Atom(l_atom) => match t_val {
                                            Value::Empty => continue,
                                            Value::Atom(r_atom) if l_atom == r_atom => continue,
                                            _ => {
                                                self.jumpf(jump);
                                                body_run = true;
                                                break;
                                            }
                                        },
                                        Value::Var(var_name) => todo!(), // TODO: fix this later
                                        _ => {
                                            self.jumpf(jump);
                                            body_run = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            self.jumpf(jump);
                            body_run = true;
                        }
                    },
                    Value::Var(var_name) => todo!(), // TODO: report error
                    _ => {
                        self.jumpf(jump);
                        body_run = true;
                    }
                }
                if !body_run && next {
                    self.push(cond);
                }
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
            Value::Var(v) => {
                let var_name = v.as_str().to_string();
                if define {
                    self.define_global(var_name, right.clone(), public);
                } else {
                    self.set_global(var_name, right.clone());
                }
            }
            Value::List(list) => {
                let left = list.borrow();
                match right.clone() {
                    Value::List(list) => {
                        let right = list.borrow();
                        if right.len() != left.len() {
                            // TODO: report error
                        }
                        for (l, r) in left.clone().into_iter().zip(right.clone().into_iter()) {
                            match *l {
                                Value::Var(v) => {
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
                match right.clone() {
                    Value::Object(map) => {
                        let right = map.borrow();
                        for (k, assignee) in assignee.clone().into_iter() {
                            match right.get(&k) {
                                Some(v) => match *assignee {
                                    Value::Var(assignee) => {
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
        if define {
            self.push(right);
        }
    }

    /// Pushes a Value onto the stack
    fn push(&mut self, value: Value) {
        if cfg!(feature = "debug") {
            self.last_value = Some(value.clone());
        }
        self.stack.push(value);
        if self.stack.capacity() == self.stack.len() {
            self.stack.reserve(10);
        }
    }

    /// Pops a Value from the stack
    fn pop(&mut self) -> Value {
        let value = self.stack.pop().unwrap();
        if cfg!(feature = "debug") {
            self.last_value = Some(value.clone());
        }
        value
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

    fn read_3bytes(&mut self) -> u32 {
        let bytes = [read_byte!(self), read_byte!(self), read_byte!(self)];
        LittleEndian::read_u24(&bytes)
    }

    fn jumpf(&mut self, offset: usize) {
        unsafe { self.ip.add(offset) };
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
    use crate::compiler::Compiler;
    use crate::frontend::lexer::Lexer;
    use crate::frontend::parser::Parser;

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

    fn get_bytecode(source: &String) -> (Vec<u8>, Vec<Value>, HashMap<usize, (usize, usize)>) {
        // for tokenizing
        let (ts, tr) = crossbeam_channel::unbounded();
        // for parsing
        let (ps, pr) = crossbeam_channel::unbounded();

        let mut compiler = Compiler::new(&source, "input", "test", &pr);

        std::thread::scope(|s| {
            s.spawn(|| {
                Lexer::new(&source, "input", &ts);
            });

            s.spawn(|| {
                Parser::new(&source, "input", &tr, &ps);
            });
        });
        compiler.compile();
        (compiler.bytecode, compiler.values, compiler.positions)
    }

    #[test]
    fn test_global() {
        let source = "a := 1 [b, c] := [2, 3] {d: d} := {d: 4} e := a+b+c+d".to_string();
        let (bytecode, values, positions) = get_bytecode(&source);
        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(vm.globals.get("e"), Some(&(Value::Int(10), false)));
    }

    #[test]
    fn test_local_def() {
        let source = "{ a := 1 [b, c] := [2, 3] {d: d} := {d: 4} a+b+c+d }".to_string();
        let (bytecode, values, positions) = get_bytecode(&source);
        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(vm.last_value, Some(Value::Int(10)));
    }

    #[test]
    fn test_local_set_var() {
        let source = "{ a := 1 a = 100 }".to_string();
        let (bytecode, values, positions) = get_bytecode(&source);
        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(vm.last_value, Some(Value::Int(100)));
    }

    #[test]
    fn test_local_set_list() {
        let source = "{ a := 1 [a, _] = [100, 200] }".to_string();
        let (bytecode, values, positions) = get_bytecode(&source);
        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        assert_eq!(
            vm.last_value,
            Some(vec![Box::new(Value::Int(100)), Box::new(Value::Int(200))].into())
        );
    }

    #[test]
    fn test_local_set_obj() {
        let source = "{ a := 1 {A: a} = {A: 100} }".to_string();
        let (bytecode, values, positions) = get_bytecode(&source);
        let mut vm = VM::new("input", &source, &bytecode, &values, &positions);
        vm.run();

        let mut value = HashMap::new();
        value.insert("A".to_string(), Box::new(Value::Int(100)));
        assert_eq!(vm.last_value, Some(value.into()));
    }
}
