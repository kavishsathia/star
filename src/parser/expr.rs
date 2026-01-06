use crate::lexer::Token;
use crate::ast::{Expr, Pattern, UnaryOp};
use super::Parser;

impl<'a> Parser<'a> {
    pub fn parse_expression(&mut self, min_bp: u8) -> Expr {
        let mut left = match self.peek() {
            Some(Token::Null) => {
                self.advance();
                Expr::Null
            }
            Some(Token::Integer) => {
                let slice = self.slice().to_string();
                self.advance();
                Expr::Integer(slice.parse().unwrap())
            }
            Some(Token::Float) => {
                let slice = self.slice().to_string();
                self.advance();
                Expr::Float(slice.parse().unwrap())
            }
            Some(Token::String) => {
                let slice = self.slice().to_string();
                self.advance();
                Expr::String(slice)
            }
            Some(Token::True) => {
                self.advance();
                Expr::Boolean(true)
            }
            Some(Token::False) => {
                self.advance();
                Expr::Boolean(false)
            }
            Some(Token::Identifier) => {
                let slice = self.slice().to_string();
                self.advance();
                Expr::Identifier(slice)
            }
            Some(Token::LParenthesis) => {
                self.advance();
                let expr = self.parse_expression(0);
                self.expect(&Token::RParenthesis);
                expr
            }
            Some(Token::Not) | Some(Token::Minus) | Some(Token::Raise) => {
                let op = self.advance().unwrap();
                let rbp = Parser::prefix_binding_power(&op).unwrap();
                let expr = self.parse_expression(rbp);
                match op {
                    Token::Minus => Expr::Unary { op: UnaryOp::Minus, expr: Box::new(expr) },
                    Token::Not => Expr::Unary { op: UnaryOp::Not, expr: Box::new(expr) },
                    Token::Raise => Expr::Unary { op: UnaryOp::Raise, expr: Box::new(expr) },
                    _ => unreachable!(),
                }
            }
            Some(Token::LBrace) => {
                self.advance();

                if self.check(&Token::RBrace) {
                    self.advance();
                    Expr::List(vec![])
                } else {
                    let first = self.parse_expression(0);
                    let mut elements = vec![first];

                    while self.check(&Token::Separator) {
                        self.advance();
                        elements.push(self.parse_expression(0));
                    }

                    self.expect(&Token::RBrace);
                    Expr::List(elements)
                }
            }
            Some(Token::New) => {
                self.advance();
                let name = if let Some(Token::Identifier) = self.peek() {
                    let name = self.current_slice.clone();
                    self.advance();
                    name
                } else {
                    panic!("Expected identifier after 'new', found {:?}", self.peek());
                };
                self.expect(&Token::LBrace);
                let mut fields = Vec::new();
                while !self.check(&Token::RBrace) {
                    let field_name = if let Some(Token::Identifier) = self.peek() {
                        let field_name = self.current_slice.clone();
                        self.advance();
                        field_name
                    } else {
                        panic!("Expected field name in struct init, found {:?}", self.peek());
                    };
                    self.expect(&Token::Colon);
                    let value = self.parse_expression(0);
                    fields.push((field_name, value));
                    if self.check(&Token::Separator) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace);
                Expr::New { name, fields }
            }
            Some(Token::Match) => {
                self.advance();
                let expr = Box::new(self.parse_expression(0));
                self.expect(&Token::As);
                let binding = if let Some(Token::Identifier) = self.peek() {
                    let name = self.current_slice.clone();
                    self.advance();
                    name
                } else {
                    panic!("Expected identifier after 'as', found {:?}", self.peek());
                };
                self.expect(&Token::LBrace);
                let mut arms = Vec::new();
                while !self.check(&Token::RBrace) {
                    let pattern = if self.check(&Token::NullOrError) {
                        self.advance();
                        Pattern::MatchAll
                    } else if self.check(&Token::Nullable) {
                        self.advance();
                        Pattern::MatchNull
                    } else if self.check(&Token::Errorable) {
                        self.advance();
                        Pattern::MatchError
                    } else if self.check(&Token::Identifier) {
                        let ty = self.parse_type();
                        Pattern::MatchType(ty)
                    } else {
                        panic!("Expected pattern in match arm, found {:?}", self.peek());
                    };
                    self.expect(&Token::Colon);
                    self.expect(&Token::LBrace);
                    let mut body = Vec::new();
                    while !self.check(&Token::RBrace) {
                        body.push(self.parse_statement(false));
                    }
                    self.expect(&Token::RBrace);
                    arms.push((pattern, body));
                }
                self.expect(&Token::RBrace);
                Expr::Match { expr, binding, arms }
            }
            _ => panic!("Unexpected token: {:?}", self.peek()),
        };

        loop {
            let op = match self.peek() {
                Some(op) => op,
                None => break,
            };

            if let Some((l_bp, r_bp)) = Self::infix_binding_power(&op) {
                if l_bp < min_bp {
                    break;
                }

                let infix = Parser::token_to_binary_op(op);
                self.advance();
                let right = self.parse_expression(r_bp);
                left = Expr::Binary { left: Box::new(left), op: infix, right: Box::new(right) }
            } else if *op == Token::LParenthesis {
                self.advance();
                let mut args: Vec<Expr> = Vec::new();
                while !self.check(&Token::RParenthesis) {
                    args.push(self.parse_expression(0));
                    if self.check(&Token::Separator) {
                        self.advance();
                    }
                }
                self.advance();
                left = Expr::Call { callee: Box::new(left), args };
            } else if *op == Token::Access {
                self.advance();
                let field = if let Some(Token::Identifier) = self.peek() {
                    let field_name = self.current_slice.clone();
                    self.advance();
                    field_name
                } else {
                    panic!("Expected identifier after '.', found {:?}", self.peek());
                };
                left = Expr::Field { object: Box::new(left), field };
            } else if *op == Token::LBracket {
                self.advance();
                let expr = self.parse_expression(0);
                self.expect(&Token::RBracket);
                left = Expr::Index { object: Box::new(left), key: Box::new(expr) };
            } else if *op == Token::NotNull {
                self.advance();
                left = Expr::UnwrapNull(Box::new(left));
            } else if *op == Token::NotError {
                self.advance();
                left = Expr::UnwrapError(Box::new(left));
            } else {
                break;
            }

        }

        left
    }
}
