pub mod debug;
pub mod opcode;

use std::collections::HashMap;
use std::process;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::{Receiver, Sender};

use self::opcode::{OpCode, Position};
use super::vm::destruct::Assignee;
use crate::frontend::ast::Expr;
use crate::frontend::token::{Token, TokenType};
use crate::vm::object::{Object, ObjectType, ObjectUnion};
use crate::vm::value::Value;

pub struct Compiler<'a> {
    /// File name
    filename: &'a str,
    /// Source
    source: &'a String,
    /// The name of this Compiler, used for debugging
    name: &'a str,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,
    /// Position information used for runtime errors
    pub positions: HashMap<usize, Position>,
    /// Local variables
    pub locals: Vec<Local>,
    score_depth: u32,
    /// Objects and lists that are part of destructuring
    pub assignees: Vec<Assignee>,

    sender: &'a Sender<Vec<u8>>,
    recv: &'a Receiver<Expr>,

    pop: bool,
}

pub struct Local {
    pub name: Token,
    pub depth: u32,
}

impl Local {
    pub fn new(token: Token, depth: u32) -> Self {
        Self { name: token, depth }
    }
}

impl<'a> Compiler<'a> {
    pub fn new<'b>(
        source: &'a String,
        filename: &'a str,
        name: &'static str,
        recv: &'a Receiver<Expr>,
        sender: &'a Sender<Vec<u8>>,
    ) -> Self {
        Self {
            filename,
            source,
            name,
            bytecode: vec![],
            positions: HashMap::new(),
            values: vec![],
            locals: vec![],
            score_depth: 0,
            assignees: vec![],
            sender,
            recv,
            pop: true,
        }
    }

    pub fn start(&mut self) {
        self.compile();
        self.sender.send(self.bytecode.to_owned()).unwrap();
    }

    fn compile(&mut self) {
        loop {
            let expr = self.recv.recv().unwrap();
            match expr {
                Expr::End => break,
                mut expr => {
                    self.compile_expr(&mut expr);
                }
            }
            if self.pop {
                self.write_opcode(OpCode::Pop, (0, 0));
            }
            self.pop = true
        }
        self.write_opcode(OpCode::Return, (0, 0));
    }

    fn compile_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Binary {
                ref mut left,
                ref mut right,
                ref op,
            } => {
                self.compile_expr(left);
                self.compile_expr(right);
                match op.kind {
                    TokenType::Plus => self.write_opcode(OpCode::Add, op.position),
                    TokenType::Minus => self.write_opcode(OpCode::Sub, op.position),
                    TokenType::Mul => self.write_opcode(OpCode::Mult, op.position),
                    TokenType::Div => self.write_opcode(OpCode::Div, op.position),
                    TokenType::Mod => self.write_opcode(OpCode::Mod, op.position),
                    _ => self.compile_error(op, "invalid binary expression".to_string()),
                }
            }
            Expr::Unary {
                ref mut right,
                ref op,
            } => {
                self.compile_expr(right);
                self.write_opcode(OpCode::Negate, op.position);
            }
            Expr::StringLiteral {
                token,
                ref mut value,
            } => {
                let obj = Object {
                    obj_type: ObjectType::String,
                    obj: &mut ObjectUnion {
                        string: value as *mut String,
                    } as *mut ObjectUnion,
                };
                let value = Value::Object(obj);
                self.write_constant(value, true, token.position);
            }
            Expr::AtomLiteral {
                token,
                ref mut value,
            } => {
                let obj = Object {
                    obj_type: ObjectType::Atom,
                    obj: &mut ObjectUnion {
                        string: value as *mut String,
                    } as *mut ObjectUnion,
                };
                let value = Value::Object(obj);
                self.write_constant(value, true, token.position);
            }
            Expr::IntegerLiteral { token, value } => {
                self.write_constant(Value::Int(*value), true, token.position);
            }
            Expr::FloatLiteral { token, value } => {
                self.write_constant(Value::Float(*value), true, token.position);
            }
            Expr::BoolLiteral { token, payload } => {
                self.write_constant(Value::Bool(*payload), true, token.position);
            }
            Expr::Underscore { token } => self.write_constant(Value::Empty, true, token.position),
            Expr::Null { token } => self.write_constant(Value::Null, true, token.position),
            Expr::ListLiteral { token, values } => {
                let mut list: Vec<Box<Value>> = vec![];
                for value in values {
                    let value = self.convert_to_value((**value).to_owned()).unwrap();
                    list.push(Box::new(value));
                }

                let obj = Object {
                    obj_type: ObjectType::List,
                    obj: &mut ObjectUnion {
                        list: &mut list as *mut Vec<Box<Value>>,
                    } as *mut ObjectUnion,
                };
                let value = Value::Object(obj);
                self.write_constant(value, true, token.position);
            }
            Expr::ObjectLiteral {
                token,
                keys,
                values,
            } => {
                let mut map: HashMap<String, Box<Value>> = HashMap::new();
                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                    let value = self.convert_to_value((**v).to_owned()).unwrap();
                    map.insert(k.value.to_owned(), Box::new(value));
                }

                let obj = Object {
                    obj_type: ObjectType::Object,
                    obj: &mut ObjectUnion {
                        object: &mut map as *mut HashMap<String, Box<Value>>,
                    },
                };
                let value = Value::Object(obj);
                self.write_constant(value, true, token.position)
            }
            Expr::Group { expr } => self.compile_expr(expr),
            Expr::Assign {
                token,
                init,
                ref left,
                ref right,
            } => {
                let right_value = self.convert_to_value((**right).to_owned()).unwrap();
                if self.score_depth == 0 {
                    match **left {
                        Expr::Variable { ref name } => {
                            self.write_destruct(Assignee::Var(name.value.clone()), name.position);
                        }
                        Expr::ListLiteral {
                            ref token,
                            ref values,
                        } => {
                            let mut assignees: Vec<String> = vec![];
                            for v in values {
                                match **v {
                                    Expr::Underscore { token: _ } => {
                                        assignees.push("_".to_string());
                                    }
                                    Expr::Variable { ref name } => {
                                        assignees.push(name.value.clone());
                                    }
                                    _ => todo!(), // cannot happen
                                }
                            }
                            self.write_destruct(Assignee::List(assignees), token.position);
                        }
                        Expr::ObjectLiteral {
                            ref token,
                            ref keys,
                            ref values,
                        } => {
                            let mut assignees: HashMap<String, (String, u8)> = HashMap::new();
                            for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                match **v {
                                    Expr::Underscore { token: _ } => continue,
                                    Expr::Variable { ref name } => {
                                        self.add_local((*name).clone());
                                        if assignees.contains_key(&k.value) {
                                            self.compile_error(
                                                k,
                                                format!("repeated key {}", k.value),
                                            );
                                        }
                                        assignees.insert(k.value.clone(), (name.value.clone(), 0));
                                    }
                                    _ => todo!(), // cannot happen
                                }
                            }
                            self.write_destruct(Assignee::Obj(assignees), token.position);
                        }
                        _ => todo!(),
                    }
                    self.write_constant(right_value, true, token.position);
                    if *init {
                        self.write_opcode(OpCode::DefineGlobalVar, token.position);
                    } else {
                        self.write_opcode(OpCode::SetGlobalVar, token.position);
                    }
                } else {
                    if *init {
                        self.check_local(left);
                        match **left {
                            Expr::Variable { ref name } => {
                                self.add_local((*name).clone());
                                self.write_destruct(
                                    Assignee::Var(name.value.clone()),
                                    name.position,
                                );
                            }
                            Expr::ListLiteral {
                                token: _,
                                ref values,
                            } => {
                                let mut assignees: Vec<String> = vec![];
                                for v in values {
                                    match **v {
                                        Expr::Underscore { token: _ } => {
                                            assignees.push("_".to_string());
                                        }
                                        Expr::Variable { ref name } => {
                                            self.add_local((*name).clone());
                                            assignees.push(name.value.clone());
                                        }
                                        _ => todo!(), // cannot happen
                                    }
                                }
                                self.write_destruct(Assignee::List(assignees), token.position);
                            }
                            Expr::ObjectLiteral {
                                token: _,
                                ref keys,
                                ref values,
                            } => {
                                let mut assignees: HashMap<String, (String, u8)> = HashMap::new();
                                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                    match **v {
                                        Expr::Underscore { token: _ } => continue,
                                        Expr::Variable { ref name } => {
                                            self.add_local((*name).clone());
                                            if assignees.contains_key(&k.value) {
                                                self.compile_error(
                                                    k,
                                                    format!("repeated key {}", k.value),
                                                );
                                            }
                                            assignees
                                                .insert(k.value.clone(), (name.value.clone(), 0));
                                        }
                                        _ => todo!(), // cannot happen
                                    }
                                }
                                self.write_destruct(Assignee::Obj(assignees), token.position);
                            }
                            _ => todo!(), // cannot happen
                        }
                        self.write_constant(right_value, true, token.position);
                        self.write_opcode(OpCode::DefineLocal, token.position);
                    } else {
                        match **left {
                            Expr::Variable { ref name } => {
                                let result = self.resolve_local(name);
                                if result >= 0 {
                                    self.write_constant(right_value, true, name.position);
                                    self.write_opcode(OpCode::SetLocalVar, name.position);
                                    self.write_byte(result as u8, name.position);
                                } else {
                                    self.compile_error(
                                        name,
                                        format!("local variable {} not defined", name.value),
                                    );
                                }
                            }
                            Expr::ListLiteral {
                                ref token,
                                ref values,
                            } => {
                                let mut assignees: Vec<String> = vec![];
                                let mut u8s: Vec<u8> = vec![];
                                for v in values {
                                    match **v {
                                        Expr::Underscore { token: _ } => {
                                            assignees.push("_".to_string());
                                        }
                                        Expr::Variable { ref name } => {
                                            let result = self.resolve_local(name);
                                            if result >= 0 {
                                                u8s.push(result as u8);
                                            } else {
                                                self.compile_error(
                                                    name,
                                                    format!(
                                                        "local variable {} not defined",
                                                        name.value
                                                    ),
                                                );
                                            }
                                            assignees.push(name.value.clone());
                                        }
                                        _ => todo!(), // cannot happen
                                    }
                                }
                                if u8s.len() > std::u8::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "too many assignee variables".to_string(),
                                    );
                                }

                                self.write_destruct(Assignee::List(assignees), token.position);
                                self.write_constant(right_value, true, token.position);
                                self.write_opcode(OpCode::SetLocalVar, token.position);
                                self.write_byte(u8s.len() as u8, token.position);
                                for b in u8s {
                                    self.write_byte(b, token.position);
                                }
                            }
                            Expr::ObjectLiteral {
                                token: _,
                                ref keys,
                                ref values,
                            } => {
                                let mut assignees: HashMap<String, (String, u8)> = HashMap::new();
                                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                    match **v {
                                        Expr::Underscore { token: _ } => continue,
                                        Expr::Variable { ref name } => {
                                            let result = self.resolve_local(name);
                                            if result >= 0 {
                                                assignees.insert(
                                                    k.value.clone(),
                                                    (name.value.clone(), result as u8),
                                                );
                                            } else {
                                                self.compile_error(
                                                    name,
                                                    format!(
                                                        "local variable {} not defined",
                                                        name.value
                                                    ),
                                                )
                                            }
                                        }
                                        _ => todo!(), // cannot happen
                                    }
                                }
                                self.write_destruct(Assignee::Obj(assignees), token.position);
                                self.write_constant(right_value, true, token.position);
                                self.write_opcode(OpCode::SetLocalObj, token.position);
                            }
                            _ => todo!(), // cannot happen
                        }
                    }
                }
            }
            Expr::Variable { name } => {
                if self.score_depth != 0 {
                    let result = self.resolve_local(name);
                    if result >= 0 {
                        self.write_opcode(OpCode::GetLocal, name.position);
                        self.write_byte(result as u8, name.position);
                    } else {
                        self.compile_error(
                            name,
                            format!("undefined {} local variable", name.value),
                        );
                    }
                } else {
                    let var = Assignee::Var(name.value.to_owned());
                    self.write_destruct(var, name.position);
                    self.write_opcode(OpCode::GetGlobalVar, name.position);
                }
            }
            Expr::Block { token, exprs } => {
                self.begin_scope();
                for mut expr in exprs {
                    self.compile_expr(&mut expr);
                }
                self.end_scope(token);
            }
            _ => {}
        }
    }

    fn add_local(&mut self, token: Token) {
        let local = Local::new(token, self.score_depth);
        self.locals.push(local);
    }

    fn resolve_local(&self, name: &Token) -> i8 {
        let mut i = self.locals.len() as i8 - 1;
        while i >= 0 {
            if self.locals[i as usize].name.value == name.value {
                return i;
            }
            i -= 1;
        }
        -1
    }

    /// Writes an opcode to the bytecode vector
    fn write_opcode(&mut self, opcode: OpCode, pos: Position) {
        let byte = opcode as u8;
        self.write_byte(byte, pos);
    }

    /// Add a constant to the values vector and adds the index to the bytecode vector
    fn write_constant(&mut self, value: Value, include_opcode: bool, pos: Position) {
        self.values.push(value);
        if self.values.len() > std::u8::MAX as usize - 1 {
            // use OP_LCONSTANT
            if include_opcode {
                let byte = OpCode::ConstantLong as u8;
                self.write_byte(byte, pos);
            }

            // convert the constant index into two u8's and writes the bytes to the bytecode vector
            let mut bytes = [0u8; 2];
            LittleEndian::write_u16(&mut bytes, (self.values.len() - 1) as u16);
            for byte in bytes {
                self.write_byte(byte, pos);
            }
        } else {
            // use OP_CONSTANT
            if include_opcode {
                let byte = OpCode::Constant as u8;
                self.write_byte(byte, pos);
            }

            self.write_byte((self.values.len() - 1) as u8, pos)
        }
    }

    /// Add a Destruct to the destructs vector and adds the index to the bytecode vector
    fn write_destruct(&mut self, assignee: Assignee, pos: Position) {
        self.assignees.push(assignee);
        let byte: u8;
        if self.values.len() > 255 {
            // use OP_LDESTRUCT
            byte = OpCode::LDestruct as u8;
            self.write_byte(byte, pos);

            let mut bytes = [0u8; 2];
            LittleEndian::write_u16(&mut bytes, (self.assignees.len() - 1) as u16);
            for byte in bytes {
                self.write_byte(byte, pos);
            }
        } else {
            // use OP_DESTRUCT
            byte = OpCode::Destruct as u8;
            self.write_byte(byte, pos);
            self.write_byte((self.assignees.len() - 1) as u8, pos);
        }
    }

    /// Writes a byte to the bytecode vector
    fn write_byte(&mut self, byte: u8, pos: Position) {
        self.bytecode.push(byte);
        self.positions.insert(self.bytecode.len() - 1, pos);
        // self.positions.entry(self.bytecode.len() - 1).or_insert(pos);
    }

    /// Converts an Expr to Value
    fn convert_to_value(&self, expr: Expr) -> Option<Value> {
        match expr {
            Expr::IntegerLiteral { token: _, value } => Some(Value::Int(value)),
            Expr::FloatLiteral { token: _, value } => Some(Value::Float(value)),
            Expr::BoolLiteral { token: _, payload } => Some(Value::Bool(payload)),
            Expr::Null { token: _ } => Some(Value::Null),
            Expr::Underscore { token: _ } => Some(Value::Empty),
            Expr::StringLiteral {
                token: _,
                mut value,
            } => {
                let obj = Object {
                    obj_type: ObjectType::String,
                    obj: &mut ObjectUnion {
                        string: &mut value as *mut String,
                    },
                };
                Some(Value::Object(obj))
            }
            Expr::AtomLiteral {
                token: _,
                mut value,
            } => {
                let obj = Object {
                    obj_type: ObjectType::Atom,
                    obj: &mut ObjectUnion {
                        string: &mut value as *mut String,
                    },
                };
                Some(Value::Object(obj))
            }
            Expr::ListLiteral { token: _, values } => {
                let mut list: Vec<Box<Value>> = vec![];

                for value in values {
                    let value = self.convert_to_value(*value).unwrap();
                    list.push(Box::new(value));
                }

                let obj = Object {
                    obj_type: ObjectType::List,
                    obj: &mut ObjectUnion {
                        list: &mut list as *mut Vec<Box<Value>>,
                    },
                };
                Some(Value::Object(obj))
            }
            Expr::ObjectLiteral {
                token: _,
                keys,
                values,
            } => {
                let mut map: HashMap<String, Box<Value>> = HashMap::new();

                for (key, value) in keys.into_iter().zip(values.into_iter()) {
                    let value = self.convert_to_value(*value).unwrap();
                    map.insert(key.value, Box::new(value));
                }
                let obj = Object {
                    obj_type: ObjectType::Object,
                    obj: &mut ObjectUnion {
                        object: &mut map as *mut HashMap<String, Box<Value>>,
                    },
                };
                Some(Value::Object(obj))
            }
            _ => None,
        }
    }

    /// Checks
    fn check_local(&self, left: &Expr) {
        for local in &self.locals {
            if local.depth < self.score_depth {
                break;
            }

            match *left {
                Expr::Underscore { token: _ } => continue,
                Expr::Variable { ref name } => {
                    if local.name.value == name.value {
                        self.compile_error(
                            &name,
                            format!("local variable {} already defined", name.value),
                        );
                    }
                }
                Expr::ListLiteral {
                    token: _,
                    ref values,
                } => {
                    for v in values {
                        match **v {
                            Expr::Underscore { token: _ } => continue,
                            Expr::Variable { ref name } => {
                                if local.name.value == name.value {
                                    self.compile_error(
                                        &name,
                                        format!("local variable {} already defined", name.value),
                                    );
                                }
                            }
                            _ => todo!(), // cannot happen
                        }
                    }
                }
                Expr::ObjectLiteral {
                    token: _,
                    keys: _,
                    ref values,
                } => {
                    for v in values {
                        match **v {
                            Expr::Underscore { token: _ } => continue,
                            Expr::Variable { ref name } => {
                                if local.name.value == name.value {
                                    self.compile_error(
                                        &name,
                                        format!("local variable {} already defined", name.value),
                                    );
                                }
                            }
                            _ => todo!(), // cannot happen
                        }
                    }
                }
                _ => todo!(), // cannot happen
            }
        }
    }

    fn begin_scope(&mut self) {
        self.score_depth += 1;
    }

    fn end_scope(&mut self, token: &Token) {
        self.score_depth -= 1;

        while self.locals.len() > 0 && self.locals.last().unwrap().depth > self.score_depth {
            self.write_opcode(OpCode::Pop, token.position);
            self.locals.pop();
        }
        self.pop = false;
    }

    fn compile_error(&self, token: &Token, message: String) {
        let message = format!(
            "{}:{}:{} error: {}\n{}",
            self.filename,
            token.position.0,
            token.position.1,
            message,
            self.source.split("\n").collect::<Vec<&str>>()[token.position.0 - 1]
        );
        eprintln!("{}", message);
        process::exit(1);
    }
}

// Tests
#[cfg(test)]
mod tests {
    use crate::compile;
    use crate::frontend::lexer::Lexer;
    use crate::frontend::parser::Parser;

    use super::*;

    #[test]
    fn test_binary() {
        let source = r#"1 + 1"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 4, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_unary() {
        let source = "not false";
        let expected: Vec<u8> = vec![1, 0, 3, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_global_def() {
        let source = r#"name := "nobu""#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 9, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_global_set() {
        let source = r#"name = "nobu""#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 11, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_global_get() {
        let source = r#"name"#;
        let expected: Vec<u8> = vec![18, 0, 10, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def() {
        let source = r#"{ a := 123 }"#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 13, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_var() {
        let source = r#"{ a := 123 a = 321 }"#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 13, 1, 1, 15, 0, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_list() {
        let source = r#"{ [a, b, c] := [1, 2, 3] [a, b, c] = [3, 2, 1] }"#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 13, 18, 1, 1, 1, 15, 3, 0, 1, 2, 12, 12, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_obj() {
        let source = r#"{ {name: a} := {name: "Nobu"} {a: a} = {a: 10, b: 11} }"#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 13, 18, 1, 1, 1, 17, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_get() {
        let source = r#"{ a := 123 a }"#;
        let expected: Vec<u8> = vec![18, 0, 1, 0, 13, 14, 0, 12, 0];
        compile!(source, expected);
    }
}
