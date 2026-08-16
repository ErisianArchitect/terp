
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident {
    pub value: Box<str>,
}

// %{ident}%
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Substitute {
    pub ident: Ident,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentCond {
    pub ident: Ident,
    pub token: Token,
    pub alt: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsentCond {
    pub ident: Ident,
    pub token: Token,
    pub alt: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqCond {
    pub lhs: Token,
    pub rhs: Token,
    pub token: Token,
    pub alt: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeCond {
    pub lhs: Token,
    pub rhs: Token,
    pub token: Token,
    pub alt: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchCond {
    pub lhs: Token,
    pub pattern: Token,
    pub token: Token,
    pub alt: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismatchCond {
    pub lhs: Token,
    pub pattern: Token,
    pub token: Token,
    pub alt: Option<Token>,
}

pub enum IfCond {
    /// Interpolates if `ident` is present.
    /// 
    /// `%{?ident|token}%`
    Present(PresentCond),
    /// Interpolates if `ident` is absent.
    ///
    /// `%{?ident!|token}%`
    Absent(AbsentCond),
    /// Interpolates if `ident` is equal to `eq_token`.
    /// 
    /// `%{?ident==eq_token|token}%`
    Equal(EqCond),
    /// Interpolates if `ident` is not equal to `ne_token`.
    /// 
    /// `%{?ident!=ne_token|token}%`
    Inequal(NeCond),
    /// Interpolates if `ident` is a regex match for `pattern_token`.
    ///
    /// `%{?ident=~pattern_token|token}%`
    Matches(MatchCond),
    /// Interpolates if `ident` is not a regex match for `pattern_token`.
    /// 
    /// `%{?ident!~pattern_token|token}%`
    NotMatches(MismatchCond),
}

// %{?ident|token}%
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstituteIf {
    pub ident: Ident,
    pub token: Token,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interpolation {
    Sub(Substitute),
    SubIf(SubstituteIf),
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Empty,
    Raw(String),
    Interp(Interpolation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInner {
    pub span: Span,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub inner: Box<TokenInner>,
}
