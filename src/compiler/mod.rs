pub mod debug;
pub mod opcode;

use std::collections::HashMap;
use std::process;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::{Receiver, Sender};

use self::opcode::{OpCode, Position};
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

    sender: &'a Sender<Vec<u8>>,
    recv: &'a Receiver<Expr>,
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
                    _ => {
                        // TODO: report error
                    }
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
                let left_value = self.convert_to_value((**left).to_owned()).unwrap();
                let right_value = self.convert_to_value((**right).to_owned()).unwrap();
                if self.score_depth == 0 {
                    self.write_constant(left_value, true, token.position);
                    self.write_constant(right_value, true, token.position);
                    if *init {
                        self.write_opcode(OpCode::DefineGlobalVar, token.position);
                    } else {
                        self.write_opcode(OpCode::SetGlobalVar, token.position);
                    }
                } else {
                    for local in &self.locals {
                        if local.depth < self.score_depth {
                            break;
                        }
                        if *init && local.name.value == token.value {
                            self.compile_error(
                                &token,
                                "a variable with this name is already in this scope",
                            )
                        }
                    }
                    self.add_local((*token).clone())
                }
            }
            Expr::Variable { name } => {
                if self.score_depth != 0 {
                    for i in self.locals.len()..0 {
                        if self.locals[i - 1].name.value == name.value {
                            self.write_opcode(OpCode::GetLocalVar, name.position);
                            self.write_byte((i - 1) as u8, name.position);
                        }
                    }
                } else {
                    let obj = Object {
                        obj_type: ObjectType::Identifier,
                        obj: &mut ObjectUnion {
                            string: &mut name.value as *mut String,
                        },
                    };
                    let value = Value::Object(obj);

                    self.write_constant(value, true, name.position);
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

    /// Writes an opcode to the bytecode vector
    fn write_opcode(&mut self, opcode: OpCode, pos: Position) {
        let byte = opcode as u8;
        self.write_byte(byte, pos);
    }

    /// Add a constant to the values vector and adds the index to the bytecode vector
    fn write_constant(&mut self, value: Value, include_opcode: bool, pos: Position) {
        self.values.push(value);
        if self.values.len() > 255 {
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

    fn begin_scope(&mut self) {
        self.score_depth += 1;
    }

    fn end_scope(&mut self, token: &Token) {
        self.score_depth -= 1;

        while self.locals.len() > 0 && self.locals.last().unwrap().depth > self.score_depth {
            self.write_opcode(OpCode::Pop, token.position);
            self.locals.pop();
        }
    }

    fn compile_error(&self, token: &Token, message: &str) {
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
        let expected: Vec<u8> = vec![1, 0, 1, 1, 4, 0];
        compile!(source, expected);
    }

    #[test]
    fn test_unary() {
        let source = "not false";
        let expected: Vec<u8> = vec![1, 0, 3, 0];
        compile!(source, expected);
    }
}
