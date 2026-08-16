
use ::core::{
    ops::Range,
    num::NonZeroUsize,
};



pub fn parse<
    'a,
    Dest: 'a,
    Src: ?Sized + 'a,
    E,
    F: FnMut(&mut Dest, &'a Src) -> Result<bool, E>,
>(
    output: &mut Dest,
    source: &'a Src,
    parse: F,
) -> Result<(), E> {
    let mut parse = parse;
    while let true = parse(output, source)? {}
    Ok(())
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BracketStyle {
    Parens = 0, // ()
    Brace = 1,  // {}
    Square = 2, // []
    Angle = 3,  // <>
}

impl BracketStyle {
    const LHS: &'static str = "({[<";
    const RHS: &'static str = ")}]>";

    #[inline(always)]
    pub fn open_str(self) -> &'static str {
        let index = self as usize;
        &Self::LHS[index..index + 1]
    }

    #[inline(always)]
    pub fn close_str(self) -> &'static str {
        let index = self as usize;
        &Self::RHS[index..index + 1]
    }
}

impl std::fmt::Display for BracketStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BracketStyle::Parens => "parenthesis",
            BracketStyle::Brace => "brace",
            BracketStyle::Square => "square bracket",
            BracketStyle::Angle => "angle bracket",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bracketed {
    count: usize,
    style: BracketStyle,
}

impl Bracketed {
    #[must_use]
    #[inline(always)]
    pub const fn new(index: usize, style: BracketStyle) -> Self {
        Self { count: index, style }
    }

    #[must_use]
    #[inline(always)]
    pub const fn range(&self, start: usize) -> Range<usize> {
        start..start + self.count
    }

    #[must_use]
    #[inline(always)]
    pub const fn new_parens(index: usize) -> Self {
        Self::new(index, BracketStyle::Parens)
    }

    #[must_use]
    #[inline(always)]
    pub const fn new_square(index: usize) -> Self {
        Self::new(index, BracketStyle::Square)
    }

    #[must_use]
    #[inline(always)]
    pub const fn new_brace(index: usize) -> Self {
        Self::new(index, BracketStyle::Brace)
    }

    #[must_use]
    #[inline(always)]
    pub const fn new_angle(index: usize) -> Self {
        Self::new(index, BracketStyle::Angle)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
    At = 0,      // @
    Octo = 1,    // #
    Dollar = 2,  // $
    Percent = 3, // %
}

impl Symbol {
    const TXT: [u8; 4] = [b'@', b'#', b'$', b'%'];
    const CHARS: [char; 4] = ['@', '#', '$', '%'];
    #[must_use]
    #[inline(always)]
    pub const fn as_str(self) -> &'static str {
        debug_assert!((self as usize) < Self::TXT.len());
        unsafe {
            let slice = ::core::slice::from_raw_parts(Self::TXT.as_ptr().byte_offset(self as isize), 1);
            str::from_utf8_unchecked(slice)
        }
    }

    #[must_use]
    #[inline(always)]
    pub const fn as_char(self) -> char {
        Self::CHARS[self as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Token<'a> {
    Str(&'a str),
    Char(char),
    /// `(token_count, bracket_style)`
    Bracketed(Bracketed),
    Symbol(Symbol),
    BracketedSymbol(Symbol, Bracketed),
    SymbolId(Symbol, &'a str),
    SymbolChar(Symbol, char),
    
}

#[derive(Debug, thiserror::Error)]
pub enum FancyError {
    #[error("Unmatched open {0} at {1}")]
    UnmatchedOpen(BracketStyle, usize),
    #[error("Unmatched close {0} at {1}")]
    UnmatchedClose(BracketStyle, usize),
    #[error("Unmatched {expected} at {expected_index}, found {found} at {found_index}.")]
    MismatchedBracket {
        found: BracketStyle,
        found_index: usize,
        expected: BracketStyle,
        expected_index: usize,
    },
    #[error("Unrecognized escape char at {0}.")]
    UnrecognizedEscape(usize),
}
/*
So the idea behind fancy parse is that it should be able to perform
more complex parsing on the source.
Some things it is planned to do:
- Handle more complex interpolation specifiers:
  - `@token` => `@(token)`, `@{token}`, `@[token]`, `@<token>`
  - `#token` => `#(token)`, `#{token}`, `#[token]`, `#<token>`
  - `$token` => `$(token)`, `${token}`, `$[token]`, `$<token>`
  - `%token` => `%(token)`, `%{token}`, `%[token]`, `%<token>`
  Each of the above classes of specifiers consumes the token/token-group immediately following it.
  if the character immediately following the interpolation specifier is an identifier start character,
  it will be treated as a name-based interpolation, where the entire name is consumed and added to the token
  list as a `Name(&str)`.
*/
pub fn fancy_parse<'a>(source: &'a str) -> Result<Vec<Token<'a>>, FancyError> {
    #[derive(Debug, Clone, Copy)]
    struct Backtrack {
        src_index: usize,
        token_index: usize,
        bracket_style: BracketStyle,
    }
    impl Backtrack {
        fn unmatched_error(self) -> FancyError {
            FancyError::UnmatchedClose(self.bracket_style, self.src_index)
        }

        fn mismatched_error(self, found: BracketStyle, found_index: usize) -> FancyError {
            FancyError::MismatchedBracket {
                found,
                found_index,
                expected: self.bracket_style,
                expected_index: self.src_index,
            }
        }
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct BuilderFlags(u8);
    impl BuilderFlags {
        const WAS_SYMBOL: Self = Self(1);
        const IN_BRACKETS: Self = Self(2);
        // const READING_RAW: Self = Self(4);

        #[must_use]
        #[inline(always)]
        fn was_symbol(self) -> bool {
            self.0 & Self::WAS_SYMBOL.0 == Self::WAS_SYMBOL.0
        }
        #[must_use]
        #[inline(always)]
        fn in_brackets(self) -> bool {
            self.0 & Self::IN_BRACKETS.0 == Self::IN_BRACKETS.0
        }
        // #[must_use]
        // #[inline(always)]
        // fn reading_raw(self) -> bool {
        //     self.0 & Self::READING_RAW.0 == Self::READING_RAW.0
        // }

        #[inline(always)]
        fn add(&mut self, other: Self) {
            self.0 |= other.0;
        }

        #[inline(always)]
        fn add_was_symbol(&mut self) {
            self.add(Self::WAS_SYMBOL);
        }

        #[inline(always)]
        fn add_in_brackets(&mut self) {
            self.add(Self::IN_BRACKETS);
        }

        // #[inline(always)]
        // fn add_reading_raw(&mut self) {
        //     self.add(Self::READING_RAW);
        // }

        #[inline(always)]
        fn remove(&mut self, other: Self) {
            self.0 &= !other.0
        }

        #[inline(always)]
        fn remove_was_symbol(&mut self) {
            self.remove(Self::WAS_SYMBOL);
        }

        #[inline(always)]
        fn remove_in_brackets(&mut self) {
            self.remove(Self::IN_BRACKETS);
        }

        // #[inline(always)]
        // fn remove_reading_raw(&mut self) {
        //     self.remove(Self::READING_RAW);
        // }

        #[inline(always)]
        fn set(&mut self, flags: Self, value: bool) {
            if value {
                self.add(flags)
            } else {
                self.remove(flags)
            }
        }

        #[inline(always)]
        fn set_was_symbol(&mut self, value: bool) {
            self.set(Self::WAS_SYMBOL, value)
        }

        #[inline(always)]
        fn set_in_brackets(&mut self, value: bool) {
            self.set(Self::IN_BRACKETS, value)
        }

        // #[inline(always)]
        // fn set_reading_raw(&mut self, value: bool) {
        //     self.set(Self::READING_RAW, value)
        // }

        #[must_use]
        #[inline(always)]
        fn and(self, other: Self) -> Self {
            Self(self.0 & other.0)
        }

        #[must_use]
        #[inline(always)]
        fn or(self, other: Self) -> Self {
            Self(self.0 | other.0)
        }

        #[must_use]
        #[inline(always)]
        fn xor(self, other: Self) -> Self {
            Self(self.0 ^ other.0)
        }
    }
    // #[derive(Debug)]
    struct Builder<'a> {
        flags: BuilderFlags,
        index: usize,
        raw_start: usize,
        tokens: Vec<Token<'a>>,
        backtrack: Vec<Backtrack>,
    }
    impl<'a> Builder<'a> {
        #[inline(always)]
        fn new() -> Self {
            Self {
                flags: BuilderFlags(0),
                index: 0,
                raw_start: 0,
                tokens: Vec::new(),
                backtrack: Vec::new(),
            }
        }

        #[must_use]
        #[inline(always)]
        fn is_reading_raw(&self) -> bool {
            self.raw_start != self.index
        }

        fn finish_raw(&mut self, src: &'a str, new_start: usize) {
            if self.is_reading_raw() {
                let raw_span = &src[self.raw_start..self.index];
                let mut raw_offset = 0usize;
                if self.flags.was_symbol() {
                    self.flags.remove_was_symbol();
                    #[inline(always)]
                    fn is_id_byte(b: u8) -> bool {
                        matches!(
                            b,
                            b'A'..=b'A' |
                            b'a'..=b'z' |
                            b'0'..=b'9' |
                            b'_' | b'-'
                        )
                    }
                    while is_id_byte(raw_span.as_bytes()[raw_offset]) {
                        raw_offset += 1;
                    }
                    if raw_offset > 0 {
                        let symbol_index = self.tokens.len() - 1;
                        let Token::Symbol(symbol) = self.tokens[symbol_index] else {
                            unreachable!("Expected symbol");
                        };
                        self.tokens[symbol_index] = Token::SymbolId(symbol, &raw_span[..raw_offset]);
                    }
                }
                self.tokens.push(Token::Str(&raw_span[raw_offset..]));
            }
            self.raw_start = new_start;
        }

        fn push_char(&mut self, src: &'a str, count: usize, chr: char) -> Result<bool, FancyError> {
            let new_index = self.index + count;
            self.finish_raw(src, new_index);
            self.flags.remove_was_symbol();
            self.tokens.push(Token::Char(chr));
            self.index = new_index;
            Ok(true)
        }

        fn push_symbol(&mut self, src: &'a str, symbol: Symbol) -> Result<bool, FancyError> {
            let new_index = self.index + 1;
            self.finish_raw(src, new_index);
            if self.flags.was_symbol() {
                self.flags.remove_was_symbol();
                debug_assert!(self.tokens.len() > 0);
                let symbol_index = self.tokens.len() - 1;
                let Token::Symbol(last_symbol) = self.tokens[symbol_index] else {
                    unreachable!("Expected symbol");
                };
                self.tokens[symbol_index] = Token::SymbolChar(last_symbol, symbol.as_char());
            } else {
                self.tokens.push(Token::Symbol(symbol));
                self.flags.add_was_symbol();
            }
            self.index = new_index;
            Ok(true)
        }

        fn begin_bracketed(&mut self, src: &'a str, bracket_style: BracketStyle) -> Result<bool, FancyError> {
            let new_index = self.index + 1;
            self.finish_raw(src, new_index);
            let token_index = if self.flags.was_symbol() {
                self.flags.remove_was_symbol();
                let index = self.tokens.len() - 1;
                let Token::Symbol(symbol) = self.tokens[index] else {
                    unreachable!("Expected a symbol token.");
                };
                self.tokens[index] = Token::BracketedSymbol(symbol, Bracketed::new(usize::MAX, bracket_style));
                index
            } else {
                let index = self.tokens.len();
                self.tokens.push(Token::Bracketed(Bracketed::new(usize::MAX, bracket_style)));
                index
            };
            self.backtrack.push(Backtrack {
                src_index: self.index,
                token_index: token_index,
                bracket_style,
            });
            self.flags.add_in_brackets();
            self.index = new_index;
            Ok(true)
       }

        fn end_bracketed(&mut self, src: &'a str, bracket_style: BracketStyle) -> Result<bool, FancyError> {
            let Some(back) = self.backtrack.pop() else {
                return Err(FancyError::UnmatchedClose(
                    bracket_style,
                    self.index,
                ));
            };
            if back.bracket_style != bracket_style {
                return Err(back.mismatched_error(bracket_style, self.index));
            }
            if self.backtrack.is_empty() {
                self.flags.remove_in_brackets();
            }
            let new_index = self.index + 1;
            self.finish_raw(src, new_index);
            // Just in case finish_raw did not remove it.
            self.flags.remove_was_symbol();
            let token_count = self.tokens.len() - back.token_index - 1;
            let brack = match &mut self.tokens[back.token_index] {
                Token::Bracketed(brack) => brack,
                Token::BracketedSymbol(_, brack) => brack,
                tt => unreachable!("Expected bracketed: {tt:#?}"),
            };
            brack.count = token_count;
            self.index = new_index;
            Ok(true)
        }
    }
    let mut builder = Builder::new();
    parse(
        &mut builder,
        source,
        move |out, src| -> Result<bool, FancyError> {
            if out.index >= src.len() {
                if let Some(unmatched) = out.backtrack.pop() {
                    return Err(unmatched.unmatched_error());
                }
                out.finish_raw(src, src.len());
                return Ok(false);
            }
            let src_at = &src[out.index..];
            match src_at.as_bytes()[0] {
                b'\\' if src_at.len() >= 2 => {
                    macro_rules! escapes {
                        ($(
                            $matched:literal => $escaped:literal,
                        )+) => {
                            match src_at.as_bytes()[1] {
                                $(
                                    $matched => {
                                        return out.push_char(src, 2, $escaped);
                                    },
                                )*
                                _ => return Err(FancyError::UnrecognizedEscape(out.index)),
                            }
                        };
                    }
                    escapes!{
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'0' => '\0',
                        b'{' => '{',
                        b'}' => '}',
                        b'[' => '[',
                        b']' => ']',
                        b'(' => '(',
                        b')' => ')',
                        b'<' => '<',
                        b'>' => '>',
                        b'@' => '@',
                        b'#' => '#',
                        b'$' => '$',
                        b'%' => '%',
                    }
                }
                b'{' => return out.begin_bracketed(src, BracketStyle::Brace),
                b'[' => return out.begin_bracketed(src, BracketStyle::Square),
                b'(' => return out.begin_bracketed(src, BracketStyle::Parens),
                b'<' => return out.begin_bracketed(src, BracketStyle::Angle),
                b'}' => return out.end_bracketed(src, BracketStyle::Brace),
                b']' => return out.end_bracketed(src, BracketStyle::Square),
                b')' => return out.end_bracketed(src, BracketStyle::Parens),
                b'>' => return out.end_bracketed(src, BracketStyle::Angle),
                b'@' => return out.push_symbol(src, Symbol::At),
                b'#' => return out.push_symbol(src, Symbol::Octo),
                b'$' => return out.push_symbol(src, Symbol::Dollar),
                b'%' => return out.push_symbol(src, Symbol::Percent),
                _ => (),
            }
            out.index += 1;
            Ok(true)
        }
    )?;
    Ok(builder.tokens)
}

pub trait FancyPerformerError<'a> {
    fn invalid_tokens(tokens: &[Token<'a>]) -> Self;
    fn invalid_bracketed() -> Self;
}

macro_rules! default_visit_fns {
    (symbol: $($symbol:ident => $name:ident),*$(,)?) => {
        paste::paste!{
            $(
                fn [< visit_ $name >](
                    &mut self,
                ) -> Result<(), Self::Error> {
                    self.visit_char(Symbol::$symbol.as_char())
                }
            )*
            
            fn visit_symbol(
                &mut self,
                symbol: Symbol,
            ) -> Result<(), Self::Error> {
                match symbol {
                    $(
                        Symbol::$symbol => self.[< visit_ $name >](),
                    )*
                }
            }
        }
    };
    (full_bracketed_symbol: $($name:ident($symbol:ident, $style:ident)),+$(,)?) => {
        $(
            fn $name(
                &mut self,
                tokens: &[Token<'a>],
            ) -> Result<(), Self::Error> {
                self.visit_symbol(Symbol::$symbol)?;
                self.visit_bracketed(BracketStyle::$style, tokens)
            }
        )*
    };
    (generic_bracketed_symbol: $($symbol:ident => $name:ident),+$(,)?) => {
        paste::paste!{
            $(
                default_visit_fns!{
                    full_bracketed_symbol:
                    [< visit_parenthesized_ $name >]($symbol, Parens),
                    [< visit_braced_ $name >]($symbol, Brace),
                    [< visit_squared_ $name >]($symbol, Square),
                    [< visit_angled_ $name >]($symbol, Angle),
                }
            )*
        }
    };
    (bracketed_symbol: $($symbol:ident => $name:ident),+$(,)?) => {
        paste::paste!{
            default_visit_fns!{
                generic_bracketed_symbol:
                $($symbol => $name,)*
            }
            $(
                fn [< visit_bracketed_ $name>](
                    &mut self,
                    style: BracketStyle,
                    tokens: &[Token<'a>],
                ) -> Result<(), Self::Error> {
                    match style {
                        BracketStyle::Parens => self.[< visit_parenthesized_ $name >](tokens),
                        BracketStyle::Brace => self.[< visit_braced_ $name >](tokens),
                        BracketStyle::Square => self.[< visit_squared_ $name >](tokens),
                        BracketStyle::Angle => self.[< visit_angled_ $name >](tokens),
                    }
                }
            )*

            fn visit_bracketed_symbol(
                &mut self,
                symbol: Symbol,
                style: BracketStyle,
                tokens: &[Token<'a>],
            ) -> Result<(), Self::Error> {
                match symbol {
                    $(
                        Symbol::$symbol => self.[< visit_bracketed_ $name >](style, tokens),
                    )*
                }
            }
        }
    };
    (symbol_char: $($symbol:ident => $name:ident),+$(,)?) => {
        paste::paste!{
            $(
                fn [< visit_ $name _char >](
                    &mut self,
                    chr: char,
                ) -> Result<(), Self::Error> {
                    self.visit_symbol(Symbol::$symbol)?;
                    self.visit_char(chr)
                }
            )*

            fn visit_symbol_char(
                &mut self,
                symbol: Symbol,
                chr: char,
            ) -> Result<(), Self::Error> {
                match symbol {
                    $(
                        Symbol::$symbol => self.[< visit_ $name _char>](chr),
                    )*
                }
            }
        }
    };
    (symbol_id: $($symbol:ident => $name:ident),*$(,)?) => {
        paste::paste!{
            $(
                fn [< visit_ $name _id >](
                    &mut self,
                    id: &'a str,
                ) -> Result<(), Self::Error> {
                    self.visit_symbol(Symbol::$symbol)?;
                    self.visit_str(id)
                }
            )*

            fn visit_symbol_id(
                &mut self,
                symbol: Symbol,
                id: &'a str,
            ) -> Result<(), Self::Error> {
                match symbol {
                    $(
                        Symbol::$symbol => self.[< visit_ $name _id >](id),
                    )*
                }
            }
        }
    };
    ($($symbol:ident => $name:ident),+$(,)?) => {
        default_visit_fns!{symbol:          $($symbol => $name,)*}
        default_visit_fns!{bracketed_symbol:$($symbol => $name,)*}
        default_visit_fns!{symbol_char:     $($symbol => $name,)*}
        default_visit_fns!{symbol_id:       $($symbol => $name,)*}
    };
}

pub trait FancyPerformer<'a> {
    type Error: FancyPerformerError<'a>;

    fn visit_tokens(
        &mut self,
        tokens: &[Token<'a>],
    ) -> Result<(), Self::Error>
     where Self::Error: FancyPerformerError<'a>{
        let mut index = 0usize;
        while index < tokens.len() {
            let token = tokens[index];
            match token {
                Token::Str(raw) => {
                    self.visit_str(raw)?;
                    index += 1;
                },
                Token::Char(esc) => {
                    self.visit_char(esc)?;
                    index += 1;
                },
                Token::Bracketed(bracketed) => {
                    let range = bracketed.range(index + 1);
                    if tokens.len() < range.end {
                        return Err(Self::Error::invalid_bracketed());
                    }
                    index = range.end;
                    let b_tokens = &tokens[range];
                    self.visit_bracketed(bracketed.style, b_tokens)?;
                },
                Token::BracketedSymbol(symbol, bracketed) => {
                    let range = bracketed.range(index + 1);
                    if tokens.len() < range.end {
                        return Err(Self::Error::invalid_bracketed());
                    }
                    index = range.end;
                    let b_tokens = &tokens[range];
                    self.visit_bracketed_symbol(symbol, bracketed.style, b_tokens)?;
                },
                Token::Symbol(symbol) => {
                    self.visit_symbol(symbol)?;
                    index += 1;
                },
                Token::SymbolId(symbol, id) => {
                    self.visit_symbol_id(symbol, id)?;
                    index += 1;
                },
                Token::SymbolChar(symbol, chr) => {
                    self.visit_symbol_char(symbol, chr)?;
                    index += 1;
                },
            }
        }
        Ok(())
    }

    fn visit_str(
        &mut self,
        raw: &str,
    ) -> Result<(), Self::Error>;

    fn visit_char(
        &mut self,
        chr: char,
    ) -> Result<(), Self::Error> {
        let mut bytes = [0u8; 4];
        self.visit_str(chr.encode_utf8(&mut bytes))
    }
    
    fn visit_bracketed(
        &mut self,
        style: BracketStyle,
        tokens: &[Token<'a>]
    ) -> Result<(), Self::Error> {
        self.visit_str(style.open_str())?;
        self.visit_tokens(tokens)?;
        self.visit_str(style.close_str())
    }

    default_visit_fns!{
        At => at,
        Octo => octo,
        Dollar => dollar,
        Percent => percent,
    }
}

impl<'a> FancyPerformerError<'a> for () {
    fn invalid_tokens(_: &[Token<'a>]) -> Self {
        ()
    }

    fn invalid_bracketed() -> Self {
        ()
    }
}

impl<'a> FancyPerformer<'a> for String {
    type Error = ();

    fn visit_str(&mut self, raw: &str) -> Result<(), Self::Error> {
        self.push_str(raw);
        Ok(())
    }
}

struct StringPool {
    pool: Vec<String>,
}

impl StringPool {
    const fn new() -> Self {
        Self { pool: Vec::new() }
    }

    fn get(&mut self) -> String {
        self.pool.pop().unwrap_or_else(|| String::with_capacity(4096))
    }

    fn get_with_capacity(&mut self, capacity: usize) -> String {
        for i in 0..self.pool.len() {
            if self.pool[i].capacity() >= capacity {
                return self.pool.swap_remove(i);
            }
        }
        String::with_capacity(capacity)
    }

    fn put(&mut self, s: String) {
        self.pool.push(s);
    }
}

pub struct FancyOptions {
    
}

#[derive(Debug, Clone, Copy)]
pub enum ParseNode<'a> {
    Raw(&'a str),
    RawInterp(&'a str),
    EscapedOpen,
    EscapedClose,
    /// `usize` here is the offset to the end.
    /// You can get the range by `index_of_interp+1..index_of_interp+offset`.
    Interp(usize),
}
const _: () = isit::assert_size::<ParseNode<'static>, 24>();

#[derive(Debug, Clone, Copy)]
pub enum Never {}
const _: () = isit::assert_uninhabited_zst::<Never>();

#[derive(Debug, thiserror::Error)]
pub enum InterpError<E = Never> {
    #[error("Open token without a close token at {0}")]
    MismatchedOpen(usize),
    #[error("Close token without an open token at {0}")]
    MismatchedClose(usize),
    #[error("User error: {0}")]
    User(E),
}

impl InterpError<Never> {
    pub const fn map_user<E>(self) -> InterpError<E> {
        match self {
            InterpError::MismatchedOpen(index) => InterpError::MismatchedOpen(index),
            InterpError::MismatchedClose(index) => InterpError::MismatchedClose(index),
            InterpError::User(_) => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn map_result<T, E>(result: Result<T, Self>) -> Result<T, InterpError<E>> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.map_user()),
        }
    }
}

/// Expects `output` to be empty, and will clear output before use.
/// So do not expect that this will continue filling `output`.
/// This algorithms relies on a backtracking stack, which isn't passed
/// between invocations. It could be, but that would make the algorithm
/// a bit more complicated.
pub fn parse_interp<'a>(
    output: &mut Vec<ParseNode<'a>>,
    source: &'a str,
) -> Result<(), InterpError<Never>> {
    // backtrack holds the indices to the most recent Open token within the `output`.
    let mut backtrack = Vec::<usize>::new();
    let mut index = 0usize;
    let mut raw_start = 0usize;
    while index < source.len() {
        let c = source.as_bytes()[index];
        let source_at = &source[index..];
        match c {
            // Check for `\%{` or `\}%`
            b'\\' if source_at.starts_with("\\%{") => {
                if raw_start != index {
                    output.push(ParseNode::Raw(&source[raw_start..index]));
                }
                raw_start = index + 3;
                index = raw_start;
                output.push(ParseNode::EscapedOpen);
            }
            b'\\' if source_at.starts_with("\\}%") => {
                if raw_start != index {
                    output.push(ParseNode::Raw(&source[raw_start..index]));
                }
                raw_start = index + 3;
                index = raw_start;
                output.push(ParseNode::EscapedClose);
            }
            // Check for `%{`
            b'%' if source_at.starts_with("%{") => {
                if raw_start != index {
                    output.push(ParseNode::Raw(&source[raw_start..index]));
                }
                raw_start = index + 2;
                index = raw_start;
                let open_index = output.len();
                backtrack.push(open_index);
                output.push(ParseNode::Interp(usize::MAX));
            }
            // Check for `}%`
            b'}' if source_at.starts_with("}%") => {
                if backtrack.is_empty() {
                    return Err(InterpError::MismatchedClose(index));
                }
                if output.len() > 1
                && matches!(&output[output.len() - 1], ParseNode::Interp(_)) {
                    let output_top = output.len() - 1;
                    backtrack.pop();
                    output[output_top] = ParseNode::RawInterp(&source[raw_start..index]);
                    raw_start = index + 2;
                    index = raw_start;
                    continue;
                }
                if raw_start != index {
                    output.push(ParseNode::Raw(&source[raw_start..index]));
                }
                raw_start = index + 2;
                index = raw_start;
                let open_index = backtrack.pop().unwrap();
                let close_index = output.len();
                let difference = close_index - open_index;
                output[open_index] = ParseNode::Interp(difference);
            }
            _ => {
                index += 1;
            }
        }
    }
    if !backtrack.is_empty() {
        let index = backtrack.pop().unwrap();
        return Err(InterpError::MismatchedOpen(index));
    }
    if raw_start != index {
        output.push(ParseNode::Raw(&source[raw_start..index]));
    }
    Ok(())
}

impl<E> InterpError<E> {
    #[inline(always)]
    pub fn map_e_result<T>(result: Result<T, E>) -> Result<T, InterpError<E>> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(InterpError::User(err)),
        }
    }
}

pub fn interpolate<
    E,
    F: FnMut(&mut String, &str) -> Result<(), E>,
>(
    format: &str,
    interpolator: &mut F,
) -> Result<String, InterpError<E>> {
    let mut nodes = Vec::new();
    InterpError::map_result(parse_interp(&mut nodes, format))?;
    fn interp_nodes<
        E,
        F: FnMut(&mut String, &str) -> Result<(), E>,
    >(
        nodes: &[ParseNode<'_>],
        f: &mut F,
    ) -> Result<Option<String>, InterpError<E>> {
        if nodes.is_empty() {
            return Ok(None);
        }
        let mut buffer = String::new();
        let mut i = 0usize;
        while i < nodes.len() {
            let node = nodes[i];
            match node {
                ParseNode::Raw(raw) => {
                    buffer.push_str(raw);
                    i += 1;
                },
                ParseNode::RawInterp(raw) => {
                    InterpError::map_e_result(f(&mut buffer, raw))?;
                    i += 1;
                },
                ParseNode::EscapedOpen => {
                    buffer.push_str("%{");
                    i += 1;
                },
                ParseNode::EscapedClose => {
                    buffer.push_str("}%");
                    i += 1;
                },
                ParseNode::Interp(offset) => {
                    let range = i+1..i+offset;
                    let interp = interp_nodes(&nodes[range], f)?;
                    match interp {
                        Some(interp) => InterpError::map_e_result(f(&mut buffer, interp.as_str()))?,
                        None => InterpError::map_e_result(f(&mut buffer, ""))?,
                    }
                    i += offset;
                },
            }
        }
        Ok(Some(buffer))
    }
    match interp_nodes(&nodes, interpolator)? {
        Some(result) => Ok(result),
        None => Ok(String::new()),
    }
}
