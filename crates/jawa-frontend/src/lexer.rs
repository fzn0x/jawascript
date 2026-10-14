//! The lexer turns source text into tokens.
//!
//! Semicolons are optional. Like Go, the lexer inserts a [`TokenKind::Newline`]
//! terminator at a line break when the last token on the line could end a
//! statement (an identifier, a literal, `bali`, `mandheg`, `terusna`, or a
//! closing `)`, `]` or `}`). Line breaks inside `(…)` and `[…]` never end a
//! statement, so argument lists and array literals can span lines.
//!
//! The lexer also keeps one mistake from spoiling the rest of the file: a
//! line that starts with a statement keyword such as `ayo` closes any `(` or
//! `[` left open above it, and `&&`, `||` and `!` are reported but read as
//! `lan`, `utawa` and `ora`.

use crate::diag::Diagnostic;
use crate::source::Span;
use crate::token::{Token, TokenKind};

pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lex(text: &str) -> Lexed {
    let mut lexer = Lexer {
        text,
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
        delimiters: Vec::new(),
        line_break_before: None,
    };
    lexer.run();
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    /// Open `(`, `[` and `{` tokens, innermost last.
    delimiters: Vec<TokenKind>,
    /// Where the current line began, while no token is on it yet.
    line_break_before: Option<usize>,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        let mut chars = self.text[self.pos..].chars();
        chars.next();
        chars.next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.line_break_before = None;
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn error(&mut self, start: usize, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(Span::new(start, self.pos), message));
    }

    /// Inserts a statement terminator for the line break at `at`, if the
    /// previous token can end a statement.
    fn inside_group(&self) -> bool {
        matches!(
            self.delimiters.last(),
            Some(TokenKind::LParen | TokenKind::LBracket)
        )
    }

    fn line_break(&mut self, at: usize) {
        if self.inside_group() {
            return;
        }
        if self
            .tokens
            .last()
            .is_some_and(|token| token.kind.ends_statement())
        {
            self.tokens.push(Token {
                kind: TokenKind::Newline,
                span: Span::new(at, at),
            });
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                '\n' => {
                    self.bump();
                    self.line_break(start);
                    self.line_break_before.get_or_insert(start);
                }
                c if c.is_whitespace() => {
                    self.bump();
                }
                '/' if self.peek_second() == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                '/' if self.peek_second() == Some('*') => self.block_comment(),
                '"' => self.string(),
                c if c.is_ascii_digit() => self.number(),
                c if c.is_alphabetic() || c == '_' => self.word(),
                _ => self.punctuation(),
            }
        }
        let end = self.text.len();
        self.line_break(end);
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(end, end),
        });
    }

    fn block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        let mut spans_lines = false;
        loop {
            match self.bump() {
                Some('*') if self.eat('/') => break,
                Some('\n') => spans_lines = true,
                Some(_) => {}
                None => {
                    self.error(start, "unterminated block comment");
                    return;
                }
            }
        }
        // A comment that spans lines acts like a line break.
        if spans_lines {
            self.line_break(start);
        }
    }

    fn word(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.bump();
        }
        let word = &self.text[start..self.pos];
        let kind = TokenKind::keyword(word).unwrap_or_else(|| TokenKind::Ident(word.to_string()));
        // `ayo`, `yen` and friends never appear inside parentheses, so a line
        // starting with one means a `(` or `[` above was never closed. Close
        // it here and end the previous line, so the error stays on that line.
        if kind.only_starts_statements() && self.inside_group() {
            if let Some(line_break) = self.line_break_before {
                while self.inside_group() {
                    self.delimiters.pop();
                }
                self.line_break(line_break);
            }
        }
        self.push(kind, start);
    }

    fn digits(&mut self, radix: u32) -> String {
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c == '_' {
                self.bump();
            } else if c.is_digit(radix) {
                digits.push(c);
                self.bump();
            } else {
                break;
            }
        }
        digits
    }

    fn number(&mut self) {
        let start = self.pos;
        if self.peek() == Some('0') && matches!(self.peek_second(), Some('x' | 'X' | 'b' | 'B')) {
            self.bump();
            let radix = if matches!(self.bump(), Some('x' | 'X')) {
                16
            } else {
                2
            };
            let digits = self.digits(radix);
            if digits.is_empty() {
                self.error(start, "expected digits after the number prefix");
                self.push(TokenKind::Int(0), start);
                return;
            }
            match u64::from_str_radix(&digits, radix) {
                Ok(value) => self.push(TokenKind::Int(value), start),
                Err(_) => {
                    self.error(start, "number is too large for `wilangan`");
                    self.push(TokenKind::Int(0), start);
                }
            }
            return;
        }

        let mut text = self.digits(10);
        let mut is_float = false;
        // `1.5` is a float, but `0..10` is a range, so a digit must follow the dot.
        if self.peek() == Some('.') && self.peek_second().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            text.push('.');
            text.push_str(&self.digits(10));
            is_float = true;
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let mark = self.pos;
            self.bump();
            let mut exponent = String::from("e");
            if let Some(sign @ ('+' | '-')) = self.peek() {
                self.bump();
                exponent.push(sign);
            }
            let digits = self.digits(10);
            if digits.is_empty() {
                // Not an exponent after all (for example `2em`); leave the
                // letters for the next token.
                self.pos = mark;
            } else {
                text.push_str(&exponent);
                text.push_str(&digits);
                is_float = true;
            }
        }

        if is_float {
            let value = text.parse::<f64>().unwrap_or(0.0);
            self.push(TokenKind::Float(value), start);
        } else {
            match text.parse::<u64>() {
                Ok(value) => self.push(TokenKind::Int(value), start),
                Err(_) => {
                    self.error(start, "number is too large for `wilangan`");
                    self.push(TokenKind::Int(0), start);
                }
            }
        }
    }

    fn string(&mut self) {
        let start = self.pos;
        self.bump();
        let mut value = String::new();
        loop {
            let escape_start = self.pos;
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('r') => value.push('\r'),
                    Some('0') => value.push('\0'),
                    Some('\\') => value.push('\\'),
                    Some('"') => value.push('"'),
                    Some('\'') => value.push('\''),
                    Some('u') => match self.unicode_escape() {
                        Some(c) => value.push(c),
                        None => {
                            self.error(escape_start, "invalid unicode escape, expected `\\u{…}`")
                        }
                    },
                    Some(other) => {
                        self.error(escape_start, format!("unknown escape `\\{other}`"));
                    }
                    None => {
                        self.error(start, "unterminated string");
                        break;
                    }
                },
                Some('\n') => {
                    // Leave the line break for the main loop so it can still
                    // end the statement.
                    self.pos -= 1;
                    self.error(start, "unterminated string");
                    break;
                }
                None => {
                    self.error(start, "unterminated string");
                    break;
                }
                Some(c) => value.push(c),
            }
        }
        self.push(TokenKind::Str(value), start);
    }

    fn unicode_escape(&mut self) -> Option<char> {
        if !self.eat('{') {
            return None;
        }
        let digits = self.digits(16);
        if !self.eat('}') || digits.is_empty() || digits.len() > 6 {
            return None;
        }
        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
    }

    fn punctuation(&mut self) {
        let start = self.pos;
        let c = self.bump().unwrap_or('\0');
        let kind = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semi,
            ':' => TokenKind::Colon,
            '.' if self.eat('.') => {
                if self.eat('=') {
                    TokenKind::DotDotEq
                } else {
                    TokenKind::DotDot
                }
            }
            '.' => TokenKind::Dot,
            '-' if self.eat('>') => TokenKind::Arrow,
            '-' if self.eat('=') => TokenKind::MinusEq,
            '-' => TokenKind::Minus,
            '+' if self.eat('=') => TokenKind::PlusEq,
            '+' => TokenKind::Plus,
            '*' if self.eat('=') => TokenKind::StarEq,
            '*' => TokenKind::Star,
            '/' if self.eat('=') => TokenKind::SlashEq,
            '/' => TokenKind::Slash,
            '%' if self.eat('=') => TokenKind::PercentEq,
            '%' => TokenKind::Percent,
            '=' if self.eat('=') => TokenKind::EqEq,
            '=' => TokenKind::Eq,
            '!' if self.eat('=') => TokenKind::NotEq,
            '<' if self.eat('=') => TokenKind::Le,
            '<' => TokenKind::Lt,
            '>' if self.eat('=') => TokenKind::Ge,
            '>' => TokenKind::Gt,
            // C-style logic: report it, but read the Javanese word so the rest
            // of the line still parses.
            '!' => {
                self.error(start, "unexpected `!`; logical not is spelled `ora`");
                TokenKind::Ora
            }
            '&' => {
                self.eat('&');
                self.error(start, "unexpected `&`; use `lan` for logic");
                TokenKind::Lan
            }
            '|' => {
                self.eat('|');
                self.error(start, "unexpected `|`; use `utawa` for logic");
                TokenKind::Utawa
            }
            other => {
                self.error(start, format!("unexpected character `{other}`"));
                return;
            }
        };
        match kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                self.delimiters.push(kind.clone());
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.delimiters.pop();
            }
            _ => {}
        }
        self.push(kind, start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn kinds(text: &str) -> Vec<TokenKind> {
        let lexed = lex(text);
        assert!(lexed.diagnostics.is_empty(), "{:?}", lexed.diagnostics);
        lexed.tokens.into_iter().map(|t| t.kind).collect()
    }

    fn ident(name: &str) -> TokenKind {
        Ident(name.to_string())
    }

    #[test]
    fn keywords_and_identifiers() {
        assert_eq!(
            kinds("gawe utama() { ayo jeneng = \"Jawa\" }"),
            vec![
                Gawe,
                ident("utama"),
                LParen,
                RParen,
                LBrace,
                Ayo,
                ident("jeneng"),
                Eq,
                Str("Jawa".into()),
                RBrace,
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn newline_ends_a_statement_after_a_value() {
        assert_eq!(
            kinds("ayo x = 1\nx += 2\n"),
            vec![
                Ayo,
                ident("x"),
                Eq,
                Int(1),
                Newline,
                ident("x"),
                PlusEq,
                Int(2),
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn newline_after_an_operator_continues_the_statement() {
        assert_eq!(
            kinds("ayo x = 1 +\n 2"),
            vec![Ayo, ident("x"), Eq, Int(1), Plus, Int(2), Newline, Eof]
        );
    }

    #[test]
    fn newlines_inside_parentheses_are_ignored() {
        assert_eq!(
            kinds("tulis(\n  1,\n  2\n)\n"),
            vec![
                ident("tulis"),
                LParen,
                Int(1),
                Comma,
                Int(2),
                RParen,
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            kinds("42 1_000 0xff 0b101 1.5 2e3 7.25e-1"),
            vec![
                Int(42),
                Int(1000),
                Int(255),
                Int(5),
                Float(1.5),
                Float(2000.0),
                Float(0.725),
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn range_is_not_a_float() {
        assert_eq!(
            kinds("0..10 1..=3"),
            vec![
                Int(0),
                DotDot,
                Int(10),
                Int(1),
                DotDotEq,
                Int(3),
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn string_escapes() {
        assert_eq!(
            kinds(r#""a\tb\n\"c\" \u{A9}""#),
            vec![Str("a\tb\n\"c\" \u{a9}".into()), Newline, Eof]
        );
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(
            kinds("1 // siji\n/* loro\n */ 2 /* telu */"),
            vec![Int(1), Newline, Int(2), Newline, Eof]
        );
    }

    #[test]
    fn unicode_identifiers() {
        assert_eq!(kinds("ayo éntuk"), vec![Ayo, ident("éntuk"), Newline, Eof]);
    }

    #[test]
    fn reports_unterminated_string() {
        let lexed = lex("tulis(\"halo)\n");
        assert_eq!(lexed.diagnostics.len(), 1);
        assert_eq!(lexed.diagnostics[0].message, "unterminated string");
    }

    #[test]
    fn suggests_javanese_logic_words() {
        let lexed = lex("yen a && b || !c {}");
        let messages: Vec<_> = lexed.diagnostics.iter().map(|d| &d.message).collect();
        assert_eq!(
            messages,
            [
                "unexpected `&`; use `lan` for logic",
                "unexpected `|`; use `utawa` for logic",
                "unexpected `!`; logical not is spelled `ora`",
            ]
        );
        // The rest of the line still reads as Jawa.
        let kinds: Vec<_> = lexed.tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Yen,
                ident("a"),
                Lan,
                ident("b"),
                Utawa,
                Ora,
                ident("c"),
                LBrace,
                RBrace,
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn statement_keyword_closes_a_forgotten_parenthesis() {
        let lexed = lex("tulis(1\nayo x = 2\n");
        let kinds: Vec<_> = lexed.tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ident("tulis"),
                LParen,
                Int(1),
                Newline,
                Ayo,
                ident("x"),
                Eq,
                Int(2),
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn values_on_new_lines_stay_inside_parentheses() {
        assert_eq!(
            kinds("f(\n  ora a,\n  bener\n)"),
            vec![
                ident("f"),
                LParen,
                Ora,
                ident("a"),
                Comma,
                Bener,
                RParen,
                Newline,
                Eof
            ]
        );
    }

    #[test]
    fn reports_overflowing_integer() {
        let lexed = lex("99999999999999999999");
        assert_eq!(
            lexed.diagnostics[0].message,
            "number is too large for `wilangan`"
        );
    }
}
