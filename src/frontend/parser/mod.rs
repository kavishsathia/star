mod expr;
mod stmt;
mod types;

use super::lexer::Token;
use crate::ast::{BinaryOp, Expr, Program, Statement, Type, TypeKind, UnaryOp};
use crate::error::CompilerError;
use logos::Logos;

pub struct Parser<'a> {
    lexer: logos::Lexer<'a, Token>,
    current: Option<Token>,
    current_slice: String,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Token::lexer(source);
        let current = lexer.next().and_then(|r| r.ok());
        let current_slice = lexer.slice().to_string();

        Parser {
            lexer,
            current,
            current_slice,
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.current.as_ref()
    }

    pub fn slice(&self) -> &str {
        &self.current_slice
    }

    pub fn advance(&mut self) -> Option<Token> {
        let token = self.current.take();
        self.current = self.lexer.next().and_then(|r| r.ok());
        self.current_slice = self.lexer.slice().to_string();
        token
    }

    pub fn check(&self, expected: &Token) -> bool {
        self.peek()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    pub fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, expected: &Token) -> Result<Token, CompilerError> {
        if self.check(expected) {
            Ok(self.advance().unwrap())
        } else {
            Err(CompilerError::Parse {
                message: format!("Expected {:?}, found {:?}", expected, self.peek()),
            })
        }
    }

    pub fn at_end(&self) -> bool {
        self.current.is_none()
    }

    pub fn parse_program(&mut self) -> Result<Program, CompilerError> {
        let mut stmts = Vec::new();
        while !self.at_end() {
            stmts.push(self.parse_statement(true)?);
        }
        Ok(Program { statements: stmts })
    }

    pub fn infix_binding_power(op: &Token) -> Option<(u8, u8)> {
        match op {
            Token::Is => Some((0, 1)),
            Token::Or => Some((1, 2)),
            Token::And => Some((3, 4)),

            Token::Eq | Token::Neq => Some((5, 6)),

            Token::Lt | Token::Gt | Token::Lte | Token::Gte => Some((7, 8)),

            Token::BitwiseOr => Some((9, 10)),
            Token::Xor => Some((11, 12)),
            Token::BitwiseAnd => Some((13, 14)),
            Token::Sll | Token::Srl => Some((15, 16)),

            Token::Plus | Token::Minus => Some((17, 18)),
            Token::Multiply | Token::Divide | Token::Modulo => Some((19, 20)),

            Token::In => Some((21, 22)),

            Token::Power => Some((24, 23)),

            _ => None,
        }
    }

    pub fn prefix_binding_power(op: &Token) -> Option<u8> {
        match op {
            Token::Minus | Token::Not | Token::Count | Token::Stringify => Some(23),
            _ => None,
        }
    }

    fn token_to_binary_op(token: &Token) -> Result<BinaryOp, CompilerError> {
        match token {
            Token::Plus => Ok(BinaryOp::Plus),
            Token::Minus => Ok(BinaryOp::Minus),
            Token::Multiply => Ok(BinaryOp::Multiply),
            Token::Divide => Ok(BinaryOp::Divide),
            Token::Power => Ok(BinaryOp::Power),
            Token::And => Ok(BinaryOp::And),
            Token::Or => Ok(BinaryOp::Or),
            Token::Eq => Ok(BinaryOp::Eq),
            Token::Neq => Ok(BinaryOp::Neq),
            Token::Lt => Ok(BinaryOp::Lt),
            Token::Gt => Ok(BinaryOp::Gt),
            Token::Lte => Ok(BinaryOp::Lte),
            Token::Gte => Ok(BinaryOp::Gte),
            Token::BitwiseAnd => Ok(BinaryOp::BitwiseAnd),
            Token::BitwiseOr => Ok(BinaryOp::BitwiseOr),
            Token::Xor => Ok(BinaryOp::Xor),
            Token::Sll => Ok(BinaryOp::Sll),
            Token::Srl => Ok(BinaryOp::Srl),
            Token::Is => Ok(BinaryOp::Is),
            Token::In => Ok(BinaryOp::In),
            Token::Modulo => Ok(BinaryOp::Modulo),
            _ => Err(CompilerError::Parse {
                message: format!("Not a binary operator: {:?}", token),
            }),
        }
    }
}
