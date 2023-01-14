use std::process;

use crate::ast::{Expr, Node, Stmt, TypeInfo};
use crate::error::ParserError;
use crate::token::{Token, TokenType};

pub struct Parser {
    c: usize,
    current: Token,
    errors: Vec<ParserError>,
    pub statements: Vec<Node>,
}

macro_rules! expect {
    ($self: expr, $kind: expr, $msg: expr, $tokens: expr) => {
        if $self.check_current($kind, $tokens) {
            $self.advance($tokens);
        } else {
            return Err($msg);
        }
    };
}

impl Parser {
    pub fn new(tokens: &Vec<Token>) -> Self {
        Parser {
            c: 0,
            current: tokens[0].clone(),
            errors: vec![],
            statements: vec![],
        }
    }

    /// Reports errors if any
    pub fn report_errors(&self, filename: &str, source: &String) {
        if self.errors.len() > 0 {
            for err in &self.errors {
                println!("{}", err.format(filename));
                println!(
                    "{}",
                    source.split("\n").collect::<Vec<&str>>()[err.line - 1]
                );
            }
            process::exit(1);
        }
    }

    /// Parses tokens to AST
    pub fn parse(&mut self, tokens: &Vec<Token>) {
        while !self.is_end(tokens) {
            let node = self.declaration(tokens);
            match node {
                Ok(node) => self.statements.push(node),
                Err(msg) => {
                    self.add_error(msg);
                    self.synchronize(tokens);
                }
            }
        }
    }

    fn expression(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        return self.assignment(tokens);
    }

    fn assignment(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let expr = self.or_expr(tokens)?;
        if self.check_current(TokenType::Equal, tokens) {
            self.advance(tokens);
            let value = Box::new(self.assignment(tokens)?);

            match expr {
                Expr::Variable { name } => return Ok(Expr::Assign { name, value }),
                Expr::Get { instance, token } => {
                    return Ok(Expr::Set {
                        instance,
                        token,
                        value,
                    })
                }
                _ => return Err("invalid assignment target"),
            }
        } else if self.does_match(
            &[
                TokenType::PlusEq,
                TokenType::MinusEq,
                TokenType::MulEq,
                TokenType::DivEq,
                TokenType::ModEq,
            ],
            tokens,
        ) {
            let op = self.previous(tokens);
            let value = self.assignment(tokens)?;
            match expr {
                Expr::Variable { ref name } => {
                    let name = name.clone();
                    return Ok(Expr::Assign {
                        name,
                        value: Box::new(Expr::Binary {
                            left: Box::new(expr),
                            right: Box::new(value),
                            op,
                        }),
                    });
                }
                _ => return Err("expected a variable"),
            };
        } else if self.does_match(&[TokenType::DPlus, TokenType::DMinus], tokens) {
            let op = self.previous(tokens);
            match expr {
                Expr::Variable { ref name } => {
                    let name = name.clone();
                    return Ok(Expr::Assign {
                        name,
                        value: Box::new(Expr::Binary {
                            left: Box::new(expr),
                            right: Box::new(Expr::Literal {
                                kind: TokenType::Num,
                                value: String::from("1"),
                            }),
                            op,
                        }),
                    });
                }
                _ => return Err("expected a variable"),
            }
        }

        Ok(expr)
    }

    fn primary(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        if self.does_match(
            &[
                TokenType::True,
                TokenType::False,
                TokenType::Null,
                TokenType::Underscore,
            ],
            tokens,
        ) {
            // Boolean, null literal, or Underscore
            let token = self.previous(tokens);
            Ok(Expr::Literal {
                kind: token.kind,
                value: token.value,
            })
        } else if self.does_match(&[TokenType::Num, TokenType::Str, TokenType::Atom], tokens) {
            // string, number literal, or atom
            let token = self.previous(tokens);
            Ok(Expr::Literal {
                kind: token.kind,
                value: token.value,
            })
        } else if self.does_match(&[TokenType::Id], tokens) {
            // identifier
            let token = self.previous(tokens);
            Ok(Expr::Variable { name: token })
        } else if self.does_match(&[TokenType::LParen], tokens) {
            // grouping
            let expr = Box::new(self.expression(tokens)?);
            expect!(self, TokenType::RParen, "expected ')'", tokens);
            Ok(Expr::Group { expr })
        } else if self.does_match(&[TokenType::LBracket], tokens) {
            // list literal
            let mut values: Vec<Box<Expr>> = vec![];
            while !self.check_current(TokenType::RBracket, tokens) && !self.is_end(tokens) {
                values.push(Box::new(self.expression(tokens)?));

                if self.check_current(TokenType::RBracket, tokens)
                    || !self.does_match(&[TokenType::Comma], tokens)
                {
                    break;
                }
            }
            expect!(self, TokenType::RBracket, "expected ']'", tokens);
            Ok(Expr::ListLiteral { values })
        } else if self.does_match(&[TokenType::LBrace], tokens) {
            // map literal
            let mut keys: Vec<Box<Expr>> = vec![];
            let mut values: Vec<Box<Expr>> = vec![];
            while !self.check_current(TokenType::RBrace, tokens) && !self.is_end(tokens) {
                keys.push(Box::new(self.expression(tokens)?));
                expect!(self, TokenType::Colon, "expected ':'", tokens);
                values.push(Box::new(self.expression(tokens)?));
                if self.check_current(TokenType::RBrace, tokens)
                    || !self.does_match(&[TokenType::Comma], tokens)
                {
                    break;
                }
            }
            expect!(self, TokenType::RBrace, "expected '}'", tokens);
            Ok(Expr::MapLiteral { keys, values })
        } else if self.does_match(&[TokenType::Func], tokens) {
            // anonymous function
            let params = self.parse_params(tokens)?;
            if self.check_current(TokenType::RBrace, tokens) {
                self.function_body(tokens)
            } else {
                // if there's no block, then expects an expression
                let token = self.previous(tokens);
                let expr = self.expression(tokens)?;
                // automatically returns the expression
                let return_node = Node::STMT(Stmt::Return {
                    token,
                    values: vec![expr],
                });
                Ok(Expr::Func {
                    params,
                    body: vec![return_node],
                })
            }
        } else {
            Err("unexpected token")
        }
    }

    fn finish_call(
        &mut self,
        callee: Expr,
        arg: Option<Expr>,
        tokens: &Vec<Token>,
    ) -> Result<Expr, &'static str> {
        let callee = Box::new(callee);
        let mut args: Vec<Box<Expr>> = vec![];
        if match arg {
            // check for |>
            Some(_) => true,
            _ => false,
        } {
            args.push(Box::new(arg.unwrap()));
        }

        if !self.check_current(TokenType::RParen, tokens) {
            args.push(Box::new(self.expression(tokens)?));
            while self.does_match(&[TokenType::Comma], tokens) {
                args.push(Box::new(self.expression(tokens)?));
            }
        }
        expect!(self, TokenType::RParen, "expected ')'", tokens);
        let token = self.previous(tokens);

        // check for <|
        if self.does_match(&[TokenType::LPipe], tokens) {
            args.push(Box::new(self.expression(tokens)?));
        }

        Ok(Expr::Call {
            callee,
            args,
            token,
        })
    }

    fn finish_struct_init(
        &mut self,
        struct_name: Expr,
        tokens: &Vec<Token>,
    ) -> Result<Expr, &'static str> {
        let struct_name = Box::new(struct_name);
        let mut args: Vec<Box<Expr>> = vec![];
        let mut fields: Vec<Token> = vec![];

        if !self.check_current(TokenType::RBrace, tokens) {
            loop {
                if self.check_current(TokenType::RBrace, tokens) {
                    break;
                }

                self.advance(tokens);
                let field = self.previous(tokens);
                if self.does_match(&[TokenType::Comma], tokens) {
                    fields.push(field.clone());
                    args.push(Box::new(Expr::Variable { name: field }));
                    continue;
                } else if self.check_current(TokenType::RBrace, tokens) {
                    fields.push(field.clone());
                    args.push(Box::new(Expr::Variable { name: field }));
                    break;
                }

                expect!(self, TokenType::Colon, "expected ':'", tokens);
                let expr = self.expression(tokens)?;
                args.push(Box::new(expr));
                fields.push(field);

                if self.check_current(TokenType::RBrace, tokens) {
                    break;
                }
                expect!(self, TokenType::Comma, "expected ','", tokens);
            }
        }
        expect!(self, TokenType::RBrace, "expected '}'", tokens);

        Ok(Expr::StructInit {
            struct_name,
            fields,
            args,
        })
    }

    fn call(&mut self, tokens: &Vec<Token>, arg: &Option<Expr>) -> Result<Expr, &'static str> {
        let mut expr = self.primary(tokens)?;
        loop {
            if self.does_match(&[TokenType::LParen], tokens) {
                expr = self.finish_call(expr, arg.clone(), tokens)?;
            } else if self.does_match(&[TokenType::LBrace], tokens) {
                expr = self.finish_struct_init(expr, tokens)?;
            } else if self.does_match(&[TokenType::Dot], tokens) {
                expect!(self, TokenType::Id, "expected an identifier", tokens);
                let name = self.previous(tokens);
                expr = Expr::Get {
                    instance: Box::new(expr),
                    token: name,
                }
            } else if self.does_match(&[TokenType::RPipe], tokens) {
                expr = self.call(tokens, &Some(expr))?;
                break;
            } else if self.does_match(&[TokenType::LBracket], tokens) {
                let token = self.previous(tokens);
                let key = self.expression(tokens)?;
                expect!(self, TokenType::RBracket, "expected ']'", tokens);
                expr = Expr::Access {
                    token,
                    expr: Box::new(expr),
                    index: Box::new(key),
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn unary(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        if self.does_match(&[TokenType::Bang, TokenType::Minus], tokens) {
            let op = self.previous(tokens);
            Ok(Expr::Unary {
                right: Box::new(self.unary(tokens)?),
                op,
            })
        } else {
            self.call(tokens, &None)
        }
    }

    fn factor(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.unary(tokens)?;
        while self.does_match(&[TokenType::Div, TokenType::Mul, TokenType::Mod], tokens) {
            let op = self.previous(tokens);
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.unary(tokens)?),
                op,
            };
        }
        Ok(expr)
    }

    fn term(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.factor(tokens)?;
        while self.does_match(&[TokenType::Minus, TokenType::Plus], tokens) {
            let op = self.previous(tokens);
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.factor(tokens)?),
                op,
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.term(tokens)?;
        while self.does_match(
            &[
                TokenType::GT,
                TokenType::GTEq,
                TokenType::LT,
                TokenType::LTEq,
            ],
            tokens,
        ) {
            let op = self.previous(tokens);
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.term(tokens)?),
                op,
            }
        }
        Ok(expr)
    }

    fn equality(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.comparison(tokens)?;
        while self.does_match(&[TokenType::BangEq, TokenType::DEq], tokens) {
            let op = self.previous(tokens);
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(self.comparison(tokens)?),
                op,
            };
        }
        Ok(expr)
    }

    fn and_expr(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.equality(tokens)?;
        while self.does_match(&[TokenType::DAmp, TokenType::And], tokens) {
            let op = self.previous(tokens);
            expr = Expr::Logical {
                left: Box::new(expr),
                right: Box::new(self.equality(tokens)?),
                op,
            };
        }
        Ok(expr)
    }

    fn or_expr(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let mut expr = self.and_expr(tokens)?;
        while self.does_match(&[TokenType::DPipe, TokenType::Or], tokens) {
            let op = self.previous(tokens);
            expr = Expr::Logical {
                left: Box::new(expr),
                right: Box::new(self.and_expr(tokens)?),
                op,
            };
        }
        Ok(expr)
    }

    fn declaration(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        if self.does_match(&[TokenType::Var], tokens) {
            self.var_declaration(tokens)
        } else if self.check_current(TokenType::Func, tokens)
            && self.check_next(TokenType::Id, tokens)
        {
            self.advance(tokens);
            self.function(tokens)
        } else if self.does_match(&[TokenType::Struct], tokens) {
            self.struct_declaration(tokens)
        } else {
            self.statement(tokens)
        }
    }

    fn statement(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        match self.current.kind {
            TokenType::LBrace => {
                self.advance(tokens);
                Ok(Node::STMT(Stmt::Block {
                    statements: self.parse_block(tokens)?,
                }))
            }
            TokenType::If => self.if_stmt(tokens),
            TokenType::While => self.while_stmt(tokens),
            TokenType::For => self.for_stmt(tokens),
            TokenType::Return => self.return_stmt(tokens),
            TokenType::Break => self.break_stmt(tokens),
            TokenType::Import => self.import_stmt(tokens),
            TokenType::Continue => self.continue_stmt(tokens),
            _ => self.expr_stmt(tokens),
        }
    }

    fn expr_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        let node = Node::EXPR(self.expression(tokens)?);
        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(node)
    }

    fn continue_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        self.advance(tokens);
        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(Node::STMT(Stmt::Continue))
    }

    fn import_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        let token = self.current.clone();
        self.advance(tokens);
        let name = self.expression(tokens)?;
        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(Node::STMT(Stmt::Import { name, token }))
    }

    fn if_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        self.advance(tokens);
        expect!(self, TokenType::LParen, "expected '('", tokens);
        let cond = self.expression(tokens)?;
        expect!(self, TokenType::RParen, "expected ')'", tokens);
        let then = Box::new(self.statement(tokens)?);

        let els: Option<Box<Node>> = if self.check_current(TokenType::Else, tokens)
            && self.check_next(TokenType::If, tokens)
        {
            self.advance(tokens);
            Some(Box::new(self.if_stmt(tokens)?))
        } else if self.check_current(TokenType::Else, tokens) {
            Some(Box::new(self.statement(tokens)?))
        } else {
            None
        };

        Ok(Node::STMT(Stmt::If {
            condition: cond,
            then,
            els,
        }))
    }

    fn break_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        self.advance(tokens);
        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(Node::STMT(Stmt::Break {}))
    }

    fn return_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        let token = self.current.clone();
        self.advance(tokens);
        let mut values: Vec<Expr> = vec![];
        if !self.check_current(TokenType::SColon, tokens) {
            loop {
                values.push(self.expression(tokens)?);
                if !self.check_current(TokenType::Comma, tokens) {
                    break;
                }
            }
        }
        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(Node::STMT(Stmt::Return { token, values }))
    }

    fn while_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        let token = self.current.clone();
        self.advance(tokens);
        expect!(self, TokenType::LParen, "expected '('", tokens);
        let cond = self.expression(tokens)?;
        expect!(self, TokenType::RParen, "expected ')'", tokens);

        let body = Box::new(self.statement(tokens)?);
        Ok(Node::STMT(Stmt::While {
            condition: cond,
            body,
            token,
        }))
    }

    fn for_stmt(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        let token = self.current.clone();
        self.advance(tokens);
        expect!(self, TokenType::LParen, "expected '('", tokens);

        let mut init: Option<Node> = None;
        if self.does_match(&[TokenType::SColon], tokens) {
            // do nothing
        } else if self.does_match(&[TokenType::Var], tokens) {
            init = Some(self.var_declaration(tokens)?);
        } else {
            init = Some(self.expr_stmt(tokens)?);
        }

        let mut condition: Option<Expr> = None;
        if !self.check_current(TokenType::SColon, tokens) {
            condition = Some(self.expression(tokens)?);
        }
        expect!(self, TokenType::SColon, "expected ';'", tokens);

        let mut increment: Option<Expr> = None;
        if !self.check_current(TokenType::RParen, tokens) {
            increment = Some(self.expression(tokens)?);
        }
        expect!(self, TokenType::RParen, "expected ')'", tokens);

        let mut body = self.statement(tokens)?;

        if let Some(increment) = increment {
            body = Node::STMT(Stmt::Block {
                statements: vec![body, Node::EXPR(increment)],
            })
        }

        let new_condition: Expr;
        if let Some(condition) = condition {
            new_condition = condition;
        } else {
            new_condition = Expr::Literal {
                kind: TokenType::True,
                value: String::new(),
            };
        }

        body = Node::STMT(Stmt::While {
            condition: new_condition,
            body: Box::new(body),
            token,
        });

        if let Some(init) = init {
            body = Node::STMT(Stmt::Block {
                statements: vec![init, body],
            });
        }

        Ok(body)
    }

    fn function(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        expect!(self, TokenType::Id, "expected an identifier", tokens);
        let name = self.previous(tokens);
        let body = self.function_body(tokens)?;
        Ok(Node::STMT(Stmt::Func {
            token: name,
            func: body,
        }))
    }

    fn function_body(&mut self, tokens: &Vec<Token>) -> Result<Expr, &'static str> {
        let params = self.parse_params(tokens)?;
        expect!(self, TokenType::LBrace, "expected '{'", tokens);
        let body = self.parse_block(tokens)?;
        Ok(Expr::Func { params, body })
    }

    fn parse_params(&mut self, tokens: &Vec<Token>) -> Result<Vec<Token>, &'static str> {
        expect!(self, TokenType::LParen, "expected '('", tokens);
        let mut params: Vec<Token> = vec![];
        if !self.check_current(TokenType::RParen, tokens) {
            loop {
                expect!(self, TokenType::Id, "expected an identifier", tokens);
                let param = self.previous(tokens);
                params.push(param);

                if !self.does_match(&[TokenType::Comma], tokens) {
                    break;
                }
            }
        }
        expect!(self, TokenType::RParen, "expected ')'", tokens);
        Ok(params)
    }

    fn parse_block(&mut self, tokens: &Vec<Token>) -> Result<Vec<Node>, &'static str> {
        let mut stmts: Vec<Node> = vec![];
        while !self.check_current(TokenType::RBrace, tokens) && !self.is_end(tokens) {
            stmts.push(self.declaration(tokens)?);
        }
        expect!(self, TokenType::RBrace, "expected '}'", tokens);
        Ok(stmts)
    }

    fn var_declaration(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        expect!(self, TokenType::Id, "expected an identifier", tokens);
        let name = self.previous(tokens);
        let mut init = Expr::Literal {
            kind: TokenType::Null,
            value: String::new(),
        };

        if self.does_match(&[TokenType::Equal], tokens) {
            init = self.expression(tokens)?;
        }

        expect!(self, TokenType::SColon, "expected ';'", tokens);
        Ok(Node::STMT(Stmt::Variable { name, init }))
    }

    fn struct_declaration(&mut self, tokens: &Vec<Token>) -> Result<Node, &'static str> {
        expect!(self, TokenType::Id, "expected an identifier", tokens);
        let token = self.previous(tokens);
        expect!(self, TokenType::LBrace, "expected '{'", tokens);
        let mut fields: Vec<Token> = vec![];
        let mut types: Vec<TypeInfo> = vec![];
        while !self.check_current(TokenType::RBrace, tokens) {
            expect!(self, TokenType::Id, "expected an identifier", tokens);
            fields.push(self.previous(tokens));
            expect!(self, TokenType::Colon, "expected ':'", tokens);
            match self.current.kind {
                TokenType::Id => types.push(match self.current.value.to_lowercase().as_str() {
                    "string" => TypeInfo::Str,
                    "number" => TypeInfo::Num,
                    "bool" => TypeInfo::Bool,
                    "any" => TypeInfo::Any,
                    "list" => TypeInfo::List,
                    "map" => TypeInfo::Map,
                    _ => TypeInfo::Id(self.current.clone()),
                }),
                _ => self.add_error("invalid type info"),
            }
            self.advance(tokens);
            if self.check_current(TokenType::RBrace, tokens) {
                break;
            } else {
                expect!(self, TokenType::Comma, "expected ','", tokens);
            }
        }
        expect!(self, TokenType::RBrace, "expected '}'", tokens);

        Ok(Node::STMT(Stmt::Struct {
            token,
            fields,
            types,
        }))
    }

    /// Checks if the current token is in the given types
    fn does_match(&mut self, these: &[TokenType], tokens: &Vec<Token>) -> bool {
        for kind in these {
            if self.check_current(*kind, tokens) {
                self.advance(tokens);
                return true;
            }
        }
        false
    }

    /// Advances one token
    fn advance(&mut self, tokens: &Vec<Token>) {
        if !self.is_end(tokens) {
            self.c += 1;
            self.current = tokens[self.c].clone();
        } else {
            self.current = tokens[tokens.len()].clone();
        }
    }

    /// Checks if the token type of the current token is the same as the expected token type
    fn check_current(&self, kind: TokenType, tokens: &Vec<Token>) -> bool {
        if tokens[self.c].clone().kind == kind {
            true
        } else {
            false
        }
    }

    /// Checks if the token type of the next token is the same as the expected token type
    fn check_next(&self, kind: TokenType, tokens: &Vec<Token>) -> bool {
        if self.is_end(tokens) {
            false
        } else {
            if tokens[self.c].clone().kind == kind {
                true
            } else {
                false
            }
        }
    }

    /// Returns the previous token
    fn previous(&self, tokens: &Vec<Token>) -> Token {
        if self.c == 0 {
            tokens[0].clone()
        } else {
            tokens[self.c - 1].clone()
        }
    }

    /// Checks if the end is reached
    fn is_end(&self, tokens: &Vec<Token>) -> bool {
        match tokens[self.c].kind {
            TokenType::EOF => true,
            _ => false,
        }
    }

    /// Checks if the next of the next token is the end or not
    fn is_next_end(&self, tokens: &Vec<Token>) -> bool {
        if tokens.len() <= self.c {
            return false;
        }
        match tokens[self.c + 1].kind {
            TokenType::EOF => true,
            _ => false,
        }
    }

    /// Appends the error created with the given error message and the current line and column
    fn add_error(&mut self, message: &str) {
        let error = ParserError::new(message, self.current.position.0, self.current.position.1);
        self.errors.push(error);
    }

    /// Discards tokens until reaching one that can appear at that point in the rule
    fn synchronize(&mut self, tokens: &Vec<Token>) {
        if !self.is_end(tokens) {
            self.advance(tokens);
        } else {
            return;
        }

        while !self.is_end(tokens) {
            if self.c > 0 {
                if self.previous(tokens).kind == TokenType::SColon {
                    return;
                }
            }

            if self.is_next_end(tokens) {
                return;
            }

            match tokens[self.c + 1].kind {
                TokenType::Func
                | TokenType::Struct
                | TokenType::Var
                | TokenType::Const
                | TokenType::If
                | TokenType::For
                | TokenType::While
                | TokenType::Import => return,
                _ => {}
            }
            self.advance(tokens);
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Node;
    use crate::lexer::Lexer;
    use crate::parse;

    #[test]
    fn test_anonymous_func() {
        let source = r#"let add = func (x, y) x + y;"#;
        let expected = "(var add (lambda (x y) (return (Plus x y))))";
        parse!(source, expected);
    }

    #[test]
    fn test_for_stmt() {
        let source = r#"for (let i = 0; i < 10; i++) { println(i); }"#;
        let expected = "(block (var i Num) (while ((LT i Num)) (block (block (println i)) (assign i (DPlus i Num)))))";
        parse!(source, expected);
    }

    #[test]
    fn test_struct_stmt() {
        let source = r#"struct Person { name: string, age: number, friends: list, book_reviews: map, others: any }"#;
        let expected =
            "(struct Person name:string age:number friends:list book_reviews:map others:any)";
        parse!(source, expected);
    }

    #[test]
    fn test_struct_init() {
        let source = r#"let nobu = Person{ name: "Nobuharu", age, others: 12345, };"#;
        let expected = r#"(var nobu (Person name:"Nobuharu" age:age others:Num))"#;
        parse!(source, expected);
    }

    #[test]
    fn test_atom_expr() {
        let source = "let name = :nobu;";
        let expected = "(var name :nobu)";
        parse!(source, expected);
    }

    #[test]
    fn test_underscore_expr() {
        let source = "let underscore = _;";
        let expected = "(var underscore :_:)";
        parse!(source, expected);
    }

    #[test]
    fn list_and_map_expr() {
        let source = r#"[1, 2, "abc", {:name: "Nobuharu", "age": 16}];"#;
        let expected = r#"(list Num Num "abc" (map :name:"Nobuharu" "age":Num))"#;
        parse!(source, expected);
    }
}
