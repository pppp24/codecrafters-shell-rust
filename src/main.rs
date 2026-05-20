mod ast;
mod evaluator;
mod lexer;
mod line_editor;
mod parser;
mod path;
mod shell;
mod token;

fn main() {
    shell::Shell::new().run();
}
