// SPDX-License-Identifier: MIT OR Apache-2.0
//! The text of `--help`: the usage lines, the overview and one page per command.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

/// The usage lines: what a usage error shows, and the top of `--help`.
pub const USAGE: &str = "\
Usage:
  dimvexa [convert] VALUE FROM TO [OPTIONS]
  dimvexa calc EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--sci] [--json]
  dimvexa units [--groups | GROUP | UNIT] [--json]
  dimvexa configs [NAME_OR_PATH] [--json]
  dimvexa constants [NAME] [--config NAME_OR_PATH]... [--json]
  dimvexa update [--check] [-y]

Run 'dimvexa --help' for commands, options and examples.
";

/// The body of `--help`, after the usage lines. Every line is at most 80 columns, and every
/// command under `Examples:` is run by a test in an empty directory, so it must work as typed.
pub const HELP: &str = "\
Usage:
  dimvexa [convert] VALUE FROM TO [OPTIONS]
  dimvexa calc EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--json]
  dimvexa units [--groups | GROUP | UNIT] [--json]
  dimvexa configs [NAME_OR_PATH] [--json]
  dimvexa constants [NAME] [--config NAME_OR_PATH]... [--json]
  dimvexa update [--check] [-y]

Converts VALUE from unit FROM to unit TO. Ambiguous input is an error.
Run 'dimvexa COMMAND --help' (convert, calc, units, configs, constants,
update) for details.

Examples:
  dimvexa convert 10 m/s km/h
  dimvexa convert 1 psi Pa --exact
  dimvexa convert 4pi rad deg
  dimvexa convert 100 degC degF --json
  dimvexa convert 1 ton kg --config us-customary
  dimvexa convert 1 L kg --config water --const water_25C
  dimvexa convert 1 L kg --const 998.2067kg/m3
  dimvexa calc \"2 kip/ft * (20 ft)^2 / 8\" --to kN*m
  dimvexa calc \"sqrt(2 * 50 kPa / 998 kg/m3)\" --to m/s
  dimvexa units volume
  dimvexa units psi
  dimvexa configs water
  dimvexa constants
  dimvexa constants boltzmann_constant

Commands:
  convert          convert VALUE from FROM to TO ('convert' may be left out)
  calc             calculate with units, exact where possible: an equation
                   such as \"2 m * 3 m\" or sqrt(2 * dp / rho) with --var
  units            list the units by quantity, and the ambiguous symbols refused
  units --groups   list only the groups (length, mass, volume, ...) and sizes
  units GROUP      list one group, for example: dimvexa units volume
  units UNIT       show one unit in detail, for example: dimvexa units psi
  configs          list the configs that ship with dimvexa (us, imperial, ...)
  configs NAME     show what a config contains: names, units, constants with
                   their conditions and sources; a path shows your own file
  constants        list the constants: name, exact value, unit, conditions and
                   source (built in, plus those of any --config)
  constants NAME   show one constant: dimvexa constants speed_of_light
  update           replace this binary with the latest GitHub release (needs
                   curl and tar; checks the published SHA-256 first; see
                   docs/adr/015-update-command.md)
  update --check   only report whether a newer release exists; exit 0 up to
                   date, 3 newer release available, 1 error
  update -y        do not ask \"Update to vX.Y.Z? [y/N]\" first (also --yes)

Options:
  --exact          print the exact result and its decimal: a fraction, with pi
                   or a power of ten when the value has one
                   (1/2 pi = 1.5707963267948966)
  --json           machine-readable output (schemas dimvexa.convert/3,
                   dimvexa.calc/1, dimvexa.units/5, dimvexa.configs/3,
                   dimvexa.constants/1)
  --pretty         --json, indented for reading (same fields, same order)
  --color WHEN     auto (default: a terminal, unless NO_COLOR is set), always
                   (also CLICOLOR_FORCE=1) or never; JSON is never coloured
  --config NAME_OR_PATH
                   (beta) apply a config; repeatable, later ones win and a
                   warning names what they replace. A shipped one is named
                   (us-customary, us-survey, imperial, metric-trade,
                   year-julian, year-civil, water); a file has a path
                   (./mine.toml, or anything with a / or ending in .toml).
                   Without --config the names stay strict.
                   See docs/adr/014-one-config-mechanism.md
  --const NAME_OR_VALUE
                   density for mass <-> volume: the name of a constant of a
                   density unit (--config water --const water_25C) or your own
                   number (--const 998.2067kg/m3). The assumption is printed
  --var NAME=VALUE the value of a name in a calc expression, a quantity such
                   as --var dp=\"50 kPa\"; repeatable
  --to UNIT        the unit a calc result is shown in
  --version        print the version and the data table version
  -h, --help       print this help

Precision: the CLI always uses the exact path, so the printed number is the
exact result rounded once to the nearest f64 (0 ulp). The library's fast f64
path is within 1 ulp (linear units) or 2 ulp (temperature scales such as degC
and degF) of it; see docs/precision.md.

Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error;
`update --check` also exits 3 when a newer release exists.
";

/// `dimvexa convert --help`. Examples are run by a test, like those of `HELP`.
pub const HELP_CONVERT: &str = "\
dimvexa convert - convert a value from one unit to another

Usage:
  dimvexa [convert] VALUE FROM TO [--exact] [--json | --pretty]
      [--config NAME_OR_PATH]... [--const NAME_OR_VALUE]

VALUE is a number, or a number with pi: 4pi, 4 pi, pi/2, 2*pi^2, 1/(2*pi). It
may use * / ^ (integer power) and parentheses; there is no addition, and only
a constant without a unit (pi) is allowed. pi stays exact: 4 pi rad is
exactly 720 deg. FROM and TO are units or unit expressions (m/s, kW.h, psi).
Units of different quantities, or an ambiguous symbol, are an error.

Examples:
  dimvexa convert 10 m/s km/h
  dimvexa convert 1 psi Pa --exact
  dimvexa convert 4pi rad deg
  dimvexa convert pi/2 rad deg --exact
  dimvexa convert 100 degC degF --json
  dimvexa convert 1 ton kg --config us-customary
  dimvexa convert 1 L kg --config water --const water_25C
  dimvexa convert 1 L kg --const 998.2067kg/m3

Options:
  --exact          print the exact result and its decimal: a fraction, with pi
                   or a power of ten when the value has one
                   (1/2 pi = 1.5707963267948966)
  --json           machine-readable output (schema dimvexa.convert/3)
  --pretty         --json, indented for reading
  --config NAME_OR_PATH
                   (beta) apply a config; repeatable, later ones win and a
                   warning names what they replace. A shipped one is named
                   (us-customary, us-survey, imperial, metric-trade,
                   year-julian, year-civil, water); a file has a path
                   (./mine.toml, or anything with a / or ending in .toml).
                   See: dimvexa configs --help
  --const NAME_OR_VALUE
                   density for mass <-> volume: the name of a constant of a
                   density unit (--config water --const water_25C) or your own
                   number (--const 998.2067kg/m3). The assumption is printed
  --color WHEN     auto, always or never (see: dimvexa --help)

Exit codes: 0 success, 1 conversion refused or invalid input, 2 usage error.
";

/// `dimvexa calc --help`. Examples are run by a test, like those of `HELP`.
pub const HELP_CALC: &str = "\
dimvexa calc - calculate with units, exact where the mathematics is exact

Usage:
  dimvexa calc [--] EXPRESSION [--var NAME=VALUE]... [--to UNIT] [--exact]
      [--sci[=N]] [--autoprefix] [--json | --pretty] [--config NAME_OR_PATH]...

EXPRESSION is an equation: numbers, quantities written with their unit
(10 cm2, 998 kg/m3, 8.314 J/(mol*K), 20 degC), names given with --var, the
constant pi, + - * / and parentheses, a power with a fraction as exponent
(x^0.5, y^(1/2) and sqrt(x) agree; R^(2/3)) and the functions sqrt, cbrt,
abs, hypot, pow, floor, ceil, round, min, max, ln, exp, log10 (lg), log2
(lb), log(x, base), sin, cos, tan, asin, acos, atan, atan2(y, x), sinh, cosh,
tanh, asinh, acosh, atanh, x.to(UNIT) (convert inside an equation:
20 degC.to(K)); the names pi and the constants of 'dimvexa constants' and of
the --config files (speed_of_light, boltzmann_constant, molar_gas_constant,
...; --var may replace one, with a warning). Exponents may be names or sums
(x^n, 2^(1+1)); 2 pi r and m² or N·m are read. Quote it, and put a space on
both sides of an operator between quantities (write -- before an expression
that starts like an option, such as -h): a unit is the text after a number up
to the next space, so 50 kPa / 998 kg/m3 is a quotient.

The result is exact when the mathematics is exact (sqrt(9 m2) is 3 m,
sin(30 deg) is 0.5) and starts with \u{2248} when it is not: an approximate f64,
and so is everything computed from it. Without --to the unit is the one
the arithmetic gave. A temperature point (20 degC) cannot be multiplied;
log(x) alone is ambiguous: write ln, lg or log10, lb or log2, or
log(x, base).

Examples:
  dimvexa calc \"2 kip/ft * (20 ft)^2 / 8\" --to kN*m
  dimvexa calc \"sqrt(2 * 50 kPa / 998 kg/m3)\" --to m/s
  dimvexa calc \"sqrt(2 * dp / rho)\" --var dp=\"50 kPa\" --var rho=998kg/m3
  dimvexa calc \"100 kg * 1 g_n * sin(30 deg)\" --to N
  dimvexa calc \"planck_constant * speed_of_light / 500 nm\" --to J --sci=4

Options:
  --var NAME=VALUE the value of a name, a quantity; repeatable
  --to UNIT        show the result in UNIT (an exact result stays exact)
  --exact          also print the exact fraction of an exact result
  --sci[=N]        write the number as 1.5e3 (all digits, or N significant
                   digits, 1 to 17: --sci=4 is 1.500e3)
  --autoprefix     choose the SI prefix that puts the number between 1 and
                   1000: 1500 m is 1.5 km (not with --to)
  --json           machine-readable output (schema dimvexa.calc/1)
  --pretty         --json, indented for reading
  --config NAME_OR_PATH
                   apply a config (your own units and names); repeatable
  -h, --help       print this help

Exit codes: 0 success, 1 the expression is refused or invalid, 2 usage error.
";

/// `dimvexa units --help`.
pub const HELP_UNITS: &str = "\
dimvexa units - list the supported units and show one in detail

Usage:
  dimvexa units [--groups | GROUP | UNIT] [--json | --pretty]
      [--config NAME_OR_PATH]...

Examples:
  dimvexa units
  dimvexa units --groups
  dimvexa units volume
  dimvexa units psi
  dimvexa units km --json

Forms:
  units            every unit by quantity, then the ambiguous symbols that are
                   refused
  units --groups   only the groups (length, mass, volume, ...) with their sizes
  units GROUP      one group
  units UNIT       one unit: id, symbol, name, factor, exactness, status
                   (current or legacy), sources; a prefixed unit (km) is
                   named from its parts

Options:
  --json           machine-readable output (schema dimvexa.units/5)
  --pretty         --json, indented for reading
  --config NAME_OR_PATH
                   include the units and names of a config; repeatable (a
                   warning names what a later config replaces)
  --color WHEN     auto, always or never (see: dimvexa --help)

Exit codes: 0 success, 1 unknown unit or group or invalid config, 2 usage error.
";

/// `dimvexa configs --help`.
pub const HELP_CONFIGS: &str = "\
dimvexa configs - list the shipped configs and show what one contains

Usage:
  dimvexa configs [NAME_OR_PATH] [--json | --pretty]

A config adds names, units and constants to the strict default tables; apply one
to a conversion with --config. A density (a constant of a density unit, the
mass <-> volume number) comes only from a config or your own number; use it
with --const. A config unit may have an offset: an affine scale of your own,
such as a gauge reading with your ambient pressure. See also: dimvexa constants
--help.

Examples:
  dimvexa configs
  dimvexa configs water
  dimvexa configs us-customary --json

Forms:
  configs          the configs that ship with dimvexa (us-customary,
                   us-survey, imperial, metric-trade, year-julian,
                   year-civil, water)
  configs NAME     its names, units, and constants with their conditions and
                   sources
  configs PATH     the same for your own file (./mine.toml, or a name with a /
                   or ending in .toml)

Options:
  --json           machine-readable output (schema dimvexa.configs/3)
  --pretty         --json, indented for reading
  --color WHEN     auto, always or never (see: dimvexa --help)

Exit codes: 0 success, 1 invalid config file, 2 usage error (an unknown config
name is one).
";

/// `dimvexa constants --help`.
pub const HELP_CONSTANTS: &str = "\
dimvexa constants - list the named constants and show one

Usage:
  dimvexa constants [NAME] [--config NAME_OR_PATH]... [--json | --pretty]

A constant is an exact value of a unit with its conditions and source; nothing
is rounded and nothing is applied unless you ask. The built-in ones (pi,
planck_constant, elementary_charge, boltzmann_constant, avogadro_constant,
speed_of_light, molar_gas_constant) cannot be redefined by a config. A config
adds its own with [[constant]]; a constant of a density unit is what --const
uses for mass <-> volume.

Examples:
  dimvexa constants
  dimvexa constants boltzmann_constant
  dimvexa constants --config water --json

Forms:
  constants        every built-in constant, then those of the --config files
  constants NAME   one constant, with its note

Options:
  --config NAME_OR_PATH
                   also list the constants of this config; repeatable (a
                   warning names what a later config replaces)
  --json           machine-readable output (schema dimvexa.constants/1)
  --pretty         --json, indented for reading
  --color WHEN     auto, always or never (see: dimvexa --help)

Exit codes: 0 success, 1 invalid config file, 2 usage error (an unknown
constant name is one).
";

/// `dimvexa update --help`.
pub const HELP_UPDATE: &str = "\
dimvexa update - replace this binary with the latest GitHub release

Usage:
  dimvexa update [--check] [-y]

Needs curl and tar. Checks the published SHA-256 of the download before it
replaces anything (see docs/adr/015-update-command.md).

Examples:
  dimvexa update --check

Options:
  --check          only report whether a newer release exists
  -y, --yes        do not ask \"Update to vX.Y.Z? [y/N]\" first

Exit codes: 0 up to date, updated, or declined at the prompt; 1 error; 2 usage
error; --check exits 3 when a newer release exists.
";

/// Shown after the project facts; the long form is `docs/licensing.md`.
pub const DISCLAIMER: &str = "\
Provided as is, without warranty. Unit factors and sources can contain errors:
verify any result you depend on against the primary source, and do not use it as
the only safeguard in safety-critical, medical, legal, financial or
custody-transfer settings. Independent project, not affiliated with other unit
libraries. Terms: LICENSE-MIT and LICENSE-APACHE.";

/// Name, version, tagline and where the project lives, from the package metadata.
pub fn about() -> String {
    format!(
        "License:    {}\nAuthors:    {}\nWebsite:    {}\n\n{DISCLAIMER}\n",
        env!("CARGO_PKG_LICENSE"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_HOMEPAGE")
    )
}

/// The text of `dimvexa COMMAND --help`.
pub fn command_help(topic: &str) -> &'static str {
    match topic {
        "calc" => HELP_CALC,
        "units" => HELP_UNITS,
        "configs" => HELP_CONFIGS,
        "constants" => HELP_CONSTANTS,
        "update" => HELP_UPDATE,
        _ => HELP_CONVERT,
    }
}

/// The text of `--help`: a title line, the usage, and the project facts.
pub fn help_text() -> String {
    format!(
        "dimvexa {} - Exact factors. Loud failures. Zero guessing.\n\n{HELP}\n{}",
        env!("CARGO_PKG_VERSION"),
        about()
    )
}
