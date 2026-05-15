use std::collections::HashMap;
use std::fs::Metadata;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use crate::ast::SimpleCommand;

pub struct Evaluator {
    builtins: HashMap<&'static str, BuiltinFn>,
}

type BuiltinFn = fn(&Evaluator, &[&str]);

impl Evaluator {
    pub fn new() -> Self {
        let entries: &[(&'static str, BuiltinFn)] = &[
            ("exit", Evaluator::builtin_exit),
            ("echo", Evaluator::builtin_echo),
            ("type", Evaluator::builtin_type),
            ("pwd", Evaluator::builtin_pwd),
            ("cd", Evaluator::builtin_cd),
        ];

        let builtins = entries.iter().copied().collect();
        Evaluator { builtins }
    }

    pub fn eval(&self, cmd: &SimpleCommand) {
        let (cmd, args) = cmd.argv.split_first().expect("Unexpected empty argv list");
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        if let Some(builtin) = self.builtins.get(cmd.as_str()) {
            builtin(self, &args);
            return;
        }

        match get_command_path(cmd) {
            Some(_) => {
                let _ = Command::new(cmd).args(&args).status();
            }
            None => println!("{}: command not found", cmd),
        }
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
}

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
