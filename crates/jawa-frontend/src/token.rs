//! Tokens produced by the lexer.

use crate::source::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(u64),
    Float(f64),
    Str(String),

    // Keywords.
    /// `gawe`: declares a function ("make, do").
    Gawe,
    /// `ayo`: declares a mutable variable ("let's").
    Ayo,
    /// `tetep`: declares an immutable variable ("stays the same").
    Tetep,
    /// `yen`: if.
    Yen,
    /// `liyane`: else ("the other").
    Liyane,
    /// `sajrone`: while ("during").
    Sajrone,
    /// `kanggo`: for.
    Kanggo,
    /// `ing`: in.
    Ing,
    /// `mandheg`: break ("stop").
    Mandheg,
    /// `terusna`: continue ("carry on").
    Terusna,
    /// `bali`: return ("go back").
    Bali,
    /// `bener`: true.
    Bener,
    /// `salah`: false.
    Salah,
    /// `lan`: logical and.
    Lan,
    /// `utawa`: logical or.
    Utawa,
    /// `ora`: logical not.
    Ora,
    /// `wujud`: declares a struct ("shape, form").
    Wujud,
    /// `kosong`: the null reference ("empty").
    Kosong,

    // Type keywords.
    /// `wilangan`: 64-bit integer ("number").
    Wilangan,
    /// `pecahan`: 64-bit float ("fraction").
    Pecahan,
    /// `logika`: boolean.
    Logika,
    /// `ukara`: string ("sentence").
    Ukara,
    /// `suwung`: the empty return type ("void").
    Suwung,

    // Punctuation.
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    /// A statement terminator inserted at a line break (see the lexer).
    Newline,
    Colon,
    Dot,
    DotDot,
    DotDotEq,
    Arrow,
    Eq,
    EqEq,
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
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,

    Eof,
}

impl TokenKind {
    pub fn keyword(word: &str) -> Option<TokenKind> {
        Some(match word {
            "gawe" => TokenKind::Gawe,
            "ayo" => TokenKind::Ayo,
            "tetep" => TokenKind::Tetep,
            "yen" => TokenKind::Yen,
            "liyane" => TokenKind::Liyane,
            "sajrone" => TokenKind::Sajrone,
            "kanggo" => TokenKind::Kanggo,
            "ing" => TokenKind::Ing,
            "mandheg" => TokenKind::Mandheg,
            "terusna" => TokenKind::Terusna,
            "bali" => TokenKind::Bali,
            "bener" => TokenKind::Bener,
            "salah" => TokenKind::Salah,
            "lan" => TokenKind::Lan,
            "utawa" => TokenKind::Utawa,
            "ora" => TokenKind::Ora,
            "wujud" => TokenKind::Wujud,
            "kosong" => TokenKind::Kosong,
            "wilangan" => TokenKind::Wilangan,
            "pecahan" => TokenKind::Pecahan,
            "logika" => TokenKind::Logika,
            "ukara" => TokenKind::Ukara,
            "suwung" => TokenKind::Suwung,
            _ => return None,
        })
    }

    /// Whether a line break after this token ends the statement.
    pub fn ends_statement(&self) -> bool {
        matches!(
            self,
            TokenKind::Ident(_)
                | TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::Str(_)
                | TokenKind::Bener
                | TokenKind::Salah
                | TokenKind::Kosong
                | TokenKind::Wilangan
                | TokenKind::Pecahan
                | TokenKind::Logika
                | TokenKind::Ukara
                | TokenKind::Suwung
                | TokenKind::Bali
                | TokenKind::Mandheg
                | TokenKind::Terusna
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
        )
    }

    /// Keywords that only ever begin a statement or an item.
    pub fn only_starts_statements(&self) -> bool {
        matches!(
            self,
            TokenKind::Ayo
                | TokenKind::Tetep
                | TokenKind::Yen
                | TokenKind::Liyane
                | TokenKind::Sajrone
                | TokenKind::Kanggo
                | TokenKind::Bali
                | TokenKind::Mandheg
                | TokenKind::Terusna
                | TokenKind::Gawe
                | TokenKind::Wujud
        )
    }

    pub fn is_terminator(&self) -> bool {
        matches!(self, TokenKind::Semi | TokenKind::Newline)
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            TokenKind::Ident(name) => return write!(f, "identifier `{name}`"),
            TokenKind::Int(value) => return write!(f, "number `{value}`"),
            TokenKind::Float(value) => return write!(f, "number `{value}`"),
            TokenKind::Str(_) => "string",
            TokenKind::Gawe => "`gawe`",
            TokenKind::Ayo => "`ayo`",
            TokenKind::Tetep => "`tetep`",
            TokenKind::Yen => "`yen`",
            TokenKind::Liyane => "`liyane`",
            TokenKind::Sajrone => "`sajrone`",
            TokenKind::Kanggo => "`kanggo`",
            TokenKind::Ing => "`ing`",
            TokenKind::Mandheg => "`mandheg`",
            TokenKind::Terusna => "`terusna`",
            TokenKind::Bali => "`bali`",
            TokenKind::Bener => "`bener`",
            TokenKind::Salah => "`salah`",
            TokenKind::Lan => "`lan`",
            TokenKind::Utawa => "`utawa`",
            TokenKind::Ora => "`ora`",
            TokenKind::Wujud => "`wujud`",
            TokenKind::Kosong => "`kosong`",
            TokenKind::Wilangan => "`wilangan`",
            TokenKind::Pecahan => "`pecahan`",
            TokenKind::Logika => "`logika`",
            TokenKind::Ukara => "`ukara`",
            TokenKind::Suwung => "`suwung`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::Comma => "`,`",
            TokenKind::Semi => "`;`",
            TokenKind::Newline => "end of line",
            TokenKind::Colon => "`:`",
            TokenKind::Dot => "`.`",
            TokenKind::DotDot => "`..`",
            TokenKind::DotDotEq => "`..=`",
            TokenKind::Arrow => "`->`",
            TokenKind::Eq => "`=`",
            TokenKind::EqEq => "`==`",
            TokenKind::NotEq => "`!=`",
            TokenKind::Lt => "`<`",
            TokenKind::Le => "`<=`",
            TokenKind::Gt => "`>`",
            TokenKind::Ge => "`>=`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Percent => "`%`",
            TokenKind::PlusEq => "`+=`",
            TokenKind::MinusEq => "`-=`",
            TokenKind::StarEq => "`*=`",
            TokenKind::SlashEq => "`/=`",
            TokenKind::PercentEq => "`%=`",
            TokenKind::Eof => "end of file",
        };
        f.write_str(text)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
