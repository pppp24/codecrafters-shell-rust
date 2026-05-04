#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    env,
    fs::{self, Metadata},
    io::{Result, stdin},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
};

fn is_executable(metadata: &Metadata) -> bool {
    let permissions = metadata.permissions();
    // 0o111 mask checks the executable bit for owner 0o100, group 0o010, and others 0o001
    permissions.mode() & 0o111 != 0
}

fn get_command_path(name: &str) -> Option<PathBuf> {
    if let Some(paths) = env::var_os("PATH") {
        for mut dir in env::split_paths(&paths) {
            dir.push(name);

            if let Ok(metadata) = fs::metadata(&dir) {
                let is_executable = metadata.is_file() && is_executable(&metadata);

                if is_executable {
                    return Some(dir);
                }
            }
        }
    }

    return None;
}

fn main() {
    loop {
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

            let path = get_command_path(rest);

            if path.is_none() {
                println!("{}: not found", rest);
            } else {
                println!("{} is {}", rest, path.unwrap().display());
            }

            continue;
        }

        let mut parts = command.split_whitespace();
        let command = parts.next();

        if command.is_none() {
            continue;
        }

        let args = parts.collect::<Vec<&str>>();

        let path = get_command_path(command.unwrap());

        if path.is_none() {
            println!("{}: command not found", command.unwrap().trim());
            continue;
        }

        let _ = Command::new(command.unwrap()).args(&args).status();
    }
}
