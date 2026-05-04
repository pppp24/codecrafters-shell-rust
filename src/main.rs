#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    env,
    fs::{self, Metadata},
    io::stdin,
    os::unix::fs::PermissionsExt,
};

fn is_executable(metadata: &Metadata) -> bool {
    let permissions = metadata.permissions();
    // 0o111 mask checks the executable bit for owner 0o100, group 0o010, and others 0o001
    permissions.mode() & 0o111 != 0
}

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

            if let Some(paths) = env::var_os("PATH") {
                for mut dir in env::split_paths(&paths) {
                    dir.push(rest);

                    if let Ok(metadata) = fs::metadata(&dir) {
                        let is_executable = metadata.is_file() && is_executable(&metadata);

                        if is_executable {
                            println!("{} is {}", rest, dir.display());
                        }
                    }
                }
            }

            println!("{}: not found", rest);
            continue;
        }

        println!("{}: command not found", command.trim());
    }
}
