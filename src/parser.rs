use crate::{
    ast::SimpleCommand,
    token::{Token, TokenType},
};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    eof: Token,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            eof: Token::new(TokenType::Eof, ""),
        }
    }
    pub fn parse(&mut self) -> Option<SimpleCommand> {
        let mut words = vec![];

        loop {
            match self.cur_token() {
                Token {
                    r#type: TokenType::Word,
                    literal,
                } => words.push(literal.clone()),
                Token {
                    r#type: TokenType::Eof,
                    ..
                } => break,
                Token {
                    r#type: TokenType::Illegal,
                    ..
                } => unreachable!(),
            }

            self.advance();
        }

        if words.is_empty() {
            None
        } else {
            Some(SimpleCommand { argv: words })
        }
    }

    fn cur_token(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.eof)
    }

    fn peek_token(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&self.eof)
    }

    fn advance(&mut self) {
        if self.cur_token().r#type != TokenType::Eof {
            self.pos += 1
        }
    }
}
