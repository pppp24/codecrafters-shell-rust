use crate::{
    ast::{RedirOp, Redirection, SimpleCommand},
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
        let mut redirs: Vec<Redirection> = vec![];
        loop {
            match self.cur_token() {
                Token {
                    r#type: TokenType::Word,
                    literal,
                } => words.push(literal.clone()),
                Token {
                    r#type: TokenType::IoNumber,
                    literal,
                } => {
                    let literal = literal.clone();
                    if !matches!(self.peek_token().r#type, TokenType::Gt | TokenType::GtGt) {
                        eprintln!("unexpected isolated IoNumber");
                        return None;
                    }

                    let mut redir_op = RedirOp::Out;

                    if self.peek_token().r#type == TokenType::GtGt {
                        redir_op = RedirOp::Append;
                    }

                    self.advance(); // consume Gt or GtGt
                    self.advance(); // position on target

                    let fd = match literal.parse::<u32>() {
                        Ok(fd) => fd,
                        Err(e) => {
                            eprintln!("invalid IoNumber type: {}", e);
                            return None;
                        }
                    };

                    if self.cur_token().r#type != TokenType::Word {
                        eprintln!("unexpected non-word token: {:?}", self.cur_token());
                        return None;
                    }

                    redirs.push(Redirection {
                        fd,
                        op: redir_op,
                        target: self.cur_token().clone().literal,
                    })
                }
                Token {
                    r#type: TokenType::Gt,
                    ..
                } => {
                    self.advance(); // consume Gt

                    if self.cur_token().r#type != TokenType::Word {
                        eprintln!("unexpected non-word token: {:?}", self.cur_token());
                        return None;
                    }

                    redirs.push(Redirection {
                        fd: 1,
                        op: RedirOp::Out,
                        target: self.cur_token().clone().literal,
                    })
                }
                Token {
                    r#type: TokenType::GtGt,
                    ..
                } => {
                    self.advance(); // consume GtGt

                    if self.cur_token().r#type != TokenType::Word {
                        eprintln!("unexpected non-word token: {:?}", self.cur_token());
                        return None;
                    }

                    redirs.push(Redirection {
                        fd: 1,
                        op: RedirOp::Append,
                        target: self.cur_token().clone().literal,
                    })
                }
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
            Some(SimpleCommand {
                argv: words,
                redirs,
            })
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
