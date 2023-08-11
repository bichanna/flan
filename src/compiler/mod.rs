pub mod opcode;
pub mod util;

use std::cmp::Ordering;
use std::sync::Arc;
use std::vec::IntoIter;

use self::opcode::OpCode;
use self::util::{to_little_endian, to_little_endian_u32, MemorySlice};
use crate::error::{ErrType, Position, Stack};
use crate::lexer::token::{Token, TokenType};
use crate::parser::expr::{Expr, MatchBranch, WhenBranch};
use crate::parser::test_parse;
use crate::util::PrevPeekable;
use crate::vm::function::Function;
use crate::vm::gc::heap::Heap;
use crate::vm::value::*;

/// Applies a classic compiler trick called back-patching with two bytes
macro_rules! backpatch {
    ($c: expr, $op: expr, $err_msg: expr, $pos: expr, $block: block) => {
        // writing the opcode
        $c.mem_slice.write_opcode($op, $pos);

        // getting ready for back-patching
        $c.mem_slice.write_bytes(&[0xFF, 0xFF], $pos);
        let prev = $c.mem_slice.bytecode.len();

        // doing whatever the caller wants to do here
        $block;

        // applying the patch
        let len = $c.mem_slice.bytecode.len() - prev;
        let idx = prev - 2;
        if len > u16::MAX as usize {
            $c.report_err($err_msg, $pos);
        }
        let bytes = to_little_endian(len as u16);
        $c.mem_slice.write_byte_with_index(idx, bytes[0]);
        $c.mem_slice.write_byte_with_index(idx + 1, bytes[1]);
    };
}

/// Applies a classic compiler trick called back-patching with four bytes
macro_rules! backpatch_u32 {
    ($c: expr, $op: expr, $err_msg: expr, $pos: expr, $block: block) => {
        // writing the opcode
        $c.mem_slice.write_opcode($op, $pos);

        // getting ready for back-patching
        $c.mem_slice.write_bytes(&[0xFF, 0xFF, 0xFF, 0xFF], $pos);
        let prev = $c.mem_slice.bytecode.len();

        // doing whatever the caller wants to do here
        $block;

        // applying the patch
        let len = $c.mem_slice.bytecode.len() - prev;
        let idx = prev - 4;
        if len > u32::MAX as usize {
            $c.report_err($err_msg, $pos);
        }
        let bytes = to_little_endian_u32(len as u32);
        $c.mem_slice.write_byte_with_index(idx, bytes[0]);
        $c.mem_slice.write_byte_with_index(idx + 1, bytes[1]);
        $c.mem_slice.write_byte_with_index(idx + 2, bytes[2]);
        $c.mem_slice.write_byte_with_index(idx + 3, bytes[3]);
    };
}

#[derive(Clone)]
struct Local {
    name: Arc<str>,
    depth: usize,
    mutable: bool,
}

pub struct Compiler {
    /// Heap used to allocated objects on
    heap: Heap,
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
    pub fn compile(exprs: Vec<Expr>, heap: Heap, tok_num: usize) -> MemorySlice {
        let mut exprs = PrevPeekable::new(exprs.into_iter());
        let current = exprs.next().unwrap();
        let mut compiler = Compiler {
            heap,
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
        while self.exprs.current.is_some() {
            self.compile_expr_to_self(self.current.clone());
            self.mem_slice.write_opcode(OpCode::Pop, (0, 0));
            self.next_expr();
        }
        self.mem_slice.write_opcode(OpCode::Halt, (0, 0));
    }

    /// Compiles an expression
    fn compile_expr_to_self(&mut self, expr: Expr) {
        match expr {
            Expr::Binary { left, right, op } => {
                let pos = op.pos;
                self.compile_expr_to_self(*left);
                self.compile_expr_to_self(*right);
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
                self.compile_expr_to_self(*expr);
            }

            Expr::Unary { right, op } => {
                self.compile_expr_to_self(*right);
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
                self.compile_expr_to_self(*left);
                self.compile_expr_to_self(*right);

                match op.kind {
                    TokenType::And => self.mem_slice.write_opcode(OpCode::And, pos),
                    TokenType::Or => self.mem_slice.write_opcode(OpCode::Or, pos),
                    _ => self.report_err("invalid logic expression".to_string(), pos),
                }
            }

            Expr::Var { name, pos } => {
                let result = self.resolve_local(name.clone(), true, false, pos);
                if self.scope_depth != 0 {
                    if let Some(result) = result {
                        self.mem_slice.write_opcode(OpCode::GetLocal, pos);
                        self.mem_slice.write_byte(result as u8, pos);
                    } else {
                        self.mem_slice.add_const(FVar::new(name), pos);
                        self.mem_slice.write_opcode(OpCode::GetGlobal, pos);
                    }
                } else {
                    self.mem_slice.add_const(FVar::new(name), pos);
                    self.mem_slice.write_opcode(OpCode::GetGlobal, pos);
                }
            }

            Expr::Assign {
                init,
                left,
                right,
                pos,
                mutable,
            } => {
                if self.scope_depth == 0 {
                    // global variable
                    fn compile(c: &mut Compiler, expr: &Expr) {
                        match expr {
                            Expr::Var { name, pos } => {
                                c.mem_slice.add_const(FVar::new(name.clone()), *pos);
                            }
                            Expr::Empty(pos) => {
                                c.mem_slice.write_opcode(OpCode::LoadEmpty, *pos);
                            }
                            _ => unreachable!(),
                        }
                    }

                    match *left {
                        Expr::Var { name, pos } => {
                            self.mem_slice.add_const(FVar::new(name), pos);
                        }
                        Expr::List { elems, pos } => self.compile_list(elems, pos, compile),
                        Expr::Obj { keys, vals, pos } => self.compile_obj(keys, vals, pos, compile),
                        _ => unreachable!(),
                    }

                    self.compile_expr_to_self(*right);

                    if init {
                        self.mem_slice.write_opcode(OpCode::DefGlobal, pos);
                        self.mem_slice.write_byte(if mutable { 1 } else { 0 }, pos);
                    } else {
                        self.mem_slice.write_opcode(OpCode::SetGlobal, pos);
                    }
                } else {
                    // local variable
                    if init {
                        // definition
                        self.check_local(&left);

                        fn compile(c: &mut Compiler, expr: &Expr) {
                            match expr {
                                Expr::Var { name, pos } => {
                                    c.mem_slice.add_const(FVar::new(name.clone()), *pos);
                                    c.add_local(name.clone(), false);
                                }
                                Expr::Empty(pos) => {
                                    c.mem_slice.write_opcode(OpCode::LoadEmpty, *pos);
                                }
                                _ => unreachable!(),
                            }
                        }

                        match *left {
                            Expr::Var { name, pos } => {
                                self.mem_slice.add_const(FVar::new(name.clone()), pos);
                                self.add_local(name, mutable);
                            }
                            Expr::List { elems, pos } => {
                                self.compile_list(elems, pos, compile);
                            }
                            Expr::Obj { keys, vals, pos } => {
                                self.compile_obj(keys, vals, pos, compile);
                            }
                            _ => unreachable!(),
                        }

                        self.compile_expr_to_self(*right);
                        self.mem_slice.write_opcode(OpCode::DefLocal, pos);
                    } else {
                        // reassignment
                        self.compile_expr_to_self(*right);

                        fn compile_for_list(c: &mut Compiler, expr: &Expr) {
                            match expr {
                                Expr::Var { name, pos } => {
                                    c.mem_slice.write_byte(1, *pos);
                                    c.mem_slice.write_byte(
                                        c.resolve_local(name.clone(), false, true, *pos).unwrap()
                                            as u8,
                                        *pos,
                                    );
                                }
                                Expr::Empty(pos) => {
                                    c.mem_slice.write_byte(0, *pos);
                                    c.mem_slice.write_byte(0, *pos);
                                }
                                _ => unreachable!(),
                            }
                        }

                        match *left {
                            Expr::Var { name, pos } => {
                                self.mem_slice.write_opcode(OpCode::SetLocalVar, pos);
                                self.mem_slice.write_byte(
                                    self.resolve_local(name, false, true, pos).unwrap() as u8,
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
                                    self.mem_slice.add_const(FVar::new(token_to_str(k)), k.pos);
                                });
                                // writing the opcode
                                self.mem_slice.write_opcode(OpCode::SetLocalObj, pos);
                                // writing the length
                                self.mem_slice.write_byte(keys.len() as u8, pos);
                                // writing the references to the local variables
                                vals.iter().for_each(|expr| match expr {
                                    Expr::Var { name, pos } => self.mem_slice.write_byte(
                                        self.resolve_local(name.clone(), false, true, *pos).unwrap()
                                            as u8,
                                        *pos,
                                    ),
                                    Expr::Empty(pos) => self.mem_slice.write_byte(0, *pos),
                                    _ => unreachable!(),
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
                        c.compile_expr_to_self(*cond);
                    }

                    // compiling the case expression
                    c.compile_expr_to_self(*branch.case);

                    backpatch_u32!(
                        c,
                        OpCode::Match,
                        "the match branch is too big".to_string(),
                        pos,
                        {
                            // if there's next branch, write 1, otherwise 0.
                            c.mem_slice
                                .write_byte(if branches.is_empty() { 0 } else { 1 }, pos);
                            // compiling the body of the branch
                            c.compile_expr_to_self(*branch.body);
                        }
                    );

                    // if this match branch is the last one, just return and do not add the Jump
                    // opcode.
                    if !branches.is_empty() {
                        // compiling the next match branch recursively
                        backpatch_u32!(
                            c,
                            OpCode::LongJump,
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

            Expr::When { mut branches, pos } => {
                fn compile_branch(c: &mut Compiler, branches: &mut Vec<WhenBranch>, pos: Position) {
                    let branch = branches.remove(0);

                    // compiling the condition expression of the branch
                    c.compile_expr_to_self(*branch.cond);

                    backpatch!(
                        c,
                        OpCode::JumpIfFalse,
                        "when expression is too big".to_string(),
                        pos,
                        {
                            // compiling the body of the branch
                            c.compile_expr_to_self(*branch.body);
                        }
                    );

                    // compiling the next when branch recursively
                    backpatch_u32!(
                        c,
                        OpCode::LongJump,
                        "when expression is too big".to_string(),
                        pos,
                        {
                            // compiling other branches if there's any
                            if !branches.is_empty() {
                                compile_branch(c, branches, pos);
                            } else {
                                c.mem_slice.write_opcode(OpCode::LoadNil, pos);
                            }
                        }
                    );
                }

                // compiling all when branches
                compile_branch(self, &mut branches, pos);
            }

            Expr::If {
                cond,
                then,
                els,
                pos,
            } => {
                // compiling the condition expression
                self.compile_expr_to_self(*cond);

                backpatch!(
                    self,
                    OpCode::JumpIfFalse,
                    "if expression is too big".to_string(),
                    pos,
                    {
                        // compiling the `then` body
                        self.compile_expr_to_self(*then);
                    }
                );

                backpatch!(
                    self,
                    OpCode::Jump,
                    "if expression is too big".to_string(),
                    pos,
                    // compiling `else` body if there's one
                    {
                        if let Some(els) = els {
                            self.compile_expr_to_self(*els);
                        } else {
                            self.mem_slice.write_opcode(OpCode::LoadNil, pos);
                        }
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
                    self.compile_expr_to_self(expr.clone());
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
                    mutable: _,
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
                self.compile_expr_to_self(last_expr);

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

            Expr::Str { val, pos } => self
                .mem_slice
                .add_const(FStr::new(&mut self.heap, val), pos),

            Expr::Atom { val, pos } => self.mem_slice.add_const(FAtom::new(val), pos),

            Expr::Int { val, pos } => match val {
                0 => self.mem_slice.write_opcode(OpCode::LoadInt0, pos),
                1 => self.mem_slice.write_opcode(OpCode::LoadInt1, pos),
                2 => self.mem_slice.write_opcode(OpCode::LoadInt2, pos),
                3 => self.mem_slice.write_opcode(OpCode::LoadInt3, pos),
                _ => self.mem_slice.add_const(FInt::new(val), pos),
            },

            Expr::Float { val, pos } => self.mem_slice.add_const(FFloat::new(val), pos),

            Expr::Bool { val, pos } => self.mem_slice.write_opcode(
                if val {
                    OpCode::LoadTrue
                } else {
                    OpCode::LoadFalse
                },
                pos,
            ),

            Expr::Empty(pos) => self.mem_slice.write_opcode(OpCode::LoadEmpty, pos),

            Expr::Nil(pos) => self.mem_slice.write_opcode(OpCode::LoadNil, pos),

            Expr::List { elems, pos } => {
                fn compile_expr(c: &mut Compiler, expr: &Expr) {
                    c.compile_expr_to_self(expr.clone());
                }
                self.compile_list(elems, pos, compile_expr);
            }

            Expr::Obj { keys, vals, pos } => {
                fn compile_expr(c: &mut Compiler, expr: &Expr) {
                    c.compile_expr_to_self(expr.clone());
                }
                self.compile_obj(keys, vals, pos, compile_expr);
            }

            Expr::Get { inst, attr, pos } => {
                self.compile_expr_to_self(*inst);
                self.compile_expr_to_self(*attr);
                self.mem_slice.write_opcode(OpCode::GetProperty, pos);
            }

            Expr::Set {
                inst,
                attr,
                val,
                pos,
            } => {
                self.compile_expr_to_self(*inst);
                self.compile_expr_to_self(*attr);
                self.compile_expr_to_self(*val);
                self.mem_slice.write_opcode(OpCode::SetProperty, pos);
            }

            Expr::Func {
                name,
                params,
                rest,
                body,
                pos,
            } => {
                let params_len = params.len();
                let has_name = name.is_some();

                // checking the number of parameters
                if params_len > u8::MAX as usize {
                    self.report_err("too many parameters".to_string(), pos);
                }

                // creating function object
                let func = FFunc::new(&mut self.heap, Function::new(params_len, rest.is_some()));

                // if there's a name for this function
                if let Some(name) = name {
                    self.mem_slice.add_const(FVar::new(name.clone()), pos);
                    if self.scope_depth != 0 {
                        self.add_local(name, true);
                    }
                }

                // writing the instruction to load the function object
                self.mem_slice.add_const(func, pos);

                // writing the instruction to define a function
                self.mem_slice.write_opcode(OpCode::SetFnAddr, pos);

                // adding the instruction to jump through the function body
                backpatch_u32!(
                    self,
                    OpCode::LongJump,
                    "function too big".to_string(),
                    pos,
                    {
                        self.begin_scope();

                        // defining the parameters
                        params.iter().for_each(|p| match p.0.kind {
                            TokenType::Id(ref name) => self.add_local(name.clone(), p.1),
                            _ => unreachable!(),
                        });

                        // if there's a rest parameter, define the parameter
                        if let Some(rest) = rest {
                            match rest.0.kind {
                                TokenType::Id(ref name) => self.add_local(name.clone(), rest.1),
                                _ => unreachable!(),
                            }
                        }

                        // compiling the body of the function
                        self.compile_expr_to_self(*body);

                        // calculating the number of pops needed to remove all local variables in this
                        // particular function body
                        let pops_needed = self.locals.iter().fold(0, |acc, local| {
                            if local.depth > self.scope_depth - 1 {
                                acc + 1
                            } else {
                                acc
                            }
                        });

                        // adding pop instructions needed according to the number of the local variables in
                        // this function
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

                        // writing the return instruction
                        self.mem_slice.write_opcode(OpCode::RetFn, pos);

                        // TODO: return instruction for the function body

                        self.end_scope();
                    }
                );

                if has_name {
                    if self.scope_depth == 0 {
                        self.mem_slice.write_opcode(OpCode::DefGlobal, pos);
                        self.mem_slice.write_byte(1, pos);
                    } else {
                        self.mem_slice.write_opcode(OpCode::DefLocal, pos);
                    }
                }
            }

            Expr::Import { exprs, pos } => todo!(),

            Expr::Panic { expr, pos } => todo!(),

            Expr::Recover {
                recoveree,
                body,
                pos,
            } => todo!(),

            Expr::Call { callee, args, pos } => {
                // TODO: handle unpacking arguments

                // checking the length of the arguments
                if args.len() > u8::MAX as usize {
                    self.report_err("too many arguments".to_string(), pos);
                }

                // compiling the callee
                self.compile_expr_to_self(*callee);

                // compiling the arguments to the function
                args.iter()
                    .for_each(|arg| self.compile_expr_to_self(*arg.expr.clone()));

                // writing the instruction to call the funcion
                self.mem_slice.write_opcode(OpCode::CallFn, pos);
                // writing the length of the arguments
                self.mem_slice.write_byte(args.len() as u8, pos);
            }
        }
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

        // adding the object initialization opcode
        self.mem_slice.write_opcode(OpCode::InitObj, pos);

        // writing the length
        self.mem_slice.write_byte(keys.len() as u8, pos);
    }

    /// Adds a reference to a local variable
    fn add_local(&mut self, name: Arc<str>, mutable: bool) {
        self.locals.push(Local {
            name,
            depth: self.scope_depth,
            mutable,
        });
    }

    /// Resolves a local variable and returns the index of the variable
    fn resolve_local(
        &self,
        name: Arc<str>,
        global: bool,
        reassign: bool,
        pos: Position,
    ) -> Option<usize> {
        if self.locals.is_empty() {
            return None;
        }
        for idx in (0..=(self.locals.len() - 1)).rev() {
            if self.locals[idx].name == name {
                if reassign && !self.locals[idx].mutable {
                    self.report_err(
                        format!("local variable {} is immutable", name.as_ref()),
                        pos,
                    );
                }
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
                _ => unreachable!(),
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

pub fn test_compile(src: &str) -> (MemorySlice, Heap) {
    let exprs = test_parse(src);
    let mut exprs = PrevPeekable::new(exprs.into_iter());
    let current = exprs.next().unwrap();
    let heap = Heap::new();
    let mut compiler = Compiler {
        heap,
        exprs,
        path_idx: 0,
        locals: Vec::new(),
        scope_depth: 0,
        mem_slice: MemorySlice::new(10),
        current,
    };
    compiler._compile();
    (compiler.mem_slice, compiler.heap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::Debug;

    #[test]
    fn literals() {
        let src = "1 2.34 :nil true false [1, :err] {name -> \"Nobu\"}";
        let (mem_slice, mut heap) = test_compile(src);
        heap.deallocate_all();
        // Debug::run("TEST 1", &mem_slice);
    }
}
