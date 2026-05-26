use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fs::{self, Metadata},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub fn longest_common_prefix(matches: &[String]) -> usize {
    let Some(first) = matches.first() else {
        return 0;
    };

    matches.iter().skip(1).fold(first.len(), |acc, s| {
        first
            .bytes()
            .zip(s.bytes())
            .take(acc)
            .take_while(|(a, b)| a == b)
            .count()
    })
}

fn is_executable(metadata: &Metadata) -> bool {
    let permissions = metadata.permissions();
    // 0o111 mask checks the executable bit for owner 0o100, group 0o010, and others 0o001
    permissions.mode() & 0o111 != 0
}

pub fn find_command(name: &str, paths: Option<&OsStr>) -> Option<PathBuf> {
    let paths = paths?;

    for mut dir in env::split_paths(paths) {
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

pub fn complete_filename(prefix: &str, cwd: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Ok(entries) = fs::read_dir(cwd) else {
        return out;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if name_str.starts_with(prefix) {
            out.push(name_str.to_string())
        }
    }

    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // --- helpers ------------------------------------------------------------

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Minimal RAII temp directory — created on `new`, removed on drop.
    /// Avoids a `tempfile` dev-dependency.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let mut p = std::env::temp_dir();
            // pid+counter for uniqueness across parallel tests in this process
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            p.push(format!("shell-path-test-{}-{}", std::process::id(), id));
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn as_os_str(&self) -> &OsStr {
            self.0.as_os_str()
        }

        /// Create an empty regular file with mode 0o755 (owner exec set).
        fn touch_exec(&self, name: &str) {
            let p = self.0.join(name);
            fs::write(&p, b"").unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }

        /// Create an empty regular file with mode 0o644 (no exec bits).
        fn touch_plain(&self, name: &str) {
            let p = self.0.join(name);
            fs::write(&p, b"").unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        }

        /// Create a subdirectory of any name (regardless of mode).
        fn mkdir(&self, name: &str) {
            fs::create_dir(self.0.join(name)).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn join<I, S>(dirs: I) -> OsString
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        env::join_paths(dirs).unwrap()
    }

    const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd"];

    // --- find_command -------------------------------------------------------

    #[test]
    fn find_command_returns_none_when_path_is_none() {
        assert_eq!(find_command("ls", None), None);
    }

    #[test]
    fn find_command_returns_none_when_path_is_empty() {
        let empty = OsString::new();
        assert_eq!(find_command("ls", Some(&empty)), None);
    }

    #[test]
    fn find_command_finds_executable_in_dir() {
        let dir = TempDir::new();
        dir.touch_exec("foo");
        assert_eq!(
            find_command("foo", Some(dir.as_os_str())).as_deref(),
            Some(dir.path().join("foo").as_path()),
        );
    }

    #[test]
    fn find_command_returns_none_for_non_executable_file() {
        let dir = TempDir::new();
        dir.touch_plain("foo");
        assert_eq!(find_command("foo", Some(dir.as_os_str())), None);
    }

    #[test]
    fn find_command_returns_none_for_subdirectory() {
        // A directory named "foo" in PATH must not match — only regular files.
        let dir = TempDir::new();
        dir.mkdir("foo");
        assert_eq!(find_command("foo", Some(dir.as_os_str())), None);
    }

    #[test]
    fn find_command_returns_first_match_in_path_order() {
        // PATH ordering: the leftmost dir with a match wins.
        let dir1 = TempDir::new();
        let dir2 = TempDir::new();
        dir1.touch_exec("foo");
        dir2.touch_exec("foo");
        let path = join([dir1.path(), dir2.path()]);
        let found = find_command("foo", Some(&path)).unwrap();
        assert_eq!(found, dir1.path().join("foo"));
    }

    #[test]
    fn find_command_skips_missing_path_dirs() {
        // A non-existent dir in PATH is silently skipped, not fatal.
        let dir = TempDir::new();
        dir.touch_exec("foo");
        let missing = Path::new("/nonexistent_xyz_dir_for_test");
        let path = join([missing, dir.path()]);
        let found = find_command("foo", Some(&path)).unwrap();
        assert_eq!(found, dir.path().join("foo"));
    }

    // --- complete_command: builtin-only -------------------------------------

    #[test]
    fn complete_command_builtin_unique_match() {
        assert_eq!(complete_command("ech", BUILTINS, None), vec!["echo"]);
    }

    #[test]
    fn complete_command_builtin_multi_match_sorted() {
        // 'e' matches both 'echo' and 'exit'; result must be sorted.
        assert_eq!(complete_command("e", BUILTINS, None), vec!["echo", "exit"]);
    }

    #[test]
    fn complete_command_no_match() {
        assert_eq!(
            complete_command("xyz", BUILTINS, None),
            Vec::<String>::new()
        );
    }

    #[test]
    fn complete_command_empty_prefix_matches_all_builtins() {
        let result = complete_command("", &["echo", "exit"], None);
        assert_eq!(result, vec!["echo", "exit"]);
    }

    // --- complete_command: PATH -------------------------------------------

    #[test]
    fn complete_command_finds_path_executable() {
        // Mirrors the codecrafters tester: `custom<TAB>` -> `custom_executable`.
        let dir = TempDir::new();
        dir.touch_exec("custom_executable");
        assert_eq!(
            complete_command("custom", &[], Some(dir.as_os_str())),
            vec!["custom_executable"]
        );
    }

    #[test]
    fn complete_command_combines_builtin_and_path() {
        let dir = TempDir::new();
        dir.touch_exec("echo_external");
        assert_eq!(
            complete_command("ech", BUILTINS, Some(dir.as_os_str())),
            vec!["echo", "echo_external"]
        );
    }

    #[test]
    fn complete_command_dedupes_builtin_and_path_with_same_name() {
        // A PATH executable named 'echo' must not double up the builtin.
        let dir = TempDir::new();
        dir.touch_exec("echo");
        assert_eq!(
            complete_command("ech", BUILTINS, Some(dir.as_os_str())),
            vec!["echo"]
        );
    }

    #[test]
    fn complete_command_excludes_non_executable_file() {
        let dir = TempDir::new();
        dir.touch_plain("custom_textfile");
        assert!(
            complete_command("custom", &[], Some(dir.as_os_str())).is_empty(),
            "non-executable file must not be a completion candidate"
        );
    }

    #[test]
    fn complete_command_excludes_subdirectory() {
        let dir = TempDir::new();
        dir.mkdir("custom_dir");
        assert!(
            complete_command("custom", &[], Some(dir.as_os_str())).is_empty(),
            "directory must not be a completion candidate"
        );
    }

    #[test]
    fn complete_command_dedupes_same_name_across_dirs() {
        let dir1 = TempDir::new();
        let dir2 = TempDir::new();
        dir1.touch_exec("same_name");
        dir2.touch_exec("same_name");
        let path = join([dir1.path(), dir2.path()]);
        assert_eq!(
            complete_command("same", &[], Some(&path)),
            vec!["same_name"]
        );
    }

    #[test]
    fn complete_command_path_multi_match_sorted() {
        let dir = TempDir::new();
        dir.touch_exec("xy_foo");
        dir.touch_exec("xy_bar");
        assert_eq!(
            complete_command("xy", &[], Some(dir.as_os_str())),
            vec!["xy_bar", "xy_foo"]
        );
    }

    #[test]
    fn complete_command_path_dir_with_no_matches_is_harmless() {
        // The PATH dir contains executables, but none match the prefix.
        // Builtins should still be considered.
        let dir = TempDir::new();
        dir.touch_exec("something_else_entirely");
        assert_eq!(
            complete_command("ech", BUILTINS, Some(dir.as_os_str())),
            vec!["echo"]
        );
    }

    #[test]
    fn complete_command_skips_missing_path_dirs() {
        // A non-existent dir in PATH is silently skipped.
        let dir = TempDir::new();
        dir.touch_exec("custom_executable");
        let missing = Path::new("/nonexistent_xyz_dir_for_test");
        let path = join([missing, dir.path()]);
        assert_eq!(
            complete_command("custom", &[], Some(&path)),
            vec!["custom_executable"]
        );
    }

    // --- longest_common_prefix ----------------------------------------------

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn lcp_empty_slice_returns_zero() {
        assert_eq!(longest_common_prefix(&[]), 0);
    }

    #[test]
    fn lcp_single_string_returns_its_length() {
        assert_eq!(longest_common_prefix(&s(&["echo"])), 4);
    }

    #[test]
    fn lcp_single_empty_string_returns_zero() {
        assert_eq!(longest_common_prefix(&s(&[""])), 0);
    }

    #[test]
    fn lcp_all_identical() {
        assert_eq!(longest_common_prefix(&s(&["echo", "echo", "echo"])), 4);
    }

    #[test]
    fn lcp_no_common_prefix() {
        assert_eq!(longest_common_prefix(&s(&["abc", "xyz"])), 0);
    }

    #[test]
    fn lcp_partial_common_prefix() {
        // 'echo' and 'exit' share only the first byte.
        assert_eq!(longest_common_prefix(&s(&["echo", "exit"])), 1);
    }

    #[test]
    fn lcp_one_string_is_prefix_of_others() {
        // 'xyz_foo' is a prefix of the other two; LCP is the length of the shortest.
        assert_eq!(
            longest_common_prefix(&s(&["xyz_foo", "xyz_foo_bar", "xyz_foo_bar_baz"])),
            7,
        );
    }

    #[test]
    fn lcp_codecrafters_scenario_step_two() {
        // After step 1 of the codecrafters LCP test, the candidate set shrinks
        // to these two; LCP becomes 'xyz_foo_bar' (length 11).
        assert_eq!(
            longest_common_prefix(&s(&["xyz_foo_bar", "xyz_foo_bar_baz"])),
            11,
        );
    }

    #[test]
    fn lcp_one_empty_string_collapses_to_zero() {
        // An empty string contributes no characters; LCP is 0.
        assert_eq!(longest_common_prefix(&s(&["echo", "", "exit"])), 0);
    }

    #[test]
    fn lcp_byte_count_matches_slicing() {
        // The returned usize must be a valid byte index into matches[0]
        // (this is how the editor uses it: &matches[0][prefix.len()..lcp]).
        let m = s(&["xyz_foo", "xyz_foo_bar"]);
        let lcp = longest_common_prefix(&m);
        assert_eq!(&m[0][..lcp], "xyz_foo");
    }

    // --- complete_filename --------------------------------------------------

    #[test]
    fn complete_filename_empty_dir_returns_empty() {
        let dir = TempDir::new();
        assert_eq!(
            complete_filename("re", dir.path()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn complete_filename_single_match_codecrafters() {
        // The first codecrafters layer-9 scenario: 'cat re<TAB>' -> readme.txt.
        let dir = TempDir::new();
        dir.touch_plain("readme.txt");
        assert_eq!(
            complete_filename("re", dir.path()),
            vec!["readme.txt"]
        );
    }

    #[test]
    fn complete_filename_prefix_filters_correctly() {
        let dir = TempDir::new();
        dir.touch_plain("readme.txt");
        dir.touch_plain("hello.py");
        assert_eq!(
            complete_filename("re", dir.path()),
            vec!["readme.txt"]
        );
    }

    #[test]
    fn complete_filename_second_codecrafters_scenario() {
        let dir = TempDir::new();
        dir.touch_plain("readme.txt");
        dir.touch_plain("hello_world.py");
        assert_eq!(
            complete_filename("hello", dir.path()),
            vec!["hello_world.py"]
        );
    }

    #[test]
    fn complete_filename_empty_prefix_lists_all_sorted() {
        let dir = TempDir::new();
        dir.touch_plain("zeta");
        dir.touch_plain("alpha");
        dir.touch_plain("mu");
        assert_eq!(
            complete_filename("", dir.path()),
            vec!["alpha", "mu", "zeta"]
        );
    }

    #[test]
    fn complete_filename_multi_match_sorted() {
        let dir = TempDir::new();
        dir.touch_plain("readme.txt");
        dir.touch_plain("readme.md");
        assert_eq!(
            complete_filename("re", dir.path()),
            vec!["readme.md", "readme.txt"]
        );
    }

    #[test]
    fn complete_filename_includes_directories() {
        // Directories are candidates too — directory completion's `/` marker
        // is a deferred future stage; this layer treats all entries equally.
        let dir = TempDir::new();
        dir.mkdir("results");
        assert_eq!(
            complete_filename("re", dir.path()),
            vec!["results"]
        );
    }

    #[test]
    fn complete_filename_no_exec_bit_filter() {
        // Unlike `complete_command`, filename completion does NOT require the
        // executable bit. A non-executable file is still a valid candidate.
        let dir = TempDir::new();
        dir.touch_plain("readme.txt"); // mode 0o644, no exec
        assert_eq!(
            complete_filename("re", dir.path()),
            vec!["readme.txt"]
        );
    }

    #[test]
    fn complete_filename_no_match_returns_empty() {
        let dir = TempDir::new();
        dir.touch_plain("readme.txt");
        assert_eq!(
            complete_filename("xyz", dir.path()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn complete_filename_nonexistent_cwd_returns_empty() {
        // Bad cwd is silent — no panic, just empty Vec.
        let bogus = Path::new("/nonexistent_dir_for_completion_test");
        assert_eq!(
            complete_filename("re", bogus),
            Vec::<String>::new()
        );
    }

    // --- complete_filename: nested-path scenarios (layer 10) ----------------
    // `complete_filename` itself didn't change for layer 10 — the editor
    // resolves `cwd.join(dir_part)` and passes the resulting directory in.
    // These tests pin the behaviour at the unit level explicitly.

    #[test]
    fn complete_filename_works_when_dir_is_a_subdirectory() {
        // Mirrors the codecrafters layer-10 scenario: caller passes a
        // resolved nested directory (cwd.join("path/to/")), and the function
        // lists entries inside it as plain filenames.
        let root = TempDir::new();
        fs::create_dir_all(root.path().join("path/to")).unwrap();
        let nested = root.path().join("path/to");
        let file = nested.join("file.txt");
        fs::write(&file, b"").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(complete_filename("f", &nested), vec!["file.txt"]);
    }

    #[test]
    fn complete_filename_only_lists_immediate_entries_not_recursive() {
        // Even when the search dir is nested, the listing is one level only —
        // entries inside nested subdirs are NOT included. (Important: the
        // editor's split is what walks the path; complete_filename itself
        // is non-recursive.)
        let root = TempDir::new();
        fs::create_dir_all(root.path().join("inner")).unwrap();
        let inner = root.path().join("inner");
        fs::write(inner.join("file.txt"), b"").unwrap();
        fs::set_permissions(&inner.join("file.txt"), fs::Permissions::from_mode(0o644)).unwrap();

        // Searching the root with prefix "f" must NOT find inner/file.txt.
        assert!(
            complete_filename("f", root.path()).is_empty(),
            "complete_filename should not recurse into subdirectories",
        );
    }

    #[test]
    fn complete_filename_empty_prefix_in_nested_dir_lists_all() {
        // The 'cat path/to/<TAB>' case: prefix_part is "", search dir is the
        // resolved nested directory. Returns everything in it, sorted.
        let root = TempDir::new();
        fs::create_dir_all(root.path().join("path/to")).unwrap();
        let nested = root.path().join("path/to");
        fs::write(nested.join("alpha.txt"), b"").unwrap();
        fs::write(nested.join("beta.txt"), b"").unwrap();
        fs::write(nested.join("gamma.txt"), b"").unwrap();
        fs::set_permissions(&nested.join("alpha.txt"), fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&nested.join("beta.txt"), fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&nested.join("gamma.txt"), fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            complete_filename("", &nested),
            vec!["alpha.txt", "beta.txt", "gamma.txt"],
        );
    }
}
