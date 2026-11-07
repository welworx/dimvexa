# dimvexa-config

User config files for [dimvexa](../../README.md) (ADR-009): your own names for units, such as
`mt = "tonne"`, in a small TOML file that you load explicitly. A separate crate on purpose,
so the core stays free of file formats and vocabulary: it only offers
`Registry::add_alias`.

```toml
[map]
mt  = "tonne"
ton = "ton_short"
```

A `[[bridge]]` holds a constant you supply for a conversion the core refuses, such as the
density of water for litres to kilograms; it needs `conditions` and a `source`, and the
result is marked `measured`. See `examples/configs/water.toml`.

A mapping points at an existing unit, carries no factor and can never redefine an existing
symbol. Nothing is read from disk by this crate; the caller passes the text to
`Config::parse` and applies the result with `apply`.
