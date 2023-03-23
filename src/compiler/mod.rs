pub mod debug;
pub mod opcode;

use std::collections::HashMap;
use std::process;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::Receiver;

use self::opcode::{OpCode, Position};
use crate::frontend::ast::{Expr, MatchBranch};
use crate::frontend::token::{Token, TokenType};
use crate::vm::value::Value;

pub struct Compiler<'a> {
    /// File name
    filename: &'a str,
    /// Source
    source: &'a String,
    /// The name of this Compiler, used for debugging
    name: &'a str,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,
    /// Position information used for runtime errors
    pub positions: HashMap<usize, Position>,
    /// Local variables
    pub locals: Vec<Local>,
    scope_depth: u32,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,

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
    ) -> Self {
        Self {
            filename,
            source,
            name,
            positions: HashMap::new(),
            values: Vec::with_capacity(15),
            recv,
            locals: Vec::with_capacity(3),
            scope_depth: 0,
            bytecode: Vec::with_capacity(15),
        }
    }

    pub fn compile(&mut self) {
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
                if *value == 1 {
                    self.write_opcode(OpCode::Load1, token.position);
                } else if *value == 2 {
                    self.write_opcode(OpCode::Load2, token.position);
                } else if *value == 3 {
                    self.write_opcode(OpCode::Load3, token.position);
                } else if *value < std::u8::MAX as i64 && *value > std::u8::MIN as i64 {
                    self.write_opcode(OpCode::LoadU8, token.position);
                    self.write_byte(*value as u8, token.position);
                } else {
                    self.write_constant(Value::Int(*value), true, token.position);
                }
            }
            Expr::FloatLiteral { token, value } => {
                self.write_constant(Value::Float(*value), true, token.position);
            }
            Expr::BoolLiteral { token, payload } => {
                if *payload {
                    self.write_opcode(OpCode::LoadTrue, token.position);
                } else {
                    self.write_opcode(OpCode::LoadFalse, token.position);
                }
            }
            Expr::Underscore { token } => self.write_opcode(OpCode::LoadEmpty, token.position),
            Expr::Null { token } => self.write_opcode(OpCode::LoadNull, token.position),
            Expr::ListLiteral { token, values } => {
                let val_len = values.len();

                // compile the values first
                for v in values {
                    self.compile_expr(&mut *v);
                }

                // check the length of the list
                if val_len > std::u16::MAX as usize {
                    self.compile_error(token, "list literal too big".to_string());
                }

                self.write_opcode(OpCode::InitList, token.position);

                // write the length
                let mut length = [0u8; 2];
                LittleEndian::write_u16(&mut length, val_len as u16);
                for b in length {
                    self.write_byte(b, token.position);
                }
            }
            Expr::ObjectLiteral {
                token,
                keys,
                values,
            } => {
                // compile the initial keys and values
                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                    // write constant key
                    let key = self.token_to_var(k);
                    self.write_constant(key, true, token.position);
                    // write value expression
                    self.compile_expr(&mut *v);
                }

                // check the length of the object
                if values.len() > std::u16::MAX as usize {
                    self.compile_error(token, "object literal too big".to_string());
                }

                self.write_opcode(OpCode::InitObj, token.position);

                // write the length
                let mut length = [0u8; 2];
                LittleEndian::write_u16(&mut length, keys.len() as u16);
                for b in length {
                    self.write_byte(b, token.position);
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
                if self.scope_depth == 0 {
                    // global variables
                    match **left {
                        Expr::Variable { ref mut name } => {
                            let var = self.token_to_var(name);
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
                            let len = values.len();

                            for v in values {
                                match **v {
                                    Expr::Variable { ref mut name } => {
                                        let var = self.token_to_var(name);
                                        self.write_constant(var, true, name.position);
                                    }
                                    Expr::Underscore { ref mut token } => {
                                        self.write_constant(Value::Empty, true, token.position)
                                    }
                                    _ => todo!(), // does not happen
                                }
                            }

                            self.write_opcode(OpCode::InitList, token.position);

                            // write length
                            let mut length = [0u8; 2];
                            LittleEndian::write_u16(&mut length, len as u16);
                            for b in length {
                                self.write_byte(b, token.position);
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

                            for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                // write constant key
                                let key = self.token_to_var(k);
                                self.write_constant(key, true, token.position);
                                // write value expression
                                match **v {
                                    Expr::Variable { ref mut name } => {
                                        let var = self.token_to_var(name);
                                        self.write_constant(var, true, token.position);
                                    }
                                    _ => todo!(), // does not happen
                                }
                            }

                            self.write_opcode(OpCode::InitObj, token.position);

                            // write length
                            let mut length = [0u8; 2];
                            LittleEndian::write_u16(&mut length, keys.len() as u16);
                            for b in length {
                                self.write_byte(b, token.position);
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
                                let var = self.token_to_var(name);
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
                                for v in values {
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            let var = self.token_to_var(&mut *name);
                                            self.write_constant(var, true, name.position);
                                            self.add_local((*name).clone());
                                        }
                                        Expr::Underscore { ref mut token } => {
                                            self.write_constant(Value::Empty, true, token.position)
                                        }
                                        _ => todo!(), // does not happen
                                    }
                                }

                                self.write_opcode(OpCode::InitList, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
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

                                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                                    // write constant key
                                    let key = self.token_to_var(k);
                                    self.write_constant(key, true, token.position);
                                    // write value expression
                                    match **v {
                                        Expr::Variable { ref mut name } => {
                                            let var = self.token_to_var(name);
                                            self.write_constant(var, true, token.position);
                                            self.add_local((*name).clone());
                                        }
                                        _ => todo!(), // does not happen
                                    }
                                }

                                self.write_opcode(OpCode::InitObj, token.position);
                                for b in length {
                                    self.write_byte(b, token.position);
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
                                                Value::new_var(name.value.clone()),
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
                                    let key = self.token_to_var(k);
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
                if self.scope_depth != 0 {
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
                    let var = self.token_to_var(name);
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
                    if local.depth > self.scope_depth - 1 {
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
                    && self.locals.last().unwrap().depth > self.scope_depth - 1
                {
                    self.locals.pop();
                }

                self.end_scope();
            }
            Expr::Match {
                token,
                ref mut condition,
                ref mut branches,
            } => {
                fn compile_match_branch(
                    compiler: &mut Compiler,
                    condition: Option<&mut Box<Expr>>,
                    branches: &mut Vec<MatchBranch>,
                    token: &Token,
                ) {
                    let mut branch = branches.remove(0);

                    // compile condition expression if there's one
                    if let Some(condition) = condition {
                        compiler.compile_expr(condition);
                    }

                    // compile target expression
                    compiler.compile_expr(&mut branch.target);

                    // Match opcode
                    compiler.write_opcode(OpCode::Match, token.position);

                    // if there's next branch, write 1, otherwise 0
                    if branches.len() > 0 {
                        compiler.write_byte(1, token.position);
                    } else {
                        compiler.write_byte(0, token.position);
                    }

                    // for backpatching
                    compiler.write_byte(255, token.position);
                    compiler.write_byte(255, token.position);
                    let prev = compiler.bytecode.len();

                    // compile the body
                    compiler.compile_expr(&mut branch.body);

                    // apply patch
                    let length = compiler.bytecode.len() - prev - 1;
                    let index = prev - 2;
                    if length > std::u16::MAX as usize {
                        compiler.compile_error(token, "target expression too big".to_string());
                    }
                    let mut buff = [0u8; 2];
                    LittleEndian::write_u16(&mut buff, length as u16);
                    compiler.bytecode[index] = buff[0];
                    compiler.bytecode[index + 1] = buff[1];

                    // if this match branch is the last one, just return and do not add Jump opcode
                    if branches.len() == 0 {
                        return;
                    }

                    // Jump opcode
                    compiler.write_opcode(OpCode::Jump, token.position);

                    // another backpatching
                    compiler.write_byte(255, token.position);
                    compiler.write_byte(255, token.position);
                    compiler.write_byte(255, token.position);
                    let prev = compiler.bytecode.len();

                    // compile the next match branch, recursively
                    compile_match_branch(compiler, None, branches, token);

                    // apply patch
                    let length = compiler.bytecode.len() - prev - 1;
                    let index = prev - 3;
                    if length > 16_777_215 {
                        // max of unsigned 24 bits
                        compiler.compile_error(token, "match expression too big".to_string());
                    }
                    let mut buff = [0u8; 3];
                    LittleEndian::write_u24(&mut buff, length as u32);
                    compiler.bytecode[index] = buff[0];
                    compiler.bytecode[index + 1] = buff[1];
                    compiler.bytecode[index + 2] = buff[2];
                }

                // compile all match branches recursively
                compile_match_branch(self, Some(condition), branches, token);
            }
            _ => {}
        }
    }

    fn add_local(&mut self, token: Token) {
        let local = Local::new(token, self.scope_depth);
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
        let length = self.bytecode.len();
        self.positions.insert(length - 1, pos);
        // self.positions.entry(self.bytecode.len() - 1).or_insert(pos);
    }

    /// Converts a Token to Value::Var
    fn token_to_var(&mut self, token: &mut Token) -> Value {
        Value::new_var(token.value.clone())
    }

    /// Checks
    fn check_local(&self, left: &Expr) {
        for local in &self.locals {
            if local.depth < self.scope_depth {
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
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
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
        let expected: Vec<u8> = vec![25, 25, 4, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_unary() {
        let source = "not false";
        let expected: Vec<u8> = vec![30, 3, 12, 0];
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
        let expected: Vec<u8> = vec![1, 0, 28, 123, 14, 15, 0, 27, 4, 21, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_list() {
        let source = r#"{ [a, b] := [1, 2] null }"#;
        let expected: Vec<u8> = vec![1, 0, 1, 1, 19, 2, 0, 25, 26, 19, 2, 0, 14, 32, 22, 2, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_def_obj() {
        let source = r#"{ {a: b} := {a: 123} null }"#;
        let expected: Vec<u8> = vec![
            1, 0, 1, 1, 20, 1, 0, 1, 2, 28, 123, 20, 1, 0, 14, 32, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_var() {
        let source = r#"{ a := 123 a = 321 }"#;
        let expected: Vec<u8> = vec![1, 0, 28, 123, 14, 1, 1, 16, 0, 21, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_list() {
        let source = r#"{ [a, _] := [1, 2] [a, _, _] = [3, 2, 1] }"#;
        let expected: Vec<u8> = vec![
            1, 0, 1, 1, 19, 2, 0, 25, 26, 19, 2, 0, 14, 27, 26, 25, 19, 3, 0, 17, 1, 2, 0, 1, 3, 0,
            1, 4, 0, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_set_obj() {
        let source = r#"{ a := 100 {a: a} = {a: 10} }"#;
        let expected: Vec<u8> = vec![
            1, 0, 28, 100, 14, 1, 1, 28, 10, 20, 1, 0, 18, 1, 0, 1, 2, 0, 21, 12, 0,
        ];
        compile!(source, expected);
    }

    #[test]
    fn test_local_get() {
        let source = r#"{ a := 123 a }"#;
        let expected: Vec<u8> = vec![1, 0, 28, 123, 14, 15, 0, 21, 12, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_match_expr() {
        let source = r#"match true { true -> { "Hello" + ", world" }, false -> 0 }"#;
        let expected: Vec<u8> = vec![
            29, 29, 23, 1, 4, 0, 1, 0, 1, 1, 4, 24, 6, 0, 0, 30, 23, 0, 1, 0, 1, 2, 12, 0,
        ];
        compile!(source, expected);
    }
}
