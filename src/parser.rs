use std::process;

use crate::ast::{Expr, MatchBranch};
use crate::error::ParserError;
use crate::token::{Token, TokenType};

pub struct Parser {
    c: usize,
    current: Token,
    errors: Vec<ParserError>,
    pub exprs: Vec<Expr>,
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
            exprs: vec![],
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
            let expr = self.expression(tokens);
            match expr {
                Ok(expr) => self.exprs.push(expr),
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
        if self.check_current(TokenType::Equal, tokens)
            || self.check_current(TokenType::ColonEq, tokens)
        {
            let init = if self.current.kind == TokenType::ColonEq {
                true
            } else {
                false
            };

            self.advance(tokens);
            let value = Box::new(self.assignment(tokens)?);

            match expr {
                Expr::Variable { name: _ }
                | Expr::ListLiteral { values: _ }
                | Expr::ObjectLiteral { keys: _, values: _ } => {
                    return Ok(Expr::Assign {
                        init,
                        left: Box::new(expr),
                        right: value,
                    })
                }
                Expr::Get { instance, token } => {
                    return Ok(Expr::Set {
                        instance,
                        token,
                        value,
                    })
                }
                _ => {
                    return Err("invalid assignment target");
                }
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
                Expr::Variable { name: _ } => {
                    return Ok(Expr::Assign {
                        init: false,
                        left: Box::new(expr.clone()),
                        right: Box::new(Expr::Binary {
                            left: Box::new(expr),
                            right: Box::new(value),
                            op,
                        }),
                    });
                }
                _ => return Err("expected a variable"),
            };
        } else if self.does_match(&[TokenType::DPlus, TokenType::DMinus], tokens) {
            let mut op = self.previous(tokens);
            match expr {
                Expr::Variable { name: _ } => {
                    op.kind = if op.kind == TokenType::DPlus {
                        TokenType::Plus
                    } else {
                        TokenType::Minus
                    };
                    return Ok(Expr::Assign {
                        init: false,
                        left: Box::new(expr.clone()),
                        right: Box::new(Expr::Binary {
                            left: Box::new(expr),
                            right: Box::new(Expr::NumberLiteral {
                                token: op.clone(),
                                value: 1.0,
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
        if self.does_match(&[TokenType::True, TokenType::False], tokens) {
            // Boolean
            let token = self.previous(tokens);
            Ok(Expr::BoolLiteral {
                token: token.clone(),
                payload: if token.kind == TokenType::True {
                    true
                } else {
                    false
                },
            })
        } else if self.does_match(&[TokenType::Underscore], tokens) {
            // Underscore
            let token = self.previous(tokens);
            Ok(Expr::Underscore { token })
        } else if self.does_match(&[TokenType::Null], tokens) {
            // Null
            let token = self.previous(tokens);
            Ok(Expr::Null { token })
        } else if self.does_match(&[TokenType::Num], tokens) {
            // Number
            let token = self.previous(tokens);
            let value = token.value.parse::<f64>();
            if let Ok(value) = value {
                Ok(Expr::NumberLiteral { token, value })
            } else {
                Err("invalid number")
            }
        } else if self.does_match(&[TokenType::Str], tokens) {
            // String
            let token = self.previous(tokens);
            Ok(Expr::StringLiteral {
                token: token.clone(),
                value: token.value,
            })
        } else if self.does_match(&[TokenType::Atom], tokens) {
            // Atom
            let token = self.previous(tokens);
            Ok(Expr::AtomLiteral {
                token: token.clone(),
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
            // object or block

            if self.check_current(TokenType::RBrace, tokens) {
                // empty {} is always considered an
                // object; an empty block is illegal
                self.advance(tokens);
                Ok(Expr::ObjectLiteral {
                    keys: vec![],
                    values: vec![],
                })
            } else {
                let previous_c = self.c;
                let first_expr = self.expression(tokens)?;
                if self.is_end(tokens) {
                    return Err("unexpected end of input inside block or object");
                }
                if self.does_match(&[TokenType::Colon], tokens) {
                    // it's an object!
                    self.c = previous_c; // reset back
                    let mut keys: Vec<Token> = vec![];
                    let mut values: Vec<Box<Expr>> = vec![];
                    while !self.check_current(TokenType::RBrace, tokens) && !self.is_end(tokens) {
                        expect!(self, TokenType::Id, "expected an identifier", tokens);
                        keys.push(self.previous(tokens).clone());
                        expect!(self, TokenType::Colon, "expected ':'", tokens);
                        values.push(Box::new(self.expression(tokens)?));
                        if self.check_current(TokenType::RBrace, tokens)
                            || !self.does_match(&[TokenType::Comma], tokens)
                        {
                            break;
                        }
                    }
                    expect!(self, TokenType::RBrace, "expected '}'", tokens);
                    Ok(Expr::ObjectLiteral { keys, values })
                } else {
                    // it's a block!
                    let mut exprs: Vec<Box<Expr>> = vec![];
                    exprs.push(Box::new(first_expr));
                    while !self.check_current(TokenType::RBrace, tokens) && !self.is_end(tokens) {
                        exprs.push(Box::new(self.expression(tokens)?));
                    }
                    expect!(self, TokenType::RBrace, "expected '}'", tokens);
                    Ok(Expr::Block { exprs })
                }
            }
        } else if self.does_match(&[TokenType::Func], tokens) {
            // function
            let name: Option<Token> = if self.check_current(TokenType::Id, tokens) {
                self.advance(tokens);
                Some(self.previous(tokens))
            } else {
                None
            };
            let params = self.parse_params(tokens)?;
            let body = self.expression(tokens)?;
            Ok(Expr::Func {
                name,
                params,
                body: Box::new(body),
            })
        } else if self.does_match(&[TokenType::Import], tokens) {
            // import
            let token = self.previous(tokens);
            expect!(self, TokenType::LParen, "expected '('", tokens);
            let name = self.expression(tokens)?;
            expect!(self, TokenType::RParen, "expected ')'", tokens);
            Ok(Expr::Import {
                name: Box::new(name),
                token,
            })
        } else if self.does_match(&[TokenType::Match], tokens) {
            // match expression
            let token = self.previous(tokens);
            let condition = self.expression(tokens)?;
            expect!(self, TokenType::LBrace, "expected '{'", tokens);

            let mut branches: Vec<MatchBranch> = vec![];
            while !self.check_current(TokenType::RBrace, tokens) {
                let expr = self.expression(tokens)?;
                expect!(self, TokenType::MinusGT, "expected '->'", tokens);

                let body = self.expression(tokens)?;
                branches.push(MatchBranch {
                    target: Box::new(expr),
                    body: Box::new(body),
                });

                if !self.does_match(&[TokenType::Comma], tokens) {
                    break;
                }
            }
            expect!(self, TokenType::RBrace, "expected '}'", tokens);

            Ok(Expr::Match {
                token,
                condition: Box::new(condition),
                branches,
            })
        } else {
            // println!("{:#?}", &self.current);
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

    fn call(&mut self, tokens: &Vec<Token>, arg: &Option<Expr>) -> Result<Expr, &'static str> {
        let mut expr = self.primary(tokens)?;
        loop {
            if self.does_match(&[TokenType::LParen], tokens) {
                expr = self.finish_call(expr, arg.clone(), tokens)?;
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
                TokenType::Func | TokenType::Id | TokenType::Import => return,
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
    use crate::lexer::Lexer;
    use crate::parse;

    #[test]
    fn test_anonymous_func() {
        let source = r#"add := func (x, y) x + y"#;
        let expected = "(assignI add (lambda (x y) (Plus x y)))";
        parse!(source, expected);
    }

    #[test]
    fn test_atom_expr() {
        let source = "name := :nobu";
        let expected = "(assignI name :nobu)";
        parse!(source, expected);
    }

    #[test]
    fn test_underscore_expr() {
        let source = "underscore := _";
        let expected = "(assignI underscore :_:)";
        parse!(source, expected);
    }

    #[test]
    fn list_and_object_expr() {
        let source = r#"[1, 2, "abc", {name: "Nobuharu", age: 16}]"#;
        let expected = r#"(list 1 2 "abc" (object name:"Nobuharu" age:16))"#;
        parse!(source, expected);
    }

    #[test]
    fn import_expr() {
        let source = r#"std := import("std")"#;
        let expected = r#"(assignI std (import "std"))"#;
        parse!(source, expected);
    }

    #[test]
    fn match_expr() {
        let source = r#"match name { "nobu" -> println("cool!"), _ -> { println("hello") } }"#;
        let expected =
            r#"(match name "nobu" -> (println "cool!") :_: -> (block (println "hello")))"#;
        parse!(source, expected);
    }

    #[test]
    fn assign_expr() {
        let source = r#"[name, _] := ["Nobu", 16]"#;
        let expected = r#"(assignI (list name :_:) (list "Nobu" 16))"#;
        parse!(source, expected);
    }
}
