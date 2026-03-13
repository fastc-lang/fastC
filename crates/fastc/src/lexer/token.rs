//! Token definitions for FastC

use logos::Logos;
use std::fmt;

/// Source span (byte offsets)
pub type Span = std::ops::Range<usize>;

/// A token with its span
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// All tokens in the FastC language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    // Comments (captured for formatting)
    #[regex(r"//[^\n]*", |lex| lex.slice().to_string())]
    LineComment(String),

    #[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", |lex| lex.slice().to_string())]
    BlockComment(String),

    // Keywords
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("const")]
    Const,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("switch")]
    Switch,
    #[token("case")]
    Case,
    #[token("default")]
    Default,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("defer")]
    Defer,
    #[token("unsafe")]
    Unsafe,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("opaque")]
    Opaque,
    #[token("extern")]
    Extern,
    #[token("use")]
    Use,
    #[token("mod")]
    Mod,
    #[token("pub")]
    Pub,

    // Type keywords
    #[token("void")]
    Void,
    #[token("bool")]
    Bool,
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,
    #[token("usize")]
    Usize,
    #[token("isize")]
    Isize,

    // Type constructors
    #[token("ref")]
    Ref,
    #[token("mref")]
    Mref,
    #[token("raw")]
    Raw,
    #[token("rawm")]
    Rawm,
    #[token("own")]
    Own,
    #[token("slice")]
    Slice,
    #[token("arr")]
    Arr,
    #[token("opt")]
    Opt,
    #[token("res")]
    Res,

    // Builtin functions
    #[token("addr")]
    Addr,
    #[token("deref")]
    Deref,
    #[token("at")]
    At,
    #[token("cast")]
    Cast,
    #[token("cstr")]
    Cstr,
    #[token("bytes")]
    Bytes,
    #[token("discard")]
    Discard,
    #[token("none")]
    None,
    #[token("some")]
    Some,
    #[token("ok")]
    Ok_,
    #[token("err")]
    Err_,
    #[token("is_some")]
    IsSome,
    #[token("is_none")]
    IsNone,
    #[token("is_ok")]
    IsOk,
    #[token("is_err")]
    IsErr,
    #[token("unwrap")]
    Unwrap,
    #[token("unwrap_or")]
    UnwrapOr,
    #[token("unwrap_err")]
    UnwrapErr,
    #[token("unwrap_checked")]
    UnwrapChecked,

    // Conversion helpers
    #[token("to_raw")]
    ToRaw,
    #[token("to_rawm")]
    ToRawm,
    #[token("from_raw")]
    FromRaw,
    #[token("from_rawm")]
    FromRawm,
    #[token("from_raw_unchecked")]
    FromRawUnchecked,
    #[token("from_rawm_unchecked")]
    FromRawmUnchecked,

    // Boolean literals
    #[token("true")]
    True,
    #[token("false")]
    False,

    // Literals
    #[regex(r"[0-9][0-9_]*", |lex| lex.slice().replace('_', "").parse::<i128>().ok())]
    IntLit(i128),

    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*", parse_hex)]
    HexLit(i128),

    #[regex(r"0b[01][01_]*", parse_binary)]
    BinLit(i128),

    #[regex(r"0o[0-7][0-7_]*", parse_octal)]
    OctLit(i128),

    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?", |lex| lex.slice().replace('_', "").parse::<f64>().ok())]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9]+", |lex| lex.slice().replace('_', "").parse::<f64>().ok())]
    FloatLit(f64),

    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    StringLit(String),

    // Identifier
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Attribute
    #[token("@repr")]
    AtRepr,

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,

    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Not,

    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,

    #[token("=")]
    Eq,

    // Punctuation
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,
    #[token("::")]
    ColonColon,

    // Special
    Eof,
}

fn parse_hex(lex: &mut logos::Lexer<Token>) -> Option<i128> {
    let s = lex.slice();
    let s = s.strip_prefix("0x").unwrap_or(s);
    i128::from_str_radix(&s.replace('_', ""), 16).ok()
}

fn parse_binary(lex: &mut logos::Lexer<Token>) -> Option<i128> {
    let s = lex.slice();
    let s = s.strip_prefix("0b").unwrap_or(s);
    i128::from_str_radix(&s.replace('_', ""), 2).ok()
}

fn parse_octal(lex: &mut logos::Lexer<Token>) -> Option<i128> {
    let s = lex.slice();
    let s = s.strip_prefix("0o").unwrap_or(s);
    i128::from_str_radix(&s.replace('_', ""), 8).ok()
}

fn parse_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let s = lex.slice();
    // Remove quotes
    let s = &s[1..s.len() - 1];
    // Process escape sequences
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('0') => result.push('\0'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    Some(result)
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::LineComment(s) => write!(f, "{}", s),
            Token::BlockComment(s) => write!(f, "{}", s),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Return => write!(f, "return"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Switch => write!(f, "switch"),
            Token::Case => write!(f, "case"),
            Token::Default => write!(f, "default"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Defer => write!(f, "defer"),
            Token::Unsafe => write!(f, "unsafe"),
            Token::Struct => write!(f, "struct"),
            Token::Enum => write!(f, "enum"),
            Token::Opaque => write!(f, "opaque"),
            Token::Extern => write!(f, "extern"),
            Token::Use => write!(f, "use"),
            Token::Mod => write!(f, "mod"),
            Token::Pub => write!(f, "pub"),
            Token::Void => write!(f, "void"),
            Token::Bool => write!(f, "bool"),
            Token::I8 => write!(f, "i8"),
            Token::I16 => write!(f, "i16"),
            Token::I32 => write!(f, "i32"),
            Token::I64 => write!(f, "i64"),
            Token::U8 => write!(f, "u8"),
            Token::U16 => write!(f, "u16"),
            Token::U32 => write!(f, "u32"),
            Token::U64 => write!(f, "u64"),
            Token::F32 => write!(f, "f32"),
            Token::F64 => write!(f, "f64"),
            Token::Usize => write!(f, "usize"),
            Token::Isize => write!(f, "isize"),
            Token::Ref => write!(f, "ref"),
            Token::Mref => write!(f, "mref"),
            Token::Raw => write!(f, "raw"),
            Token::Rawm => write!(f, "rawm"),
            Token::Own => write!(f, "own"),
            Token::Slice => write!(f, "slice"),
            Token::Arr => write!(f, "arr"),
            Token::Opt => write!(f, "opt"),
            Token::Res => write!(f, "res"),
            Token::Addr => write!(f, "addr"),
            Token::Deref => write!(f, "deref"),
            Token::At => write!(f, "at"),
            Token::Cast => write!(f, "cast"),
            Token::Cstr => write!(f, "cstr"),
            Token::Bytes => write!(f, "bytes"),
            Token::Discard => write!(f, "discard"),
            Token::None => write!(f, "none"),
            Token::Some => write!(f, "some"),
            Token::Ok_ => write!(f, "ok"),
            Token::Err_ => write!(f, "err"),
            Token::IsSome => write!(f, "is_some"),
            Token::IsNone => write!(f, "is_none"),
            Token::IsOk => write!(f, "is_ok"),
            Token::IsErr => write!(f, "is_err"),
            Token::Unwrap => write!(f, "unwrap"),
            Token::UnwrapOr => write!(f, "unwrap_or"),
            Token::UnwrapErr => write!(f, "unwrap_err"),
            Token::UnwrapChecked => write!(f, "unwrap_checked"),
            Token::ToRaw => write!(f, "to_raw"),
            Token::ToRawm => write!(f, "to_rawm"),
            Token::FromRaw => write!(f, "from_raw"),
            Token::FromRawm => write!(f, "from_rawm"),
            Token::FromRawUnchecked => write!(f, "from_raw_unchecked"),
            Token::FromRawmUnchecked => write!(f, "from_rawm_unchecked"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::IntLit(n) => write!(f, "{}", n),
            Token::HexLit(n) => write!(f, "0x{:x}", n),
            Token::BinLit(n) => write!(f, "0b{:b}", n),
            Token::OctLit(n) => write!(f, "0o{:o}", n),
            Token::FloatLit(n) => write!(f, "{}", n),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::Ident(s) => write!(f, "{}", s),
            Token::AtRepr => write!(f, "@repr"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::LtEq => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::GtEq => write!(f, ">="),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::And => write!(f, "&"),
            Token::Or => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::Shl => write!(f, "<<"),
            Token::Shr => write!(f, ">>"),
            Token::Eq => write!(f, "="),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semi => write!(f, ";"),
            Token::Dot => write!(f, "."),
            Token::Arrow => write!(f, "->"),
            Token::ColonColon => write!(f, "::"),
            Token::Eof => write!(f, "<EOF>"),
        }
    }
}

impl Token {
    /// Check if this token is a binary operator
    pub fn is_binary_op(&self) -> bool {
        matches!(
            self,
            Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::EqEq
                | Token::NotEq
                | Token::Lt
                | Token::LtEq
                | Token::Gt
                | Token::GtEq
                | Token::AndAnd
                | Token::OrOr
                | Token::And
                | Token::Or
                | Token::Caret
                | Token::Shl
                | Token::Shr
        )
    }

    /// Check if this token is a unary operator
    pub fn is_unary_op(&self) -> bool {
        matches!(self, Token::Not | Token::Minus | Token::Tilde)
    }
}
