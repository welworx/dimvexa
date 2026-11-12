// SPDX-License-Identifier: MIT OR Apache-2.0
//! Constants in a value (ADR-018, step 3): `4 pi`, `pi/2`, `2*pi^2`, `1/(2 pi)`.
//!
//! The grammar is deliberately small and has no free-form formulas (ADR-001):
//!
//! ```text
//! value  = factor { ("*" | "/" | <constant>) factor }
//! factor = ["+" | "-"] atom ["^" ["+" | "-"] digits]
//! atom   = number | constant | "(" value ")"
//! ```
//!
//! `*`, `/` and the juxtaposition of a constant (`4 pi`, `4pi`) have the same precedence and
//! group to the left, so `1/2 pi` is `pi/2`, the notation of the unit tables. There is no
//! addition: dimvexa converts, it does not calculate (ADR-014).

use crate::constant::ConstantDef;
use crate::error::{Error, Result};
use crate::ratio::Ratio;

/// Deepest nesting of parentheses; a longer chain is refused instead of risking the stack.
const MAX_DEPTH: u32 = 32;

impl Ratio {
    /// Reads a value that may contain constants and operators (`4 pi`, `pi/2`, `1/(2*pi)`).
    ///
    /// Only the dimensionless constants of `constants` (unit `1`, that is `pi`) can appear in a
    /// value; `π` is accepted as `pi`. Integer powers (`pi^2`, `2^-3`), `*`, `/`, a leading
    /// sign and parentheses are allowed. The result is exact: `pi` stays symbolic, so
    /// `4 pi rad` to `deg` is exactly 720. Every plain number [`Ratio`] reads (`n`, `n/d`, a
    /// decimal) gives the same value here.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNumber`] for anything that is not a value, including an unknown name;
    /// [`Error::Refused`] for a constant that has a unit (`h_P`), since a value is a number;
    /// [`Error::DivisionByZero`] and [`Error::Overflow`] from the arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::{Ratio, ConstantDef, Exactness};
    /// let pi = ConstantDef {
    ///     id: "pi",
    ///     name: "Pi",
    ///     value: (1, 1),
    ///     value_exp: (0, 1),
    ///     unit: "1",
    ///     exactness: Exactness::Defined,
    ///     codata: "",
    ///     conditions: "none (defined)",
    ///     source: "ISO 80000-2:2019",
    ///     note: "",
    /// };
    /// let table = [pi];
    /// assert_eq!(Ratio::parse_expression("4 pi", &table)?, Ratio::from_int(4).checked_mul(Ratio::PI)?);
    /// assert_eq!(Ratio::parse_expression("pi/2", &table)?, Ratio::parse_symbolic("1/2 pi")?);
    /// assert_eq!(Ratio::parse_expression("1.5", &table)?, Ratio::new(3, 2)?);
    /// assert!(Ratio::parse_expression("tau", &table).is_err());
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn parse_expression(s: &str, constants: &[ConstantDef]) -> Result<Self> {
        let (value, rest) = Self::parse_expression_prefix(s, constants)?;
        if rest.trim().is_empty() {
            Ok(value)
        } else {
            Err(invalid(s))
        }
    }

    /// Reads a value from the start of `s` as [`parse_expression`](Self::parse_expression)
    /// does and returns it with the unread rest, so a unit can follow (`2 pi rad`).
    ///
    /// The value ends at the first word that is not a constant, so a unit is never mistaken
    /// for part of the value. An operator with no operand after it is left in the rest.
    ///
    /// # Errors
    ///
    /// As [`parse_expression`](Self::parse_expression), except that text after the value is
    /// returned, not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Ratio;
    /// let (v, rest) = Ratio::parse_expression_prefix("2.5 km", &[])?;
    /// assert_eq!(v, Ratio::new(5, 2)?);
    /// assert_eq!(rest.trim(), "km");
    /// # Ok::<(), dimvexa_core::Error>(())
    /// ```
    pub fn parse_expression_prefix<'a>(
        s: &'a str,
        constants: &[ConstantDef],
    ) -> Result<(Self, &'a str)> {
        let mut p = Parser {
            src: s,
            pos: 0,
            constants,
        };
        let value = p.value(0)?;
        Ok((value, s.get(p.pos..).unwrap_or("")))
    }
}

fn invalid(input: &str) -> Error {
    Error::InvalidNumber {
        input: input.to_string(),
    }
}

struct Parser<'a, 'c> {
    src: &'a str,
    pos: usize,
    constants: &'c [ConstantDef],
}

impl Parser<'_, '_> {
    fn rest(&self) -> &str {
        self.src.get(self.pos..).unwrap_or("")
    }

    fn skip_space(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.src.len().saturating_sub(trimmed.len());
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    const fn bump(&mut self, c: char) {
        self.pos = self.pos.saturating_add(c.len_utf8());
    }

    /// The word at the cursor (letters, digits, `_`; it starts with a letter or `_`).
    fn word(&self) -> &str {
        let rest = self.rest();
        let starts = rest
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_');
        if !starts {
            return "";
        }
        let end = rest
            .char_indices()
            .find(|&(_, c)| !(c.is_alphanumeric() || c == '_'))
            .map_or(rest.len(), |(i, _)| i);
        rest.get(..end).unwrap_or("")
    }

    fn constant(&self, word: &str) -> Option<&ConstantDef> {
        let id = if word == "π" { "pi" } else { word };
        self.constants.iter().find(|c| c.id == id)
    }

    /// `factor { op factor }`. Stops, leaving the cursor before it, at an operator with no
    /// operand and at a word that is not a constant.
    fn value(&mut self, depth: u32) -> Result<Ratio> {
        if depth > MAX_DEPTH {
            return Err(invalid(self.src));
        }
        self.skip_space();
        let mut acc = self.factor(depth)?.ok_or_else(|| invalid(self.src))?;
        loop {
            self.skip_space();
            let before = self.pos;
            match self.peek() {
                Some(op @ ('*' | '/')) => {
                    self.bump(op);
                    self.skip_space();
                    let Some(rhs) = self.factor(depth)? else {
                        self.pos = before;
                        return Ok(acc);
                    };
                    acc = if op == '*' {
                        acc.checked_mul(rhs)?
                    } else {
                        acc.checked_div(rhs)?
                    };
                }
                Some(c) if c.is_alphabetic() || c == '_' => {
                    // juxtaposition is only for a constant: `4 pi`, never `4 m`
                    if self.constant(self.word()).is_none() {
                        return Ok(acc);
                    }
                    let Some(rhs) = self.factor(depth)? else {
                        return Ok(acc);
                    };
                    acc = acc.checked_mul(rhs)?;
                }
                _ => return Ok(acc),
            }
        }
    }

    /// `[sign] atom [^ [sign] digits]`, or `None` (cursor unchanged) when no atom starts here.
    fn factor(&mut self, depth: u32) -> Result<Option<Ratio>> {
        let start = self.pos;
        let mut negative = false;
        if let Some(sign @ ('+' | '-')) = self.peek() {
            negative = sign == '-';
            self.bump(sign);
            self.skip_space();
        }
        let Some(mut atom) = self.atom(depth)? else {
            self.pos = start;
            return Ok(None);
        };
        if self.peek() == Some('^') {
            self.bump('^');
            let exp = self.exponent()?;
            atom = atom.checked_pow(exp)?;
        }
        if negative {
            atom = atom.checked_neg()?;
        }
        Ok(Some(atom))
    }

    fn atom(&mut self, depth: u32) -> Result<Option<Ratio>> {
        match self.peek() {
            Some('(') => {
                self.bump('(');
                self.skip_space();
                let inner = self.value(depth.saturating_add(1))?;
                self.skip_space();
                if self.peek() != Some(')') {
                    return Err(invalid(self.src));
                }
                self.bump(')');
                Ok(Some(inner))
            }
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let (n, rest) = Ratio::parse_prefix(self.rest())?;
                self.pos = self.src.len().saturating_sub(rest.len());
                Ok(Some(n))
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let word = self.word();
                let Some(def) = self.constant(word) else {
                    return Ok(None);
                };
                if def.unit != "1" {
                    return Err(Error::Refused {
                        reason: format!(
                            "'{}' is a constant with the unit {}; a value is a plain number, \
                             so only dimensionless constants (pi) can be written in one",
                            def.id, def.unit
                        ),
                        suggestions: Vec::new(),
                    });
                }
                let value = def.value()?;
                let len = word.len();
                self.pos = self.pos.saturating_add(len);
                Ok(Some(value))
            }
            _ => Ok(None),
        }
    }

    fn exponent(&mut self) -> Result<i32> {
        let mut negative = false;
        if let Some(sign @ ('+' | '-')) = self.peek() {
            negative = sign == '-';
            self.bump(sign);
        }
        let rest = self.rest();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let digits = rest.get(..end).unwrap_or("");
        let n: i32 = digits.parse().map_err(|_| invalid(self.src))?;
        self.pos = self.pos.saturating_add(end);
        if negative {
            n.checked_neg().ok_or_else(|| invalid(self.src))
        } else {
            Ok(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::Exactness;

    fn def(id: &'static str, unit: &'static str, exp: (i32, i32)) -> ConstantDef {
        ConstantDef {
            id,
            name: id,
            value: (1, 1),
            value_exp: exp,
            unit,
            exactness: Exactness::Defined,
            codata: "",
            conditions: "none (defined)",
            source: "test",
            note: "",
        }
    }

    fn table() -> Vec<ConstantDef> {
        vec![def("pi", "1", (0, 1)), def("h_P", "J.s", (-34, 0))]
    }

    fn val(s: &str) -> Result<Ratio> {
        Ratio::parse_expression(s, &table())
    }

    fn sym(s: &str) -> Ratio {
        Ratio::parse_symbolic(s).unwrap()
    }

    #[test]
    fn constants_and_operators() {
        for (text, want) in [
            ("pi", "pi"),
            ("π", "pi"),
            ("4 pi", "4 pi"),
            ("4pi", "4 pi"),
            ("4π", "4 pi"),
            ("4*pi", "4 pi"),
            ("-4 pi", "-4 pi"),
            ("pi/2", "1/2 pi"),
            ("1/2 pi", "1/2 pi"),
            ("1/2*pi", "1/2 pi"),
            ("1/(2 pi)", "1/2 pi^-1"),
            ("2*pi^2", "2 pi^2"),
            ("pi^-1", "pi^-1"),
            ("2 pi pi", "2 pi^2"),
            ("1.5 pi", "3/2 pi"),
            ("(3 pi)/6", "1/2 pi"),
            ("  pi / 2  ", "1/2 pi"),
        ] {
            assert_eq!(val(text).unwrap(), sym(want), "{text:?}");
        }
    }

    #[test]
    fn plain_numbers_read_as_before() {
        for text in [
            "0", "5", "-1.25e-3", "+5", "1e3", "6/4", ".5", "-.25", "1E2",
        ] {
            assert_eq!(
                val(text).unwrap(),
                text.parse::<Ratio>().unwrap(),
                "{text:?}"
            );
        }
    }

    #[test]
    fn errors() {
        for bad in [
            "",
            "abc",
            "tau",
            "2 tau",
            "pi pi pi tau",
            "*2",
            "2*",
            "2/",
            "(2",
            "2)",
            "()",
            "2^",
            "2^x",
            "2 3",
            "pi pi 3",
            "--2",
            "1+2",
            "2 pi rad",
        ] {
            assert!(
                matches!(val(bad), Err(Error::InvalidNumber { .. })),
                "{bad:?} gave {:?}",
                val(bad)
            );
        }
        assert_eq!(val("1/0"), Err(Error::DivisionByZero));
        assert_eq!(val("pi/0"), Err(Error::DivisionByZero));
        assert!(matches!(val("2 h_P"), Err(Error::Refused { .. })));
        assert!(matches!(val("2^200"), Err(Error::Overflow { .. })));
        let deep = format!("{}1{}", "(".repeat(100), ")".repeat(100));
        assert!(matches!(val(&deep), Err(Error::InvalidNumber { .. })));
    }

    #[test]
    fn prefix_leaves_the_unit() {
        let t = table();
        for (text, want, rest) in [
            ("2 pi rad", "2 pi", "rad"),
            ("2pi rad", "2 pi", "rad"),
            ("pi/2 rad", "1/2 pi", "rad"),
            ("2.5 km", "5/2", "km"),
            ("2 m/s", "2", "m/s"),
            ("2 /s", "2", "/s"),
            ("2 * m", "2", "* m"),
            ("2 (m)", "2", "(m)"),
            ("3", "3", ""),
        ] {
            let (v, r) = Ratio::parse_expression_prefix(text, &t).unwrap();
            assert_eq!(v, sym(want), "{text:?}");
            assert_eq!(r.trim(), rest, "{text:?}");
        }
        assert!(Ratio::parse_expression_prefix("km", &t).is_err());
    }

    #[test]
    fn arbitrary_text_never_panics() {
        let t = table();
        for text in [
            "π²",
            "2 π",
            "\u{0}",
            "1e",
            "2e",
            "é",
            "pi^99999999999",
            "pi^2147483647",
        ] {
            let _ = Ratio::parse_expression(text, &t);
        }
    }
}
