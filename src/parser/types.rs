use crate::lexer::Token;
use crate::ast::{Type, TypeKind};
use super::Parser;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Type {
        let kind = if self.check(&Token::Identifier) {
            let name = self.current_slice.clone();
            self.advance();
            TypeKind::Primitive(name)
        } else if self.match_token(&Token::LBrace) {
            let element = Box::new(self.parse_type());
            if self.match_token(&Token::Colon) {
                let value = Box::new(self.parse_type());
                self.expect(&Token::RBrace);
                TypeKind::Dict { key_type: element, value_type: value }
            } else {
                self.expect(&Token::RBrace);
                TypeKind::List(element)
            }
        } else if self.match_token(&Token::LParenthesis) {
            // (int, int : int)
            let mut param_types = Vec::new();
            while !self.check(&Token::Colon) {
                param_types.push(self.parse_type());
                if self.check(&Token::Separator) {
                    self.advance();
                }
            }
            self.expect(&Token::Colon);
            let return_type = Box::new(self.parse_type());
            self.expect(&Token::RParenthesis);
            TypeKind::Function { param_types, return_type }
        } else {
            panic!("Unexpected token in type annotation: {:?}", self.peek());
        };

        Type {
            kind,
            nullable: self.match_token(&Token::Nullable),
            errorable: self.match_token(&Token::Errorable),
        }
    }
}
