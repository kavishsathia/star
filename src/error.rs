use std::fmt;

#[derive(Debug, Clone)]
pub enum CompilerError {
    Parse { message: String },
    Type { message: String },
    Locals { message: String },
    IRGen { message: String },
    Codegen { message: String },
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::Parse { message } => write!(f, "Parse error: {}", message),
            CompilerError::Type { message } => write!(f, "Type error: {}", message),
            CompilerError::Locals { message } => write!(f, "Locals error: {}", message),
            CompilerError::IRGen { message } => write!(f, "IR generation error: {}", message),
            CompilerError::Codegen { message } => write!(f, "Codegen error: {}", message),
        }
    }
}

impl std::error::Error for CompilerError {}
