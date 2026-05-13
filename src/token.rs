#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    // Literals
    Word, // a bare word or quoted string after expansion

    // Operators
    // Pipe,        // |
    // RedirectIn,  //
    // RedirectOut, // >
    // AppendOut,   // >>
    // And,         // &&
    // Or,          // ||
    // Semicolon,   // ;
    // Ampersand,   // &

    // Special
    Eof,
    Illegal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub r#type: TokenType,
    pub literal: String,
}

impl Token {
    pub fn new(r#type: TokenType, literal: impl Into<String>) -> Self {
        Self {
            r#type,
            literal: literal.into(),
        }
    }
}
