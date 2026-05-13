mod lexer;
mod shell;
mod token;

fn main() {
    shell::Shell::new().run();
}
