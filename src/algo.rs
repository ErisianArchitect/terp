
use ::core::{
    ops::Range,
    num::NonZeroUsize,
};



/// Used to parse
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
    Parens = 0,
    Brace = 1,
    Square = 2,
    Angle = 3,
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

#[derive(Debug, Clone, Copy)]
pub enum Token<'a> {
    Raw(&'a str),
    Esc(char),
    /// `(token_count, bracket_style)`
    Bracketed(Bracketed),
    At,
    Octo,
    Dollar,
    Percent,
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
    #[derive(Debug)]
    struct Builder<'a> {
        index: usize,
        raw_start: usize,
        tokens: Vec<Token<'a>>,
        backtrack: Vec<Backtrack>,
    }
    impl<'a> Builder<'a> {
        #[inline(always)]
        fn new() -> Self {
            Self {
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
                self.tokens.push(Token::Raw(&src[self.raw_start..self.index.min(src.len())]));
            }
            self.raw_start = new_start;
        }

        fn push_wrap(&mut self, src: &'a str, count: usize, token: Token<'a>) -> Result<bool, FancyError> {
            let new_index = self.index + count;
            self.finish_raw(src, new_index);
            self.tokens.push(token);
            self.index = new_index;
            Ok(true)
        }

        fn begin_bracketed(&mut self, src: &'a str, count: usize, bracket_style: BracketStyle) -> Result<bool, FancyError> {
            let new_index = self.index + count;
            self.finish_raw(src, new_index);
            self.backtrack.push(Backtrack {
                src_index: self.index,
                token_index: self.tokens.len(),
                bracket_style,
            });
            self.index = new_index;
            self.tokens.push(Token::Bracketed(Bracketed::new(usize::MAX, bracket_style)));
            Ok(true)
       }

        fn end_bracketed(&mut self, src: &'a str, count: usize, bracket_style: BracketStyle) -> Result<bool, FancyError> {
            let Some(back) = self.backtrack.pop() else {
                return Err(FancyError::UnmatchedClose(
                    bracket_style,
                    self.index,
                ));
            };
            if back.bracket_style != bracket_style {
                return Err(back.mismatched_error(bracket_style, self.index));
            }
            let new_index = self.index + count;
            self.finish_raw(src, new_index);
            let token_count = self.tokens.len() - back.token_index - 1;
            let Token::Bracketed(brack) = &mut self.tokens[back.token_index] else {
                unreachable!();
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
                                        return out.push_wrap(src, 2, Token::Esc($escaped));
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
                b'{' => return out.begin_bracketed(src, 1, BracketStyle::Brace),
                b'[' => return out.begin_bracketed(src, 1, BracketStyle::Square),
                b'(' => return out.begin_bracketed(src, 1, BracketStyle::Parens),
                b'<' => return out.begin_bracketed(src, 1, BracketStyle::Angle),
                b'}' => return out.end_bracketed(src, 1, BracketStyle::Brace),
                b']' => return out.end_bracketed(src, 1, BracketStyle::Square),
                b')' => return out.end_bracketed(src, 1, BracketStyle::Parens),
                b'>' => return out.end_bracketed(src, 1, BracketStyle::Angle),
                b'@' => return out.push_wrap(src, 1, Token::At),
                b'#' => return out.push_wrap(src, 1, Token::Octo),
                b'$' => return out.push_wrap(src, 1, Token::Dollar),
                b'%' => return out.push_wrap(src, 1, Token::Percent),
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

pub trait FancyPerformer<'a> {
    type Error: FancyPerformerError<'a>;

    fn visit_tokens(&mut self, tokens: &[Token<'a>]) -> Result<(), Self::Error>
     where Self::Error: FancyPerformerError<'a>{
        let mut index = 0usize;
        macro_rules! visit_symbol {
            ($function:ident($token:literal)) => {
                {
                    let next = index + 1;
                    if next >= tokens.len() {
                        self.visit_raw($token)?;
                        break;
                    }
                    let next_token= tokens[next];
                    match next_token {
                        Token::Bracketed(br) => {
                            let count = br.count;
                            let br_end = next + 1 + count;
                            if br_end > tokens.len() {
                                return Err(Self::Error::invalid_bracketed());
                            }
                            self.$function(&tokens[next..br_end])?;
                            index = br_end;
                        }
                        _ => {
                            let next_index = next + 1;
                            self.$function(&tokens[next..next_index])?;
                            index = next_index;
                        }
                    }
                }
            };
        }
        while index < tokens.len() {
            let token = tokens[index];
            match token {
                Token::Raw(raw) => {
                    self.visit_raw(raw)?;
                    index += 1;
                },
                Token::Esc(esc) => {
                    self.visit_esc(esc)?;
                    index += 1;
                },
                Token::Bracketed(bracketed) => {
                    let count = bracketed.count;
                    let range_start = index + 1;
                    let range_end = range_start + count;
                    if tokens.len() < range_end {
                        return Err(Self::Error::invalid_bracketed());
                    }
                    let b_tokens = &tokens[range_start..range_end];
                    self.visit_bracketed(bracketed, b_tokens)?;
                    index = range_end;
                },
                Token::At => visit_symbol!(visit_at("@")),
                Token::Octo => visit_symbol!(visit_octo("#")),
                Token::Dollar => visit_symbol!(visit_dollar("$")),
                Token::Percent => visit_symbol!(visit_percent("%")),
            }
        }
        Ok(())
    }

    fn visit_raw(&mut self, raw: &str) -> Result<(), Self::Error>;

    fn visit_esc(&mut self, esc: char) -> Result<(), Self::Error> {
        let mut bytes = [0u8; 4];
        self.visit_raw(esc.encode_utf8(&mut bytes))?;
        Ok(())
    }
    
    fn visit_bracketed(&mut self, bracketed: Bracketed, tokens: &[Token<'a>]) -> Result<(), Self::Error> {
        self.visit_raw(bracketed.style.open_str())?;
        self.visit_tokens(tokens)?;
        self.visit_raw(bracketed.style.close_str())?;
        Ok(())
    }

    fn visit_at(&mut self, tokens: &[Token<'a>]) -> Result<(), Self::Error> {
        self.visit_raw("@")?;
        self.visit_tokens(tokens)?;
        Ok(())
    }

    fn visit_octo(&mut self, tokens: &[Token<'a>]) -> Result<(), Self::Error> {
        self.visit_raw("#")?;
        self.visit_tokens(tokens)?;
        Ok(())
    }

    fn visit_dollar(&mut self, tokens: &[Token<'a>]) -> Result<(), Self::Error> {
        self.visit_raw("$")?;
        self.visit_tokens(tokens)?;
        Ok(())
    }

    fn visit_percent(&mut self, tokens: &[Token<'a>]) -> Result<(), Self::Error> {
        self.visit_raw("%")?;
        self.visit_tokens(tokens)?;
        Ok(())
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

    fn visit_raw(&mut self, raw: &str) -> Result<(), Self::Error> {
        self.push_str(raw);
        Ok(())
    }
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
