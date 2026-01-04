use crate::lexer::Token;
use crate::ast::Statement;
use super::Parser;

impl<'a> Parser<'a> {
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

    fn parse_print_statement(&mut self) -> Statement {
        self.expect(&Token::Print);
        let expr = Box::new(self.parse_expression(0));
        self.expect(&Token::Semicolon);
        Statement::Print(expr)
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
            Some(Token::Print) => self.parse_print_statement(),
            _ if !self.at_end() => self.parse_expression_statement(),
            _ => panic!("Unexpected token in statement: {:?}", self.peek()),
        }
    }
}
