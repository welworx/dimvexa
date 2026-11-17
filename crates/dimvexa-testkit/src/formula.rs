// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equations as text, evaluated with units: the shape the engineering tests want.
//!
//! ```
//! use dimvexa_testkit::formula::Formula;
//!
//! let reg = dimvexa::registry()?;
//! let f = Formula::new(&reg)
//!     .var("w", "2 kip/ft")?
//!     .var("L", "20 ft")?;
//! let m = f.eval("w * L^2 / 8")?;
//! assert_eq!(f.f64_in(&m, "kN*m")?, 135.581_794_833_140_04);
//!
//! // a root, a fractional power and a logarithm: exact when the mathematics is exact
//! let f = Formula::new(&reg).var("dp", "50 kPa")?.var("rho", "998 kg/m^3")?;
//! let v = f.eval("sqrt(2 * dp / rho)")?;
//! assert!(!v.is_exact()); // an approximate value, shown with ≈ (ADR-029)
//! assert!((f.f64_in(&v, "m/s")? - (100_000.0_f64 / 998.0).sqrt()).abs() < 1e-12);
//! assert!(f.eval("sqrt(4 * dp / dp)")?.is_exact());
//! # Ok::<(), dimvexa::Error>(())
//! ```
//!
//! Grammar: numbers (`2`, `0.5`, `1e-3`), variables, `+ - * /`, a unary minus, parentheses, a
//! power with an integer, a decimal or a fraction as exponent (`L^4`, `v^-2`, `x^0.5`,
//! `R^(2/3)`), and the functions `sqrt`, `cbrt`, `abs`, `hypot(x, y)`, `min(x, y, ...)`,
//! `max(x, y, ...)`, `ln`, `exp`, `log10` (`lg`), `log2` (`lb`), `log(x, base)`, `sin`, `cos`, `sinh`, `cosh`,
//! `tanh`, `asinh`, `acosh` and `atanh` (`log(x)` alone is ambiguous and refused). Any other function is a
//! `refused` error that says `'tan' is not implemented` (and how to write it, where there is a
//! way: `sin(x) / cos(x)`), an unknown name says `unknown function`. Every step is an
//! operation of [`Math`] or of the core, so a result is exact whenever the mathematics is exact
//! and an explicitly approximate [`Number`] otherwise (ADR-029). Read it in a unit with
//! [`Formula::f64_in`] or [`Formula::to_unit`]; [`Formula::eval_exact`] refuses an approximate
//! result.

use dimvexa::{Error, Quantity, Ratio, Registry, Result};
use dimvexa_calc::{Math, Number};

/// A set of named quantities and an evaluator for formulas over them.
#[derive(Debug)]
pub struct Formula<'r> {
    reg: &'r Registry,
    math: Math<'r>,
    vars: Vec<(String, Number)>,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
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

fn lex(input: &str) -> Result<Vec<(usize, Tok)>> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let start = i;
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            while chars
                .get(i)
                .is_some_and(|d| d.is_ascii_digit() || *d == '.')
            {
                i += 1;
            }
            let exp = chars.get(i).is_some_and(|e| *e == 'e' || *e == 'E');
            let sign = chars.get(i + 1).is_some_and(|s| *s == '-' || *s == '+');
            let digit_at = i + 1 + usize::from(sign);
            if exp && chars.get(digit_at).is_some_and(char::is_ascii_digit) {
                i = digit_at;
                while chars.get(i).is_some_and(char::is_ascii_digit) {
                    i += 1;
                }
            }
            out.push((start, Tok::Num(chars[start..i].iter().collect())));
        } else if c.is_alphabetic() || c == '_' {
            while chars
                .get(i)
                .is_some_and(|d| d.is_alphanumeric() || *d == '_')
            {
                i += 1;
            }
            out.push((start, Tok::Id(chars[start..i].iter().collect())));
        } else if "+-*/^(),".contains(c) {
            out.push((start, Tok::Op(c)));
            i += 1;
        } else {
            return Err(parse_error(input, start, "unexpected character"));
        }
    }
    Ok(out)
}

/// The functions the evaluator implements (ADR-029), with the number of arguments: `0` means
/// two or more (`min`, `max`).
const IMPLEMENTED: [(&str, usize); 21] = [
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
    ("sinh", 1),
    ("cosh", 1),
    ("tanh", 1),
    ("asinh", 1),
    ("acosh", 1),
    ("atanh", 1),
];

/// Well-known functions that are **not implemented**, each with the way to write it from the
/// ones that are (empty where there is none). Using one is an error, never a guess.
const NOT_IMPLEMENTED: [(&str, &str); 8] = [
    ("tan", "sin(x) / cos(x)"),
    ("asin", ""),
    ("acos", ""),
    ("atan", ""),
    ("atan2", ""),
    ("pow", "x^p"),
    ("floor", ""),
    ("round", ""),
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
    f: &'a Formula<'r>,
}

impl Parser<'_, '_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }

    fn here(&self) -> usize {
        self.toks.get(self.pos).map_or(self.src.len(), |(p, _)| *p)
    }

    fn eat(&mut self, op: char) -> bool {
        if self.peek() == Some(&Tok::Op(op)) {
            self.pos += 1;
            true
        } else {
            false
        }
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
            } else {
                return Ok(acc);
            }
        }
    }

    fn unary(&mut self) -> Result<Number> {
        if self.eat('-') {
            let inner = self.unary()?;
            return self.f.math.neg(&inner);
        }
        self.power()
    }

    /// An integer: the digits of the token at the cursor.
    fn int(&mut self) -> Result<i32> {
        let at = self.here();
        let Some(Tok::Num(text)) = self.peek().cloned() else {
            return Err(parse_error(self.src, at, "an integer is expected"));
        };
        self.pos += 1;
        text.parse()
            .map_err(|_| parse_error(self.src, at, "an integer is expected"))
    }

    /// The exponent after `^` as `(p, q)`: `3`, `-2`, `0.5` or `(2/3)` (also `(-2/3)`).
    fn exponent(&mut self) -> Result<(i32, i32)> {
        let at = self.here();
        if self.eat('(') {
            let negative = self.eat('-');
            let num = self.int()?;
            let den = if self.eat('/') { self.int()? } else { 1 };
            if !self.eat(')') {
                return Err(parse_error(self.src, self.here(), "')' is expected"));
            }
            return Ok((if negative { -num } else { num }, den));
        }
        let negative = self.eat('-');
        let at_num = self.here();
        let Some(Tok::Num(text)) = self.peek().cloned() else {
            return Err(parse_error(self.src, at, "an exponent is expected"));
        };
        self.pos += 1;
        let ratio: Ratio = text
            .parse()
            .map_err(|_| parse_error(self.src, at_num, "the exponent must be a number"))?;
        let num = i32::try_from(ratio.num())
            .map_err(|_| parse_error(self.src, at_num, "the exponent is too large"))?;
        let den = i32::try_from(ratio.den())
            .map_err(|_| parse_error(self.src, at_num, "the exponent is too precise"))?;
        Ok((if negative { -num } else { num }, den))
    }

    fn power(&mut self) -> Result<Number> {
        let base = self.atom()?;
        if !self.eat('^') {
            return Ok(base);
        }
        let (num, den) = self.exponent()?;
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
                self.pos += 1;
                self.f.reg.quantity(&format!("{text} 1")).map(Number::from)
            }
            Some(Tok::Id(name)) => {
                self.pos += 1;
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
                self.pos += 1;
                let inner = self.expr()?;
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
        self.pos += 1; // the '('
        let mut args = vec![self.expr()?];
        while self.eat(',') {
            args.push(self.expr()?);
        }
        if !self.eat(')') {
            return Err(parse_error(self.src, self.here(), "')' is expected"));
        }
        let wrong = match arity {
            0 => args.len() < 2,
            n => args.len() != *n,
        };
        if wrong {
            // `log(x)` is the one name that is ambiguous in itself: natural or base 10?
            let reason = if name == "log" && args.len() == 1 {
                "log(x) is ambiguous (natural or base 10?): write ln(x) for the natural logarithm, \
                 lg(x) or log10(x) for base 10, lb(x) or log2(x) for base 2, or log(x, base)"
                    .to_owned()
            } else {
                format!(
                    "{name} takes {} argument(s), not {}",
                    if *arity == 0 {
                        "two or more".to_owned()
                    } else {
                        arity.to_string()
                    },
                    args.len()
                )
            };
            return Err(if name == "log" && args.len() == 1 {
                Error::Refused {
                    reason,
                    suggestions: ["ln", "lg", "log10", "lb", "log2", "log"]
                        .map(str::to_owned)
                        .to_vec(),
                }
            } else {
                parse_error(self.src, at, &reason)
            });
        }
        let math = self.f.math;
        let one = args
            .first()
            .ok_or_else(|| parse_error(self.src, at, "an argument is expected"))?;
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
                for next in args.iter().skip(1) {
                    acc = if name == "min" {
                        math.min(&acc, next)?
                    } else {
                        math.max(&acc, next)?
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
    /// An evaluator with no variables yet.
    #[must_use]
    pub const fn new(reg: &'r Registry) -> Self {
        Self {
            reg,
            math: Math::new(reg),
            vars: Vec::new(),
        }
    }

    /// Binds `name` to the quantity written in `quantity` (`"998 kg/m^3"`).
    ///
    /// # Errors
    ///
    /// The error of [`Registry::quantity`] when the text is not a quantity.
    pub fn var(mut self, name: &str, quantity: &str) -> Result<Self> {
        let q = Number::from(self.reg.quantity(quantity)?);
        self.vars.retain(|(n, _)| n != name);
        self.vars.push((name.to_owned(), q));
        Ok(self)
    }

    /// Evaluates `formula`: exact where the mathematics is exact, an approximate [`Number`]
    /// otherwise; the unit is whatever the arithmetic produced.
    ///
    /// # Errors
    ///
    /// `parse_error` for a syntax error or an unknown variable, `refused` for an unknown
    /// function, and the error of the operation that failed (`dimension_mismatch`,
    /// `overflow`, ...). An exact operation that fails is never answered with an approximate
    /// value.
    pub fn eval(&self, formula: &str) -> Result<Number> {
        let mut p = Parser {
            src: formula,
            toks: lex(formula)?,
            pos: 0,
            f: self,
        };
        let q = p.expr()?;
        if p.pos < p.toks.len() {
            return Err(parse_error(formula, p.here(), "unexpected input"));
        }
        Ok(q)
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
