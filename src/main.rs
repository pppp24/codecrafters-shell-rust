use std::io::stdin;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");
    io::stdout().flush().unwrap();
    let mut command: String = String::new();
    stdin().read_line(&mut command).unwrap();

    println!("{}: command not found", command.trim());
}
