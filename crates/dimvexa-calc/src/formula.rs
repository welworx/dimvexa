// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equations as text, evaluated with units (ADR-030): `"0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)"`,
//! or an equation over named values, `"Cd * A * sqrt(2 * dp / rho)"`.
//!
//! ```
//! use dimvexa_calc::Formula;
//!
//! let reg = dimvexa::registry()?;
//!
//! // quantities written in the equation
//! let f = Formula::new(&reg);
//! let m = f.eval("2 kip/ft * (20 ft)^2 / 8")?;
//! assert_eq!(f.f64_in(&m, "kN*m")?, 135.581_794_833_140_04);
//!
//! // or an equation over named values
//! let f = Formula::new(&reg).var("dp", "50 kPa")?.var("rho", "998 kg/m^3")?;
//! let v = f.eval("sqrt(2 * dp / rho)")?;
//! assert!(!v.is_exact()); // an approximate value, shown with ≈ (ADR-029)
//! assert!((f.f64_in(&v, "m/s")? - (100_000.0_f64 / 998.0).sqrt()).abs() < 1e-12);
//! assert!(f.eval("sqrt(4 * dp / dp)")?.is_exact());
//! # Ok::<(), dimvexa_core::Error>(())
//! ```
//!
//! # Grammar
//!
//! * **Numbers** (`2`, `0.5`, `1e-3`), **quantities** (a number and a unit: `10 cm2`,
//!   `998 kg/m3`, `8.314 J/(mol*K)`, `20 degC`), **names** of values given with
//!   [`Formula::var`], the constant `pi`, the SI constants and, with [`Formula::from_loaded`],
//!   the constants of the configs.
//! * `+ - * /`, a unary minus, parentheses, and the power `^` with an exact number without a
//!   unit as exponent, written as a number, a name or an expression: `L^4`, `v^-2`, `x^0.5`,
//!   `R^(2/3)`, `x^n`, `2^(1+1)` (`2^3^2` is `2^(3^2)`). `2 pi`, `2 pi r` and `pi r^2` multiply.
//!   `m²`, `m³`, `s⁻¹`, `·`, `×`, `÷`, `−`, tabs and newlines are read.
//! * The functions `sqrt`, `cbrt`, `abs`, `hypot(x, y)`, `min(x, y, ...)`, `max(x, y, ...)`,
//!   `pow(x, y)` (the same as `x^y`), `floor`, `ceil`, `round` (to a whole number, the unit stays),
//!   `ln`, `exp`, `log10` (`lg`), `log2` (`lb`), `log(x, base)`, `sin`, `cos`, `tan`, `asin`,
//!   `acos`, `atan`, `atan2(y, x)`, `sinh`, `cosh`,
//!   `tanh`, `asinh`, `acosh` and `atanh`.
//!   `log(x)` alone is ambiguous and refused. Any other well-known function (`pow`, `floor`,
//!   `round`) is a `refused` error that says `'floor' is not implemented`, and how to write it
//!   where there is a way (`x^p`); an unknown name says `unknown function`.
//!
//! **A unit is the text that follows a number without a space inside it**: `998 kg/m3` is one
//! quantity, and `50 kPa / 998 kg/m3` is a quotient of two. So write a space on both sides of
//! an operator between quantities, and no space inside a unit (`J/(mol*K)`, not `J / (mol * K)`).
//! A `*` or `/` that is directly followed by a unit symbol belongs to the unit; one followed by
//! a digit does not (`50 kPa/998 kg/m3` is also a quotient).
//!
//! Every step is an operation of [`Math`], so a result is exact whenever the mathematics is exact
//! and an explicitly approximate [`Number`] otherwise (ADR-029). Read it in a unit with
//! [`Formula::f64_in`] or [`Formula::to_unit`]; [`Formula::eval_exact`] refuses an approximate
//! result. The input is bounded (`MAX_EQUATION_CHARS`, `MAX_DEPTH`): hostile text is an error.

use dimvexa_core::{Error, Kind, Quantity, Ratio, Registry, Result, UnitExpr};

use crate::{Math, Number};

/// The longest equation accepted, in characters.
pub const MAX_EQUATION_CHARS: usize = 4096;

/// The most names one [`Formula`] binds (besides the built-in constants): binding is linear in their number, and an
/// equation of 4096 characters cannot use more than a few hundred.
pub const MAX_VARS: usize = 512;

/// How deeply parentheses and function calls may nest.
const MAX_DEPTH: usize = 64;

/// The step of [`Formula::calculate`] that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcStep {
    /// Reading and evaluating the equation (and describing the result).
    Expression,
    /// The conversion to the unit asked for.
    Unit,
    /// The choice of an SI prefix.
    Prefix,
}

/// A failure of [`Formula::calculate`]: the step, and the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalcError {
    /// Which step failed.
    pub step: CalcStep,
    /// The error, with its stable code.
    pub error: Error,
}

/// A set of named quantities and an evaluator for equations over them.
#[derive(Debug)]
pub struct Formula<'r> {
    reg: &'r Registry,
    math: Math<'r>,
    vars: Vec<(String, Number)>,
    /// How many of `vars` are built in (`pi` and the SI constants), before any the caller binds.
    builtin: usize,
    /// The names the caller bound, in order.
    given: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// A number without a unit.
    Num(String),
    /// A number and the unit written after it: `("998", "kg/m3")`.
    Qty(String, String),
    Id(String),
    Op(char),
    /// The multiplication in `2 pi`: no operator was written.
    Implicit,
}

fn parse_error(input: &str, position: usize, message: &str) -> Error {
    Error::Parse {
        input: input.to_owned(),
        position,
        message: message.to_owned(),
    }
}

const fn next(i: usize) -> usize {
    i.saturating_add(1)
}

/// The text of `chars[from..to]` (empty if the range is not valid).
fn text(chars: &[char], from: usize, to: usize) -> String {
    chars
        .get(from..to)
        .map(|s| s.iter().collect())
        .unwrap_or_default()
}

/// The end of the number that starts at `i`: digits, a point, and an exponent that has digits.
fn scan_number(chars: &[char], start: usize) -> usize {
    let mut i = start;
    while chars
        .get(i)
        .is_some_and(|d| d.is_ascii_digit() || *d == '.')
    {
        i = next(i);
    }
    if chars.get(i).is_some_and(|e| *e == 'e' || *e == 'E') {
        let mut j = next(i);
        if chars.get(j).is_some_and(|s| *s == '-' || *s == '+') {
            j = next(j);
        }
        if chars.get(j).is_some_and(char::is_ascii_digit) {
            while chars.get(j).is_some_and(char::is_ascii_digit) {
                j = next(j);
            }
            return j;
        }
    }
    i
}

/// A character that can start a unit symbol.
fn starts_unit(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '°'
}

/// Whether `.to(` starts at `dot`: the conversion `value.to(UNIT)`.
fn conversion_follows(chars: &[char], dot: usize) -> bool {
    chars.get(dot.saturating_add(1)..dot.saturating_add(4)) == Some(&['t', 'o', '('][..])
}

/// The end of `^(2)` or `^(-1)` that starts at `caret` (the index after the closing
/// parenthesis): a whole-number power written in parentheses.
fn power_in_parentheses(chars: &[char], caret: usize) -> Option<usize> {
    if chars.get(caret) != Some(&'^') || chars.get(next(caret)) != Some(&'(') {
        return None;
    }
    let mut j = next(next(caret));
    if chars.get(j) == Some(&'-') {
        j = next(j);
    }
    let digits = j;
    while chars.get(j).is_some_and(char::is_ascii_digit) {
        j = next(j);
    }
    (j > digits && chars.get(j) == Some(&')')).then(|| next(j))
}

/// A unit text with `^(2)` written as `^2` (the registry reads the second form).
fn plain_powers(unit: &str) -> String {
    let chars: Vec<char> = unit.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if let Some(end) = power_in_parentheses(&chars, i) {
            out.push('^');
            let inside = i.saturating_add(2)..end.saturating_sub(1);
            out.extend(chars.get(inside).unwrap_or_default());
            i = end;
        } else {
            out.push(c);
            i = next(i);
        }
    }
    out
}

/// The end of the unit that starts at `start`: symbols, digits (`m3`), `^` powers, `.`, and a
/// `*` or `/` only when a unit symbol follows it; parentheses must balance. A space ends it.
fn scan_unit(chars: &[char], start: usize) -> usize {
    let mut j = start;
    let mut depth = 0_usize;
    while let Some(&c) = chars.get(j) {
        let following = chars.get(next(j)).copied();
        let atom = |n: Option<char>| n.is_some_and(|n| starts_unit(n) || n == '(');
        let take = match c {
            c if c.is_alphanumeric() || c == '_' || c == '°' => true,
            '(' => {
                depth = next(depth);
                true
            }
            ')' => {
                if depth > 0 {
                    depth = depth.saturating_sub(1);
                    true
                } else {
                    false
                }
            }
            '*' | '/' => atom(following),
            // `20 degC.to(K)`: the dot before `to(` is the conversion, not a product of units
            '.' => following.is_some_and(starts_unit) && !conversion_follows(chars, j),
            '^' => {
                // `m^2`, `m^-1` and `m^(2)`, `m^(-1)`: the power belongs to the unit
                if let Some(end) = power_in_parentheses(chars, j) {
                    j = end;
                    continue;
                }
                following.is_some_and(|n| n.is_ascii_digit() || n == '-')
            }
            '-' => j > start && chars.get(j.saturating_sub(1)) == Some(&'^'),
            _ => false,
        };
        if !take {
            break;
        }
        j = next(j);
    }
    j
}

/// The characters an engineer writes that mean something simpler: tabs, newlines and the
/// no-break space are spaces; `·`, `⋅` and `×` are `*`; `÷` is `/`; the minus sign `−` is `-`;
/// `²`, `³` and `⁻¹` are `^2`, `^3` and `^-1`. (An error position counts the characters after
/// this rewrite.)
fn normalise(input: &str) -> Vec<char> {
    // a superscript digit, as its digit
    let superscript = |c: char| match c {
        '⁰' => Some('0'),
        '¹' => Some('1'),
        '²' => Some('2'),
        '³' => Some('3'),
        '⁴' => Some('4'),
        '⁵' => Some('5'),
        '⁶' => Some('6'),
        '⁷' => Some('7'),
        '⁸' => Some('8'),
        '⁹' => Some('9'),
        _ => None,
    };
    let mut out = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if superscript(c).is_some() || c == '⁻' {
            // a superscript belongs to what stands before it, spaces or not: `2 m ²` is `2 m²`;
            // a run of superscripts (`s⁻²`, `m¹⁰`) is one power
            while out.last() == Some(&' ') {
                out.pop();
            }
            out.push('^');
            let mut current = c;
            loop {
                out.push(superscript(current).unwrap_or('-'));
                match chars.peek().copied() {
                    Some(n) if n == '⁻' || superscript(n).is_some() => {
                        chars.next();
                        current = n;
                    }
                    _ => break,
                }
            }
            continue;
        }
        match c {
            '\t' | '\n' | '\r' | '\u{a0}' | '\u{2009}' | '\u{202f}' => out.push(' '),
            '·' | '⋅' | '×' => out.push('*'),
            '÷' => out.push('/'),
            '−' => out.push('-'),
            c => out.push(c),
        }
    }
    out
}

fn lex(input: &str) -> Result<Vec<(usize, Tok)>> {
    if input
        .chars()
        .take(MAX_EQUATION_CHARS.saturating_add(1))
        .count()
        > MAX_EQUATION_CHARS
    {
        return Err(parse_error(input, 0, "the equation is too long"));
    }
    let chars = normalise(input);
    if chars.len() > MAX_EQUATION_CHARS.saturating_mul(3) {
        return Err(parse_error(input, 0, "the equation is too long"));
    }
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let start = i;
        if c.is_whitespace() {
            i = next(i);
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(next(i)).is_some_and(char::is_ascii_digit))
        {
            i = scan_number(&chars, i);
            let number = text(&chars, start, i);
            let mut after = i;
            while chars.get(after).is_some_and(|s| *s == ' ') {
                after = next(after);
            }
            // `2 pi` is 2 times the constant, not a quantity of the unit `pi`
            let mut word_end = after;
            while chars
                .get(word_end)
                .is_some_and(|d| d.is_alphanumeric() || *d == '_')
            {
                word_end = next(word_end);
            }
            if text(&chars, after, word_end) == "pi" {
                out.push((start, Tok::Num(number)));
                out.push((after, Tok::Implicit));
                out.push((after, Tok::Id("pi".to_owned())));
                i = word_end;
                continue;
            }
            // a unit starts with a symbol, or with a parenthesis that opens on one: `998 (kg/m3)`
            let opens_unit = chars.get(after) == Some(&'(')
                && chars.get(next(after)).copied().is_some_and(starts_unit);
            // a reciprocal unit: `5 1/s`, `60 1/min`
            let reciprocal = chars.get(after) == Some(&'1') && {
                let mut j = next(after);
                while chars.get(j) == Some(&' ') {
                    j = next(j);
                }
                let slash = chars.get(j) == Some(&'/');
                j = next(j);
                while chars.get(j) == Some(&' ') {
                    j = next(j);
                }
                slash
                    && chars
                        .get(j)
                        .copied()
                        .is_some_and(|c| starts_unit(c) || c == '(')
            };
            if chars.get(after).copied().is_some_and(starts_unit) || opens_unit || reciprocal {
                let end = scan_unit(&chars, after);
                out.push((
                    start,
                    Tok::Qty(number, plain_powers(&text(&chars, after, end))),
                ));
                i = end;
            } else {
                out.push((start, Tok::Num(number)));
            }
        } else if c.is_alphabetic() || c == '_' || c == '°' {
            while chars
                .get(i)
                .is_some_and(|d| d.is_alphanumeric() || *d == '_' || *d == '°')
            {
                i = next(i);
            }
            out.push((start, Tok::Id(text(&chars, start, i))));
        } else if "+-*/^(),.".contains(c) {
            out.push((start, Tok::Op(c)));
            i = next(i);
        } else {
            return Err(parse_error(input, start, "unexpected character"));
        }
    }
    Ok(out)
}

/// Whether the token ends an operand (a number, a name, a quantity or a closing parenthesis).
const fn ends_operand(t: &Tok) -> bool {
    matches!(t, Tok::Num(_) | Tok::Qty(..) | Tok::Id(_) | Tok::Op(')'))
}

/// A term made only of plain numbers and `pi` followed by `/ N unit` has the unit as a whole:
/// `1 / 2 kg` is `0.5 kg`, `5 * 1 / 2 s` is `2.5 s`, `(2 / 3) / 4 s` and `1 / -2 s` too, written
/// as `((...) / N) * 1 unit`. Anywhere else (`10 m / 2 s`, `x / 2 s`, `sin(1) / 2 s`) a unit
/// belongs to its own number and divides. `true` when `out` was rewritten.
fn number_over_quantity(
    out: &mut Vec<(usize, Tok)>,
    (number, unit): (&String, &String),
    at: usize,
) -> bool {
    // `/ N unit` or `/ -N unit`
    let negative = matches!(out.last(), Some((_, Tok::Op('-'))))
        && out
            .len()
            .checked_sub(2)
            .and_then(|i| out.get(i))
            .is_some_and(|(_, t)| *t == Tok::Op('/'));
    let slash = if negative {
        out.len().checked_sub(1)
    } else {
        Some(out.len())
    };
    let Some(slash) = slash else {
        return false;
    };
    let before = slash.checked_sub(1).and_then(|i| out.get(i));
    if !matches!(before, Some((_, Tok::Op('/')))) {
        return false;
    }
    // the start of the term: after the last `(`, `,` or binary `+`/`-` outside parentheses
    let mut start = 0;
    let mut depth = 0_usize;
    for (i, (_, t)) in out.iter().enumerate().take(slash).rev() {
        match t {
            Tok::Op(')') => depth = depth.saturating_add(1),
            Tok::Op('(' | ',') if depth == 0 => {
                start = next(i);
                break;
            }
            Tok::Op('(') => depth = depth.saturating_sub(1),
            Tok::Op('+' | '-') if depth == 0 => {
                let operand = i
                    .checked_sub(1)
                    .and_then(|j| out.get(j))
                    .is_some_and(|(_, p)| ends_operand(p));
                if operand {
                    start = next(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let plain = out.get(start..slash).is_some_and(|term| {
        !term.is_empty()
            && term.iter().all(|(_, t)| match t {
                Tok::Num(_) | Tok::Implicit => true,
                Tok::Id(name) => name == "pi",
                Tok::Op(c) => "*/^+-()".contains(*c),
                Tok::Qty(..) => false,
            })
    });
    if !plain {
        return false;
    }
    let minus = negative.then(|| out.pop()).flatten();
    out.insert(start, (at, Tok::Op('(')));
    if let Some((_, sign)) = minus {
        out.push((at, sign));
    }
    for t in [
        Tok::Num(number.clone()),
        Tok::Op(')'),
        Tok::Op('*'),
        Tok::Qty("1".to_owned(), unit.clone()),
    ] {
        out.push((at, t));
    }
    true
}

/// The functions the evaluator implements (ADR-029), with the number of arguments: `0` means
/// two or more (`min`, `max`).
const IMPLEMENTED: [(&str, usize); 30] = [
    ("sqrt", 1),
    ("cbrt", 1),
    ("abs", 1),
    ("hypot", 2),
    ("pow", 2),
    ("floor", 1),
    ("ceil", 1),
    ("round", 1),
    ("min", 0),
    ("max", 0),
    ("ln", 1),
    ("exp", 1),
    ("log10", 1),
    ("lg", 1),
    ("log2", 1),
    ("lb", 1),
    ("log", 2),
    ("sin", 1),
    ("cos", 1),
    ("tan", 1),
    ("asin", 1),
    ("acos", 1),
    ("atan", 1),
    ("atan2", 2),
    ("sinh", 1),
    ("cosh", 1),
    ("tanh", 1),
    ("asinh", 1),
    ("acosh", 1),
    ("atanh", 1),
];

fn function_list() -> String {
    let names: Vec<&str> = IMPLEMENTED.iter().map(|(name, _)| *name).collect();
    names.join(", ")
}

fn implemented_names() -> Vec<String> {
    IMPLEMENTED
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect()
}

struct Parser<'a, 'r> {
    src: &'a str,
    toks: Vec<(usize, Tok)>,
    pos: usize,
    depth: usize,
    f: &'a Formula<'r>,
    /// The names the equation read.
    used: Vec<String>,
    /// The text the token positions index (the input after `normalise`).
    norm: Vec<char>,
    /// How many powers are being read (`2^3^2` is two).
    pow_depth: usize,
    /// The unit written after the exponent of the outermost power (`2^3 m`), with its position.
    tail_unit: Option<(String, usize)>,
}

impl Parser<'_, '_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }

    fn here(&self) -> usize {
        self.toks.get(self.pos).map_or(self.src.len(), |(p, _)| *p)
    }

    const fn advance(&mut self) {
        self.pos = next(self.pos);
    }

    fn eat(&mut self, op: char) -> bool {
        if self.peek() == Some(&Tok::Op(op)) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Enters a parenthesis or a call; nesting is bounded.
    fn enter(&mut self) -> Result<()> {
        self.depth = next(self.depth);
        if self.depth > MAX_DEPTH {
            return Err(parse_error(self.src, self.here(), "nested too deeply"));
        }
        Ok(())
    }

    const fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn expr(&mut self) -> Result<Number> {
        let mut acc = self.term()?;
        loop {
            if self.eat('+') {
                let rhs = self.term()?;
                acc = self.f.math.add(&acc, &rhs)?;
            } else if self.eat('-') {
                let rhs = self.term()?;
                acc = self.f.math.sub(&acc, &rhs)?;
            } else {
                return Ok(acc);
            }
        }
    }

    fn term(&mut self) -> Result<Number> {
        let mut acc = self.unary()?;
        // the last operator of this chain was `/`: a product written without an operator right
        // after it (`a / 2 pi`) is read two ways, and is refused
        let mut divided = false;
        loop {
            if self.eat('*') {
                let rhs = self.unary()?;
                acc = self.f.math.mul(&acc, &rhs)?;
                divided = false;
            } else if self.eat('/') {
                let rhs = self.unary()?;
                acc = self.f.math.div(&acc, &rhs)?;
                divided = true;
            } else if self.peek() == Some(&Tok::Implicit) || self.after_pi_name() {
                // `2 pi`, `2 pi r` and `pi r^2`: pi multiplies the number or names around it
                if divided {
                    return Err(parse_error(
                        self.src,
                        self.here(),
                        "a division followed by a product written with `pi` and no operator \
                         (`a / 2 pi`) can be read (a / 2) * pi or a / (2 * pi): write the \
                         parentheses, a / (2 pi) or (a / 2) * pi",
                    ));
                }
                if self.peek() == Some(&Tok::Implicit) {
                    self.advance();
                }
                let rhs = self.unary()?;
                acc = self.f.math.mul(&acc, &rhs)?;
            } else {
                return Ok(acc);
            }
        }
    }

    /// Whether the next token is a name that follows a run of names that starts with `pi`.
    fn after_pi_name(&self) -> bool {
        if !matches!(self.peek(), Some(Tok::Id(_))) {
            return false;
        }
        let mut start = self.pos;
        while start > 0
            && matches!(
                self.toks.get(start.saturating_sub(1)).map(|(_, t)| t),
                Some(Tok::Id(_))
            )
        {
            start = start.saturating_sub(1);
        }
        start < self.pos && matches!(self.toks.get(start), Some((_, Tok::Id(n))) if n == "pi")
    }

    fn unary(&mut self) -> Result<Number> {
        let mut minus = 0_usize;
        while self.eat('-') {
            minus = next(minus);
        }
        // `-40 degC` is the point minus forty degrees Celsius, not the negation of a point
        if minus > 0 {
            if let Some(Tok::Qty(number, unit)) = self.peek().cloned() {
                let at = self.here();
                let positive = self
                    .f
                    .reg
                    .quantity(&format!("{number} {unit}"))
                    .map(Number::from)
                    .ok();
                let point = positive
                    .as_ref()
                    .is_some_and(|p| !p.unit().offset().is_zero());
                if point && !matches!(self.toks.get(next(self.pos)), Some((_, Tok::Op('^')))) {
                    self.advance();
                    let negative = self
                        .f
                        .reg
                        .quantity(&format!("-{number} {unit}"))
                        .map(Number::from)
                        .map_err(|e| match e {
                            Error::Parse { .. } => parse_error(self.src, at, "a value is expected"),
                            other => other,
                        })?;
                    let mut value = self.conversions(negative)?;
                    for _ in 1..minus {
                        value = self.f.math.neg(&value)?;
                    }
                    return Ok(value);
                }
            }
        }
        let mut value = self.power()?;
        for _ in 0..minus {
            value = self.f.math.neg(&value)?;
        }
        Ok(value)
    }

    /// The exponent after `^` as `(p, q)`: an exact dimensionless number written as a number
    /// (`3`, `-2`, `0.5`), a fraction in parentheses (`(2/3)`), a name (`n`) or any expression
    /// in parentheses (`(1 + 1)`); `2^3^2` is `2^(3^2)`.
    fn exponent(&mut self) -> Result<(i32, i32)> {
        let at = self.here();
        let negative = self.eat('-');
        let value = if let Some(Tok::Qty(number, unit)) = self.peek().cloned() {
            // `2^3 m` is `(2^3) m`: the exponent is the number, the unit belongs to the power
            // (a unit cannot be part of an exponent, so there is no other reading)
            self.advance();
            self.tail_unit = Some((unit, at));
            self.f
                .reg
                .quantity(&format!("{number} 1"))
                .map(Number::from)?
        } else {
            self.power()?
        };
        let (num, den) = self.exponent_parts(&value, at)?;
        Ok((if negative { num.saturating_neg() } else { num }, den))
    }

    /// The fraction `(p, q)` of an exponent that is an exact number without a unit.
    fn exponent_parts(&self, value: &Number, at: usize) -> Result<(i32, i32)> {
        let unit = value.unit();
        let ratio = match value {
            Number::Exact(q) if unit.kind() == Kind::Plain && unit.dim().is_dimensionless() => {
                q.value().checked_mul(unit.factor())?
            }
            Number::Exact(_) => {
                return Err(parse_error(
                    self.src,
                    at,
                    "the exponent must be a plain number without a unit",
                ));
            }
            Number::Approx(_) => {
                return Err(parse_error(
                    self.src,
                    at,
                    "the exponent must be exact (an approximate value such as sqrt(2) has no \
                     fraction)",
                ));
            }
        };
        // `1e-50` is stored as a digit and a power of ten: fold it before reading the fraction,
        // and refuse an exponent that has no `i32` fraction (it is never silently truncated)
        let ratio = ratio
            .folded()
            .ok()
            .filter(|r| r.pi_exp() == 0)
            .ok_or_else(|| {
                parse_error(
                    self.src,
                    at,
                    "the exponent must be a fraction: it is too large or too precise, or it has pi \
                     (an irrational exponent is not exact)",
                )
            })?;
        let num = i32::try_from(ratio.num())
            .map_err(|_| parse_error(self.src, at, "the exponent is too large"))?;
        let den = i32::try_from(ratio.den())
            .map_err(|_| parse_error(self.src, at, "the exponent is too precise"))?;
        Ok((num, den))
    }

    fn power(&mut self) -> Result<Number> {
        let base = self.atom()?;
        if !self.eat('^') {
            return Ok(base);
        }
        self.pow_depth = next(self.pow_depth);
        self.enter()?;
        let exponent = self.exponent();
        self.leave();
        self.pow_depth = self.pow_depth.saturating_sub(1);
        let (num, den) = exponent?;
        let power = if den == 1 {
            self.f.math.pow(&base, num)?
        } else {
            self.f.math.root(&base, num, den)?
        };
        // the unit after the exponent of the outermost power multiplies the whole power
        if self.pow_depth == 0 {
            if let Some((unit, at)) = self.tail_unit.take() {
                let one = self.quantity_of("1", &unit, at)?;
                return self.f.math.mul(&power, &one);
            }
        }
        Ok(power)
    }

    fn atom(&mut self) -> Result<Number> {
        let value = self.value()?;
        self.conversions(value)
    }

    fn value(&mut self) -> Result<Number> {
        let at = self.here();
        match self.peek().cloned() {
            Some(Tok::Num(text)) => {
                self.advance();
                self.f
                    .reg
                    .quantity(&format!("{text} 1"))
                    .map(Number::from)
                    .map_err(|e| match e {
                        Error::Parse { message, .. } => parse_error(
                            self.src,
                            at,
                            &format!("'{text}' is not a number ({message}); check the dots"),
                        ),
                        other => other,
                    })
            }
            Some(Tok::Qty(number, unit)) => {
                self.advance();
                self.quantity_of(&number, &unit, at)
            }
            Some(Tok::Id(name)) => {
                self.advance();
                if self.peek() == Some(&Tok::Op('(')) {
                    return self.call(&name);
                }
                let found = self
                    .f
                    .vars
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, q)| q.clone())
                    .ok_or_else(|| {
                        let ambiguous = match self.f.reg.parse(&name) {
                            Err(Error::AmbiguousSymbol { candidates, .. }) => Some(candidates),
                            _ => None,
                        };
                        let message = if let Some(candidates) = ambiguous {
                            format!(
                                "'{name}' is an ambiguous unit symbol ({}) and not a variable \
                                 of this equation (define it with --var {name}=VALUE)",
                                candidates.join(", ")
                            )
                        } else if self.f.reg.parse(&name).is_ok() {
                            format!(
                                "'{name}' is a unit, and a unit needs a number in front \
                                 of it (1 {name})"
                            )
                        } else {
                            format!(
                                "'{name}' is not a variable of this equation (define it with \
                                 --var {name}=VALUE) and not a unit"
                            )
                        };
                        parse_error(self.src, at, &message)
                    })?;
                if !self.used.contains(&name) {
                    self.used.push(name);
                }
                Ok(found)
            }
            Some(Tok::Op('(')) => {
                self.advance();
                self.enter()?;
                let inner = self.expr()?;
                self.leave();
                if self.eat(')') {
                    Ok(inner)
                } else {
                    Err(parse_error(self.src, self.here(), "')' is expected"))
                }
            }
            _ => Err(parse_error(self.src, at, "a value is expected")),
        }
    }

    /// The quantity `number unit`. A unit that does not exist is most likely a variable that was
    /// not defined: the error says what the text is not, never only "unknown unit".
    fn quantity_of(&self, number: &str, unit: &str, at: usize) -> Result<Number> {
        self.f
            .reg
            .quantity(&format!("{number} {unit}"))
            .map(Number::from)
            .map_err(|e| match e {
                // a name with no close unit is most likely a variable that was not
                // defined: say what it is not, never only "unknown unit"
                Error::UnknownUnit {
                    symbol,
                    suggestions,
                } => {
                    // as written: `r2`, not the `r` the unit parser stripped it to
                    let written = unit
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .find(|run| run.starts_with(symbol.as_str()))
                        .map_or_else(|| symbol.clone(), str::to_owned);
                    if self.f.vars.iter().any(|(n, _)| *n == written) {
                        // the name is defined: the unit text swallowed it (`2 m/r`)
                        let spaced = if unit.starts_with('(') {
                            format!("{number} * {unit}")
                        } else {
                            format!("{number} {}", unit.replace('/', " / ").replace('*', " * "))
                        };
                        return parse_error(
                            self.src,
                            at,
                            &format!(
                                "'{written}' is a variable, not part of the unit in \
                                         '{number} {unit}': write the operator with spaces, \
                                         as in '{spaced}'"
                            ),
                        );
                    }
                    let did_you_mean = if suggestions.is_empty() {
                        String::new()
                    } else {
                        format!(" (did you mean the unit {}?)", suggestions.join(" or "))
                    };
                    parse_error(
                        self.src,
                        at,
                        &format!(
                            "'{written}' in '{number} {unit}' is not a unit and not a \
                                     variable{did_you_mean}; a variable is defined with \
                                     --var {written}=VALUE"
                        ),
                    )
                }
                // `2 sin(30 deg)`: a function or name after a number is read as a unit
                Error::Parse { ref message, .. }
                    if message.contains("missing operator between units") =>
                {
                    let lower = unit.to_ascii_lowercase();
                    let advice = if lower.contains(".to(") && !unit.contains(".to(") {
                        "the conversion is written .to(UNIT), in lower case".to_owned()
                    } else {
                        let name = unit
                            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                            .next()
                            .unwrap_or_default();
                        format!(
                            "a function or a name after a number needs an operator: \
                             write {number} * {name}(...)"
                        )
                    };
                    parse_error(self.src, at, &format!("'{number} {unit}': {advice}"))
                }
                // the registry quotes its own text (`..5 1`): quote the equation instead
                Error::Parse { message, .. } => parse_error(
                    self.src,
                    at,
                    &format!("this is not a number with a unit ({message}); check the dots (a conversion is written (3 m).to(UNIT))"),
                ),
                other => other,
            })
    }

    /// `value.to(UNIT)`, repeated: the value in another unit. The unit is written as it is
    /// everywhere else (`K`, `degC`, `kW*h`, `J/(mol*K)`), not as an expression. The dimension
    /// must match, so the user decides what the conversion is and the units are checked
    /// against it.
    fn conversions(&mut self, mut value: Number) -> Result<Number> {
        while self.peek() == Some(&Tok::Op('.')) {
            let named = matches!(self.toks.get(next(self.pos)), Some((_, Tok::Id(n))) if n == "to")
                && matches!(self.toks.get(next(next(self.pos))), Some((_, Tok::Op('('))));
            if !named {
                return Ok(value);
            }
            let at = self.here();
            self.advance(); // the '.'
            self.advance(); // `to`
            self.advance(); // the '('
            let start = self.pos;
            let mut depth = 0_usize;
            while let Some(tok) = self.peek() {
                match tok {
                    Tok::Op('(') => depth = next(depth),
                    Tok::Op(')') if depth == 0 => break,
                    Tok::Op(')') => depth = depth.saturating_sub(1),
                    _ => {}
                }
                self.advance();
            }
            let from = self.toks.get(start).map(|(p, _)| *p);
            let to = self.toks.get(self.pos).map(|(p, _)| *p);
            let unit: String = match (from, to) {
                (Some(from), Some(to)) => {
                    self.norm.get(from..to).unwrap_or_default().iter().collect()
                }
                _ => String::new(),
            };
            if unit.trim().is_empty() {
                return Err(parse_error(
                    self.src,
                    at,
                    "to takes a unit: 20 degC.to(K), (a + b).to(ft)",
                ));
            }
            self.advance(); // the ')'
            value = self.f.math.to_unit(&value, unit.trim())?;
        }
        Ok(value)
    }

    /// `name(argument, ...)`.
    fn call(&mut self, name: &str) -> Result<Number> {
        let Some((_, arity)) = IMPLEMENTED.iter().find(|(n, _)| *n == name) else {
            return Err(Error::Refused {
                reason: format!(
                    "unknown function '{name}' (the functions are {})",
                    function_list()
                ),
                suggestions: implemented_names(),
            });
        };
        let at = self.here();
        self.advance(); // the '('
        self.enter()?;
        let mut args = vec![self.expr()?];
        while self.eat(',') {
            args.push(self.expr()?);
        }
        self.leave();
        if !self.eat(')') {
            return Err(parse_error(self.src, self.here(), "')' is expected"));
        }
        if name == "log" && args.len() == 1 {
            // `log(x)` is the one name that is ambiguous in itself: natural or base 10?
            return Err(Error::Refused {
                reason: "log(x) is ambiguous (natural or base 10?): write ln(x) for the natural \
                         logarithm, lg(x) or log10(x) for base 10, lb(x) or log2(x) for base 2, \
                         or log(x, base)"
                    .to_owned(),
                suggestions: ["ln", "lg", "log10", "lb", "log2", "log"]
                    .map(str::to_owned)
                    .to_vec(),
            });
        }
        let wrong = match arity {
            0 => args.len() < 2,
            n => args.len() != *n,
        };
        if wrong {
            let wanted = if *arity == 0 {
                "two or more".to_owned()
            } else {
                arity.to_string()
            };
            return Err(parse_error(
                self.src,
                at,
                &format!("{name} takes {wanted} argument(s), not {}", args.len()),
            ));
        }
        self.apply(name, &args, at)
    }

    fn apply(&self, name: &str, args: &[Number], at: usize) -> Result<Number> {
        let math = self.f.math;
        let missing = || parse_error(self.src, at, "an argument is expected");
        let one = args.first().ok_or_else(missing)?;
        let two = args.get(1);
        match (name, two) {
            ("sqrt", _) => math.sqrt(one),
            ("cbrt", _) => math.cbrt(one),
            ("abs", _) => math.abs(one),
            ("ln", _) => math.ln(one),
            ("exp", _) => math.exp(one),
            ("log10" | "lg", _) => math.log10(one),
            ("log2" | "lb", _) => math.log2(one),
            ("sin", _) => math.sin(one),
            ("cos", _) => math.cos(one),
            ("tan", _) => math.tan(one),
            ("asin", _) => math.asin(one),
            ("acos", _) => math.acos(one),
            ("atan", _) => math.atan(one),
            ("atan2", Some(x)) => math.atan2(one, x),
            ("sinh", _) => math.sinh(one),
            ("cosh", _) => math.cosh(one),
            ("tanh", _) => math.tanh(one),
            ("asinh", _) => math.asinh(one),
            ("acosh", _) => math.acosh(one),
            ("atanh", _) => math.atanh(one),
            ("hypot", Some(other)) => math.hypot(one, other),
            ("pow", Some(exponent)) => {
                // the same as `x^y`: an exact exponent without a unit
                let (num, den) = self.exponent_parts(exponent, at)?;
                if den == 1 {
                    math.pow(one, num)
                } else {
                    math.root(one, num, den)
                }
            }
            ("floor", _) => math.floor(one),
            ("ceil", _) => math.ceil(one),
            ("round", _) => math.round(one),
            ("log", Some(base)) => math.log(one, base),
            ("min" | "max", _) => {
                let mut acc = one.clone();
                for following in args.iter().skip(1) {
                    acc = if name == "min" {
                        math.min(&acc, following)?
                    } else {
                        math.max(&acc, following)?
                    };
                }
                Ok(acc)
            }
            _ => Err(parse_error(
                self.src,
                at,
                "the function is not available here",
            )),
        }
    }
}

impl<'r> Formula<'r> {
    /// An evaluator with the built-in names: `pi` and the constants of `dimvexa constants`
    /// (`planck_constant`, `elementary_charge`, `boltzmann_constant`, `avogadro_constant`,
    /// `speed_of_light`, `molar_gas_constant`), each with its unit, exact.
    #[must_use]
    pub fn new(reg: &'r Registry) -> Self {
        let pi = Number::from(Quantity::new(Ratio::PI, UnitExpr::unity()));
        let mut vars = vec![("pi".to_owned(), pi)];
        for def in dimvexa_data::CONSTANTS.iter().filter(|c| c.id != "pi") {
            let bound = def
                .value()
                .and_then(|v| Ok(Quantity::new(v, reg.parse(def.unit)?)));
            if let Ok(q) = bound {
                vars.push((def.id.to_owned(), Number::from(q)));
            }
        }
        Self {
            reg,
            math: Math::new(reg),
            builtin: vars.len(),
            vars,
            given: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// With `true` a sum with no exact value (`1 + pi`) is `unrepresentable`; by default it is
    /// approximate (`≈`).
    #[must_use]
    pub const fn exact_only(mut self, on: bool) -> Self {
        self.math = self.math.exact_only(on);
        self
    }

    /// An evaluator over the registry of `loaded` (the configs' units and names) whose built-in
    /// names are `pi` and every constant in effect: the built-in ones and those of the configs
    /// (`water_25C`, your own `[[constant]]`), each exact and with its unit. A later config's
    /// constant replaces an earlier one of the same name.
    #[must_use]
    pub fn from_loaded(loaded: &'r dimvexa_config::Loaded) -> Self {
        let mut formula = Self::new(&loaded.reg);
        for c in loaded.constants.iter().filter(|c| c.id != "pi") {
            // `dimvexa_config::load` has checked that every constant's unit exists
            let Ok(unit) = loaded.reg.parse(&c.unit) else {
                continue;
            };
            let number = Number::from(Quantity::new(c.value, unit));
            match formula.vars.iter().position(|(n, _)| *n == c.id) {
                Some(i) => {
                    if let Some(slot) = formula.vars.get_mut(i) {
                        *slot = (c.id.clone(), number);
                    }
                }
                None => formula.vars.push((c.id.clone(), number)),
            }
        }
        formula.builtin = formula.vars.len();
        formula
    }

    /// What the caller should be told about: a name that replaces a built-in constant, or that
    /// is bound twice (the last value is used).
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Binds `name` to the quantity written in `quantity` (`"998 kg/m^3"`); a second binding of
    /// the same name replaces the first.
    ///
    /// # Errors
    ///
    /// `invalid_definition` when `name` is not letters, digits and `_` starting with a letter or
    /// `_`, is `pi` (the constant cannot be redefined), or when more than [`MAX_VARS`] names
    /// are bound; the error of [`Registry::quantity`] when the text is not a quantity.
    pub fn var(mut self, name: &str, quantity: &str) -> Result<Self> {
        let mut chars = name.chars();
        let valid = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && chars.all(|c| c.is_alphanumeric() || c == '_');
        let refuse = |message: &str| Error::InvalidDefinition {
            symbol: name.to_owned(),
            message: message.to_owned(),
        };
        if !valid {
            return Err(refuse(
                "a name is letters, digits and '_' and starts with a letter or '_'",
            ));
        }
        if name == "pi" {
            return Err(refuse("'pi' is the constant and cannot be redefined"));
        }
        let known = self.vars.iter().position(|(n, _)| n == name);
        if known.is_none() && self.vars.len() >= self.builtin.saturating_add(MAX_VARS) {
            return Err(refuse("too many names"));
        }
        let q = Number::from(self.reg.quantity(quantity)?);
        if let Some(i) = known {
            if self.given.iter().any(|n| n == name) {
                self.warnings.push(format!(
                    "the name '{name}' is given twice; the last value is used"
                ));
            } else if i < self.builtin {
                self.warnings.push(format!(
                    "the name '{name}' replaces the constant {name}; \
                     the value you gave is used"
                ));
            }
        }
        if !self.given.iter().any(|n| n == name) {
            self.given.push(name.to_owned());
        }
        match known {
            Some(i) => {
                if let Some(slot) = self.vars.get_mut(i) {
                    *slot = (name.to_owned(), q);
                }
            }
            None => self.vars.push((name.to_owned(), q)),
        }
        Ok(self)
    }

    /// Evaluates `formula`: exact where the mathematics is exact, an approximate [`Number`]
    /// otherwise; the unit is whatever the arithmetic produced.
    ///
    /// # Errors
    ///
    /// `parse_error` for a syntax error or an unknown variable, `unknown_unit` for a unit that
    /// does not exist, `refused` for an unknown or unimplemented function, and the error of the
    /// operation that failed (`dimension_mismatch`, `overflow`, ...). An exact operation that
    /// fails is never answered with an approximate value.
    pub fn eval(&self, formula: &str) -> Result<Number> {
        self.run(formula).map(|(number, _)| number)
    }

    /// Like [`eval`](Self::eval), and every name bound with [`var`](Self::var) must be used by
    /// the equation: a value that nothing reads is most likely a typo in the equation or in the
    /// name (`2 r2` with `r` and `r2` given), so it is an error, never ignored. The built-in
    /// constants need not be used.
    ///
    /// # Errors
    ///
    /// The errors of [`eval`](Self::eval), and `invalid_definition` naming the first unused
    /// variable.
    pub fn eval_strict(&self, formula: &str) -> Result<Number> {
        let (number, used) = self.run(formula)?;
        self.given
            .iter()
            .find(|name| !used.contains(name))
            .map_or(Ok(number), |name| {
                Err(Error::InvalidDefinition {
                    symbol: name.clone(),
                    message: format!(
                        "the variable '{name}' is not used in the equation '{formula}'"
                    ),
                })
            })
    }

    fn run(&self, formula: &str) -> Result<(Number, Vec<String>)> {
        let toks = self.names_after_numbers(formula, lex(formula)?)?;
        let mut p = Parser {
            src: formula,
            toks,
            pos: 0,
            depth: 0,
            f: self,
            used: Vec::new(),
            norm: normalise(formula),
            pow_depth: 0,
            tail_unit: None,
        };
        let q = p.expr()?;
        if p.pos < p.toks.len() {
            let message = match p.toks.get(p.pos) {
                Some((_, Tok::Id(name))) => format!(
                    "unexpected '{name}': two names or values in a row need an operator, \
                     write the product with * between them"
                ),
                _ => "unexpected input".to_owned(),
            };
            return Err(parse_error(formula, p.here(), &message));
        }
        Ok((q, p.used))
    }

    /// What a name written after a number means. `2 r` with a variable `r` that is not a unit is
    /// `2 * r` (the text becomes `(2 * r)`, so `10 m / 2 r` is `10 m / (2 r)`); a name that is
    /// also a unit (`0.5 h` with a variable `h`) is ambiguous, because the reader sees one thing
    /// and the evaluator would take the other, and is refused.
    fn names_after_numbers(&self, src: &str, toks: Vec<(usize, Tok)>) -> Result<Vec<(usize, Tok)>> {
        let mut out: Vec<(usize, Tok)> = Vec::with_capacity(toks.len());
        let mut toks = toks.into_iter().peekable();
        while let Some((at, tok)) = toks.next() {
            let Tok::Qty(number, unit) = &tok else {
                out.push((at, tok));
                continue;
            };
            let mut symbol = String::new();
            let mut leading = true;
            let mut rewrite: Option<String> = None;
            for c in unit.chars().chain(std::iter::once(' ')) {
                if c.is_alphanumeric() || c == '_' {
                    symbol.push(c);
                    continue;
                }
                // `m3` and `m^3` are the unit `m` (a power of it): a name that equals the symbol
                // or the symbol without its trailing digits collides with a real unit
                let bare = symbol
                    .trim_end_matches(|d: char| d.is_ascii_digit())
                    .to_owned();
                let usable = |n: &&String| *n != "pi" && !n.is_empty();
                let named = |text: &str| {
                    self.vars
                        .iter()
                        .map(|(n, _)| n)
                        .filter(usable)
                        .find(|n| n.as_str() == text)
                };
                // an ambiguous symbol (`ton`, `cal`) is a unit name too: a variable may not take it
                let unit_name = |t: &str| {
                    matches!(
                        self.reg.parse(t),
                        Ok(_) | Err(Error::AmbiguousSymbol { .. })
                    )
                };
                if unit_name(&symbol) || unit_name(&bare) {
                    if let Some(name) = named(&symbol).or_else(|| named(&bare)) {
                        return Err(Error::InvalidDefinition {
                            symbol: name.clone(),
                            message: format!(
                                "the name '{name}' is also a unit in '{number} {unit}' of the \
                                 equation: rename the variable"
                            ),
                        });
                    }
                } else if leading && named(&symbol).is_some() {
                    // the whole symbol is a variable that is not a unit: `2 r`, `2 r2` and
                    // `2 r^2` are the number times the variable (`r2` is a name, not `r^2`)
                    rewrite = Some(unit.clone());
                }
                leading = false;
                symbol.clear();
            }
            // a temperature point (`degC`) is never moved to the result: `40/2 degC.to(K)` would
            // convert the `1 degC` alone; it divides by the point and is refused as ambiguous
            let point = self.reg.parse(unit).is_ok_and(|u| !u.offset().is_zero());
            if rewrite.is_none() && !point && number_over_quantity(&mut out, (number, unit), at) {
                continue;
            }
            match rewrite {
                Some(_) if matches!(out.last(), Some((_, Tok::Op('/')))) => {
                    // `a / 2 r` is (a / 2) * r or a / (2 * r)
                    return Err(parse_error(
                        src,
                        at,
                        &format!(
                            "a division followed by '{number} {unit}' can be read (a / {number}) \
                             * {unit} or a / ({number} * {unit}): write the parentheses, \
                             a / ({number} {unit}) or (a / {number}) * {unit}"
                        ),
                    ));
                }
                Some(_) if matches!(out.last(), Some((_, Tok::Op('^')))) => {
                    // `x^2 n` would read `2 n` as the exponent, and `x^2 * n` is what is meant
                    return Err(parse_error(
                        src,
                        at,
                        &format!(
                            "'{number} {unit}' after '^' is ambiguous: write ^{number} * {unit} \
                             for the power times {unit}, or ^({number} {unit}) for the exponent"
                        ),
                    ));
                }
                Some(_) if matches!(toks.peek(), Some((_, Tok::Op('^')))) => {
                    // `2 x ^ 2` would square the product `(2 * x)`, `2 * x ^ 2` is what is meant
                    return Err(parse_error(
                        src,
                        at,
                        &format!(
                            "'{number} {unit}' followed by '^' is ambiguous: write \
                             {number} * {unit}^... for the power times {number}, or \
                             ({number} {unit})^... for the power of the product"
                        ),
                    ));
                }
                Some(text) => {
                    out.push((at, Tok::Op('(')));
                    out.push((at, Tok::Num(number.clone())));
                    out.push((at, Tok::Op('*')));
                    out.extend(lex(&text)?.into_iter().map(|(_, t)| (at, t)));
                    out.push((at, Tok::Op(')')));
                }
                None => out.push((at, tok)),
            }
        }
        Ok(out)
    }

    /// Like [`eval`](Self::eval), but only an exact result is accepted.
    ///
    /// # Errors
    ///
    /// The errors of [`eval`](Self::eval), and `refused` for an approximate result.
    pub fn eval_exact(&self, formula: &str) -> Result<Quantity> {
        match self.eval(formula)? {
            Number::Exact(q) => Ok(q),
            Number::Approx(a) => Err(Error::Refused {
                reason: format!("{formula} is approximate ({a}), not exact"),
                suggestions: Vec::new(),
            }),
        }
    }

    /// `n` in `unit`; an exact number stays exact.
    ///
    /// # Errors
    ///
    /// The error of [`Math::to_unit`].
    pub fn to_unit(&self, n: &Number, unit: &str) -> Result<Number> {
        self.math.to_unit(n, unit)
    }

    /// Evaluates `expression` and describes the result, the whole of what `dimvexa calc` does:
    /// [`eval_strict`](Self::eval_strict), then the conversion to `to` (a unit symbol) or the
    /// SI prefix that suits the number when `autoprefix` is set (the two exclude each other).
    ///
    /// # Errors
    ///
    /// A [`CalcError`] with the step that failed and the errors of `eval_strict`,
    /// [`to_unit`](Self::to_unit) and [`autoprefix`](Self::autoprefix), and `refused` when both
    /// `to` and `autoprefix` are given.
    pub fn calculate(
        &self,
        expression: &str,
        to: Option<&str>,
        autoprefix: bool,
    ) -> std::result::Result<crate::Report, CalcError> {
        self.calculate_in(expression, to, autoprefix, crate::Notation::Plain)
    }

    /// As [`calculate`](Self::calculate) for a result that will be shown in `notation`: with
    /// `autoprefix` and significant digits the prefix is chosen from the rounded number
    /// (`999.9996 m` with 3 digits is `1.00 km`), and `Scientific` then needs no exponent.
    ///
    /// # Errors
    ///
    /// As [`calculate`](Self::calculate).
    pub fn calculate_in(
        &self,
        expression: &str,
        to: Option<&str>,
        autoprefix: bool,
        notation: crate::Notation,
    ) -> std::result::Result<crate::Report, CalcError> {
        let at = |step: CalcStep| move |error: Error| CalcError { step, error };
        let to = to.map(str::trim).filter(|u| !u.is_empty());
        if autoprefix && to.is_some() {
            return Err(CalcError {
                step: CalcStep::Prefix,
                error: Error::Refused {
                    reason: "autoprefix chooses the unit, so it cannot be used with a target unit"
                        .to_owned(),
                    suggestions: Vec::new(),
                },
            });
        }
        let mut number = self
            .eval_strict(expression)
            .map_err(at(CalcStep::Expression))?;
        if let Some(unit) = to {
            number = self.to_unit(&number, unit).map_err(at(CalcStep::Unit))?;
        } else if autoprefix {
            let digits = match notation {
                crate::Notation::Scientific(d) => d,
                crate::Notation::Plain => None,
            };
            number = self
                .math
                .autoprefix_in(&number, digits)
                .map_err(at(CalcStep::Prefix))?;
        }
        let report = crate::Report::new(&number).map_err(at(CalcStep::Expression))?;
        Ok(
            match self.math.prefix_power(&number).filter(|_| autoprefix) {
                Some(power) => report.with_prefix(power),
                None => report,
            },
        )
    }

    /// `n` with the SI prefix that puts its number between 1 and 1000 (`1500 m` is `1.5 km`).
    ///
    /// # Errors
    ///
    /// The error of [`Math::autoprefix`].
    pub fn autoprefix(&self, n: &Number) -> Result<Number> {
        self.math.autoprefix(n)
    }

    /// The value of `n` in `unit`, rounded once (exact) or converted exactly (approximate).
    ///
    /// # Errors
    ///
    /// The error of [`Math::to_f64_in`].
    pub fn f64_in(&self, n: &Number, unit: &str) -> Result<f64> {
        self.math.to_f64_in(n, unit)
    }
}
