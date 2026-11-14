// SPDX-License-Identifier: MIT OR Apache-2.0
//! The single error type of the library and its stable machine-readable codes.

use core::fmt;

use crate::dim::Dim;
use crate::kind::Kind;

/// Everything that can go wrong in dimvexa. Library code never panics: every fallible
/// operation returns this type.
///
/// Each variant has a stable, documented machine-readable code (see [`Error::code`] and
/// [`Error::ALL_CODES`]). The enum is `#[non_exhaustive]`: new variants may be added in
/// minor releases, codes are never renamed within a schema version.
///
/// # Examples
///
/// ```
/// use dimvexa_core::{Error, Ratio};
///
/// let err = Ratio::new(1, 0).unwrap_err();
/// assert_eq!(err.code(), "division_by_zero");
/// assert!(matches!(err, Error::DivisionByZero));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Syntax error in a unit expression or number. Code `parse_error`.
    Parse {
        /// The complete input that failed to parse.
        input: String,
        /// Zero-based character position of the problem.
        position: usize,
        /// Human-readable explanation.
        message: String,
    },
    /// A number could not be read. Code `invalid_number`.
    InvalidNumber {
        /// The text that is not a valid number.
        input: String,
    },
    /// The symbol is not in the registry. Code `unknown_unit`.
    UnknownUnit {
        /// The unknown symbol.
        symbol: String,
        /// Close matches, for example the same text with different capitalisation.
        suggestions: Vec<String>,
    },
    /// The bare symbol has several meanings and no config resolves it.
    /// Code `ambiguous_symbol`.
    AmbiguousSymbol {
        /// The ambiguous symbol as written.
        symbol: String,
        /// Explicit unit symbols the caller may use instead.
        candidates: Vec<String>,
    },
    /// The expression is syntactically valid but its meaning depends on operator
    /// precedence conventions. Code `ambiguous_expression`.
    AmbiguousExpression {
        /// The offending expression.
        input: String,
        /// Explanation including how to write it unambiguously.
        message: String,
    },
    /// The two units measure different physical dimensions. Code `dimension_mismatch`.
    DimensionMismatch {
        /// Dimension of the source.
        from: Dim,
        /// Dimension of the target.
        to: Dim,
    },
    /// The two units have the same dimension but incompatible kinds, for example an
    /// absolute temperature and a temperature difference. Code `kind_mismatch`.
    KindMismatch {
        /// Kind of the left or source operand.
        from: Kind,
        /// Kind of the right or target operand.
        to: Kind,
        /// What was attempted.
        operation: &'static str,
        /// Units to use instead, for example `delta_K` for a temperature difference.
        suggestions: Vec<String>,
    },
    /// A checked integer operation overflowed. Code `overflow`.
    Overflow {
        /// The operation that overflowed.
        operation: &'static str,
    },
    /// The exact result exists but cannot be written in the number form of the library
    /// (`num/den * 10^k * pi^m`), for example `1 + pi`. The value is never approximated. Not an
    /// overflow: no bound of the integers is involved. Code `unrepresentable`.
    Unrepresentable {
        /// The operation that has no exact result in this form.
        operation: &'static str,
    },
    /// Division by zero in exact arithmetic. Code `division_by_zero`.
    DivisionByZero,
    /// Input and output slices have different lengths. Code `length_mismatch`.
    LengthMismatch {
        /// Length of the input slice.
        input: usize,
        /// Length of the output slice.
        output: usize,
    },
    /// The operation is deliberately not supported because it would require guessing.
    /// Code `refused`.
    Refused {
        /// Why the operation is refused and what to do instead.
        reason: String,
        /// Units or expressions to use instead (may be empty).
        suggestions: Vec<String>,
    },
    /// A unit definition is inconsistent. Code `invalid_definition`.
    InvalidDefinition {
        /// Symbol of the offending definition.
        symbol: String,
        /// What is wrong.
        message: String,
    },
    /// A symbol or alias is registered twice. Code `duplicate_symbol`.
    DuplicateSymbol {
        /// The symbol that is already taken.
        symbol: String,
    },
}

impl Error {
    /// Every stable machine-readable error code, in declaration order.
    pub const ALL_CODES: &'static [&'static str] = &[
        "parse_error",
        "invalid_number",
        "unknown_unit",
        "ambiguous_symbol",
        "ambiguous_expression",
        "dimension_mismatch",
        "kind_mismatch",
        "overflow",
        "unrepresentable",
        "division_by_zero",
        "length_mismatch",
        "refused",
        "invalid_definition",
        "duplicate_symbol",
    ];

    /// The stable machine-readable code of this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Error;
    /// let e = Error::Overflow { operation: "mul" };
    /// assert_eq!(e.code(), "overflow");
    /// assert!(Error::ALL_CODES.contains(&e.code()));
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Parse { .. } => "parse_error",
            Self::InvalidNumber { .. } => "invalid_number",
            Self::UnknownUnit { .. } => "unknown_unit",
            Self::AmbiguousSymbol { .. } => "ambiguous_symbol",
            Self::AmbiguousExpression { .. } => "ambiguous_expression",
            Self::DimensionMismatch { .. } => "dimension_mismatch",
            Self::KindMismatch { .. } => "kind_mismatch",
            Self::Overflow { .. } => "overflow",
            Self::Unrepresentable { .. } => "unrepresentable",
            Self::DivisionByZero => "division_by_zero",
            Self::LengthMismatch { .. } => "length_mismatch",
            Self::Refused { .. } => "refused",
            Self::InvalidDefinition { .. } => "invalid_definition",
            Self::DuplicateSymbol { .. } => "duplicate_symbol",
        }
    }

    /// Everything the error recommends instead: close matches for an unknown unit, the explicit
    /// symbols for an ambiguous one, and the difference unit (`delta_K`, ...) for a refused or
    /// mismatched temperature operation. Empty if the error has no recommendation.
    ///
    /// # Examples
    ///
    /// ```
    /// use dimvexa_core::Error;
    /// let e = Error::UnknownUnit { symbol: "PSI".into(), suggestions: vec!["psi".into()] };
    /// assert_eq!(e.suggestions(), ["psi"]);
    /// assert!(Error::DivisionByZero.suggestions().is_empty());
    /// ```
    #[must_use]
    pub fn suggestions(&self) -> &[String] {
        match self {
            Self::AmbiguousSymbol { candidates, .. } => candidates,
            Self::UnknownUnit { suggestions, .. }
            | Self::KindMismatch { suggestions, .. }
            | Self::Refused { suggestions, .. } => suggestions,
            _ => &[],
        }
    }

    /// Candidate symbols attached to the error, if any (ambiguous or unknown symbols).
    #[must_use]
    pub fn candidates(&self) -> &[String] {
        match self {
            Self::AmbiguousSymbol { candidates, .. } => candidates,
            Self::UnknownUnit { suggestions, .. } => suggestions,
            _ => &[],
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                input,
                position,
                message,
            } => {
                write!(
                    f,
                    "cannot parse '{input}' at position {position}: {message}"
                )
            }
            Self::InvalidNumber { input } => write!(f, "'{input}' is not a valid number"),
            Self::UnknownUnit {
                symbol,
                suggestions,
            } => {
                write!(f, "unknown unit '{symbol}'")?;
                if !suggestions.is_empty() {
                    write!(f, " (did you mean: {}?)", suggestions.join(", "))?;
                }
                Ok(())
            }
            Self::AmbiguousSymbol { symbol, candidates } => write!(
                f,
                "'{symbol}' is ambiguous: use {}, or load a config that resolves it",
                candidates.join(" or ")
            ),
            Self::AmbiguousExpression { input, message } => {
                write!(f, "ambiguous expression '{input}': {message}")
            }
            Self::DimensionMismatch { from, to } => {
                write!(
                    f,
                    "dimension mismatch: [{from}] cannot be converted to [{to}]"
                )
            }
            Self::KindMismatch {
                from,
                to,
                operation,
                suggestions,
            } => {
                write!(
                    f,
                    "kind mismatch in {operation}: {from} and {to} are not compatible"
                )?;
                if from.is_angular() && to.is_angular() {
                    write!(
                        f,
                        " (an angle per time and a cycle rate differ by 2 pi per cycle, f = w / 2 pi; \
                         the library does not guess which you mean: convert to a plain unit such \
                         as 1/s and apply 2 pi yourself)"
                    )?;
                }
                if !suggestions.is_empty() {
                    write!(
                        f,
                        " (a temperature point and a temperature difference are different kinds; \
                         for a difference use {})",
                        suggestions.join(" or ")
                    )?;
                }
                Ok(())
            }
            Self::Overflow { operation } => {
                write!(f, "integer overflow in exact arithmetic ({operation})")
            }
            Self::Unrepresentable { operation } => write!(
                f,
                "the exact result cannot be written as a fraction times powers of ten and pi \
                 ({operation}); it is never approximated"
            ),
            Self::DivisionByZero => f.write_str("division by zero"),
            Self::LengthMismatch { input, output } => {
                write!(
                    f,
                    "input has {input} values but output has room for {output}"
                )
            }
            Self::Refused { reason, .. } => write!(f, "refused: {reason}"),
            Self::InvalidDefinition { symbol, message } => {
                write!(f, "invalid definition of '{symbol}': {message}")
            }
            Self::DuplicateSymbol { symbol } => write!(f, "symbol '{symbol}' is already defined"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias used throughout the crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrepresentable_is_not_overflow() {
        let e = Error::Unrepresentable {
            operation: "ratio add (different exponents)",
        };
        assert_eq!(e.code(), "unrepresentable");
        assert!(Error::ALL_CODES.contains(&e.code()));
        let text = e.to_string();
        assert!(text.contains("never approximated"), "{text}");
        assert!(!text.contains("overflow"), "{text}");
        assert_ne!(e.code(), Error::Overflow { operation: "x" }.code());
    }

    #[test]
    fn every_code_is_listed_once() {
        let mut codes = Error::ALL_CODES.to_vec();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Error::ALL_CODES.len());
    }
}
