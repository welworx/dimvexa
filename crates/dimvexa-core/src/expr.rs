// SPDX-License-Identifier: MIT OR Apache-2.0
//! Syntax of unit expressions: tokenizer, parser and canonical printer.
//!
//! # Grammar
//!
//! ```text
//! expr   = factor { ("." | "*" | "/") factor }
//! factor = atom [ "^" int | int ]        (a bare int is allowed only right after an identifier)
//! atom   = ident | "1" | "(" expr ")"
//! int    = [ "+" | "-" ] digits
//! ident  = letter-or-underscore-or-degree-sign { letter | underscore | degree-sign }
//! ```
//!
//! # Precedence (documented, deliberately strict)
//!
//! * `.` and `*` (multiplication) and `/` (division) have **the same precedence and are
//!   left-associative**, as in UCUM: `a/b/c` is `(a/b)/c`, `a.b/c` is `(a.b)/c`.
//! * Exponents bind tighter than every operator: `m/s2` is `m/(s^2)`.
//! * A multiplication that follows a division on the same nesting level is rejected as
//!   ambiguous ([`Error::AmbiguousExpression`]): `J/mol.K` could mean `J/(mol.K)` or
//!   `(J/mol).K`. Write `J/(mol.K)` or `(J/mol).K`.
//! * Whitespace separates tokens but never multiplies: `kg m` is an error, write `kg.m`.
//! * Only the literal `1` is allowed as a number (`1/s`). Numeric scaling belongs to the
//!   quantity, not to the unit.
//! * Case matters: `Pa` is not `pa`, `M` (mega) is not `m` (milli).

use core::fmt;

use crate::error::{Error, Result};

/// Maximum accepted input length in characters.
pub const MAX_INPUT_CHARS: usize = 1024;
/// Maximum nesting depth of parentheses.
pub const MAX_DEPTH: usize = 32;

/// A parsed unit expression, before symbols are looked up.
///
/// `Display` prints a canonical form with the fewest parentheses that parse back to the
/// same tree: `Expr::parse(&e.to_string()) == Ok(e)` for every tree.
///
/// # Examples
///
/// ```
/// use dimvexa_core::Expr;
///
/// let e = Expr::parse("kg.m/s2")?;
/// assert_eq!(e.to_string(), "kg.m/s^2");
/// assert_eq!(Expr::parse(&e.to_string())?, e);
/// # Ok::<(), dimvexa_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// The literal `1`.
    One,
    /// A unit symbol, possibly carrying a prefix (`km`).
    Unit(String),
    /// Product.
    Mul(Box<Self>, Box<Self>),
    /// Quotient.
    Div(Box<Self>, Box<Self>),
    /// Integer power.
    Pow(Box<Self>, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    /// Integer immediately attached to an identifier, or following `^`.
    Int(i32),
    /// A bare number elsewhere.
    Num(String),
    Mul,
    Div,
    Caret,
    Open,
    Close,
}

/// True if `c` may start or continue a unit symbol.
#[must_use]
pub fn is_symbol_char(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '\u{b0}'
}

/// True for a superscript digit (`²`, `³`, ...). It may end the symbol of a reference unit
/// (`Nm³`, ADR-013), never start a symbol.
#[must_use]
pub const fn is_superscript_digit(c: char) -> bool {
    matches!(
        c,
        '\u{b2}' | '\u{b3}' | '\u{b9}' | '\u{2070}' | '\u{2074}'..='\u{2079}'
    )
}

/// True if `s` is a well-formed unit symbol (non-empty, only symbol characters).
#[must_use]
pub fn is_valid_symbol(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_symbol_char)
}

/// True if `s` is a well-formed symbol for a unit of kind `kind`.
///
/// A unit tied to a reference state may end in digits (`Nm3`, ADR-013), because that is how it is written in practice;
/// every other symbol is letters only, so `m2` always means `m^2`.
#[must_use]
pub fn is_valid_symbol_for(s: &str, kind: crate::kind::Kind) -> bool {
    if is_valid_symbol(s) {
        return true;
    }
    let base = s.trim_end_matches(|c: char| c.is_ascii_digit() || is_superscript_digit(c));
    kind.is_reference() && base.len() < s.len() && is_valid_symbol(base)
}

struct Parser<'a> {
    input: &'a str,
    toks: Vec<(usize, Tok)>,
    pos: usize,
}

fn parse_err(input: &str, position: usize, message: impl Into<String>) -> Error {
    Error::Parse {
        input: input.to_string(),
        position,
        message: message.into(),
    }
}

fn lex(input: &str) -> Result<Vec<(usize, Tok)>> {
    let chars: Vec<(usize, char)> = input.chars().enumerate().collect();
    let mut out = Vec::new();
    let mut i = 0_usize;
    let at = |i: usize| chars.get(i).map(|&(_, c)| c);
    let next = |i: usize| i.saturating_add(1);
    let mut after_caret = false;
    while let Some(c) = at(i) {
        let start = i;
        if c.is_whitespace() {
            i = next(i);
            continue;
        }
        let was_caret = after_caret;
        after_caret = false;
        if is_symbol_char(c) {
            let mut s = String::new();
            while let Some(ch) = at(i).filter(|&ch| is_symbol_char(ch) || is_superscript_digit(ch))
            {
                s.push(ch);
                i = next(i);
            }
            out.push((start, Tok::Ident(s)));
            // An integer directly attached to an identifier is its exponent.
            let signed =
                matches!(at(i), Some('+' | '-')) && at(next(i)).is_some_and(|d| d.is_ascii_digit());
            if at(i).is_some_and(|d| d.is_ascii_digit()) || signed {
                let pos = i;
                let n = lex_int(input, &chars, &mut i)?;
                out.push((pos, Tok::Int(n)));
            }
            continue;
        }
        if c.is_ascii_digit() || (was_caret && matches!(c, '+' | '-')) {
            if was_caret {
                let n = lex_int(input, &chars, &mut i)?;
                out.push((start, Tok::Int(n)));
            } else {
                let mut s = String::new();
                while let Some(ch) = at(i).filter(char::is_ascii_digit) {
                    s.push(ch);
                    i = next(i);
                }
                out.push((start, Tok::Num(s)));
            }
            continue;
        }
        let tok = match c {
            '.' | '*' => Tok::Mul,
            '/' => Tok::Div,
            '^' => {
                after_caret = true;
                Tok::Caret
            }
            '(' => Tok::Open,
            ')' => Tok::Close,
            other => {
                return Err(parse_err(
                    input,
                    start,
                    format!("unexpected character '{other}'"),
                ));
            }
        };
        out.push((start, tok));
        i = next(i);
    }
    Ok(out)
}

fn lex_int(input: &str, chars: &[(usize, char)], i: &mut usize) -> Result<i32> {
    let start = *i;
    let mut s = String::new();
    if let Some(&(_, c @ ('+' | '-'))) = chars.get(*i) {
        s.push(c);
        *i = i.saturating_add(1);
    }
    let digits_start = s.len();
    while let Some(&(_, c)) = chars.get(*i).filter(|(_, c)| c.is_ascii_digit()) {
        s.push(c);
        *i = i.saturating_add(1);
    }
    if s.len() == digits_start {
        return Err(parse_err(input, start, "expected an integer exponent"));
    }
    s.parse::<i32>()
        .map_err(|_| parse_err(input, start, "exponent out of range"))
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }

    fn here(&self) -> usize {
        self.toks
            .get(self.pos)
            .map_or_else(|| self.input.chars().count(), |(p, _)| *p)
    }

    const fn bump(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    fn err(&self, message: impl Into<String>) -> Error {
        parse_err(self.input, self.here(), message)
    }

    fn expr(&mut self, depth: usize) -> Result<Expr> {
        let mut lhs = self.factor(depth)?;
        let mut seen_div = false;
        loop {
            match self.peek() {
                Some(Tok::Mul) => {
                    if seen_div {
                        return Err(Error::AmbiguousExpression {
                            input: self.input.to_string(),
                            message: format!(
                                "a multiplication after a division (position {}) can be read two ways; \
                                 write a/(b.c) or (a/b).c",
                                self.here()
                            ),
                        });
                    }
                    self.bump();
                    let rhs = self.factor(depth)?;
                    lhs = Expr::Mul(Box::new(lhs), Box::new(rhs));
                }
                Some(Tok::Div) => {
                    seen_div = true;
                    self.bump();
                    let rhs = self.factor(depth)?;
                    lhs = Expr::Div(Box::new(lhs), Box::new(rhs));
                }
                _ => return Ok(lhs),
            }
        }
    }

    fn factor(&mut self, depth: usize) -> Result<Expr> {
        let atom = self.atom(depth)?;
        match self.peek() {
            Some(Tok::Caret) => {
                self.bump();
                match self.peek().cloned() {
                    Some(Tok::Int(n)) => {
                        self.bump();
                        Ok(Expr::Pow(Box::new(atom), n))
                    }
                    _ => Err(self.err("expected an integer after '^'")),
                }
            }
            Some(Tok::Int(n)) if matches!(atom, Expr::Unit(_)) => {
                let n = *n;
                self.bump();
                Ok(Expr::Pow(Box::new(atom), n))
            }
            _ => Ok(atom),
        }
    }

    fn atom(&mut self, depth: usize) -> Result<Expr> {
        match self.peek().cloned() {
            Some(Tok::Ident(s)) => {
                self.bump();
                Ok(Expr::Unit(s))
            }
            Some(Tok::Num(n)) => {
                if n == "1" {
                    self.bump();
                    Ok(Expr::One)
                } else {
                    Err(self.err(format!(
                        "number '{n}' is not allowed in a unit expression (only the literal 1); \
                         put numeric factors on the quantity"
                    )))
                }
            }
            Some(Tok::Open) => {
                if depth >= MAX_DEPTH {
                    return Err(self.err("parentheses nested too deeply"));
                }
                self.bump();
                let inner = self.expr(depth.saturating_add(1))?;
                if self.peek() == Some(&Tok::Close) {
                    self.bump();
                    Ok(inner)
                } else {
                    Err(self.err("missing ')'"))
                }
            }
            Some(Tok::Int(_)) => Err(self.err("an exponent must follow a unit symbol or '^'")),
            Some(Tok::Mul | Tok::Div | Tok::Caret | Tok::Close) => {
                Err(self.err("expected a unit symbol, '1' or '('"))
            }
            None => Err(self.err("unexpected end of expression")),
        }
    }
}

impl Expr {
    /// Parses a unit expression. See the module documentation for the grammar and the
    /// precedence rules.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for syntax errors and [`Error::AmbiguousExpression`] for patterns such
    /// as `J/mol.K`.
    pub fn parse(input: &str) -> Result<Self> {
        if input.chars().count() > MAX_INPUT_CHARS {
            return Err(parse_err(input, MAX_INPUT_CHARS, "expression too long"));
        }
        let toks = lex(input)?;
        if toks.is_empty() {
            return Err(parse_err(input, 0, "empty expression"));
        }
        let mut p = Parser {
            input,
            toks,
            pos: 0,
        };
        let e = p.expr(0)?;
        if p.pos < p.toks.len() {
            return Err(match p.peek() {
                Some(Tok::Close) => p.err("unmatched ')'"),
                _ => p.err("missing operator between units (use '.' to multiply)"),
            });
        }
        Ok(e)
    }

    fn fmt_operand(&self, f: &mut fmt::Formatter<'_>, parens: bool) -> fmt::Result {
        if parens {
            write!(f, "({self})")
        } else {
            write!(f, "{self}")
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One => f.write_str("1"),
            Self::Unit(s) => f.write_str(s),
            Self::Mul(l, r) => {
                l.fmt_operand(f, matches!(**l, Self::Div(..)))?;
                f.write_str(".")?;
                r.fmt_operand(f, matches!(**r, Self::Mul(..) | Self::Div(..)))
            }
            Self::Div(l, r) => {
                l.fmt_operand(f, false)?;
                f.write_str("/")?;
                r.fmt_operand(f, matches!(**r, Self::Mul(..) | Self::Div(..)))
            }
            Self::Pow(b, n) => {
                // A symbol that ends in a digit (`Nm3`) is parenthesised so `(Nm3)^2` reads back.
                let digit_symbol =
                    matches!(&**b, Self::Unit(s) if s.ends_with(|c: char| c.is_ascii_digit()));
                b.fmt_operand(f, digit_symbol || !matches!(**b, Self::Unit(_) | Self::One))?;
                write!(f, "^{n}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Box<Expr> {
        Box::new(Expr::Unit(s.to_string()))
    }

    #[test]
    fn basic_forms() {
        assert_eq!(Expr::parse("m"), Ok(Expr::Unit("m".into())));
        assert_eq!(Expr::parse("m2"), Ok(Expr::Pow(u("m"), 2)));
        assert_eq!(Expr::parse("m^2"), Ok(Expr::Pow(u("m"), 2)));
        assert_eq!(Expr::parse("s-1"), Ok(Expr::Pow(u("s"), -1)));
        assert_eq!(Expr::parse("s^-1"), Ok(Expr::Pow(u("s"), -1)));
        assert_eq!(Expr::parse("m^+3"), Ok(Expr::Pow(u("m"), 3)));
        assert_eq!(
            Expr::parse("1/s"),
            Ok(Expr::Div(Box::new(Expr::One), u("s")))
        );
        assert_eq!(Expr::parse("kg.m"), Ok(Expr::Mul(u("kg"), u("m"))));
        assert_eq!(Expr::parse("kg*m"), Ok(Expr::Mul(u("kg"), u("m"))));
        assert_eq!(Expr::parse(" km / h "), Ok(Expr::Div(u("km"), u("h"))));
        assert_eq!(Expr::parse("\u{b0}C"), Ok(Expr::Unit("\u{b0}C".into())));
    }

    #[test]
    fn left_associative_same_precedence() {
        assert_eq!(
            Expr::parse("a/b/c"),
            Ok(Expr::Div(Box::new(Expr::Div(u("a"), u("b"))), u("c")))
        );
        assert_eq!(
            Expr::parse("a.b/c"),
            Ok(Expr::Div(Box::new(Expr::Mul(u("a"), u("b"))), u("c")))
        );
    }

    #[test]
    fn multiplication_after_division_is_ambiguous() {
        for bad in ["J/mol.K", "kg/m*s", "a/b/c.d"] {
            assert!(
                matches!(Expr::parse(bad), Err(Error::AmbiguousExpression { .. })),
                "{bad} must be ambiguous"
            );
        }
        // Parentheses resolve it, in both directions.
        assert!(Expr::parse("J/(mol.K)").is_ok());
        assert!(Expr::parse("(J/mol).K").is_ok());
        // Inside parentheses the division flag is reset.
        assert!(Expr::parse("a/(b.c).d").is_err());
        assert!(Expr::parse("a/(b.c)").is_ok());
    }

    #[test]
    fn exponents_bind_tightest() {
        assert_eq!(
            Expr::parse("m/s2"),
            Ok(Expr::Div(u("m"), Box::new(Expr::Pow(u("s"), 2))))
        );
        assert_eq!(
            Expr::parse("(m/s)^2"),
            Ok(Expr::Pow(Box::new(Expr::Div(u("m"), u("s"))), 2))
        );
    }

    #[test]
    fn rejects_malformed_input() {
        let bad = [
            "", "  ", "/", "m/", "(m", "m)", "()", "m..s", "m^", "m^x", "m s", "m 2", "2m", "10",
            "(m/s)2", "m-", "m^2^3", "m$", "m/s +", "1 2", "^2", "m(s)",
        ];
        for b in bad {
            assert!(Expr::parse(b).is_err(), "{b:?} should be rejected");
        }
    }

    #[test]
    fn rejects_huge_input_and_nesting() {
        let long = "m.".repeat(MAX_INPUT_CHARS);
        assert!(Expr::parse(&long).is_err());
        let deep = format!(
            "{}m{}",
            "(".repeat(MAX_DEPTH + 2),
            ")".repeat(MAX_DEPTH + 2)
        );
        assert!(Expr::parse(&deep).is_err());
        let ok = format!("{}m{}", "(".repeat(MAX_DEPTH), ")".repeat(MAX_DEPTH));
        assert!(Expr::parse(&ok).is_ok());
        assert!(Expr::parse("m^99999999999").is_err());
    }

    #[test]
    fn printer_round_trips_hand_picked_trees() {
        let trees = [
            Expr::Mul(Box::new(Expr::Div(u("a"), u("b"))), u("c")),
            Expr::Div(u("a"), Box::new(Expr::Div(u("b"), u("c")))),
            Expr::Div(
                Box::new(Expr::Mul(u("a"), u("b"))),
                Box::new(Expr::Mul(u("c"), u("d"))),
            ),
            Expr::Pow(Box::new(Expr::Pow(u("m"), 2)), 3),
            Expr::Pow(Box::new(Expr::Mul(u("a"), u("b"))), -2),
            Expr::Mul(u("a"), Box::new(Expr::Mul(u("b"), u("c")))),
            Expr::Pow(Box::new(Expr::One), 4),
            Expr::Div(Box::new(Expr::One), Box::new(Expr::Pow(u("s"), 2))),
        ];
        for t in trees {
            let printed = t.to_string();
            assert_eq!(Expr::parse(&printed), Ok(t), "printed as {printed}");
        }
    }

    #[test]
    fn error_positions_point_at_the_problem() {
        match Expr::parse("m/s$") {
            Err(Error::Parse { position, .. }) => assert_eq!(position, 3),
            other => panic!("unexpected {other:?}"),
        }
    }
}
