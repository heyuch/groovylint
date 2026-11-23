//! Tokens and token kinds.

use groovylint_core::Span;

/// A lexical token kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    /// Identifier or keyword-like name.
    Ident,
    /// Numeric literal.
    Number,
    /// String literal, including `GString` source.
    String,

    // Keywords.
    Package,
    Import,
    As,
    Class,
    Interface,
    Trait,
    Enum,
    Record,
    Def,
    Var,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Throw,
    Try,
    Catch,
    Finally,
    Break,
    Continue,
    New,
    This,
    Super,
    True,
    False,
    Null,
    Void,
    Assert,
    Switch,
    Case,
    Default,
    Do,
    Static,
    Final,
    Abstract,
    Public,
    Private,
    Protected,
    Synchronized,
    Transient,
    Volatile,
    Throws,
    Instanceof,
    Extends,
    Implements,
    Permits,

    // Punctuation.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    SafeDot,
    Colon,
    Semicolon,
    Newline,
    Arrow,
    At,
    Question,
    Elvis,

    // Operators.
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Power,
    Inc,
    Dec,
    Not,
    AndAnd,
    OrOr,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Range,
    RangeExclusive,
    ShiftLeft,
    ShiftRight,

    /// End of input.
    Eof,
}

impl TokenKind {
    /// Returns a short description for parser errors.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Ident => "identifier",
            Self::Number => "number",
            Self::String => "string",
            Self::Package => "`package`",
            Self::Import => "`import`",
            Self::As => "`as`",
            Self::Class => "`class`",
            Self::Interface => "`interface`",
            Self::Trait => "`trait`",
            Self::Enum => "`enum`",
            Self::Record => "`record`",
            Self::Def => "`def`",
            Self::Var => "`var`",
            Self::If => "`if`",
            Self::Else => "`else`",
            Self::While => "`while`",
            Self::For => "`for`",
            Self::In => "`in`",
            Self::Return => "`return`",
            Self::Throw => "`throw`",
            Self::Try => "`try`",
            Self::Catch => "`catch`",
            Self::Finally => "`finally`",
            Self::Break => "`break`",
            Self::Continue => "`continue`",
            Self::New => "`new`",
            Self::This => "`this`",
            Self::Super => "`super`",
            Self::True => "`true`",
            Self::False => "`false`",
            Self::Null => "`null`",
            Self::Void => "`void`",
            Self::Assert => "`assert`",
            Self::Switch => "`switch`",
            Self::Case => "`case`",
            Self::Default => "`default`",
            Self::Do => "`do`",
            Self::Static => "`static`",
            Self::Final => "`final`",
            Self::Abstract => "`abstract`",
            Self::Public => "`public`",
            Self::Private => "`private`",
            Self::Protected => "`protected`",
            Self::Synchronized => "`synchronized`",
            Self::Transient => "`transient`",
            Self::Volatile => "`volatile`",
            Self::Throws => "`throws`",
            Self::Instanceof => "`instanceof`",
            Self::Extends => "`extends`",
            Self::Implements => "`implements`",
            Self::Permits => "`permits`",
            Self::LeftParen => "`(`",
            Self::RightParen => "`)`",
            Self::LeftBrace => "`{`",
            Self::RightBrace => "`}`",
            Self::LeftBracket => "`[`",
            Self::RightBracket => "`]`",
            Self::Comma => "`,`",
            Self::Dot => "`.`",
            Self::SafeDot => "`?.`",
            Self::Colon => "`:`",
            Self::Semicolon => "`;`",
            Self::Newline => "newline",
            Self::Arrow => "`->`",
            Self::At => "`@`",
            Self::Question => "`?`",
            Self::Elvis => "`?:`",
            Self::Assign => "`=`",
            Self::PlusAssign => "`+=`",
            Self::MinusAssign => "`-=`",
            Self::StarAssign => "`*=`",
            Self::SlashAssign => "`/=`",
            Self::PercentAssign => "`%=`",
            Self::Eq => "`==`",
            Self::NotEq => "`!=`",
            Self::Lt => "`<`",
            Self::Le => "`<=`",
            Self::Gt => "`>`",
            Self::Ge => "`>=`",
            Self::Plus => "`+`",
            Self::Minus => "`-`",
            Self::Star => "`*`",
            Self::Slash => "`/`",
            Self::Percent => "`%`",
            Self::Power => "`**`",
            Self::Inc => "`++`",
            Self::Dec => "`--`",
            Self::Not => "`!`",
            Self::AndAnd => "`&&`",
            Self::OrOr => "`||`",
            Self::Amp => "`&`",
            Self::Pipe => "`|`",
            Self::Caret => "`^`",
            Self::Tilde => "`~`",
            Self::Range => "`..`",
            Self::RangeExclusive => "`..<`",
            Self::ShiftLeft => "`<<`",
            Self::ShiftRight => "`>>`",
            Self::Eof => "end of file",
        }
    }

    /// Returns whether this token can terminate a statement.
    #[must_use]
    pub const fn is_statement_end(self) -> bool {
        matches!(
            self,
            Self::Newline | Self::Semicolon | Self::RightBrace | Self::Eof
        )
    }
}

/// A token with its source span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Source span.
    pub span: Span,
}

impl Token {
    /// Creates a token.
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Returns the token text from the source.
    #[must_use]
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        source.get(self.span.range()).unwrap_or_default()
    }
}
