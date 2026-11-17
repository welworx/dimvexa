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
//!   `ln`, `exp`, `log10` (`lg`), `log2` (`lb`), `log(x, base)`, `sin`, `cos`, `tan`, `asin`,
//!   `acos`, `atan`, `atan2(y, x)`, `kelvin` (a temperature as absolute kelvin), `sinh`, `cosh`,
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
            '.' => following.is_some_and(starts_unit),
            '^' => following.is_some_and(|n| n.is_ascii_digit() || n == '-'),
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
    let mut out = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\t' | '\n' | '\r' | '\u{a0}' | '\u{2009}' | '\u{202f}' => out.push(' '),
            '·' | '⋅' | '×' => out.push('*'),
            '÷' => out.push('/'),
            '−' => out.push('-'),
            '²' => out.extend(['^', '2']),
            '³' => out.extend(['^', '3']),
            '⁻' if chars.peek() == Some(&'¹') => {
                chars.next();
                out.extend(['^', '-', '1']);
            }
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
        } else if c.is_ascii_digit() || c == '.' {
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
                out.push((after, Tok::Op('*')));
                out.push((after, Tok::Id("pi".to_owned())));
                i = word_end;
                continue;
            }
            // a unit starts with a symbol, or with a parenthesis that opens on one: `998 (kg/m3)`
            let opens_unit = chars.get(after) == Some(&'(')
                && chars.get(next(after)).copied().is_some_and(starts_unit);
            if chars.get(after).copied().is_some_and(starts_unit) || opens_unit {
                let end = scan_unit(&chars, after);
                out.push((start, Tok::Qty(number, text(&chars, after, end))));
                i = end;
            } else {
                out.push((start, Tok::Num(number)));
            }
        } else if c.is_alphabetic() || c == '_' {
            while chars
                .get(i)
                .is_some_and(|d| d.is_alphanumeric() || *d == '_')
            {
                i = next(i);
            }
            out.push((start, Tok::Id(text(&chars, start, i))));
        } else if "+-*/^(),".contains(c) {
            out.push((start, Tok::Op(c)));
            i = next(i);
        } else {
            return Err(parse_error(input, start, "unexpected character"));
        }
    }
    Ok(out)
}

/// The functions the evaluator implements (ADR-029), with the number of arguments: `0` means
/// two or more (`min`, `max`).
const IMPLEMENTED: [(&str, usize); 27] = [
    ("sqrt", 1),
    ("cbrt", 1),
    ("abs", 1),
    ("hypot", 2),
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
    ("kelvin", 1),
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

/// Well-known functions that are **not implemented**, each with the way to write it from the
/// ones that are (empty where there is none). Using one is an error, never a guess.
const NOT_IMPLEMENTED: [(&str, &str); 3] = [("pow", "x^p"), ("floor", ""), ("round", "")];

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

/// `refused`, in the wording of the other unsupported requests (`'psig' is not implemented`).
fn not_implemented(name: &str, hint: &str) -> Error {
    let how = if hint.is_empty() {
        String::new()
    } else {
        format!("; write {hint}")
    };
    Error::Refused {
        reason: format!(
            "'{name}' is not implemented (the functions are {}, ADR-029){how}",
            function_list()
        ),
        suggestions: implemented_names(),
    }
}

struct Parser<'a, 'r> {
    src: &'a str,
    toks: Vec<(usize, Tok)>,
    pos: usize,
    depth: usize,
    f: &'a Formula<'r>,
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
        loop {
            if self.eat('*') {
                let rhs = self.unary()?;
                acc = self.f.math.mul(&acc, &rhs)?;
            } else if self.eat('/') {
                let rhs = self.unary()?;
                acc = self.f.math.div(&acc, &rhs)?;
            } else if self.after_pi_name() {
                // `2 pi r` and `pi r^2`: a name written right after `pi` multiplies
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
        let value = self.power()?;
        let unit = value.unit();
        let ratio = match &value {
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
                    "the exponent is too large or too precise (or has pi)",
                )
            })?;
        let num = i32::try_from(ratio.num())
            .map_err(|_| parse_error(self.src, at, "the exponent is too large"))?;
        let den = i32::try_from(ratio.den())
            .map_err(|_| parse_error(self.src, at, "the exponent is too precise"))?;
        Ok((if negative { num.saturating_neg() } else { num }, den))
    }

    fn power(&mut self) -> Result<Number> {
        let base = self.atom()?;
        if !self.eat('^') {
            return Ok(base);
        }
        self.enter()?;
        let (num, den) = self.exponent()?;
        self.leave();
        if den == 1 {
            self.f.math.pow(&base, num)
        } else {
            self.f.math.root(&base, num, den)
        }
    }

    fn atom(&mut self) -> Result<Number> {
        let at = self.here();
        match self.peek().cloned() {
            Some(Tok::Num(text)) => {
                self.advance();
                self.f.reg.quantity(&format!("{text} 1")).map(Number::from)
            }
            Some(Tok::Qty(number, unit)) => {
                self.advance();
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
                        } if suggestions.is_empty() => {
                            // as written: `r2`, not the `r` the unit parser stripped it to
                            let written = unit
                                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                                .find(|run| run.starts_with(symbol.as_str()))
                                .map_or_else(|| symbol.clone(), str::to_owned);
                            parse_error(
                                self.src,
                                at,
                                &format!(
                                    "'{written}' in '{number} {unit}' is not a unit and not a \
                                     variable (a variable is defined with --var {written}=VALUE)"
                                ),
                            )
                        }
                        other => other,
                    })
            }
            Some(Tok::Id(name)) => {
                self.advance();
                if self.peek() == Some(&Tok::Op('(')) {
                    return self.call(&name);
                }
                self.f
                    .vars
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, q)| q.clone())
                    .ok_or_else(|| parse_error(self.src, at, "unknown variable"))
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

    /// `name(argument, ...)`.
    fn call(&mut self, name: &str) -> Result<Number> {
        // the name decides before the arguments are read: `atan2(y, x)` must say that it is not
        // implemented, not complain about the comma
        if let Some((_, hint)) = NOT_IMPLEMENTED.iter().find(|(n, _)| *n == name) {
            return Err(not_implemented(name, hint));
        }
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
            ("kelvin", _) => math.kelvin(one),
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
    /// (`h_P`, `e`, `k_B`, `N_A`, `c`, `R`), each with its unit, exact.
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

    /// An evaluator over the registry of `loaded` (the configs' units and names) whose built-in
    /// names are `pi` and every constant in effect: the built-in ones and those of the configs
    /// (`water_25C`, your own `[[constant]]`), each exact and with its unit. A later config's
    /// constant replaces an earlier one of the same name.
    #[must_use]
    pub fn from_loaded(loaded: &'r dimvexa_config::Loaded) -> Self {
        let mut formula = Self::new(&loaded.reg);
        for c in loaded.constants.iter().filter(|c| c.id != "pi") {
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
                    "the name '{name}' replaces the built-in constant {name}; \
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
        let toks = self.names_after_numbers(lex(formula)?)?;
        let mut p = Parser {
            src: formula,
            toks,
            pos: 0,
            depth: 0,
            f: self,
        };
        let q = p.expr()?;
        if p.pos < p.toks.len() {
            return Err(parse_error(formula, p.here(), "unexpected input"));
        }
        Ok(q)
    }

    /// What a name written after a number means. `2 r` with a variable `r` that is not a unit is
    /// `2 * r` (the text becomes `(2 * r)`, so `10 m / 2 r` is `10 m / (2 r)`); a name that is
    /// also a unit (`0.5 h` with a variable `h`) is ambiguous, because the reader sees one thing
    /// and the evaluator would take the other, and is refused.
    fn names_after_numbers(&self, toks: Vec<(usize, Tok)>) -> Result<Vec<(usize, Tok)>> {
        let mut out = Vec::with_capacity(toks.len());
        for (at, tok) in toks {
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
                if self.reg.parse(&symbol).is_ok() || self.reg.parse(&bare).is_ok() {
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
            match rewrite {
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

    /// The value of `n` in `unit`, rounded once (exact) or converted exactly (approximate).
    ///
    /// # Errors
    ///
    /// The error of [`Math::to_f64_in`].
    pub fn f64_in(&self, n: &Number, unit: &str) -> Result<f64> {
        self.math.to_f64_in(n, unit)
    }
}
