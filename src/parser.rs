use core::panic;

use logos::Logos;
use crate::lexer::Token;
use crate::ast::{BinaryOp, Expr, Statement, Type, TypeKind, UnaryOp};

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
        self.peek().map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected)).unwrap_or(false)
    }

    pub fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, expected: &Token) -> Token {
        if self.check(expected) {
            self.advance().unwrap()
        } else {
            panic!(
                "Expected {:?}, found {:?}",
                expected,
                self.peek()
            );
        }
    }

    pub fn at_end(&self) -> bool {
        self.current.is_none()
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
            Token::Multiply | Token::Divide => Some((19, 20)),

            Token::Power => Some((22, 21)),

            _ => None,
        }
    }

    pub fn prefix_binding_power(op: &Token) -> Option<u8> {
        match op {
            Token::Minus | Token::Not | Token::Raise => Some(23),
            _ => None,
        }
    }


    fn token_to_binary_op(token: &Token) -> BinaryOp {
        match token {
            Token::Plus => BinaryOp::Plus,
            Token::Minus => BinaryOp::Minus,
            Token::Multiply => BinaryOp::Multiply,
            Token::Divide => BinaryOp::Divide,
            Token::Power => BinaryOp::Power,
            Token::And => BinaryOp::And,
            Token::Or => BinaryOp::Or,
            Token::Eq => BinaryOp::Eq,
            Token::Neq => BinaryOp::Neq,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::Lte => BinaryOp::Lte,
            Token::Gte => BinaryOp::Gte,
            Token::BitwiseAnd => BinaryOp::BitwiseAnd,
            Token::BitwiseOr => BinaryOp::BitwiseOr,
            Token::Xor => BinaryOp::Xor,
            Token::Sll => BinaryOp::Sll,
            Token::Srl => BinaryOp::Srl,
            Token::Is => BinaryOp::Is,
            _ => panic!("Not a binary operator: {:?}", token),
        }
    }

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

                    if self.check(&Token::Colon) {
                        let mut kvs = Vec::new();
                        self.advance();
                        let value = self.parse_expression(0);
                        kvs.push((first, value));

                        while self.check(&Token::Separator) {
                            self.advance();
                            let key = self.parse_expression(0);
                            self.expect(&Token::Colon);
                            let value = self.parse_expression(0);
                            kvs.push((key, value));
                        }

                        self.expect(&Token::RBrace);
                        Expr::Dict(kvs)
                    } else {
                        let mut elements = vec![first];

                        while self.check(&Token::Separator) {
                            self.advance();
                            elements.push(self.parse_expression(0));
                        }

                        self.expect(&Token::RBrace);
                        Expr::List(elements)
                    }
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
                Expr::Init { name, fields }
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
                left = Expr::MemberAccess { object: Box::new(left), field };
            } else if *op == Token::LBracket {
                self.advance();
                let expr = self.parse_expression(0);
                self.expect(&Token::RBracket);
                left = Expr::KeyAccess { dict: Box::new(left), key: Box::new(expr) };
            } else if (*op == Token::NotNull) {
                self.advance();
                left = Expr::NotNull(Box::new(left));
            } else if (*op == Token::NotError) {
                self.advance();
                left = Expr::NotError(Box::new(left));
            } else if (*op == Token::NotNullOrError) {
                self.advance();
                left = Expr::NotNullOrError(Box::new(left));
            } else {
                break;
            }

        }

        left
    }

    fn parse_type(&mut self) -> Type {
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

    fn parse_let_statement(&mut self) -> Statement {
        self.expect(&Token::Let);
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            panic!("Expected identifier after 'let', found {:?}", self.peek());
        };

        self.expect(&Token::Colon);

        let type_annotation = self.parse_type();

        let value = if self.match_token(&Token::Is) {
            Some(Box::new(self.parse_expression(0)))
        } else {
            None
        };

        self.expect(&Token::Semicolon);

        Statement::Let { name, value, type_annotation }
    }

    fn parse_const_statement(&mut self) -> Statement {
        self.expect(&Token::Const);
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            panic!("Expected identifier after 'let', found {:?}", self.peek());
        };

        self.expect(&Token::Colon);

        let type_annotation = self.parse_type();

        let value = if self.match_token(&Token::Is) {
            Box::new(self.parse_expression(0))
        } else {
            panic!("Expected '=' after const declaration, found {:?}", self.peek());
        };

        self.expect(&Token::Semicolon);

        Statement::Const { name, value, type_annotation }
    }

    fn parse_expression_statement(&mut self) -> Statement {
        let expr = self.parse_expression(0);
        self.expect(&Token::Semicolon);
        Statement::Expr(expr)
    }

    fn parse_return_statement(&mut self) -> Statement {
        self.expect(&Token::Return);
        let expr = if !self.check(&Token::Semicolon) {
            Some(Box::new(self.parse_expression(0)))
        } else {
            None
        };
        self.expect(&Token::Semicolon);
        Statement::Return(expr)
    }

    fn parse_break_statement(&mut self) -> Statement {
        self.expect(&Token::Break);
        self.expect(&Token::Semicolon);
        Statement::Break
    }

    fn parse_continue_statement(&mut self) -> Statement {
        self.expect(&Token::Continue);
        self.expect(&Token::Semicolon);
        Statement::Continue
    }


    fn parse_if_statement(&mut self) -> Statement {
        self.expect(&Token::If);
        let condition = Box::new(self.parse_expression(0));
        self.expect(&Token::LBrace);
        let mut consequent = Vec::new();
        while !self.check(&Token::RBrace) {
            consequent.push(self.parse_statement());
        }
        self.expect(&Token::RBrace);

        let alternate = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                Some(vec![self.parse_if_statement()])
            } else {
                self.expect(&Token::LBrace);
                let mut alternate_block = Vec::new();
                while !self.check(&Token::RBrace) {
                    alternate_block.push(self.parse_statement());
                }
                self.expect(&Token::RBrace);
                Some(alternate_block)
            }
        } else {
            None
        };

        Statement::If { condition, consequent, alternate }
    }

    fn parse_for_statement(&mut self) -> Statement {
        self.expect(&Token::For);
        let initializer = Box::new(self.parse_statement());
        let condition = Box::new(self.parse_expression(0));
        self.expect(&Token::Semicolon);
        let increment = Box::new(self.parse_statement());
        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement());
        }
        self.expect(&Token::RBrace);
        Statement::For { initializer, condition, increment, body }
    }

    fn parse_while_statement(&mut self) -> Statement {
        self.expect(&Token::While);
        let condition = Box::new(self.parse_expression(0));
        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement());
        }
        self.expect(&Token::RBrace);
        Statement::While { condition, body }
    }

    fn parse_struct_definition(&mut self) -> Statement {
        self.expect(&Token::Struct);
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            panic!("Expected identifier after 'struct', found {:?}", self.peek());
        };

        self.expect(&Token::LBrace);
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            let field_name = if let Some(Token::Identifier) = self.peek() {
                let field_name = self.current_slice.clone();
                self.advance();
                field_name
            } else {
                panic!("Expected field name in struct definition, found {:?}", self.peek());
            };

            self.expect(&Token::Colon);

            let field_type = self.parse_type();

            fields.push((field_name, field_type));

            if self.check(&Token::Separator) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace);

        Statement::Struct { name, fields }
    }

    fn parse_error_definition(&mut self) -> Statement {
        self.expect(&Token::Error);
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            panic!("Expected identifier after 'error', found {:?}", self.peek());
        };

        self.expect(&Token::Semicolon);
        Statement::Error { name }
    }

    fn parse_match_statement(&mut self) -> Statement {
        self.expect(&Token::Match);
        let expr = Box::new(self.parse_expression(0));
        self.expect(&Token::LBrace);
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) {
            let pattern = if let Some(Token::Identifier) = self.peek() {
                let pattern = self.current_slice.clone();
                self.advance();
                pattern
            } else {
                panic!("Expected pattern in match arm, found {:?}", self.peek());
            };

            self.expect(&Token::Colon);

            self.expect(&Token::LBrace);
            let mut body = Vec::new();
            while !self.check(&Token::RBrace) {
                body.push(self.parse_statement());
            }
            self.expect(&Token::RBrace);

            arms.push((pattern, body));
        }
        self.expect(&Token::RBrace);

        Statement::Match { expr, arms }
    }

    fn parse_function_definition(&mut self) -> Statement {
        self.expect(&Token::Fn);
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            panic!("Expected identifier after 'function', found {:?}", self.peek());
        };

        self.expect(&Token::LParenthesis);
        let mut params = Vec::new();
        while !self.check(&Token::RParenthesis) {
            let param_name = if let Some(Token::Identifier) = self.peek() {
                let param_name = self.current_slice.clone();
                self.advance();
                param_name
            } else {
                panic!("Expected parameter name in function definition, found {:?}", self.peek());
            };

            self.expect(&Token::Colon);

            let param_type = self.parse_type();

            params.push((param_name, param_type));

            if self.check(&Token::Separator) {
                self.advance();
            }
        }
        self.expect(&Token::RParenthesis);

        self.expect(&Token::Colon);

        let return_type = self.parse_type();

        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement());
        }
        self.expect(&Token::RBrace);

        Statement::Function { name, params, return_type, body }
    }

    pub fn parse_statement(&mut self) -> Statement {
        match self.peek() {
            Some(Token::Let) => self.parse_let_statement(),
            Some(Token::Const) => self.parse_const_statement(),
            Some(Token::Return) => self.parse_return_statement(),
            Some(Token::Break) => self.parse_break_statement(),
            Some(Token::Continue) => self.parse_continue_statement(),
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::For) => self.parse_for_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::Struct) => self.parse_struct_definition(),
            Some(Token::Error) => self.parse_error_definition(),
            Some(Token::Match) => self.parse_match_statement(),
            Some(Token::Fn) => self.parse_function_definition(),
            _ if !self.at_end() => self.parse_expression_statement(),
            _ => panic!("Unexpected token in statement: {:?}", self.peek()),
        }
    }
}
