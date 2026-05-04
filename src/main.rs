use std::collections::HashMap;
use std::fs::Metadata;
use std::io::{self, BufRead, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

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

type BuiltinFn = fn(&Shell, &[&str]);

struct Shell {
    builtins: HashMap<&'static str, BuiltinFn>,
}

impl Shell {
    fn new() -> Self {
        let entries: &[(&'static str, BuiltinFn)] = &[
            ("exit", Shell::builtin_exit),
            ("echo", Shell::builtin_echo),
            ("type", Shell::builtin_type),
            ("pwd", Shell::builtin_pwd),
        ];

        let builtins = entries.iter().copied().collect();
        Shell { builtins }
    }

    fn builtin_exit(&self, _args: &[&str]) {
        std::process::exit(0);
    }

    fn builtin_echo(&self, args: &[&str]) {
        println!("{}", args.join(" "));
    }

    fn builtin_type(&self, args: &[&str]) {
        let Some(name) = args.first() else { return };
        if self.builtins.contains_key(name) {
            println!("{} is a shell builtin", name);
        } else if let Some(path) = get_command_path(name) {
            println!("{} is {}", name, path.display());
        } else {
            println!("{}: not found", name);
        }
    }

    fn builtin_pwd(&self, args: &[&str]) {
        unimplemented!()
    }

    fn run(&self) {
        let stdin = io::stdin();
        loop {
            print!("$ ");
            io::stdout().flush().unwrap();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap() == 0 {
                break;
            }

            let mut parts = line.split_whitespace();
            let Some(cmd) = parts.next() else { continue };
            let args: Vec<&str> = parts.collect();

            if let Some(builtin) = self.builtins.get(cmd) {
                builtin(self, &args);
                continue;
            }

            match get_command_path(cmd) {
                Some(path) => {
                    let _ = Command::new(path).args(&args).status();
                }
                None => println!("{}: command not found", cmd),
            }
        }
    }
}

fn main() {
    Shell::new().run();
}
