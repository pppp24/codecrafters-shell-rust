use std::io::stdin;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        // TODO: Uncomment the code below to pass the first stage
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command: String = String::new();
        stdin().read_line(&mut command).unwrap();

        if command.trim() == "exit" {
            break;
        }

        if command.starts_with("echo ") {
            println!("{}", &command[5..].trim_end());
            continue;
        }

        if command.starts_with("type ") {
            let rest = command[5..].trim_end();

            if rest == "exit" || rest == "echo" || rest == "type" {
                println!("{} is a shell builtin", rest);
                continue;
            }

            println!("{}: not found", rest);
            continue;
        }

        println!("{}: command not found", command.trim());
    }
}
