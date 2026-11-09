# dimvexa-config

**Beta.** Younger and less proven than `dimvexa-core`: the file format and the API may still
change, and the known limitations are in `docs/known-limitations.md`. The core does not depend
on this crate.

User config files for [dimvexa](../../README.md) (ADR-009, ADR-014): your own names for units, such as
`mt = "tonne"`, in a small TOML file. A separate crate on purpose, so the core stays free of file
formats and vocabulary: it only offers `Registry::add_alias`.

```toml
[map]
mt  = "tonne"
ton = "ton_short"
```

A `[[unit]]` defines your own unit as an exact multiple of an existing one (`bbl_oil` = 42
`gal_us`), with a mandatory source; it can never take an existing name. See
`examples/configs/oil.toml`.

A `[[density]]` holds a constant you supply for a conversion the core refuses, such as the
density of water for litres to kilograms; it needs `conditions` and a `source`, and the
result is marked `measured`. See `examples/configs/water.toml`. `Constant::density_from_text`
reads a density the caller writes (`998.2067kg/m3`).

Configs also ship inside the crate: `us`, `gb`, `metric-trade`, `year-julian`, `year-civil` and
`water` (`BUNDLED`, `bundled(name)`). `is_path` tells a path from a shipped name: a value with
a `/` or `\`, or ending in `.toml`, is a path.

A mapping points at an existing unit, carries no factor and can never redefine an existing
symbol. Nothing is read from disk by this crate; the caller passes the text to
`Config::parse` and applies the result with `apply`.
