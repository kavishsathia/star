use crate::ast::{Type, TypeKind};
use crate::error::CompilerError;
use crate::lexer::Token;
use super::Parser;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Result<Type, CompilerError> {
        let kind = if self.check(&Token::Identifier) {
            let name = self.current_slice.clone();
            self.advance();
            match name.as_str() {
                "integer" => TypeKind::Integer,
                "float" => TypeKind::Float,
                "boolean" => TypeKind::Boolean,
                "string" => TypeKind::String,
                _ => TypeKind::Struct { name },
            }
        } else if self.match_token(&Token::LBrace) {
            let element = Box::new(self.parse_type()?);
            self.expect(&Token::RBrace)?;
            TypeKind::List { element }
        } else if self.match_token(&Token::LParenthesis) {
            let mut params = Vec::new();
            while !self.check(&Token::Colon) {
                params.push(self.parse_type()?);
                if self.check(&Token::Separator) {
                    self.advance();
                }
            }
            self.expect(&Token::Colon)?;
            let returns = Box::new(self.parse_type()?);
            self.expect(&Token::RParenthesis)?;
            TypeKind::Function { params, returns }
        } else {
            return Err(CompilerError::Parse {
                message: format!("Unexpected token in type annotation: {:?}", self.peek()),
            });
        };

        Ok(Type {
            kind,
            nullable: self.match_token(&Token::Nullable),
            errorable: self.match_token(&Token::Errorable),
        })
    }
}
