pub fn tokenize(input: &str) -> (Vec<String>, bool) {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut has_token = false;

    for c in input.chars() {
        match c {
            '\'' => {
                in_single = !in_single;
                has_token = true;
            }
            c if c.is_whitespace() && !in_single => {
                if has_token {
                    tokens.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            _ => {
                current.push(c);
                has_token = true;
            }
        }
    }

    if has_token {
        tokens.push(current);
    }

    (tokens, in_single)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper for tests that only care about tokens, not the unterminated flag.
    fn tok(input: &str) -> Vec<String> {
        tokenize(input).0
    }

    /// Helper for tests that only care about whether quoting was left open.
    fn open(input: &str) -> bool {
        tokenize(input).1
    }

    // --- empty / whitespace-only inputs ---

    #[test]
    fn empty_input() {
        assert_eq!(tok(""), Vec::<String>::new());
        assert!(!open(""));
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(tok("   "), Vec::<String>::new());
        assert!(!open("   "));
    }

    #[test]
    fn tabs_and_newlines_only() {
        assert_eq!(tok("\t\n  \t"), Vec::<String>::new());
        assert!(!open("\t\n  \t"));
    }

    // --- basic splitting ---

    #[test]
    fn single_word() {
        assert_eq!(tok("echo"), vec!["echo"]);
    }

    #[test]
    fn two_words() {
        assert_eq!(tok("echo hello"), vec!["echo", "hello"]);
    }

    #[test]
    fn many_words() {
        assert_eq!(tok("a b c d e"), vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn collapses_multiple_spaces() {
        assert_eq!(tok("a    b"), vec!["a", "b"]);
    }

    #[test]
    fn mixed_whitespace_separators() {
        assert_eq!(tok("a\tb\nc  d"), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn leading_whitespace_ignored() {
        assert_eq!(tok("   echo hi"), vec!["echo", "hi"]);
    }

    #[test]
    fn trailing_whitespace_ignored() {
        assert_eq!(tok("echo hi   "), vec!["echo", "hi"]);
    }

    #[test]
    fn surrounding_whitespace_ignored() {
        assert_eq!(tok("   echo hi   "), vec!["echo", "hi"]);
    }

    // --- single quotes: balanced ---

    #[test]
    fn quotes_preserve_internal_spaces() {
        assert_eq!(tok("echo 'hello world'"), vec!["echo", "hello world"]);
    }

    #[test]
    fn quotes_preserve_runs_of_internal_spaces() {
        assert_eq!(tok("echo 'hello   world'"), vec!["echo", "hello   world"]);
    }

    #[test]
    fn quotes_strip_the_quote_characters() {
        assert_eq!(tok("'abc'"), vec!["abc"]);
    }

    #[test]
    fn empty_quotes_produce_empty_token() {
        assert_eq!(tok("echo ''"), vec!["echo", ""]);
    }

    #[test]
    fn empty_quotes_alone() {
        assert_eq!(tok("''"), vec![""]);
    }

    #[test]
    fn multiple_empty_quoted_tokens() {
        assert_eq!(tok("'' '' ''"), vec!["", "", ""]);
    }

    #[test]
    fn quotes_preserve_tabs() {
        assert_eq!(tok("'a\tb'"), vec!["a\tb"]);
    }

    #[test]
    fn quotes_preserve_newlines() {
        assert_eq!(tok("'a\nb'"), vec!["a\nb"]);
    }

    #[test]
    fn newline_inside_quote_across_logical_input() {
        // simulates the buffer after a continuation read
        let (tokens, unterminated) = tokenize("echo 'hello\nworld'");
        assert_eq!(tokens, vec!["echo", "hello\nworld"]);
        assert!(!unterminated);
    }

    #[test]
    fn adjacent_quoted_segments_form_one_token() {
        // 'foo''bar' should be a single token "foobar" - quotes are mode toggles
        assert_eq!(tok("'foo''bar'"), vec!["foobar"]);
    }

    #[test]
    fn quoted_and_unquoted_concatenate() {
        // foo'bar baz'qux -> "foobar bazqux"
        assert_eq!(tok("foo'bar baz'qux"), vec!["foobar bazqux"]);
    }

    #[test]
    fn quote_at_start_of_token() {
        assert_eq!(tok("'hello'world"), vec!["helloworld"]);
    }

    #[test]
    fn quote_at_end_of_token() {
        assert_eq!(tok("hello'world'"), vec!["helloworld"]);
    }

    // --- mixed scenarios ---

    #[test]
    fn quoted_and_unquoted_args_mixed() {
        assert_eq!(
            tok("echo 'hello world' foo 'bar baz'"),
            vec!["echo", "hello world", "foo", "bar baz"]
        );
    }

    #[test]
    fn unquoted_then_empty_quote() {
        assert_eq!(tok("echo a ''"), vec!["echo", "a", ""]);
    }

    // --- unicode and special characters ---

    #[test]
    fn unicode_content() {
        assert_eq!(tok("echo café"), vec!["echo", "café"]);
    }

    #[test]
    fn unicode_inside_quotes() {
        assert_eq!(tok("'caf\u{00e9} ☕'"), vec!["café ☕"]);
    }

    #[test]
    fn special_chars_outside_quotes() {
        assert_eq!(
            tok("echo $HOME /tmp/foo-bar.txt"),
            vec!["echo", "$HOME", "/tmp/foo-bar.txt"]
        );
    }

    // --- unterminated-quote flag ---

    #[test]
    fn balanced_quotes_report_closed() {
        assert!(!open("echo 'hello'"));
    }

    #[test]
    fn no_quotes_report_closed() {
        assert!(!open("echo hello"));
    }

    #[test]
    fn opening_quote_reports_open() {
        let (tokens, unterminated) = tokenize("echo 'hello");
        assert_eq!(tokens, vec!["echo", "hello"]);
        assert!(unterminated);
    }

    #[test]
    fn unterminated_quote_preserves_internal_whitespace() {
        let (tokens, unterminated) = tokenize("echo 'a b c");
        assert_eq!(tokens, vec!["echo", "a b c"]);
        assert!(unterminated);
    }

    #[test]
    fn lone_opening_quote_reports_open() {
        let (tokens, unterminated) = tokenize("'");
        assert_eq!(tokens, vec![""]);
        assert!(unterminated);
    }

    #[test]
    fn three_quotes_report_open() {
        // odd number of quotes => unbalanced
        let (tokens, unterminated) = tokenize("'a'b'c");
        assert_eq!(tokens, vec!["abc"]);
        assert!(unterminated);
    }

    #[test]
    fn four_quotes_report_closed() {
        // even number of quotes => balanced
        assert!(!open("'a'b'c'"));
    }
}
