pub mod debug;
pub mod opcode;

use std::collections::HashMap;
use std::process;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::{Receiver, Sender};

use self::opcode::{OpCode, Position};
use crate::frontend::ast::Expr;
use crate::frontend::token::{Token, TokenType};
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
}

#[derive(Debug, Clone)]
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
                    self.write_opcode(OpCode::Pop, (0, 0));
                }
            }
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
                let value: Value = value.clone().into();
                self.write_constant(value, true, token.position);
            }
            Expr::AtomLiteral { token, value } => {
                let value: Value = value.clone().as_str().into();
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
                    let key = self.token_to_string(k);
                    self.write_constant(key, true, token.position);
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
                            let var = self.token_to_string(name);
                            self.write_constant(var, true, token.position)
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
                                    Expr::Variable { ref mut name } => {
                                        let var = self.token_to_string(name);
                                        self.write_constant(var, true, name.position);
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
                                let key = self.token_to_string(k);
                                self.write_constant(key, true, token.position);
                                // write value expression
                                match **v {
                                    Expr::Variable { ref mut name } => {
                                        let var = self.token_to_string(name);
                                        self.write_constant(var, true, token.position);
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
                                let var = self.token_to_string(name);
                                self.write_constant(var, true, token.position);
                                self.add_local((*name).clone());
                            }
                            Expr::ListLiteral {
                                ref token,
                                ref mut values,
                            } => {
                                if values.len() > std::u16::MAX as usize {
                                    self.compile_error(
                                        token,
                                        "too big list literal for destructuring assignment"
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
                                            let var = self.token_to_string(&mut *name);
                                            self.write_constant(var, true, name.position);
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
                                    let key = self.token_to_string(k);
                                    self.write_constant(key, true, token.position);
                                    // write value expression
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            let var = self.token_to_string(name);
                                            self.write_constant(var, true, token.position);
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
                        self.compile_expr(&mut *right);

                        match **left {
                            Expr::Variable { ref name } => {
                                let result = self.resolve_local(name);
                                if result >= 0 {
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

                                // u8 args & CONSTs
                                for v in values {
                                    match **v {
                                        Expr::Variable { ref name } => {
                                            let result = self.resolve_local(&name);
                                            if result < 0 {
                                                self.compile_error(
                                                    &name,
                                                    format!(
                                                        "local variable {} not defined",
                                                        name.value
                                                    ),
                                                );
                                            }
                                            self.write_constant(
                                                name.value.as_str().into(),
                                                true,
                                                name.position,
                                            );
                                            // u8 argument
                                            self.write_byte(result as u8, name.position);
                                        }
                                        Expr::Underscore { ref token } => {
                                            self.write_constant(Value::Empty, true, token.position);
                                            self.write_byte(0u8, token.position);
                                        }
                                        _ => todo!(), // does not happen
                                    }
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

                                // u16 arg
                                for b in length {
                                    self.write_byte(b, token.position);
                                }

                                // u8 args
                                for (k, v) in &mut keys.into_iter().zip(values.into_iter()) {
                                    let key = self.token_to_string(k);
                                    self.write_constant(key, true, token.position);
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            let result = self.resolve_local(&name);
                                            if result < 0 {
                                                self.compile_error(
                                                    &name,
                                                    format!(
                                                        "local variable {} not defined",
                                                        name.value
                                                    ),
                                                );
                                            }
                                            // u8 argument
                                            self.write_byte(result as u8, name.position);
                                        }
                                        _ => todo!(), // does not happen
                                    }
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
                    let var = self.token_to_string(name);
                    self.write_constant(var, true, name.position);
                    self.write_opcode(OpCode::GetGlobal, name.position);
                }
            }
            Expr::Block {
                token,
                ref mut exprs,
            } => {
                self.begin_scope();

                // get the last expression in the block
                let mut last_expr = match exprs.pop() {
                    Some(expr) => *expr,
                    None => Expr::Null {
                        token: token.clone(),
                    },
                };

                // check whether the last expression in this block is a local variable declaration
                // or not
                match last_expr {
                    Expr::Assign {
                        init,
                        public: _,
                        ref token,
                        left: _,
                        right: _,
                    } => {
                        if init {
                            self.compile_error(
                                token,
                                "the last expression of a block cannot be a variable declaration"
                                    .to_string(),
                            )
                        }
                    }
                    _ => {}
                }

                // compile all expressions in the block except for the last expression and local
                // variable declarations
                for mut expr in exprs {
                    self.compile_expr(&mut expr);
                    match **expr {
                        Expr::Assign {
                            init,
                            public: _,
                            ref token,
                            left: _,
                            right: _,
                        } => {
                            if !init {
                                self.write_opcode(OpCode::Pop, token.position);
                            }
                        }
                        _ => self.write_opcode(OpCode::Pop, token.position),
                    }
                }

                // calculate the number of pops needed to remove all the unused values
                let mut pop_nums = 0;
                for local in self.locals.clone().into_iter().rev() {
                    if local.depth > self.score_depth - 1 {
                        pop_nums += 1;
                    }
                }

                // compile the last expression, and this expression is the value of the block
                self.compile_expr(&mut last_expr);

                // add OpCode::PopExceptLast instructions according to the number of local variables
                if pop_nums > 1 {
                    self.write_opcode(OpCode::PopExceptLastN, token.position);
                    self.write_byte(pop_nums as u8, token.position);
                } else if pop_nums == 1 {
                    self.write_opcode(OpCode::PopExceptLast, token.position);
                }

                // remove all local variables
                while self.locals.len() > 0
                    && self.locals.last().unwrap().depth > self.score_depth - 1
                {
                    self.locals.pop();
                }

                self.end_scope();
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
    fn token_to_string(&mut self, token: &mut Token) -> Value {
        token.value.clone().as_str().into()
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

    fn end_scope(&mut self) {
        self.score_depth -= 1;
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
        let source = r#"{ a := 123 a + 3 }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 15, 0, 1, 2, 4, 21, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_list() {
        let source = r#"{ [a, b] := [1, 2] null }"#;
        let expected: Vec<u8> = vec![
            19, 2, 0, 1, 0, 1, 1, 19, 2, 0, 1, 2, 1, 3, 14, 1, 4, 22, 2, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_obj() {
        let source = r#"{ {a: b} := {a: 123} null }"#;
        let expected: Vec<u8> = vec![
            20, 1, 0, 1, 0, 1, 1, 20, 1, 0, 1, 2, 1, 3, 14, 1, 4, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_var() {
        let source = r#"{ a := 123 a = 321 }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 1, 2, 16, 0, 21, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_list() {
        let source = r#"{ [a, _] := [1, 2] [a, _, _] = [3, 2, 1] }"#;
        let expected: Vec<u8> = vec![
            19, 2, 0, 1, 0, 1, 1, 19, 2, 0, 1, 2, 1, 3, 14, 19, 3, 0, 1, 4, 1, 5, 1, 6, 17, 1, 7,
            0, 1, 8, 0, 1, 9, 0, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_obj() {
        let source = r#"{ a := 100 {a: a} = {a: 10} }"#;
        let expected: Vec<u8> = vec![
            1, 0, 1, 1, 14, 20, 1, 0, 1, 2, 1, 3, 18, 1, 0, 1, 4, 0, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_get() {
        let source = r#"{ a := 123 a }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 14, 15, 0, 21, 12, 0];
        compile!(source, expected);
    }
}
