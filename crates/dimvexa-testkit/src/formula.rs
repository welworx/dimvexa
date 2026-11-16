// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equations as text, evaluated exactly with units: the shape the engineering tests want.
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
//! # Ok::<(), dimvexa::Error>(())
//! ```
//!
//! Grammar: numbers (`2`, `0.5`, `1e-3`), variables, `+ - * /`, a unary minus, parentheses and
//! an integer power (`L^4`, `v^-2`). Every step is a [`Calc`] or core operation, so the result
//! is exact and carries the unit the arithmetic produced; read it in a unit with
//! [`Formula::f64_in`] or [`Formula::to_unit`]. A function call such as `sqrt(x)` is parsed and
//! refused with `refused`: `dimvexa-calc` has no such operation yet, and the tests that need
//! one stay open until it has.

use dimvexa::{Error, Quantity, Registry, Result};
use dimvexa_calc::Calc;

/// A set of named quantities and an evaluator for formulas over them.
#[derive(Debug)]
pub struct Formula<'r> {
    reg: &'r Registry,
    calc: Calc<'r>,
    vars: Vec<(String, Quantity)>,
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

    fn expr(&mut self) -> Result<Quantity> {
        let mut acc = self.term()?;
        loop {
            if self.eat('+') {
                let rhs = self.term()?;
                acc = self.f.calc.add(&acc, &rhs)?;
            } else if self.eat('-') {
                let rhs = self.term()?;
                acc = self.f.calc.sub(&acc, &rhs)?;
            } else {
                return Ok(acc);
            }
        }
    }

    fn term(&mut self) -> Result<Quantity> {
        let mut acc = self.unary()?;
        loop {
            if self.eat('*') {
                let rhs = self.unary()?;
                acc = acc.checked_mul(&rhs)?;
            } else if self.eat('/') {
                let rhs = self.unary()?;
                acc = acc.checked_div(&rhs)?;
            } else {
                return Ok(acc);
            }
        }
    }

    fn unary(&mut self) -> Result<Quantity> {
        if self.eat('-') {
            let inner = self.unary()?;
            return self.f.calc.neg(&inner);
        }
        self.power()
    }

    fn power(&mut self) -> Result<Quantity> {
        let base = self.atom()?;
        if !self.eat('^') {
            return Ok(base);
        }
        let negative = self.eat('-');
        let at = self.here();
        let Some(Tok::Num(text)) = self.peek().cloned() else {
            return Err(parse_error(self.src, at, "an integer exponent is expected"));
        };
        self.pos += 1;
        let exp: i32 = text
            .parse()
            .map_err(|_| parse_error(self.src, at, "the exponent must be an integer"))?;
        self.f.calc.pow(&base, if negative { -exp } else { exp })
    }

    fn atom(&mut self) -> Result<Quantity> {
        let at = self.here();
        match self.peek().cloned() {
            Some(Tok::Num(text)) => {
                self.pos += 1;
                self.f.reg.quantity(&format!("{text} 1"))
            }
            Some(Tok::Id(name)) => {
                self.pos += 1;
                if self.peek() == Some(&Tok::Op('(')) {
                    return Err(Error::Refused {
                        reason: format!(
                            "the function '{name}' is not available in dimvexa-calc yet"
                        ),
                        suggestions: Vec::new(),
                    });
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
}

impl<'r> Formula<'r> {
    /// An evaluator with no variables yet.
    #[must_use]
    pub const fn new(reg: &'r Registry) -> Self {
        Self {
            reg,
            calc: Calc::new(reg),
            vars: Vec::new(),
        }
    }

    /// Binds `name` to the quantity written in `quantity` (`"998 kg/m^3"`).
    ///
    /// # Errors
    ///
    /// The error of [`Registry::quantity`] when the text is not a quantity.
    pub fn var(mut self, name: &str, quantity: &str) -> Result<Self> {
        let q = self.reg.quantity(quantity)?;
        self.vars.retain(|(n, _)| n != name);
        self.vars.push((name.to_owned(), q));
        Ok(self)
    }

    /// Evaluates `formula` exactly; the unit is whatever the arithmetic produced.
    ///
    /// # Errors
    ///
    /// `parse_error` for a syntax error or an unknown variable, `refused` for a function call,
    /// and the error of the operation that failed (`dimension_mismatch`, `overflow`, ...).
    pub fn eval(&self, formula: &str) -> Result<Quantity> {
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

    /// The exact value of `q` in `unit`.
    ///
    /// # Errors
    ///
    /// The error of [`Calc::to_unit`].
    pub fn to_unit(&self, q: &Quantity, unit: &str) -> Result<Quantity> {
        self.calc.to_unit(q, unit)
    }

    /// The value of `q` in `unit`, rounded once to the nearest `f64`.
    ///
    /// # Errors
    ///
    /// The error of [`Calc::to_f64_in`].
    pub fn f64_in(&self, q: &Quantity, unit: &str) -> Result<f64> {
        self.calc.to_f64_in(q, unit)
    }
}
