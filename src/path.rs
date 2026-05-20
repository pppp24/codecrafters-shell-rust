use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fs::{self, Metadata},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

fn is_executable(metadata: &Metadata) -> bool {
    let permissions = metadata.permissions();
    // 0o111 mask checks the executable bit for owner 0o100, group 0o010, and others 0o001
    permissions.mode() & 0o111 != 0
}

pub fn find_command(name: &str, paths: Option<&OsStr>) -> Option<PathBuf> {
    let paths = paths?;

    for mut dir in env::split_paths(&paths) {
        dir.push(name);
        if let Ok(metadata) = fs::metadata(&dir) {
            let is_executable = metadata.is_file() && is_executable(&metadata);
            if is_executable {
                return Some(dir);
            }
        }
    }

    None
}

pub fn complete_command(prefix: &str, builtins: &[&str], paths: Option<&OsStr>) -> Vec<String> {
    let mut set = HashSet::new();
    for builtin in builtins
        .iter()
        .filter(|builtin| builtin.starts_with(prefix))
    {
        set.insert((builtin).to_string());
    }

    if let Some(path) = paths {
        for dir in env::split_paths(path) {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };

                if !name.starts_with(prefix) {
                    continue;
                }

                let Ok(metadata) = entry.metadata() else {
                    continue;
                };

                if metadata.is_file() && is_executable(&metadata) {
                    set.insert(name.to_string());
                }
            }
        }
    }

    let mut out: Vec<String> = set.into_iter().collect();
    out.sort();

    return out;
}
