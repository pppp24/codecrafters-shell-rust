use crate::token::{Token, TokenType};
use std::env;

pub struct Lexer {
    is_unterminated: bool,
    input: Vec<char>,
    position: usize,      // index of current char
    read_position: usize, // index of next char to read
    ch: char,             // current char ('\0' = EOF)
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut l = Lexer {
            is_unterminated: false,
            input: input.chars().collect(),
            position: 0,
            read_position: 0,
            ch: '\0',
        };

        l.read_char();
        l
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let tok = match self.ch {
            '\0' => Token::new(TokenType::Eof, ""),
            '|' => {
                unreachable!()
            }
            '&' => {
                unreachable!()
            }
            '>' => {
                if self.peek_char() == '>' {
                    self.read_char();
                    Token::new(TokenType::GtGt, ">>")
                } else {
                    Token::new(TokenType::Gt, '>')
                }
            }
            '<' => {
                unreachable!()
            }
            ';' => {
                unreachable!()
            }
            _ => {
                if self.ch.is_ascii_digit() {
                    let mut digits = String::new();
                    while self.ch.is_ascii_digit() {
                        digits.push(self.ch);
                        self.read_char();
                    }

                    if self.ch == '>' {
                        return Token::new(TokenType::IoNumber, digits);
                    }

                    let rest = self.read_word();
                    return Token::new(TokenType::Word, format!("{}{}", digits, rest));
                }

                // Anything else starts a Word - possibly with embedded
                // quotes and variable expansion.
                let literal = self.read_word();
                return Token::new(crate::token::TokenType::Word, literal);
                // early return - read_word already advanced past the word
            }
        };

        self.read_char();
        tok
    }

    pub fn is_unterminated(&mut self) -> bool {
        self.is_unterminated
    }

    fn read_char(&mut self) {
        self.ch = if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input[self.read_position]
        };

        self.position = self.read_position;
        self.read_position += 1;
    }

    fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input[self.read_position]
        }
    }

    fn skip_whitespace(&mut self) {
        while self.ch == ' ' || self.ch == '\t' || self.ch == '\n' || self.ch == '\r' {
            self.read_char()
        }
    }

    // Reads a "word": a run of characters that may include unquoted segments,
    // single-quoted segments, double-quoted segments, and $VAR expansions,
    // all concatenated into one token. Stops at unquoted whitespace or operator
    fn read_word(&mut self) -> String {
        let mut buf = String::new();

        loop {
            match self.ch {
                '\0' => break,
                c if is_word_terminator(c) => break,
                '\'' => self.read_single_quoted(&mut buf),
                '"' => self.read_double_quoted(&mut buf),
                '$' => {
                    self.read_char(); // consume $
                    let name = self.read_var_name();
                    if name.is_empty() {
                        buf.push('$');
                    } else {
                        buf.push_str(&env::var(&name).unwrap_or_default())
                    }
                }
                '\\' => {
                    self.read_char();
                    if self.ch != '\0' {
                        buf.push(self.ch);
                        self.read_char();
                    }
                }
                c => {
                    buf.push(c);
                    self.read_char();
                }
            }
        }

        buf
    }

    fn read_single_quoted(&mut self, buf: &mut String) {
        self.read_char(); // consume opening
        while self.ch != '\'' && self.ch != '\0' {
            buf.push(self.ch);
            self.read_char();
        }

        if self.ch == '\0' {
            self.is_unterminated = true
        }

        if self.ch == '\'' {
            self.read_char(); // consume closing
        }
    }

    fn read_double_quoted(&mut self, buf: &mut String) {
        self.read_char(); // consume opening
        while self.ch != '"' && self.ch != '\0' {
            match self.ch {
                '$' => {
                    self.read_char();
                    let name = self.read_var_name();
                    if name.is_empty() {
                        buf.push('$');
                    } else {
                        buf.push_str(&env::var(&name).unwrap_or_default())
                    }
                }
                '\\' => {
                    let next = self.peek_char();
                    if matches!(next, '"' | '\\' | '$' | '`') {
                        self.read_char();
                        buf.push(self.ch);
                        self.read_char();
                    } else {
                        buf.push('\\');
                        self.read_char();
                    }
                }
                c => {
                    buf.push(c);
                    self.read_char();
                }
            }
        }

        if self.ch == '\0' {
            self.is_unterminated = true
        }

        if self.ch == '"' {
            self.read_char()
        }
    }

    fn read_var_name(&mut self) -> String {
        let mut name = String::new();
        while is_var_char(self.ch) {
            name.push(self.ch);
            self.read_char();
        }
        name
    }
}

fn is_word_terminator(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '|' | '&' | ';' | '<' | '>')
}

fn is_var_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn lex(input: &str) -> (Vec<Token>, bool) {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        if tok.r#type == TokenType::Eof {
            tokens.push(tok);
            break;
        }
        tokens.push(tok)
    }

    (tokens, lexer.is_unterminated())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper for tests that only care about tokens, not the unterminated flag.
    /// Filters out the trailing Eof so assertions can compare against plain Vec<String>.
    fn tok(input: &str) -> Vec<String> {
        tok_and_open(input).0
    }

    /// Helper for tests that only care about whether quoting was left open.
    fn open(input: &str) -> bool {
        lex(input).1
    }

    /// Helper for tests that need both: token literals (Eof stripped) and the flag.
    fn tok_and_open(input: &str) -> (Vec<String>, bool) {
        let (tokens, unterminated) = lex(input);
        let literals = tokens
            .into_iter()
            .filter(|t| t.r#type != TokenType::Eof)
            .map(|t| t.literal)
            .collect();
        (literals, unterminated)
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
        let (tokens, unterminated) = tok_and_open("echo 'hello\nworld'");
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
        // Adapted from the original tokenizer suite: the old test used `$HOME`
        // unquoted, but the new lexer expands `$VAR` at lex time. Single-quoting
        // suppresses expansion and preserves the spirit of the test (special
        // characters in mixed unquoted/quoted args survive intact).
        assert_eq!(
            tok("echo '$HOME' /tmp/foo-bar.txt"),
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
        let (tokens, unterminated) = tok_and_open("echo 'hello");
        assert_eq!(tokens, vec!["echo", "hello"]);
        assert!(unterminated);
    }

    #[test]
    fn unterminated_quote_preserves_internal_whitespace() {
        let (tokens, unterminated) = tok_and_open("echo 'a b c");
        assert_eq!(tokens, vec!["echo", "a b c"]);
        assert!(unterminated);
    }

    #[test]
    fn lone_opening_quote_reports_open() {
        let (tokens, unterminated) = tok_and_open("'");
        assert_eq!(tokens, vec![""]);
        assert!(unterminated);
    }

    #[test]
    fn three_quotes_report_open() {
        // odd number of quotes => unbalanced
        let (tokens, unterminated) = tok_and_open("'a'b'c");
        assert_eq!(tokens, vec!["abc"]);
        assert!(unterminated);
    }

    #[test]
    fn four_quotes_report_closed() {
        // even number of quotes => balanced
        assert!(!open("'a'b'c'"));
    }

    // --- redirection operators (Layer 4) ---

    /// Helper for tests that need token kinds, not just literals.
    /// Returns (type, literal) pairs with the trailing Eof stripped.
    fn typed_tok(input: &str) -> Vec<(TokenType, String)> {
        lex(input)
            .0
            .into_iter()
            .filter(|t| t.r#type != TokenType::Eof)
            .map(|t| (t.r#type, t.literal))
            .collect()
    }

    #[test]
    fn redirect_out_emits_gt() {
        assert_eq!(
            typed_tok("echo > foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn redirect_out_glued_to_words() {
        // `>` is a word terminator, so no whitespace is required around it
        assert_eq!(
            typed_tok("echo>foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn bare_redirect_operator() {
        assert_eq!(typed_tok(">"), vec![(TokenType::Gt, ">".into())]);
    }

    #[test]
    fn double_redirect_operator_lexes_normally() {
        // `echo > > foo` is a syntax error, but the lexer emits tokens normally;
        // detecting the error is the parser's job.
        assert_eq!(
            typed_tok("echo > > foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn io_number_with_space_after_operator() {
        assert_eq!(
            typed_tok("echo 1> foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::IoNumber, "1".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn io_number_glued_to_operator_and_target() {
        assert_eq!(
            typed_tok("echo 1>foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::IoNumber, "1".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn io_number_multi_digit() {
        // POSIX places no upper bound on the fd in an IO_NUMBER
        assert_eq!(
            typed_tok("echo 99>foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::IoNumber, "99".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn digit_separated_from_operator_by_space_is_a_word() {
        // whitespace between the digits and `>` means the digits are an
        // ordinary argument, not an fd prefix
        assert_eq!(
            typed_tok("echo 1 > foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::Word, "1".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn digit_run_not_before_operator_is_a_word() {
        // digits followed by non-redirect characters fold into a normal word
        assert_eq!(typed_tok("12foo"), vec![(TokenType::Word, "12foo".into())]);
    }

    #[test]
    fn digit_run_at_eof_is_a_word() {
        // digits with nothing after them: the read_word fallthrough yields ""
        assert_eq!(typed_tok("12"), vec![(TokenType::Word, "12".into())]);
    }

    // --- append operator (>>) ---

    #[test]
    fn append_out_emits_gtgt() {
        assert_eq!(
            typed_tok("echo >> foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::GtGt, ">>".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn append_out_glued_to_words() {
        // `>` terminates a word, so no whitespace is required around `>>`
        assert_eq!(
            typed_tok("echo>>foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::GtGt, ">>".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn bare_append_operator() {
        assert_eq!(typed_tok(">>"), vec![(TokenType::GtGt, ">>".into())]);
    }

    #[test]
    fn io_number_before_append() {
        assert_eq!(
            typed_tok("echo 1>>foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::IoNumber, "1".into()),
                (TokenType::GtGt, ">>".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn io_number_before_append_with_space() {
        assert_eq!(
            typed_tok("echo 2>> foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::IoNumber, "2".into()),
                (TokenType::GtGt, ">>".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn spaced_redirects_are_two_gt_not_gtgt() {
        // `> >` (whitespace between) must stay two separate `Gt` tokens —
        // only adjacent `>>` pairs into a single `GtGt`. This is what lets
        // the parser reject `> >` as a syntax error while accepting `>>`.
        assert_eq!(
            typed_tok("echo > > foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }

    #[test]
    fn triple_redirect_pairs_greedily() {
        // `>>>` pairs the first two into `GtGt`; the third is a lone `Gt`
        assert_eq!(
            typed_tok("echo >>> foo"),
            vec![
                (TokenType::Word, "echo".into()),
                (TokenType::GtGt, ">>".into()),
                (TokenType::Gt, ">".into()),
                (TokenType::Word, "foo".into()),
            ]
        );
    }
}
