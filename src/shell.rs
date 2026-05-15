use std::io::{self, BufRead, Write};

use crate::{evaluator::Evaluator, lexer::lex, parser::Parser, token::Token};

pub struct Shell {
    evaluator: Evaluator,
}

fn read_command(stdin: &io::Stdin) -> Option<Vec<Token>> {
    let mut buffer = String::new();

    print!("$ ");
    io::stdout().flush().unwrap();

    loop {
        let mut line = String::new();
        let bytes = stdin.lock().read_line(&mut line).unwrap();
        if bytes == 0 {
            if buffer.is_empty() {
                return None;
            } else {
                eprintln!("shell: unexpected EOF while looking for matching quote");
                return Some(vec![]); // continuation EOF -> cancel + re-prompt
            }
        }

        buffer.push_str(&line);

        let (tokens, unterminated) = lex(&buffer);
        if !unterminated {
            return Some(tokens);
        }

        // Quote still open - print continuation prompt and read another line.
        print!("> ");
        io::stdout().flush().unwrap();
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            evaluator: Evaluator::new(),
        }
    }

    pub fn run(&self) {
        let stdin = io::stdin();

        loop {
            let tokens = match read_command(&stdin) {
                Some(tokens) => tokens,
                None => break,
            };

            if let Some(cmd) = Parser::new(tokens).parse() {
                self.evaluator.eval(&cmd);
            }
        }
    }
}
