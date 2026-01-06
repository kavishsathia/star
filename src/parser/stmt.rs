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

        let ty = self.parse_type();

        let value = if self.match_token(&Token::Is) {
            Some(self.parse_expression(0))
        } else {
            None
        };

        self.expect(&Token::Semicolon);

        Statement::Let { name, ty, value }
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

        let ty = self.parse_type();

        let value = if self.match_token(&Token::Is) {
            self.parse_expression(0)
        } else {
            panic!("Expected '=' after const declaration, found {:?}", self.peek());
        };

        self.expect(&Token::Semicolon);

        Statement::Const { name, ty, value }
    }

    fn parse_expression_statement(&mut self) -> Statement {
        let expr = self.parse_expression(0);
        self.expect(&Token::Semicolon);
        Statement::Expr(expr)
    }

    fn parse_return_statement(&mut self) -> Statement {
        self.expect(&Token::Return);
        let expr = if !self.check(&Token::Semicolon) {
            Some(self.parse_expression(0))
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
        let expr = self.parse_expression(0);
        self.expect(&Token::Semicolon);
        Statement::Print(expr)
    }

    fn parse_produce_statement(&mut self) -> Statement {
        self.expect(&Token::Produce);
        let expr = self.parse_expression(0);
        self.expect(&Token::Semicolon);
        Statement::Produce(expr)
    }

    fn parse_if_statement(&mut self) -> Statement {
        self.expect(&Token::If);
        let condition = self.parse_expression(0);
        self.expect(&Token::LBrace);
        let mut then_block = Vec::new();
        while !self.check(&Token::RBrace) {
            then_block.push(self.parse_statement(false));
        }
        self.expect(&Token::RBrace);

        let else_block = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                Some(vec![self.parse_if_statement()])
            } else {
                self.expect(&Token::LBrace);
                let mut alternate_block = Vec::new();
                while !self.check(&Token::RBrace) {
                    alternate_block.push(self.parse_statement(false));
                }
                self.expect(&Token::RBrace);
                Some(alternate_block)
            }
        } else {
            None
        };

        Statement::If { condition, then_block, else_block }
    }

    fn parse_for_statement(&mut self) -> Statement {
        self.expect(&Token::For);
        let init = Box::new(self.parse_statement(false));
        let condition = self.parse_expression(0);
        self.expect(&Token::Semicolon);
        let update = Box::new(self.parse_statement(false));
        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false));
        }
        self.expect(&Token::RBrace);
        Statement::For { init, condition, update, body }
    }

    fn parse_while_statement(&mut self) -> Statement {
        self.expect(&Token::While);
        let condition = self.parse_expression(0);
        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false));
        }
        self.expect(&Token::RBrace);
        Statement::While { condition, body }
    }

    fn parse_struct_definition(&mut self, top_level: bool) -> Statement {
        if !top_level {
            panic!("Struct definitions must be at top level");
        }
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

    fn parse_error_definition(&mut self, top_level: bool) -> Statement {
        if !top_level {
            panic!("Error definitions must be at top level");
        }
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

        let returns = self.parse_type();

        self.expect(&Token::LBrace);
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false));
        }
        self.expect(&Token::RBrace);

        Statement::Function { name, params, returns, body }
    }

    pub fn parse_statement(&mut self, top_level: bool) -> Statement {
        match self.peek() {
            Some(Token::Let) => self.parse_let_statement(),
            Some(Token::Const) => self.parse_const_statement(),
            Some(Token::Return) => self.parse_return_statement(),
            Some(Token::Break) => self.parse_break_statement(),
            Some(Token::Continue) => self.parse_continue_statement(),
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::For) => self.parse_for_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::Struct) => self.parse_struct_definition(top_level),
            Some(Token::Error) => self.parse_error_definition(top_level),
            Some(Token::Fn) => self.parse_function_definition(),
            Some(Token::Print) => self.parse_print_statement(),
            Some(Token::Produce) => self.parse_produce_statement(),
            _ if !self.at_end() => self.parse_expression_statement(),
            _ => panic!("Unexpected token in statement: {:?}", self.peek()),
        }
    }
}
