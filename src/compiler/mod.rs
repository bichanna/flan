pub mod debug;
pub mod opcode;

use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::process;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::{Receiver, Sender};

use self::opcode::{OpCode, Position};
use crate::frontend::ast::Expr;
use crate::frontend::token::{Token, TokenType};
use crate::vm::object::RawObject;
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
            Expr::StringLiteral { token, value } => {
                self.write_constant(
                    Value::Object(RawObject::String(
                        &mut ManuallyDrop::new((*value).clone()) as *mut ManuallyDrop<String>
                    )),
                    true,
                    token.position,
                );
            }
            Expr::AtomLiteral { token, value } => {
                self.write_constant(
                    Value::Object(RawObject::Atom(
                        &mut ManuallyDrop::new((*value).clone()) as *mut ManuallyDrop<String>
                    )),
                    true,
                    token.position,
                );
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
                if values.len() > std::u16::MAX as usize {
                    self.compile_error(token, "list literal too big".to_string());
                }

                let mut length = [0u8; 2];
                LittleEndian::write_u16(&mut length, values.len() as u16);

                self.write_opcode(OpCode::InitList, token.position);
                for b in length {
                    self.write_byte(b, token.position);
                }
                for v in values {
                    self.compile_expr(&mut *v);
                }
            }
            Expr::ObjectLiteral {
                token,
                keys,
                values,
            } => {
                if values.len() > std::u16::MAX as usize {
                    self.compile_error(token, "object literal too big".to_string());
                }

                let mut length = [0u8; 2];
                LittleEndian::write_u16(&mut length, keys.len() as u16);

                self.write_opcode(OpCode::InitObj, token.position);
                for b in length {
                    self.write_byte(b, token.position);
                }

                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                    // write constant key
                    self.write_constant(Self::token_to_string(k), true, token.position);
                    // write value expression
                    self.compile_expr(&mut *v);
                }
            }
            Expr::Group { ref mut expr } => self.compile_expr(&mut *expr),
            Expr::Assign {
                token,
                init,
                public,
                ref mut left,
                ref mut right,
            } => {
                if self.score_depth == 0 {
                    // global variables
                    match **left {
                        Expr::Variable { ref mut name } => {
                            self.write_constant(Self::token_to_string(name), true, token.position)
                        }
                        Expr::ListLiteral {
                            ref token,
                            ref mut values,
                        } => {
                            if values.len() > std::u16::MAX as usize {
                                self.compile_error(
                                    token,
                                    "list literal too big for destructuring assignment".to_string(),
                                );
                            }

                            let mut length = [0u8; 2];
                            LittleEndian::write_u16(&mut length, values.len() as u16);

                            self.write_opcode(OpCode::InitList, token.position);
                            for b in length {
                                self.write_byte(b, token.position);
                            }

                            for v in values {
                                match **v {
                                    Expr::Variable { ref mut name } => self.write_constant(
                                        Self::token_to_string(&mut *name),
                                        true,
                                        name.position,
                                    ),
                                    Expr::Underscore { ref mut token } => {
                                        self.write_constant(Value::Empty, true, token.position)
                                    }
                                    _ => todo!(), // does not happen
                                }
                            }
                        }
                        Expr::ObjectLiteral {
                            ref token,
                            ref mut keys,
                            ref mut values,
                        } => {
                            if values.len() > std::u16::MAX as usize {
                                self.compile_error(
                                    token,
                                    "object literal too big for destructuring assignment"
                                        .to_string(),
                                );
                            }

                            let mut length = [0u8; 2];
                            LittleEndian::write_u16(&mut length, keys.len() as u16);

                            self.write_opcode(OpCode::InitObj, token.position);
                            for b in length {
                                self.write_byte(b, token.position);
                            }

                            for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                // write constant key
                                self.write_constant(Self::token_to_string(k), true, token.position);
                                // write value expression
                                match **v {
                                    Expr::Variable { ref mut name } => {
                                        self.write_constant(
                                            Self::token_to_string(name),
                                            true,
                                            token.position,
                                        );
                                    }
                                    _ => todo!(), // does not happen
                                }
                            }
                        }
                        _ => todo!(), // does not happen
                    }

                    self.compile_expr(&mut *right);
                    if *init {
                        self.write_opcode(OpCode::DefineGlobal, token.position);
                        if *public {
                            self.write_byte(1, token.position);
                        } else {
                            self.write_byte(0, token.position);
                        }
                    } else {
                        self.write_opcode(OpCode::SetGlobal, token.position);
                    }
                } else {
                    // local variables
                    if *init {
                        self.check_local(left);
                        match **left {
                            Expr::Variable { ref mut name } => {
                                self.write_constant(
                                    Self::token_to_string(name),
                                    true,
                                    token.position,
                                );
                                self.add_local((*name).clone());
                            }
                            Expr::ListLiteral {
                                ref token,
                                ref mut values,
                            } => {
                                if values.len() > std::u16::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "too big list literal for destructuringassignment"
                                            .to_string(),
                                    );
                                }

                                let mut length = [0u8; 2];
                                LittleEndian::write_u16(&mut length, values.len() as u16);

                                self.write_opcode(OpCode::InitList, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
                                }

                                for v in values {
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            self.write_constant(
                                                Self::token_to_string(&mut *name),
                                                true,
                                                name.position,
                                            );
                                            self.add_local((*name).clone());
                                        }
                                        Expr::Underscore { ref mut token } => {
                                            self.write_constant(Value::Empty, true, token.position)
                                        }
                                        _ => todo!(), // does not happen
                                    }
                                }
                            }
                            Expr::ObjectLiteral {
                                ref token,
                                ref mut keys,
                                ref mut values,
                            } => {
                                if values.len() > std::u16::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "too big object literal for destructuring assignment"
                                            .to_string(),
                                    );
                                }

                                let mut length = [0u8; 2];
                                LittleEndian::write_u16(&mut length, keys.len() as u16);

                                self.write_opcode(OpCode::InitObj, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
                                }

                                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                    // write constant key
                                    self.write_constant(
                                        Self::token_to_string(k),
                                        true,
                                        token.position,
                                    );
                                    // write value expression
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            self.write_constant(
                                                Self::token_to_string(name),
                                                true,
                                                token.position,
                                            );
                                            self.add_local((*name).clone());
                                        }
                                        _ => todo!(), // does not happen
                                    }
                                }
                            }
                            _ => todo!(), // does not happen
                        }
                        self.compile_expr(&mut *right);
                        self.write_opcode(OpCode::DefineLocal, token.position);
                    } else {
                        fn followed_by_u8arg(compiler: &mut Compiler, v: &mut Expr) {
                            match v {
                                Expr::Variable { ref mut name } => {
                                    let result = compiler.resolve_local(&name);
                                    if result < 0 {
                                        compiler.compile_error(
                                            &name,
                                            format!("local variable {} not defined", name.value),
                                        );
                                    }
                                    compiler.write_constant(
                                        Value::Object(RawObject::Atom(&mut ManuallyDrop::new(
                                            name.value.clone(),
                                        )
                                            as *mut ManuallyDrop<String>)),
                                        true,
                                        name.position,
                                    );
                                    // u8 argument
                                    compiler.write_byte(result as u8, name.position);
                                }
                                Expr::Underscore { token } => {
                                    compiler.write_constant(Value::Empty, true, token.position);
                                    compiler.write_byte(0, token.position);
                                }
                                _ => todo!(), // does not happen
                            }
                        }

                        match **left {
                            Expr::Variable { ref name } => {
                                let result = self.resolve_local(name);
                                if result >= 0 {
                                    self.compile_expr(&mut *right);
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
                                ref mut token,
                                ref mut values,
                            } => {
                                self.write_opcode(OpCode::SetLocalList, token.position);

                                if values.len() > std::u16::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "list literal too big for destructuring assignment"
                                            .to_string(),
                                    )
                                }
                                let mut length = [0u8; 2];
                                LittleEndian::write_u16(&mut length, values.len() as u16);

                                self.write_opcode(OpCode::InitList, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
                                }

                                for v in values {
                                    followed_by_u8arg(self, &mut *v);
                                }
                            }
                            Expr::ObjectLiteral {
                                token: _,
                                ref mut keys,
                                ref mut values,
                            } => {
                                self.write_opcode(OpCode::SetLocalObj, token.position);

                                if values.len() > std::u16::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "object literal too big for destructuring assignment"
                                            .to_string(),
                                    )
                                }
                                let mut length = [0u8; 2];
                                LittleEndian::write_u16(&mut length, keys.len() as u16);

                                self.write_opcode(OpCode::InitObj, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
                                }

                                for (k, v) in &mut keys.into_iter().zip(values.into_iter()) {
                                    self.write_constant(
                                        Self::token_to_string(k),
                                        true,
                                        token.position,
                                    );

                                    followed_by_u8arg(self, &mut **v);
                                }
                            }
                            _ => todo!(), // does not happen
                        }
                    }
                }
            }
            Expr::Variable { ref mut name } => {
                if self.score_depth != 0 {
                    let result = self.resolve_local(name);
                    if result >= 0 {
                        self.write_opcode(OpCode::GetLocal, name.position);
                        self.write_byte(result as u8, name.position);
                    } else {
                        self.compile_error(
                            name,
                            format!("local variable {} undefined", name.value),
                        );
                    }
                } else {
                    self.write_constant(Self::token_to_string(name), true, name.position);
                    self.write_opcode(OpCode::GetGlobal, name.position);
                }
            }
            Expr::Block {
                token,
                ref mut exprs,
            } => {
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

    /// Writes a byte to the bytecode vector
    fn write_byte(&mut self, byte: u8, pos: Position) {
        self.bytecode.push(byte);
        self.positions.insert(self.bytecode.len() - 1, pos);
        // self.positions.entry(self.bytecode.len() - 1).or_insert(pos);
    }

    /// Converts a Token to Value::Atom
    fn token_to_string(token: &mut Token) -> Value {
        Value::Object(RawObject::Atom(
            &mut ManuallyDrop::new(token.value.clone()) as *mut ManuallyDrop<String>
        ))
    }

    /// Checks
    fn check_local(&self, left: &Expr) {
        for local in &self.locals {
            if local.depth < self.score_depth {
                break;
            }

            match *left {
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

        let mut pop_nums = 0;
        while self.locals.len() > 0 && self.locals.last().unwrap().depth > self.score_depth {
            pop_nums += 1;
            self.locals.pop();
        }

        if pop_nums > 1 {
            self.write_opcode(OpCode::PopN, token.position);
            self.write_byte(pop_nums as u8, token.position);
        } else if pop_nums == 1 {
            self.write_opcode(OpCode::Pop, token.position);
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
        let expected: Vec<u8> = vec![1, 0, 1, 1, 9, 0, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_global_set() {
        let source = r#"name = "nobu""#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 11, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_global_get() {
        let source = r#"name"#;
        let expected: Vec<u8> = vec![1, 0, 10, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_var() {
        let source = r#"{ a := 123 }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_list() {
        let source = r#"{ [a, b] := [1, 2] }"#;
        let expected: Vec<u8> = vec![19, 2, 0, 1, 0, 1, 1, 19, 2, 0, 1, 2, 1, 3, 14, 13, 2, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_obj() {
        let source = r#"{ {a: b} := {a: 123} }"#;
        let expected: Vec<u8> = vec![20, 1, 0, 1, 0, 1, 1, 20, 1, 0, 1, 2, 1, 3, 14, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_var() {
        let source = r#"{ a := 123 a = 321 }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 1, 2, 16, 0, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_list() {
        let source = r#"{ [a, b, c] := [1, 2, 3] [a, b, c] = [3, 2, 1] }"#;
        let expected: Vec<u8> = vec![
            19, 3, 0, 1, 0, 1, 1, 1, 2, 19, 3, 0, 1, 3, 1, 4, 1, 5, 14, 17, 19, 3, 0, 1, 6, 0, 1,
            7, 1, 1, 8, 2, 13, 3, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_obj() {
        let source = r#"{ a := 100 {a: a} = {a: 10} }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 18, 20, 1, 0, 1, 2, 1, 3, 0, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_get() {
        let source = r#"{ a := 123 a }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 15, 0, 12, 0];
        compile!(source, expected);
    }
}
