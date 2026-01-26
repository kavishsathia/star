use crate::ast::Statement;
use crate::error::CompilerError;
use crate::lexer::Token;
use super::Parser;

impl<'a> Parser<'a> {
    fn parse_let_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Let)?;
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected identifier after 'let', found {:?}", self.peek()),
            });
        };

        self.expect(&Token::Colon)?;

        let ty = self.parse_type()?;

        let value = if self.match_token(&Token::Is) {
            Some(self.parse_expression(0)?)
        } else {
            None
        };

        self.expect(&Token::Semicolon)?;

        Ok(Statement::Let { name, ty, value })
    }

    fn parse_const_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Const)?;
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected identifier after 'const', found {:?}", self.peek()),
            });
        };

        self.expect(&Token::Colon)?;

        let ty = self.parse_type()?;

        let value = if self.match_token(&Token::Is) {
            self.parse_expression(0)?
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected '=' after const declaration, found {:?}", self.peek()),
            });
        };

        self.expect(&Token::Semicolon)?;

        Ok(Statement::Const { name, ty, value })
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, CompilerError> {
        let expr = self.parse_expression(0)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Expr(expr))
    }

    fn parse_return_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Return)?;
        let expr = if !self.check(&Token::Semicolon) {
            Some(self.parse_expression(0)?)
        } else {
            None
        };
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Return(expr))
    }

    fn parse_break_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Break)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Break)
    }

    fn parse_continue_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Continue)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Continue)
    }

    fn parse_print_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Print)?;
        let expr = self.parse_expression(0)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Print(expr))
    }

    fn parse_produce_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Produce)?;
        let expr = self.parse_expression(0)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Produce(expr))
    }

    fn parse_raise_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Raise)?;
        let expr = self.parse_expression(0)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Raise(expr))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::If)?;
        let condition = self.parse_expression(0)?;
        self.expect(&Token::LBrace)?;
        let mut then_block = Vec::new();
        while !self.check(&Token::RBrace) {
            then_block.push(self.parse_statement(false)?);
        }
        self.expect(&Token::RBrace)?;

        let else_block = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                Some(vec![self.parse_if_statement()?])
            } else {
                self.expect(&Token::LBrace)?;
                let mut alternate_block = Vec::new();
                while !self.check(&Token::RBrace) {
                    alternate_block.push(self.parse_statement(false)?);
                }
                self.expect(&Token::RBrace)?;
                Some(alternate_block)
            }
        } else {
            None
        };

        Ok(Statement::If { condition, then_block, else_block })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::For)?;
        let init = Box::new(self.parse_statement(false)?);
        let condition = self.parse_expression(0)?;
        self.expect(&Token::Semicolon)?;
        let update = Box::new(self.parse_statement(false)?);
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false)?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Statement::For { init, condition, update, body })
    }

    fn parse_while_statement(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::While)?;
        let condition = self.parse_expression(0)?;
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false)?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Statement::While { condition, body })
    }

    fn parse_struct_definition(&mut self, top_level: bool) -> Result<Statement, CompilerError> {
        if !top_level {
            return Err(CompilerError::Parse {
                message: "Struct definitions must be at top level".to_string(),
            });
        }
        self.expect(&Token::Struct)?;
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected identifier after 'struct', found {:?}", self.peek()),
            });
        };

        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            let field_name = if let Some(Token::Identifier) = self.peek() {
                let field_name = self.current_slice.clone();
                self.advance();
                field_name
            } else {
                return Err(CompilerError::Parse {
                    message: format!("Expected field name in struct definition, found {:?}", self.peek()),
                });
            };

            self.expect(&Token::Colon)?;

            let field_type = self.parse_type()?;

            fields.push((field_name, field_type));

            if self.check(&Token::Separator) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(Statement::Struct { name, fields })
    }

    fn parse_error_definition(&mut self, top_level: bool) -> Result<Statement, CompilerError> {
        if !top_level {
            return Err(CompilerError::Parse {
                message: "Error definitions must be at top level".to_string(),
            });
        }
        self.expect(&Token::Error)?;
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected identifier after 'error', found {:?}", self.peek()),
            });
        };

        self.expect(&Token::Semicolon)?;
        Ok(Statement::Error { name })
    }

    fn parse_function_definition(&mut self) -> Result<Statement, CompilerError> {
        self.expect(&Token::Fn)?;
        let name = if let Some(Token::Identifier) = self.peek() {
            let name = self.current_slice.clone();
            self.advance();
            name
        } else {
            return Err(CompilerError::Parse {
                message: format!("Expected identifier after 'fn', found {:?}", self.peek()),
            });
        };

        self.expect(&Token::LParenthesis)?;
        let mut params = Vec::new();
        while !self.check(&Token::RParenthesis) {
            let param_name = if let Some(Token::Identifier) = self.peek() {
                let param_name = self.current_slice.clone();
                self.advance();
                param_name
            } else {
                return Err(CompilerError::Parse {
                    message: format!("Expected parameter name in function definition, found {:?}", self.peek()),
                });
            };

            self.expect(&Token::Colon)?;

            let param_type = self.parse_type()?;

            params.push((param_name, param_type));

            if self.check(&Token::Separator) {
                self.advance();
            }
        }
        self.expect(&Token::RParenthesis)?;

        self.expect(&Token::Colon)?;

        let returns = self.parse_type()?;

        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement(false)?);
        }
        self.expect(&Token::RBrace)?;

        Ok(Statement::Function { name, params, returns, body })
    }

    pub fn parse_statement(&mut self, top_level: bool) -> Result<Statement, CompilerError> {
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
            Some(Token::Raise) => self.parse_raise_statement(),
            _ if !self.at_end() => self.parse_expression_statement(),
            _ => Err(CompilerError::Parse {
                message: format!("Unexpected token in statement: {:?}", self.peek()),
            }),
        }
    }
}
