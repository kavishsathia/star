use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[token("let")]
    Let,

    #[token("const")]
    Const,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("for")]
    For,

    #[token("while")]
    While,

    #[token("match")]
    Match,

    #[token("fn")]
    Fn,

    #[token("import")]
    Import,

    #[token("from")]
    From,

    #[token("return")]
    Return,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("??")]
    NotNull,

    #[token("!!")]
    NotError,
    
    #[token("==")]
    Eq,

    #[token("<=")]
    Lte,

    #[token(">=")]
    Gte,

    #[token(">")]
    Gt,

    #[token("<")]
    Lt,

    #[token("!=")]
    Neq,

    #[token("=")]
    Is,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("/")]
    Divide,

    #[token("*")]
    Multiply,

    #[token("**")]
    Power,

    #[token("and")]
    And,

    #[token("or")]
    Or,

    #[token("not")]
    Not,
    
    #[token("&")]
    BitwiseAnd,

    #[token("|")]
    BitwiseOr,

    #[token("^")]
    Xor,

    #[token("<<")]
    Sll,

    #[token(">>")]
    Srl,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("(")]
    LParenthesis,

    #[token(")")]
    RParenthesis,

    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

    #[token(",")]
    Separator,

    #[token(".")]
    Access,

    #[token("?")]
    Nullable,

    #[token("!")]
    Errorable,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("null")]
    Null,
    
    #[token("struct")]
    Struct,
    
    #[token("error")]
    Error,

    #[token("raise")]
    Raise,

    #[token("print")]
    Print,

    #[token("new")]
    New,

    #[token("as")]
    As,

    #[token("produce")]
    Produce,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    #[regex(r"[0-9]+")]
    Integer,

    #[regex(r"[0-9]+\.[0-9]+")]
    Float,

    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
}