use std::collections::HashMap;
use std::fs::Metadata;
use std::io::{self, BufRead, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use crate::lexer::lex;

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

fn read_command(stdin: &io::Stdin) -> Option<Vec<String>> {
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
            return Some(tokens.into_iter().map(|token| token.literal).collect());
        }

        // Quote still open - print continuation prompt and read another line.
        print!("> ");
        io::stdout().flush().unwrap();
    }
}

type BuiltinFn = fn(&Shell, &[&str]);

pub struct Shell {
    builtins: HashMap<&'static str, BuiltinFn>,
}

impl Shell {
    pub fn new() -> Self {
        let entries: &[(&'static str, BuiltinFn)] = &[
            ("exit", Shell::builtin_exit),
            ("echo", Shell::builtin_echo),
            ("type", Shell::builtin_type),
            ("pwd", Shell::builtin_pwd),
            ("cd", Shell::builtin_cd),
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

    fn builtin_pwd(&self, _: &[&str]) {
        match std::env::current_dir() {
            Ok(p) => println!("{}", p.display()),
            Err(e) => eprintln!("pwd: {}", e),
        }
    }

    fn builtin_cd(&self, args: &[&str]) {
        if args.len() > 1 {
            eprintln!("Too many args for cd command")
        }

        let target = args[0];

        let target = if target == "~" || target.starts_with("~/") {
            let home = env::var("HOME").unwrap_or_default();
            target.replacen("~", &home, 1)
        } else {
            target.to_string()
        };

        match std::env::set_current_dir(&target) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("cd: {}: No such file or directory", target);
            }
        }
    }

    pub fn run(&self) {
        let stdin = io::stdin();
        loop {
            let tokens = match read_command(&stdin) {
                Some(t) => t,
                None => break, // EOF
            };

            let Some((cmd, args)) = tokens.split_first() else {
                continue; // empty input (just whitespace)
            };

            let args: Vec<&str> = args.iter().map(String::as_str).collect();

            if let Some(builtin) = self.builtins.get(cmd.as_str()) {
                builtin(self, &args);
                continue;
            }

            match get_command_path(cmd) {
                Some(_) => {
                    let _ = Command::new(cmd).args(&args).status();
                }
                None => println!("{}: command not found", cmd),
            }
        }
    }
}
