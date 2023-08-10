use std::sync::Arc;
use std::vec::IntoIter;

use self::expr::{CallArg, CallArgType, Expr, MatchBranch, WhenBranch};
use crate::error::{ErrType, Position, Stack};
use crate::lexer::test_tokenize;
use crate::lexer::token::{Token, TokenType};
use crate::util::PrevPeekable;

pub mod expr;

pub struct Parser {
    /// The tokens being parsed
    tokens: PrevPeekable<IntoIter<Token>>,
    /// The current token being parsed
    current: Token,
    /// The path index of the source being tokenized
    path_idx: usize,
    /// The parsed expressions
    exprs: Vec<Expr>,
}

impl Parser {
    pub fn parse(tokens: Vec<Token>) -> Vec<Expr> {
        let mut tokens = PrevPeekable::new(tokens.into_iter());
        let current_token = tokens.next();

        // no tokens
        if current_token.is_none() {
            std::process::exit(0);
        }

        let mut parser = Parser {
            current: current_token.unwrap(),
            tokens,
            path_idx: Stack::last_path_index(),
            exprs: vec![],
        };
        parser._parse();
        parser.exprs
    }

    /// Parses the tokens into AST expressions
    fn _parse(&mut self) {
        while !self.is_end() {
            let expr = self.expression();
            self.exprs.push(expr);
        }
    }

    /// Returns an expression
    fn expression(&mut self) -> Expr {
        self.assignment()
    }

    /// Returns an expression related to assignment (and ranges)
    fn assignment(&mut self) -> Expr {
        let expr = self.or_expression();
        // variable assignment
        if self.matches_either(&[TokenType::Equal, TokenType::ColonEq]) {
            let init = self.matches(TokenType::ColonEq);

            self.advance();

            let val = self.expression();
            if let Expr::Assign {
                init: _,
                left: _,
                right: _,
                pos: _,
            } = val
            {
                self.report_err("assignment value should not be an assignment");
            }

            // check for destructuring assignment
            fn check(p: &mut Parser, pos: Position, v: Expr, msg: &str) {
                match v {
                    Expr::Var { name: _, pos: _ }
                    | Expr::Get {
                        inst: _,
                        attr: _,
                        pos: _,
                    }
                    | Expr::Empty(_) => {}
                    _ => p.report_err_with_pos(msg, pos),
                }
            }

            match expr.clone() {
                Expr::Obj { keys: _, vals, pos } => {
                    vals.into_iter()
                        .for_each(|v| check(self, pos, v, "expected variable name or _ as values"));
                    return Expr::Assign {
                        init,
                        left: Box::new(expr),
                        right: Box::new(val),
                        pos,
                    };
                }
                Expr::List { elems, pos } => {
                    elems.into_iter().for_each(|v| {
                        check(self, pos, v, "expected variable name or _ as elements");
                    });
                    return Expr::Assign {
                        init,
                        left: Box::new(expr),
                        right: Box::new(val),
                        pos,
                    };
                }
                Expr::Var { name: _, pos } => {
                    return Expr::Assign {
                        init,
                        left: Box::new(expr),
                        right: Box::new(val),
                        pos,
                    };
                }
                Expr::Get { inst, attr, pos } => {
                    return Expr::Set {
                        inst,
                        attr,
                        val: Box::new(val),
                        pos,
                    };
                }
                _ => self.report_err("invalid assignment target"),
            }
        // short cut assignments
        } else if self.matches_either(&[
            TokenType::PlusEq,
            TokenType::MinusEq,
            TokenType::MultEq,
            TokenType::DivEq,
            TokenType::ModEq,
        ]) {
            self.advance();
            let op = self.previous();
            let val = self.expression();

            match expr {
                Expr::Var { name: _, pos: _ } => {
                    return Expr::Assign {
                        init: false,
                        left: Box::new(expr.clone()),
                        right: Box::new(Expr::Binary {
                            left: Box::new(expr),
                            right: Box::new(val),
                            op: op.clone(),
                        }),
                        pos: op.pos,
                    }
                }
                _ => self.report_err_with_token("expected a variable", op),
            }
        }
        expr
    }

    /// Returns an expression related to 'or' logical expression
    fn or_expression(&mut self) -> Expr {
        let mut expr = self.and_expression();
        while self.matches(TokenType::Or) {
            self.advance();
            let token = self.previous();
            expr = Expr::Logic {
                left: Box::new(expr),
                right: Box::new(self.and_expression()),
                op: token,
            };
        }
        expr
    }

    /// Returns an expression related to 'and' logical expression
    fn and_expression(&mut self) -> Expr {
        let mut expr = self.eq_expression();
        while self.matches(TokenType::And) {
            self.advance();
            let token = self.previous();
            expr = Expr::Logic {
                left: Box::new(expr),
                right: Box::new(self.eq_expression()),
                op: token,
            };
        }
        expr
    }

    /// Returns an expression related to equality ('!=' and '==') expressions
    fn eq_expression(&mut self) -> Expr {
        let mut expr = self.comp_expression();
        while self.matches_either(&[TokenType::BangEq, TokenType::DoubleEq]) {
            self.advance();
            let token = self.previous();
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.comp_expression()),
                op: token,
            };
        }
        expr
    }

    /// Returns an expression related to comparison (<=, >, etc.) expressions
    fn comp_expression(&mut self) -> Expr {
        let mut expr = self.term_expression();
        while self.matches_either(&[
            TokenType::GT,
            TokenType::LT,
            TokenType::GTEq,
            TokenType::LTEq,
        ]) {
            self.advance();
            let token = self.previous();
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.term_expression()),
                op: token,
            };
        }
        expr
    }

    /// Returns an expression related to terms (subtraction and addition)
    fn term_expression(&mut self) -> Expr {
        let mut expr = self.factor_expression();
        while self.matches_either(&[TokenType::Minus, TokenType::Plus]) {
            self.advance();
            let token = self.previous();
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.factor_expression()),
                op: token,
            };
        }
        expr
    }

    /// Parses an expression related to factors (division, multiplication, etc.)
    fn factor_expression(&mut self) -> Expr {
        let mut expr = self.unary_expression();
        while self.matches_either(&[TokenType::Div, TokenType::Mult, TokenType::Mod]) {
            self.advance();
            let token = self.previous();
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.unary_expression()),
                op: token,
            };
        }
        expr
    }

    /// Parses a unary expression (negation)
    fn unary_expression(&mut self) -> Expr {
        if self.matches_either(&[TokenType::Bang, TokenType::Minus, TokenType::Not]) {
            self.advance();
            let token = self.previous();
            Expr::Unary {
                right: Box::new(self.unary_expression()),
                op: token,
            }
        } else {
            self.call(None)
        }
    }

    /// Handles a function call, object access, list indexing, and pipe expressions
    fn call(&mut self, arg: Option<Expr>) -> Expr {
        let mut expr = self.primary_expression();

        loop {
            if self.matches(TokenType::LParen) {
                // a function call
                expr = self.finish_call(expr, arg.clone());
            } else if self.matches(TokenType::Dot) {
                // object access or list indexing
                self.advance();
                let token = self.previous();
                expr = Expr::Get {
                    inst: Box::new(expr),
                    attr: Box::new(self.expression()),
                    pos: token.pos,
                };
            } else if self.matches(TokenType::BarGT) {
                // pipe expression |>
                self.advance();
                expr = self.call(Some(expr));
            } else {
                break;
            }
        }
        expr
    }

    /// Finishes the rest of the parsing of a function call
    fn finish_call(&mut self, callee: Expr, arg: Option<Expr>) -> Expr {
        self.expect(TokenType::LParen, "expected '('");

        let mut args: Vec<CallArg> = vec![];

        if let Some(arg) = arg {
            args.push(CallArg {
                kind: CallArgType::Positional,
                expr: Box::new(arg),
            });
        }

        if !self.matches(TokenType::RParen) {
            args.push(CallArg {
                kind: if self.matches(TokenType::Ellipsis) {
                    CallArgType::Unpacking
                } else {
                    CallArgType::Positional
                },
                expr: Box::new(self.expression()),
            });

            while self.matches(TokenType::Comma) {
                self.advance();
                if self.matches(TokenType::Ellipsis) {
                    self.advance();
                    args.push(CallArg {
                        kind: CallArgType::Unpacking,
                        expr: Box::new(self.expression()),
                    });
                } else {
                    args.push(CallArg {
                        kind: CallArgType::Positional,
                        expr: Box::new(self.expression()),
                    });
                }
            }
        }

        self.expect(TokenType::RParen, "expected ')' after arguments");

        let token = self.previous();

        // check for `<|`
        if self.matches(TokenType::BarLT) {
            self.advance();
            args.push(CallArg {
                kind: CallArgType::Positional,
                expr: Box::new(self.expression()),
            });
        // check for `<~`
        } else if self.matches(TokenType::LTilde) {
            self.advance();
            let token = self.previous();
            let param = Token {
                kind: TokenType::Id(Arc::from("it")),
                pos: token.pos,
            };
            let body = self.expression();
            let func = Expr::Func {
                name: None,
                params: vec![param],
                rest: None,
                body: Box::new(body),
                pos: token.pos,
            };

            args.push(CallArg {
                kind: CallArgType::Positional,
                expr: Box::new(func),
            });
        // check for `~`
        } else if self.matches(TokenType::Tilde) {
            self.advance();
            let token = self.previous();
            let (params, rest) = if self.matches(TokenType::LParen) {
                self.parse_params()
            } else {
                (vec![], None)
            };
            let body = self.expression();
            let func = Expr::Func {
                name: None,
                params,
                rest,
                body: Box::new(body),
                pos: token.pos,
            };

            args.push(CallArg {
                kind: CallArgType::Positional,
                expr: Box::new(func),
            });
        }

        Expr::Call {
            callee: Box::new(callee),
            args,
            pos: token.pos,
        }
    }

    /// Handles `if`, `try`, function, primitive and complex types, and block expressions
    fn primary_expression(&mut self) -> Expr {
        match self.current.clone().kind {
            // booleans
            TokenType::True | TokenType::False => {
                self.advance();
                let token = self.previous();
                Expr::Bool {
                    val: token.kind == TokenType::True,
                    pos: token.pos,
                }
            }
            // empty
            TokenType::Empty => {
                self.advance();
                Expr::Empty(self.previous().pos)
            }
            // nil
            TokenType::Nil => {
                self.advance();
                Expr::Nil(self.previous().pos)
            }
            // integer
            TokenType::Int(v) => {
                self.advance();
                Expr::Int {
                    val: v,
                    pos: self.previous().pos,
                }
            }
            // float
            TokenType::Float(v) => {
                self.advance();
                Expr::Float {
                    val: v,
                    pos: self.previous().pos,
                }
            }
            // string
            TokenType::Str(v) => {
                self.advance();
                Expr::Str {
                    val: v,
                    pos: self.previous().pos,
                }
            }
            // atom
            TokenType::Atom(v) => {
                self.advance();
                Expr::Atom {
                    val: v,
                    pos: self.previous().pos,
                }
            }
            // variable
            TokenType::Id(v) => {
                self.advance();
                Expr::Var {
                    name: v,
                    pos: self.previous().pos,
                }
            }
            // grouping
            TokenType::LParen => {
                self.advance();
                let expr = self.expression();

                // grouping
                self.expect(TokenType::RParen, "expected ')'");
                Expr::Group(Box::new(expr))
            }
            // list or range literal
            TokenType::LBracket => {
                self.advance();
                let token = self.previous();
                let mut elems: Vec<Expr> = vec![];

                // appends elements of the list
                while !self.is_end() && !self.matches(TokenType::RBracket) {
                    elems.push(self.expression());
                    if self.matches(TokenType::RBracket) || !self.matches(TokenType::Comma) {
                        break;
                    } else {
                        self.advance();
                    }
                }

                self.expect(TokenType::RBracket, "expected ']' after elements");

                Expr::List {
                    elems,
                    pos: token.pos,
                }
            }
            // set object
            TokenType::SLBrace => {
                self.advance();
                let token = self.previous();
                let mut keys: Vec<Token> = vec![];

                while !self.matches(TokenType::RBrace) && !self.is_end() {
                    match self.current.kind {
                        TokenType::Id(_) => self.advance(),
                        _ => self.report_err("expected an identifier"),
                    }
                    keys.push(self.previous());

                    if self.matches(TokenType::RBrace) {
                        self.advance();
                        break;
                    } else if self.matches(TokenType::Comma) {
                        self.advance();
                    } else {
                        self.report_err("expected ',' or '}'");
                    }
                }

                let vals = keys
                    .clone()
                    .into_iter()
                    .map(|t| match t.clone().kind {
                        TokenType::Id(v) => Expr::Atom { val: v, pos: t.pos },
                        _ => todo!(), // does not happen
                    })
                    .collect::<Vec<Expr>>();

                Expr::Obj {
                    keys,
                    vals,
                    pos: token.pos,
                }
            }
            // identifier object
            TokenType::ILBrace => {
                self.advance();
                let token = self.previous();
                let mut keys: Vec<Token> = vec![];

                while !self.matches(TokenType::RBrace) && !self.is_end() {
                    match self.current.kind {
                        TokenType::Id(_) => self.advance(),
                        _ => self.report_err("expected an identifier"),
                    }
                    keys.push(self.previous());

                    if self.matches(TokenType::RBrace) {
                        self.advance();
                        break;
                    } else if self.matches(TokenType::Comma) {
                        self.advance();
                    } else {
                        self.report_err("expected ',' or '}'");
                    }
                }

                let vals = keys
                    .clone()
                    .into_iter()
                    .map(|t| match t.clone().kind {
                        TokenType::Id(v) => Expr::Var {
                            name: v,
                            pos: t.pos,
                        },
                        _ => todo!(), // does not happen
                    })
                    .collect::<Vec<Expr>>();

                Expr::Obj {
                    keys,
                    vals,
                    pos: token.pos,
                }
            }

            // object literal or block
            TokenType::LBrace => {
                self.advance();
                let token = self.previous();

                if self.matches(TokenType::RBrace) {
                    // an empty {} is considered an object, not a block
                    self.advance();

                    Expr::Obj {
                        keys: vec![],
                        vals: vec![],
                        pos: token.pos,
                    }
                } else {
                    let first_expr = self.expression();

                    if self.is_end() {
                        self.report_err(
                            "unexpected end of input while parsing a block or an object",
                        )
                    }

                    if self.matches(TokenType::MinusGT) {
                        // it's an object
                        self.advance();
                        let key = match first_expr {
                            Expr::Var { name, pos } => Token {
                                kind: TokenType::Id(name),
                                pos,
                            },
                            _ => {
                                self.report_err("expected an identifier");
                                Token {
                                    kind: TokenType::Plus,
                                    pos: (0, 0),
                                } // dummy
                            }
                        };
                        let mut keys: Vec<Token> = vec![key];
                        let mut vals: Vec<Expr> = vec![];

                        if self.previous().kind != TokenType::MinusGT {
                            self.report_err("expected '->'");
                        }
                        vals.push(self.expression());

                        if self.matches(TokenType::Comma) {
                            self.advance();

                            while !self.matches(TokenType::RBrace) && !self.is_end() {
                                match self.current.kind {
                                    TokenType::Id(_) => self.advance(),
                                    _ => self.report_err("expected an identifier"),
                                }
                                keys.push(self.previous());
                                self.expect(TokenType::MinusGT, "expected '->'");
                                vals.push(self.expression());

                                if self.matches(TokenType::RBrace)
                                    || !self.matches(TokenType::Comma)
                                {
                                    break;
                                } else {
                                    self.advance();
                                }
                            }
                        }

                        self.expect(TokenType::RBrace, "expected '}'");

                        Expr::Obj {
                            keys,
                            vals,
                            pos: token.pos,
                        }
                    } else {
                        // it's a block
                        let mut exprs: Vec<Expr> = vec![first_expr];

                        while !self.matches(TokenType::RBrace) && !self.is_end() {
                            exprs.push(self.expression());
                        }

                        self.expect(TokenType::RBrace, "expected '}' after a block");

                        Expr::Block {
                            exprs,
                            pos: token.pos,
                        }
                    }
                }
            }
            // anonymous function
            TokenType::BackDiv => {
                self.advance();
                let token = self.previous();
                let (params, rest) = self.parse_params();
                self.expect(
                    TokenType::MinusGT,
                    "expected '->' after anonymous function parameters",
                );
                let body = self.expression();

                Expr::Func {
                    name: None,
                    params,
                    rest,
                    body: Box::new(body),
                    pos: token.pos,
                }
            }
            // function
            TokenType::Func => {
                self.advance();
                let token = self.previous();
                let name: Option<Arc<str>> = match self.current.clone().kind {
                    TokenType::Id(name) => {
                        self.advance();
                        Some(name)
                    }
                    _ => None,
                };
                let (params, rest) = self.parse_params();
                self.expect(TokenType::Equal, "expected '=' after function parameters");
                let body = self.expression();

                Expr::Func {
                    name,
                    params,
                    rest,
                    body: Box::new(body),
                    pos: token.pos,
                }
            }
            // if expression
            TokenType::If => {
                self.advance();
                let token = self.previous();
                let cond = self.expression();
                self.expect(TokenType::Then, "expected 'then' after condition");
                let then = self.expression();
                let mut els: Option<Box<Expr>> = None;

                // check for an else clause
                if self.matches(TokenType::Else) {
                    self.advance();
                    els = Some(Box::new(self.expression()));
                }

                Expr::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els,
                    pos: token.pos,
                }
            }
            // match expression
            TokenType::Match => {
                self.advance();
                let token = self.previous();
                let cond = self.expression();
                let mut branches: Vec<MatchBranch> = vec![];

                self.expect(TokenType::With, "expected 'with' keyword");

                // an empty match expression is not allowed
                self.expect(TokenType::Bar, "expected '|'");
                let case = self.expression();
                self.expect(TokenType::MinusGT, "expected '->'");
                let body = self.expression();
                branches.push(MatchBranch {
                    case: Box::new(case),
                    body: Box::new(body),
                });

                while self.matches(TokenType::Bar) && !self.is_end() {
                    self.expect(TokenType::Bar, "expected '|'");
                    let case = self.expression();
                    self.expect(TokenType::MinusGT, "expected '->'");
                    let body = self.expression();
                    branches.push(MatchBranch {
                        case: Box::new(case),
                        body: Box::new(body),
                    });
                }

                Expr::Match {
                    cond: Box::new(cond),
                    branches,
                    pos: token.pos,
                }
            }
            // when expression
            TokenType::When => {
                self.advance();
                let token = self.previous();
                let mut branches: Vec<WhenBranch> = vec![];

                // an empty when expression is not allowed
                self.expect(TokenType::Bar, "expected '|'");
                let cond = self.expression();
                self.expect(TokenType::MinusGT, "expected '->'");
                let body = self.expression();
                branches.push(WhenBranch {
                    cond: Box::new(cond),
                    body: Box::new(body),
                });

                while self.matches(TokenType::Bar) && !self.is_end() {
                    self.expect(TokenType::Bar, "expected '|'");
                    let cond = self.expression();
                    self.expect(TokenType::MinusGT, "expected '->'");
                    let body = self.expression();
                    branches.push(WhenBranch {
                        cond: Box::new(cond),
                        body: Box::new(body),
                    });
                }

                Expr::When {
                    branches,
                    pos: token.pos,
                }
            }
            // import expression
            TokenType::Import => {
                self.advance();
                let token = self.previous();
                let mut args: Vec<Expr> = vec![];

                self.expect(TokenType::LParen, "expected '('");

                if !self.matches(TokenType::RParen) {
                    args.push(self.expression());
                    while self.matches(TokenType::Comma) {
                        self.advance();
                        args.push(self.expression());
                    }
                }

                self.expect(TokenType::RParen, "expected ')' after arguments");

                Expr::Import {
                    exprs: args,
                    pos: token.pos,
                }
            }
            // recover expression
            TokenType::Recover => {
                self.advance();
                let token = self.previous();
                let recoveree = self.expression();

                self.expect(TokenType::MinusGT, "expected '->'");

                let body = self.expression();

                Expr::Recover {
                    recoveree: Box::new(recoveree),
                    body: Box::new(body),
                    pos: token.pos,
                }
            }
            // panic expression
            TokenType::Panic => {
                self.advance();
                let token = self.previous();
                let expr = self.expression();

                Expr::Panic {
                    expr: Box::new(expr),
                    pos: token.pos,
                }
            }
            _ => {
                self.report_err(&format!("unexpected token: {:?}", self.current.kind));
                Expr::Empty((0, 0)) // dummy
            }
        }
    }

    /* Helper functions */

    /// Parses parameters of a function
    fn parse_params(&mut self) -> (Vec<Token>, Option<Token>) {
        self.expect(TokenType::LParen, "expected '('");
        let mut params: Vec<Token> = vec![];
        let mut rest: Option<Token> = None;

        if !self.matches(TokenType::RParen) {
            while !self.is_end() {
                if rest.is_some() {
                    self.report_err("required parameter cannot follow a rest parameter")
                }

                match self.current.kind {
                    TokenType::Id(_) => {
                        self.advance();
                        let param = self.previous();
                        if self.matches(TokenType::Plus) {
                            self.advance();
                            rest = Some(param);
                        } else {
                            params.push(param);
                        }
                    }
                    TokenType::Empty => {
                        self.advance();
                        params.push(self.previous().clone());
                    }
                    _ => {}
                }

                if !self.matches(TokenType::Comma) {
                    break;
                } else {
                    self.advance();
                }
            }
        }

        self.expect(TokenType::RParen, "expected ')' after parameters");

        (params, rest)
    }

    /// Checks if the type of the current token is in the list of the expected token types
    fn matches_either(&self, these: &[TokenType]) -> bool {
        for i in these {
            if self.matches(i.clone()) {
                return true;
            }
        }
        false
    }

    /// Returns the previous token
    fn previous(&mut self) -> Token {
        self.tokens.prev().unwrap()
    }

    /// If the type of the current token is as expected, do `advance`, if not, `report_err` with the given message
    fn expect(&mut self, kind: TokenType, msg: &str) {
        if self.matches(kind) {
            self.advance();
        } else {
            self.report_err(msg);
        }
    }

    /// Checks whether the type of the current token is the same one as the expected one or not
    fn matches(&self, kind: TokenType) -> bool {
        self.current.kind == kind
    }

    /// Moves to the next token item
    fn advance(&mut self) {
        self.tokens.next();
        if self.tokens.current.is_some() {
            self.current = self.tokens.current.clone().unwrap();
        }
    }

    /// Checks whether all tokens are consumed or not
    fn is_end(&mut self) -> bool {
        self.tokens.peek().is_none()
    }

    /// Reports an error with the given message and the position
    fn report_err_with_pos(&self, msg: &str, pos: Position) {
        Stack::new(ErrType::Syntax, msg.to_string(), pos, self.path_idx).report(65);
    }

    /// Reports an error with the given message and the current token
    fn report_err(&self, msg: &str) {
        self.report_err_with_pos(msg, self.current.pos);
    }

    /// Reports an error with the given message and the token
    fn report_err_with_token(&self, msg: &str, token: Token) {
        self.report_err_with_pos(msg, token.pos);
    }
}

pub fn test_parse(src: &str) -> Vec<Expr> {
    let tokens = test_tokenize(src);
    let mut tokens = PrevPeekable::new(tokens.into_iter());
    let current_token = tokens.next();

    // no tokens
    if current_token.is_none() {
        std::process::exit(0);
    }

    let mut parser = Parser {
        current: current_token.unwrap(),
        tokens,
        path_idx: 0,
        exprs: vec![],
    };
    parser._parse();
    parser.exprs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_expr() {
        let expr = "1 - 4 * (2 + 3)";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn simple_primitives() {
        let expr = "true false 123 1.23 0xABC \"Hello, world\" :someAtom nil";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn lists_and_objs() {
        let expr = "[1, 2, :null] {name->\"Nobu\", country->:jp}";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn block_and_objs() {
        let expr = "{} {123} {age->17}";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn set_and_identifier_objs() {
        let expr = "s{A, B, C} i{あ, い, エ}";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn functions() {
        let expr = "fn foo(a, b, c+) = _ fn() = _";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn match_expr() {
        let expr = "match name with | \"Nobu\" -> \"Cool!\" | _ -> \"Nice\"";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn when_expr() {
        let expr = "when | name == \"Nobu\" -> \"Cool!\" | _ -> \"Hi\"";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn if_expr() {
        let expr = "if isCool then 1 else 0";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn import_expr() {
        let expr = "import(\"std\", \"fmt\")";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn function_call() {
        let expr = "someFunc(:abc, ...someList) anotherFunc() anotherrrrr([1, 2, 3])";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn anonymous_func() {
        let expr = r#"\(a) -> \(b) -> a + b"#;
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn pipe_expr() {
        let expr = "someFunc() |> chained(\"hello\") <| 123";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn implicit_param_callback_expr() {
        let expr = "[1, 2, 3] |> map() <~ println(it)";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn callback_expr() {
        let expr = "[1, 2, 3] |> map() ~ (each) println(each)";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn recover_expr() {
        let expr = "recover 1 / 0 -> println(\"recovered!\")";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn panic_expr() {
        let expr = "panic \"Waaa something bad happened\"";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }

    #[test]
    fn edge_case() {
        // TODO: hmm, how should i fix this? Refer to https://github.com/bichanna/flan/issues/1
        let expr = "var := 3 * (3 - 4) (1 + 3) |> someFunc()";
        let exprs = test_parse(expr);
        // println!("{:#?}", exprs);
    }
}
