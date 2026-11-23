//! A small, newline-aware lexer for the supported Groovy subset.

use groovylint_core::Span;

use crate::token::{Token, TokenKind};

/// A lexical error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    /// Human-readable error.
    pub message: String,
    /// Error location.
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LexError {}

/// Tokenizes a Groovy source file.
///
/// The lexer preserves newline and semicolon tokens because Groovy statement
/// termination is newline-sensitive.
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        while self.pos < self.bytes.len() {
            let start = self.pos;
            let byte = self.bytes[self.pos];

            match byte {
                b' ' | b'\t' | b'\r' => self.pos += 1,
                b'\n' => {
                    self.pos += 1;
                    self.push(TokenKind::Newline, start, self.pos);
                }
                b'#' if self.pos == 0 && self.peek_byte(1) == Some(b'!') => {
                    self.skip_line_comment();
                }
                b'/' if self.peek_byte(1) == Some(b'/') => self.skip_line_comment(),
                b'/' if self.peek_byte(1) == Some(b'*') => self.skip_block_comment()?,
                b'\'' | b'"' => self.lex_string(byte)?,
                b'0'..=b'9' => self.lex_number(),
                _ if is_ident_start(byte) => self.lex_identifier(),
                _ => self.lex_operator(byte)?,
            }
        }

        self.tokens
            .push(Token::new(TokenKind::Eof, Span::empty(self.source.len())));
        Ok(self.tokens)
    }

    fn peek_byte(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token::new(kind, Span::new(start, end)));
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        self.pos += 2;

        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'*' && self.peek_byte(1) == Some(b'/') {
                self.pos += 2;
                return Ok(());
            }
            if self.bytes[self.pos] == b'\n' {
                let newline_start = self.pos;
                self.pos += 1;
                self.push(TokenKind::Newline, newline_start, self.pos);
            } else {
                self.pos += 1;
            }
        }

        Err(LexError {
            message: "unterminated block comment".to_string(),
            span: Span::new(start, self.source.len()),
        })
    }

    fn lex_string(&mut self, quote: u8) -> Result<(), LexError> {
        let start = self.pos;
        let triple = self.bytes[self.pos..].starts_with(&[quote, quote, quote]);
        let delimiter_len = if triple { 3 } else { 1 };
        self.pos += delimiter_len;

        while self.pos < self.bytes.len() {
            if triple
                && self.pos + 2 < self.bytes.len()
                && self.bytes[self.pos] == quote
                && self.bytes[self.pos + 1] == quote
                && self.bytes[self.pos + 2] == quote
            {
                self.pos += 3;
                self.push(TokenKind::String, start, self.pos);
                return Ok(());
            }

            if !triple && self.bytes[self.pos] == quote {
                self.pos += 1;
                self.push(TokenKind::String, start, self.pos);
                return Ok(());
            }

            match self.bytes[self.pos] {
                b'\\' => {
                    self.pos += 1;
                    if self.pos < self.bytes.len() {
                        self.pos += 1;
                    }
                }
                b'\n' if !triple => break,
                _ => self.pos += 1,
            }
        }

        Err(LexError {
            message: "unterminated string literal".to_string(),
            span: Span::new(start, self.pos.min(self.source.len())),
        })
    }

    fn lex_number(&mut self) {
        let start = self.pos;

        if self.bytes[self.pos] == b'0'
            && matches!(self.peek_byte(1), Some(b'x' | b'X' | b'b' | b'B'))
        {
            self.pos += 2;
            while let Some(byte) = self.bytes.get(self.pos) {
                if byte.is_ascii_alphanumeric() || *byte == b'_' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        } else {
            self.consume_digits();
            if self.bytes.get(self.pos) == Some(&b'.')
                && self.bytes.get(self.pos + 1).is_some_and(u8::is_ascii_digit)
            {
                self.pos += 1;
                self.consume_digits();
            }
            if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
                let exponent_start = self.pos;
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                let digits_start = self.pos;
                self.consume_digits();
                if self.pos == digits_start {
                    self.pos = exponent_start;
                }
            }
            if let Some(byte) = self.bytes.get(self.pos)
                && matches!(
                    byte,
                    b'l' | b'L'
                        | b'i'
                        | b'I'
                        | b'g'
                        | b'G'
                        | b'f'
                        | b'F'
                        | b'd'
                        | b'D'
                        | b'n'
                        | b'N'
                )
            {
                self.pos += 1;
            }
        }

        self.push(TokenKind::Number, start, self.pos);
    }

    fn consume_digits(&mut self) {
        while let Some(byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_digit() || *byte == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn lex_identifier(&mut self) {
        let start = self.pos;
        self.pos += 1;
        while let Some(byte) = self.bytes.get(self.pos) {
            if is_ident_continue(*byte) {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        self.push(keyword_kind(text), start, self.pos);
    }

    fn lex_operator(&mut self, byte: u8) -> Result<(), LexError> {
        let start = self.pos;
        let kind = match byte {
            b'(' => Some(TokenKind::LeftParen),
            b')' => Some(TokenKind::RightParen),
            b'{' => Some(TokenKind::LeftBrace),
            b'}' => Some(TokenKind::RightBrace),
            b'[' => Some(TokenKind::LeftBracket),
            b']' => Some(TokenKind::RightBracket),
            b',' => Some(TokenKind::Comma),
            b':' => Some(TokenKind::Colon),
            b';' => Some(TokenKind::Semicolon),
            b'@' => Some(TokenKind::At),
            b'~' => Some(TokenKind::Tilde),
            b'^' => Some(TokenKind::Caret),
            b'?' => {
                if self.peek_byte(1) == Some(b'.') {
                    self.pos += 2;
                    self.push(TokenKind::SafeDot, start, self.pos);
                    return Ok(());
                }
                if self.peek_byte(1) == Some(b':') {
                    self.pos += 2;
                    self.push(TokenKind::Elvis, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Question)
            }
            b'.' => {
                if self.peek_byte(1) == Some(b'.') {
                    if self.peek_byte(2) == Some(b'<') {
                        self.pos += 3;
                        self.push(TokenKind::RangeExclusive, start, self.pos);
                    } else {
                        self.pos += 2;
                        self.push(TokenKind::Range, start, self.pos);
                    }
                    return Ok(());
                }
                Some(TokenKind::Dot)
            }
            b'=' => {
                if self.peek_byte(1) == Some(b'=') {
                    self.pos += 2;
                    self.push(TokenKind::Eq, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Assign)
            }
            b'!' => {
                if self.peek_byte(1) == Some(b'=') {
                    self.pos += 2;
                    self.push(TokenKind::NotEq, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Not)
            }
            b'<' => match self.peek_byte(1) {
                Some(b'=') => {
                    self.pos += 2;
                    self.push(TokenKind::Le, start, self.pos);
                    return Ok(());
                }
                Some(b'<') => {
                    self.pos += 2;
                    self.push(TokenKind::ShiftLeft, start, self.pos);
                    return Ok(());
                }
                _ => Some(TokenKind::Lt),
            },
            b'>' => match self.peek_byte(1) {
                Some(b'=') => {
                    self.pos += 2;
                    self.push(TokenKind::Ge, start, self.pos);
                    return Ok(());
                }
                Some(b'>') => {
                    self.pos += 2;
                    self.push(TokenKind::ShiftRight, start, self.pos);
                    return Ok(());
                }
                _ => Some(TokenKind::Gt),
            },
            b'+' => match self.peek_byte(1) {
                Some(b'+') => {
                    self.pos += 2;
                    self.push(TokenKind::Inc, start, self.pos);
                    return Ok(());
                }
                Some(b'=') => {
                    self.pos += 2;
                    self.push(TokenKind::PlusAssign, start, self.pos);
                    return Ok(());
                }
                _ => Some(TokenKind::Plus),
            },
            b'-' => match self.peek_byte(1) {
                Some(b'-') => {
                    self.pos += 2;
                    self.push(TokenKind::Dec, start, self.pos);
                    return Ok(());
                }
                Some(b'=') => {
                    self.pos += 2;
                    self.push(TokenKind::MinusAssign, start, self.pos);
                    return Ok(());
                }
                Some(b'>') => {
                    self.pos += 2;
                    self.push(TokenKind::Arrow, start, self.pos);
                    return Ok(());
                }
                _ => Some(TokenKind::Minus),
            },
            b'*' => match self.peek_byte(1) {
                Some(b'*') => {
                    self.pos += 2;
                    self.push(TokenKind::Power, start, self.pos);
                    return Ok(());
                }
                Some(b'=') => {
                    self.pos += 2;
                    self.push(TokenKind::StarAssign, start, self.pos);
                    return Ok(());
                }
                _ => Some(TokenKind::Star),
            },
            b'/' => {
                if self.peek_byte(1) == Some(b'=') {
                    self.pos += 2;
                    self.push(TokenKind::SlashAssign, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Slash)
            }
            b'%' => {
                if self.peek_byte(1) == Some(b'=') {
                    self.pos += 2;
                    self.push(TokenKind::PercentAssign, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Percent)
            }
            b'&' => {
                if self.peek_byte(1) == Some(b'&') {
                    self.pos += 2;
                    self.push(TokenKind::AndAnd, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Amp)
            }
            b'|' => {
                if self.peek_byte(1) == Some(b'|') {
                    self.pos += 2;
                    self.push(TokenKind::OrOr, start, self.pos);
                    return Ok(());
                }
                Some(TokenKind::Pipe)
            }
            _ => None,
        };

        if let Some(kind) = kind {
            self.pos += 1;
            self.push(kind, start, self.pos);
            Ok(())
        } else {
            Err(LexError {
                message: format!("unexpected character `{}`", byte as char),
                span: Span::new(start, start + 1),
            })
        }
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$') || byte >= 0x80
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$') || byte >= 0x80
}

fn keyword_kind(text: &str) -> TokenKind {
    match text {
        "package" => TokenKind::Package,
        "import" => TokenKind::Import,
        "as" => TokenKind::As,
        "class" => TokenKind::Class,
        "interface" => TokenKind::Interface,
        "trait" => TokenKind::Trait,
        "enum" => TokenKind::Enum,
        "record" => TokenKind::Record,
        "def" => TokenKind::Def,
        "var" => TokenKind::Var,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "return" => TokenKind::Return,
        "throw" => TokenKind::Throw,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "finally" => TokenKind::Finally,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "new" => TokenKind::New,
        "this" => TokenKind::This,
        "super" => TokenKind::Super,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "void" => TokenKind::Void,
        "assert" => TokenKind::Assert,
        "switch" => TokenKind::Switch,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        "do" => TokenKind::Do,
        "static" => TokenKind::Static,
        "final" => TokenKind::Final,
        "abstract" => TokenKind::Abstract,
        "public" => TokenKind::Public,
        "private" => TokenKind::Private,
        "protected" => TokenKind::Protected,
        "synchronized" => TokenKind::Synchronized,
        "transient" => TokenKind::Transient,
        "volatile" => TokenKind::Volatile,
        "throws" => TokenKind::Throws,
        "instanceof" => TokenKind::Instanceof,
        _ => TokenKind::Ident,
    }
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::token::TokenKind;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .expect("source should lex")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn emits_newlines_and_semicolons() {
        let kinds = kinds("def x = 1;\nif (x) {}\n");
        assert!(kinds.contains(&TokenKind::Semicolon));
        assert!(kinds.contains(&TokenKind::Newline));
    }

    #[test]
    fn skips_comments_but_preserves_their_newlines() {
        let kinds = kinds("if (x) {\n/* comment */\n}\n");
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == TokenKind::Newline)
                .count(),
            3
        );
    }

    #[test]
    fn recognizes_keywords_and_operators() {
        let kinds = kinds("if (a != null) { a?.b() }");
        assert!(kinds.contains(&TokenKind::If));
        assert!(kinds.contains(&TokenKind::NotEq));
        assert!(kinds.contains(&TokenKind::Null));
        assert!(kinds.contains(&TokenKind::SafeDot));
    }

    #[test]
    fn rejects_unterminated_strings() {
        assert!(lex("def x = \"oops").is_err());
    }
}
