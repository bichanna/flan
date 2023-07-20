pub mod opcode;
pub mod util;

use std::cmp::Ordering;
use std::sync::Arc;
use std::vec::IntoIter;

use self::opcode::OpCode;
use self::util::{to_little_endian, MemorySlice};
use crate::error::{ErrType, Position, Stack};
use crate::lexer::token::{Token, TokenType};
use crate::parser::expr::{Expr, MatchBranch};
use crate::util::PrevPeekable;
use crate::vm::value::*;

/// Applies a classic compiler trick called back-patching with two bytes
macro_rules! backpatch {
    ($c: expr, $op: expr, $err_msg: expr, $pos: expr, $block: expr) => {
        // writing the opcode
        $c.mem_slice.write_opcode($op, $pos);

        // getting ready for back-patching
        $c.mem_slice.write_bytes(&[0xFF, 0xFF], $pos);
        let prev = $c.mem_slice.bytecode.len() - 1;

        // doing whatever the caller wants to do here
        $block;

        // applying the patch
        let len = $c.mem_slice.bytecode.len() - prev - 1;
        let idx = prev - 2;
        if len > u16::MAX as usize {
            $c.report_err($err_msg, $pos);
        }
        let bytes = to_little_endian(len as u16);
        $c.mem_slice.write_byte_with_index(idx, bytes[0]);
        $c.mem_slice.write_byte_with_index(idx + 1, bytes[1]);
    };
}

#[derive(Clone)]
struct Local {
    name: Arc<str>,
    depth: usize,
}

struct Compiler {
    /// The iterator over the parsed expressions
    exprs: PrevPeekable<IntoIter<Expr>>,
    /// The path of the source being compiled
    path_idx: usize,
    /// All local variables
    locals: Vec<Local>,
    /// The depth of the current scope
    scope_depth: usize,
    /// The memory slice
    mem_slice: MemorySlice,
    /// The current expression being compiled
    current: Expr,
}

impl Compiler {
    pub fn compile(exprs: Vec<Expr>, tok_num: usize) -> MemorySlice {
        let mut exprs = PrevPeekable::new(exprs.into_iter());
        let current = exprs.next().unwrap();
        let mut compiler = Compiler {
            exprs,
            path_idx: Stack::last_path_index(),
            locals: Vec::new(),
            scope_depth: 0,
            mem_slice: MemorySlice::new(tok_num),
            current,
        };
        compiler._compile();
        compiler.mem_slice
    }

    fn _compile(&mut self) {
        while self.exprs.prev().is_some() {
            self.next_expr();
            self.compile_expr(self.current.clone());
            self.mem_slice.write_opcode(OpCode::Pop, (0, 0));
        }
        self.mem_slice.write_opcode(OpCode::Return, (0, 0));
    }

    /// Compiles an expression
    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Binary { left, right, op } => {
                let pos = op.pos;
                self.compile_expr(*left);
                self.compile_expr(*right);
                match op.kind {
                    TokenType::Plus => self.mem_slice.write_opcode(OpCode::Add, pos),
                    TokenType::Minus => self.mem_slice.write_opcode(OpCode::Sub, pos),
                    TokenType::Mult => self.mem_slice.write_opcode(OpCode::Mult, pos),
                    TokenType::Div => self.mem_slice.write_opcode(OpCode::Div, pos),
                    TokenType::Mod => self.mem_slice.write_opcode(OpCode::Rem, pos),
                    TokenType::DoubleEq => self.mem_slice.write_opcode(OpCode::Equal, pos),
                    TokenType::BangEq => self.mem_slice.write_opcode(OpCode::NotEqual, pos),
                    TokenType::GT => self.mem_slice.write_opcode(OpCode::GT, pos),
                    TokenType::LT => self.mem_slice.write_opcode(OpCode::LT, pos),
                    TokenType::GTEq => self.mem_slice.write_opcode(OpCode::GTEq, pos),
                    TokenType::LTEq => self.mem_slice.write_opcode(OpCode::LTEq, pos),
                    _ => self.report_err("invalid binary expression".to_string(), pos),
                }
            }

            Expr::Group(expr) => {
                self.compile_expr(*expr);
            }

            Expr::Unary { right, op } => {
                self.compile_expr(*right);
                self.mem_slice.write_opcode(
                    if let TokenType::Bang = op.kind {
                        OpCode::NegateBool
                    } else {
                        OpCode::Negate
                    },
                    op.pos,
                );
            }

            Expr::Logic { left, right, op } => {
                let pos = op.pos;
                self.compile_expr(*left);
                self.compile_expr(*right);
                match op.kind {
                    TokenType::And => self.mem_slice.write_opcode(OpCode::And, pos),
                    TokenType::Or => self.mem_slice.write_opcode(OpCode::Or, pos),
                    _ => self.report_err("invalid logic expression".to_string(), pos),
                }
            }

            Expr::Var { name, pos } => {
                let result = self.resolve_local(name.clone(), true, pos);
                if self.scope_depth != 0 {
                    if let Some(result) = result {
                        self.mem_slice.write_opcode(OpCode::GetLocal, pos);
                        self.mem_slice.write_byte(result as u8, pos);
                    } else {
                        self.mem_slice.add_const(Box::new(FVar(name)), pos);
                        self.mem_slice.write_opcode(OpCode::GetGlobal, pos);
                    }
                } else {
                    self.mem_slice.add_const(Box::new(FVar(name)), pos);
                    self.mem_slice.write_opcode(OpCode::GetGlobal, pos);
                }
            }

            Expr::Assign {
                init,
                left,
                right,
                pos,
            } => {
                if self.scope_depth == 0 {
                    // global variable
                    fn compile(c: &mut Compiler, expr: &Expr) {
                        match expr {
                            Expr::Var { name, pos } => {
                                c.mem_slice.add_const(Box::new(FVar(name.clone())), *pos);
                            }
                            Expr::Empty(pos) => {
                                c.mem_slice.write_opcode(OpCode::LoadEmpty, *pos);
                            }
                            _ => {} // does not happen
                        }
                    }

                    match *left {
                        Expr::Var { name, pos } => {
                            self.mem_slice.add_const(Box::new(FVar(name)), pos);
                        }
                        Expr::List { elems, pos } => self.compile_list(elems, pos, compile),
                        Expr::Obj { keys, vals, pos } => self.compile_obj(keys, vals, pos, compile),
                        _ => {} // does not happen
                    }

                    self.compile_expr(*right);
                    self.mem_slice.write_opcode(
                        if init {
                            OpCode::DefGlobal
                        } else {
                            OpCode::SetGlobal
                        },
                        pos,
                    );
                } else {
                    // local variable
                    if init {
                        // definition
                        self.check_local(&left);

                        fn compile(c: &mut Compiler, expr: &Expr) {
                            match expr {
                                Expr::Var { name, pos } => {
                                    c.mem_slice.add_const(Box::new(FVar(name.clone())), *pos);
                                    c.add_local(name.clone());
                                }
                                Expr::Empty(pos) => {
                                    c.mem_slice.write_opcode(OpCode::LoadEmpty, *pos);
                                }
                                _ => {} // does not happen
                            }
                        }

                        match *left {
                            Expr::Var { name, pos } => {
                                self.mem_slice.add_const(Box::new(FVar(name.clone())), pos);
                                self.add_local(name);
                            }
                            Expr::List { elems, pos } => {
                                self.compile_list(elems, pos, compile);
                            }
                            Expr::Obj { keys, vals, pos } => {
                                self.compile_obj(keys, vals, pos, compile);
                            }
                            _ => {} // does not happen
                        }

                        self.compile_expr(*right);
                        self.mem_slice.write_opcode(OpCode::DefLocal, pos);
                    } else {
                        // reassignment
                        self.compile_expr(*right);

                        fn compile_for_list(c: &mut Compiler, expr: &Expr) {
                            match expr {
                                Expr::Var { name, pos } => {
                                    c.mem_slice.write_byte(1, *pos);
                                    c.mem_slice.write_byte(
                                        c.resolve_local(name.clone(), false, *pos).unwrap() as u8,
                                        *pos,
                                    );
                                }
                                Expr::Empty(pos) => {
                                    c.mem_slice.write_byte(0, *pos);
                                    c.mem_slice.write_byte(0, *pos);
                                }
                                _ => {} // does not happen
                            }
                        }

                        match *left {
                            Expr::Var { name, pos } => {
                                self.mem_slice.write_opcode(OpCode::SetLocalVar, pos);
                                self.mem_slice.write_byte(
                                    self.resolve_local(name, false, pos).unwrap() as u8,
                                    pos,
                                );
                            }
                            Expr::List { elems, pos } => {
                                self.mem_slice.write_opcode(OpCode::SetLocalList, pos);
                                self.compile_list(elems, pos, compile_for_list);
                            }
                            Expr::Obj { keys, vals, pos } => {
                                // checking the length of the object
                                if keys.len() > u8::MAX as usize {
                                    self.report_err("object literal too big".to_string(), pos);
                                }

                                fn token_to_str(t: &Token) -> Arc<str> {
                                    match t.clone().kind {
                                        TokenType::Id(name) => name,
                                        _ => todo!(), // just panic
                                    }
                                }

                                // compiling the keys
                                keys.iter().for_each(|k| {
                                    self.mem_slice
                                        .add_const(Box::new(FVar(token_to_str(k))), k.pos);
                                });
                                // writing the opcode
                                self.mem_slice.write_opcode(OpCode::SetLocalObj, pos);
                                // writing the length
                                self.mem_slice.write_byte(keys.len() as u8, pos);
                                // writing the references to the local variables
                                vals.iter().for_each(|expr| match expr {
                                    Expr::Var { name, pos } => self.mem_slice.write_byte(
                                        self.resolve_local(name.clone(), false, *pos).unwrap()
                                            as u8,
                                        *pos,
                                    ),
                                    Expr::Empty(pos) => self.mem_slice.write_byte(0, *pos),
                                    _ => {} // does not happen
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            Expr::Match {
                cond,
                mut branches,
                pos,
            } => {
                fn compile_branch(
                    c: &mut Compiler,
                    cond: Option<Box<Expr>>,
                    branches: &mut Vec<MatchBranch>,
                    pos: Position,
                ) {
                    let branch = branches.remove(0);

                    // compiling the condition expression if there's one
                    if let Some(cond) = cond {
                        c.compile_expr(*cond);
                    }

                    // compiling the case expression
                    c.compile_expr(*branch.case);

                    backpatch!(
                        c,
                        OpCode::Match,
                        "the match branch is too big".to_string(),
                        pos,
                        {
                            // if there's next branch, write 1, otherwise 0.
                            c.mem_slice
                                .write_byte(if branches.is_empty() { 0 } else { 1 }, pos);
                            // compiling the body of the branch
                            c.compile_expr(*branch.body);
                        }
                    );

                    // if this match branch is the last one, just return and do not add the Jump
                    // opcode.
                    if !branches.is_empty() {
                        // compiling the next match branch recursively
                        backpatch!(
                            c,
                            OpCode::Jump,
                            "the whole match expression is too big".to_string(),
                            pos,
                            {
                                compile_branch(c, None, branches, pos);
                            }
                        );
                    }
                }

                // compiling all match branches
                compile_branch(self, Some(cond), &mut branches, pos);
            }

            Expr::If {
                cond,
                then,
                els,
                pos,
            } => {
                // compiling the condition expression
                self.compile_expr(*cond);

                backpatch!(
                    self,
                    OpCode::JumpIfFalse,
                    "if expression is too big".to_string(),
                    pos,
                    {
                        // compiling the `then` body
                        self.compile_expr(*then);
                    }
                );

                backpatch!(
                    self,
                    OpCode::Jump,
                    "if expression is too big".to_string(),
                    pos,
                    // compiling `else` body if there's one
                    if let Some(els) = els {
                        self.compile_expr(*els);
                    } else {
                        self.mem_slice.add_const(self.error_atom(), pos);
                    }
                );
            }

            Expr::Block { mut exprs, pos } => {
                self.begin_scope();

                // getting the last expression in the block
                let last_expr = exprs.pop().unwrap(); // empty {} is an object, not a block, so
                                                      // there's at least one expression in a
                                                      // block

                // compiling all expressions in the block except for the last expression
                exprs.iter().for_each(|expr| {
                    self.compile_expr(expr.clone());
                    self.mem_slice.write_opcode(OpCode::Pop, pos);
                });

                // calculating the number of pops needed to remove all local variables in this
                // particular block
                let pops_needed = self.locals.iter().fold(0, |acc, local| {
                    if local.depth > self.scope_depth - 1 {
                        acc + 1
                    } else {
                        acc
                    }
                });

                // checking if the number of pops required is over 0xFF or not
                if pops_needed > u8::MAX as usize {
                    self.report_err("too many local variables in this scope".to_string(), pos);
                }

                // checking if the last expression is an assignment
                if let Expr::Assign {
                    init,
                    left: _,
                    right: _,
                    pos,
                } = &last_expr
                {
                    if *init {
                        self.report_err(
                            "last expression of a block cannot be an assignment".to_string(),
                            *pos,
                        );
                    }
                }

                // compiling the last expression, which is the value of the block
                self.compile_expr(last_expr);

                // adding pop instructions needed according to the number of the local variables in
                // this block
                match pops_needed.cmp(&1) {
                    Ordering::Greater => {
                        self.mem_slice.write_opcode(OpCode::PopExceptLastN, pos);
                        self.mem_slice.write_byte(pops_needed as u8, pos);
                    }
                    Ordering::Equal => {
                        self.mem_slice.write_opcode(OpCode::PopExceptLast, pos);
                    }
                    Ordering::Less => {} // do nothing
                }

                // removing all local variables in this block
                while !self.locals.is_empty()
                    && self.locals.last().unwrap().depth > self.scope_depth - 1
                {
                    self.locals.pop();
                }

                self.end_scope();
            }

            Expr::Str { val, pos } => self.mem_slice.add_const(Box::new(FStr(val)), pos),

            Expr::Atom { val, pos } => self.mem_slice.add_const(Box::new(FAtom(val)), pos),

            Expr::Int { val, pos } => match val {
                0 => self.mem_slice.write_opcode(OpCode::LoadInt0, pos),
                1 => self.mem_slice.write_opcode(OpCode::LoadInt1, pos),
                2 => self.mem_slice.write_opcode(OpCode::LoadInt2, pos),
                3 => self.mem_slice.write_opcode(OpCode::LoadInt3, pos),
                _ => self.mem_slice.add_const(Box::new(FInt(val)), pos),
            },

            Expr::Float { val, pos } => self.mem_slice.add_const(Box::new(FFloat(val)), pos),

            Expr::Bool { val, pos } => self.mem_slice.write_opcode(
                if val {
                    OpCode::LoadTrue
                } else {
                    OpCode::LoadFalse
                },
                pos,
            ),

            Expr::Empty(pos) => self.mem_slice.write_opcode(OpCode::LoadEmpty, pos),

            Expr::List { elems, pos } => {
                fn compile_expr(c: &mut Compiler, expr: &Expr) {
                    c.compile_expr(expr.clone());
                }
                self.compile_list(elems, pos, compile_expr);
            }

            Expr::Obj { keys, vals, pos } => {
                fn compile_expr(c: &mut Compiler, expr: &Expr) {
                    c.compile_expr(expr.clone());
                }
                self.compile_obj(keys, vals, pos, compile_expr);
            }

            Expr::Get { inst, attr, pos } => {
                self.compile_expr(*inst);
                self.compile_expr(*attr);
                self.mem_slice.write_opcode(OpCode::Get, pos);
            }

            Expr::Set {
                inst,
                attr,
                val,
                pos,
            } => {
                self.compile_expr(*inst);
                self.compile_expr(*attr);
                self.compile_expr(*val);
                self.mem_slice.write_opcode(OpCode::Set, pos);
            }

            Expr::Func {
                name,
                params,
                rest,
                body,
                pos,
            } => todo!(),

            Expr::Import { exprs, pos } => todo!(),

            Expr::Panic { expr, pos } => todo!(),

            Expr::Recover {
                recoveree,
                body,
                pos,
            } => todo!(),

            Expr::Call { callee, args, pos } => todo!(),
        }
    }

    fn error_atom(&self) -> Box<FAtom> {
        Box::new(FAtom(Arc::from("err")))
    }

    /// Compiles into a list
    fn compile_list(&mut self, elems: Vec<Expr>, pos: Position, func: fn(&mut Compiler, &Expr)) {
        // checking the length of the list
        if elems.len() > u8::MAX as usize {
            self.report_err("list literal too big".to_string(), pos);
        }

        // compiling the elements
        elems.iter().for_each(|expr| func(self, expr));

        // adding the list initialization opcode
        self.mem_slice.write_opcode(OpCode::InitList, pos);

        // writing the length
        self.mem_slice.write_byte(elems.len() as u8, pos);
    }

    /// Compiles into an object
    fn compile_obj(
        &mut self,
        keys: Vec<Token>,
        vals: Vec<Expr>,
        pos: Position,
        func: fn(&mut Compiler, &Expr),
    ) {
        // checking the size of the object
        if keys.len() > u8::MAX as usize {
            self.report_err("object literal too big".to_string(), pos);
        }

        keys.iter().zip(vals.iter()).for_each(|(k, v)| {
            // compiling the value
            func(self, v);
            // writing the key
            if let TokenType::Id(v) = k.clone().kind {
                self.mem_slice.add_const(Box::new(FVar(v)), pos)
            }
        });
    }

    /// Adds a reference to a local variable
    fn add_local(&mut self, name: Arc<str>) {
        self.locals.push(Local {
            name,
            depth: self.scope_depth,
        });
    }

    /// Resolves a local variable and returns the index of the variable
    fn resolve_local(&self, name: Arc<str>, global: bool, pos: Position) -> Option<usize> {
        for idx in (0..=(self.locals.len() - 1)).rev() {
            if self.locals[idx].name.as_ref() == name.as_ref() {
                return Some(idx);
            }
        }
        if !global {
            self.report_err(
                format!("local variable {} is not defined", name.as_ref()),
                pos,
            );
        }
        None
    }

    /// Checks if the local variable is already defined or not
    fn check_local(&self, left: &Expr) {
        fn check(s: &Compiler, local: Local, l: Expr) {
            match l {
                Expr::Var { name, pos } => {
                    if local.name.as_ref() == name.as_ref() {
                        s.report_err(
                            format!(
                                "local variable {} is already defined in this scope",
                                name.as_ref()
                            ),
                            pos,
                        );
                    }
                }
                Expr::List { elems, pos: _ } => elems
                    .iter()
                    .for_each(|i| check(s, local.clone(), i.clone())),
                Expr::Obj {
                    keys: _,
                    vals,
                    pos: _,
                } => vals.iter().for_each(|i| check(s, local.clone(), i.clone())),
                Expr::Empty(_) => {} // do nothing
                _ => {}              // does not happen
            }
        }

        self.locals.iter().for_each(|local| {
            if local.depth >= self.scope_depth {
                check(self, local.clone(), left.clone());
            }
        });
    }

    /// Moves the iterator to the next expression to compile
    fn next_expr(&mut self) {
        if let Some(next) = self.exprs.next() {
            self.current = next;
        }
    }

    /// Reports an error to the user with the given message
    fn report_err(&self, msg: String, pos: Position) {
        Stack::new(ErrType::Syntax, msg, pos, self.path_idx).report(65);
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
    }
}

#[cfg(test)]
mod tests {}
