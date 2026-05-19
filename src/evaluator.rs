use std::collections::HashMap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use crate::ast::{RedirOp, Redirection, SimpleCommand};

pub struct Evaluator {
    builtins: HashMap<&'static str, BuiltinFn>,
}

type BuiltinFn = fn(&Evaluator, &[&str], &mut Stdio);

pub struct Stdio<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

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
        let stdout_file = open_redir_target(&cmd.redirs, 1);
        let stderr_file = open_redir_target(&cmd.redirs, 2);

        let (name, args) = cmd.argv.split_first().expect("Unexpected empty argv list");
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        if let Some(builtin) = self.builtins.get(name.as_str()) {
            let mut stdout_writer: Box<dyn Write> = match &stdout_file {
                Some(f) => Box::new(f),
                None => Box::new(io::stdout()),
            };

            let mut stderr_writer: Box<dyn Write> = match &stderr_file {
                Some(f) => Box::new(f),
                None => Box::new(io::stderr()),
            };

            let mut stdio = Stdio {
                out: &mut *stdout_writer,
                err: &mut *stderr_writer,
            };

            builtin(self, &args, &mut stdio);
            return;
        }

        match get_command_path(name) {
            Some(_) => {
                let mut command = Command::new(name);
                command.args(&args);

                if let Some(f) = stdout_file {
                    command.stdout(f);
                }

                if let Some(f) = stderr_file {
                    command.stderr(f);
                }

                let _ = command.status();
            }
            None => {
                let mut stderr_writer: Box<dyn Write> = match &stderr_file {
                    Some(f) => Box::new(f),
                    None => Box::new(io::stderr()),
                };

                let _ = writeln!(*stderr_writer, "{}: command not found", name);
            }
        }
    }

    fn builtin_exit(&self, _: &[&str], _: &mut Stdio) {
        std::process::exit(0);
    }

    fn builtin_echo(&self, args: &[&str], stdio: &mut Stdio) {
        let _ = writeln!(stdio.out, "{}", args.join(" "));
    }

    fn builtin_type(&self, args: &[&str], stdio: &mut Stdio) {
        let Some(name) = args.first() else { return };
        if self.builtins.contains_key(name) {
            let _ = writeln!(stdio.out, "{} is a shell builtin", name);
        } else if let Some(path) = get_command_path(name) {
            let _ = writeln!(stdio.out, "{} is {}", name, path.display());
        } else {
            let _ = writeln!(stdio.err, "{}: not found", name);
        }
    }

    fn builtin_pwd(&self, _: &[&str], stdio: &mut Stdio) {
        match std::env::current_dir() {
            Ok(p) => {
                let _ = writeln!(stdio.out, "{}", p.display());
            }
            Err(e) => {
                let _ = writeln!(stdio.err, "pwd: {}", e);
            }
        }
    }

    fn builtin_cd(&self, args: &[&str], stdio: &mut Stdio) {
        if args.len() > 1 {
            let _ = writeln!(stdio.err, "Too many args for cd command");
        }

        let target = args.first().copied().unwrap_or("~");

        let target = if target == "~" || target.starts_with("~/") {
            let home = env::var("HOME").unwrap_or_default();
            target.replacen("~", &home, 1)
        } else {
            target.to_string()
        };

        match std::env::set_current_dir(&target) {
            Ok(()) => {}
            Err(_) => {
                let _ = writeln!(stdio.err, "cd: {}: No such file or directory", target);
            }
        }
    }

    pub fn builtin_names(&self) -> Vec<&'static str> {
        self.builtins.keys().copied().collect()
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

fn open_redir_target(redirs: &[Redirection], fd: u32) -> Option<File> {
    let mut active: Option<File> = None;

    for r in redirs {
        if r.op == RedirOp::Out && r.fd == fd {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&r.target)
            {
                Ok(f) => active = Some(f),
                Err(e) => {
                    eprintln!("unexpected error encountered during file handling: {}", e);
                }
            }
        }

        if r.op == RedirOp::Append && r.fd == fd {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&r.target)
            {
                Ok(f) => active = Some(f),
                Err(e) => {
                    eprintln!("unexpected error encountered during file handling: {}", e);
                }
            }
        }
    }

    active
}
