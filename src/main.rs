mod ast;
mod evaluator;
mod lexer;
mod parser;
mod shell;
mod token;

fn main() {
    shell::Shell::new().run();
}
