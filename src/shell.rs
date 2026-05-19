use crate::{
    evaluator::Evaluator, lexer::lex, line_editor::read_line, parser::Parser, token::Token,
};

pub struct Shell {
    evaluator: Evaluator,
}

fn read_command(builtins: &[&str]) -> Option<Vec<Token>> {
    let mut buffer = String::new();
    let mut prompt = "$ ";

    loop {
        let line = match read_line(prompt, builtins) {
            Some(line) => line,
            None => {
                if buffer.is_empty() {
                    return None;
                } else {
                    eprintln!("shell: unexpected EOF while looking for matching quote");
                    return Some(vec![]); // continuation EOF -> cancel + re-prompt
                }
            }
        };

        buffer.push_str(&line);

        let (tokens, unterminated) = lex(&buffer);

        if !unterminated {
            return Some(tokens);
        }

        prompt = "> "
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            evaluator: Evaluator::new(),
        }
    }

    pub fn run(&self) {
        loop {
            let tokens = match read_command(&self.evaluator.builtin_names()) {
                Some(tokens) => tokens,
                None => break,
            };

            if let Some(cmd) = Parser::new(tokens).parse() {
                self.evaluator.eval(&cmd);
            }
        }
    }
}
