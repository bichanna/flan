pub mod debug;
pub mod opcode;

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};
use crossbeam_channel::Receiver;

use self::opcode::{OpCode, Position};
use crate::frontend::ast::Expr;
use crate::frontend::token::TokenType;
use crate::vm::object::{Object, ObjectType, ObjectUnion};
use crate::vm::value::Value;

pub struct Compiler {
    /// The name of this Compiler, used for debugging
    name: &'static str,
    /// The compiled bytecode
    pub bytecode: Vec<u8>,
    /// For simplicity's sake, we'll put all constants in here
    pub values: Vec<Value>,
    /// Position information used for runtime errors
    pub positions: HashMap<usize, Position>,
}

impl Compiler {
    pub fn new(name: &'static str, recv: &Receiver<Expr>) {
        let mut compiler = Self {
            name,
            bytecode: vec![],
            positions: HashMap::new(),
            values: vec![],
        };

        compiler.compile(recv);
    }

    fn compile(&mut self, recv: &Receiver<Expr>) {
        loop {
            let expr = recv.recv().unwrap();
            match expr {
                Expr::End => break,
                mut expr => {
                    self.compile_expr(&mut expr);
                }
            }
        }
        self.write_opcode(OpCode::Return, (0, 0))
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
            Expr::ListLiteral {
                token: _,
                values: _,
            } => {
                // TODO: implement this
            }
            Expr::ObjectLiteral {
                token: _,
                keys: _,
                values: _,
            } => {
                // TODO: implement this
            }
            Expr::Group { expr } => self.compile_expr(expr),
            Expr::Assign {
                token,
                init,
                ref left,
                ref right,
            } => {
                if *init {
                    let left_value = self.convert_to_value((**left).clone()).unwrap();
                    let right_value = self.convert_to_value((**right).clone()).unwrap();
                    self.define_variable(right_value, left_value, token.position);
                } else {
                    // TODO: handle this
                }
            }
            _ => {}
        }
    }

    /// Writes bytecode that defines a global variable
    fn define_variable(&mut self, right: Value, left: Value, pos: Position) {
        self.write_constant(left, true, pos);
        self.write_constant(right, true, pos);
        self.write_opcode(OpCode::DefineGlobalVar, pos);
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
}
