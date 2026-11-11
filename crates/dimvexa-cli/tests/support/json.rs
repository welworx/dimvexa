// SPDX-License-Identifier: MIT OR Apache-2.0
//! A minimal JSON reader for the CLI tests (the CLI itself writes JSON by hand).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    dead_code,
    unreachable_pub,
    missing_docs,
    clippy::use_self,
    clippy::many_single_char_names,
    clippy::doc_markdown,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum V {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<V>),
    Obj(BTreeMap<String, V>),
}

impl V {
    pub fn get(&self, k: &str) -> Option<&V> {
        if let V::Obj(m) = self { m.get(k) } else { None }
    }
    pub fn as_str(&self) -> &str {
        if let V::Str(s) = self {
            s
        } else {
            panic!("not a string: {self:?}")
        }
    }
    pub const fn is_null(&self) -> bool {
        matches!(self, V::Null)
    }

    pub fn as_bool(&self) -> bool {
        if let V::Bool(b) = self {
            *b
        } else {
            panic!("not a bool: {self:?}")
        }
    }
    pub fn as_f64(&self) -> f64 {
        if let V::Num(n) = self {
            *n
        } else {
            panic!("not a number: {self:?}")
        }
    }
    pub fn as_array(&self) -> &[V] {
        if let V::Arr(a) = self {
            a
        } else {
            panic!("not an array: {self:?}")
        }
    }
}

pub fn parse(text: &str) -> Result<V, String> {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut i = 0;
    let v = value(&chars, &mut i)?;
    if i != chars.len() {
        return Err(format!("trailing characters at {i}"));
    }
    Ok(v)
}

fn ws(c: &[char], i: &mut usize) {
    while c.get(*i).is_some_and(|x| x.is_whitespace()) {
        *i += 1;
    }
}

fn value(c: &[char], i: &mut usize) -> Result<V, String> {
    ws(c, i);
    match c.get(*i) {
        Some('{') => {
            *i += 1;
            let mut m = BTreeMap::new();
            ws(c, i);
            if c.get(*i) == Some(&'}') {
                *i += 1;
                return Ok(V::Obj(m));
            }
            loop {
                ws(c, i);
                let k = match value(c, i)? {
                    V::Str(s) => s,
                    other => return Err(format!("object key must be a string, got {other:?}")),
                };
                ws(c, i);
                if c.get(*i) != Some(&':') {
                    return Err(format!("expected ':' at {i}"));
                }
                *i += 1;
                let v = value(c, i)?;
                if m.insert(k.clone(), v).is_some() {
                    return Err(format!("duplicate key {k}"));
                }
                ws(c, i);
                match c.get(*i) {
                    Some(',') => *i += 1,
                    Some('}') => {
                        *i += 1;
                        return Ok(V::Obj(m));
                    }
                    _ => return Err(format!("expected ',' or '}}' at {i}")),
                }
            }
        }
        Some('[') => {
            *i += 1;
            let mut a = Vec::new();
            ws(c, i);
            if c.get(*i) == Some(&']') {
                *i += 1;
                return Ok(V::Arr(a));
            }
            loop {
                a.push(value(c, i)?);
                ws(c, i);
                match c.get(*i) {
                    Some(',') => *i += 1,
                    Some(']') => {
                        *i += 1;
                        return Ok(V::Arr(a));
                    }
                    _ => return Err(format!("expected ',' or ']' at {i}")),
                }
            }
        }
        Some('"') => {
            *i += 1;
            let mut s = String::new();
            while let Some(&ch) = c.get(*i) {
                *i += 1;
                match ch {
                    '"' => return Ok(V::Str(s)),
                    '\\' => {
                        let e = *c.get(*i).ok_or("dangling escape")?;
                        *i += 1;
                        match e {
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            '/' => s.push('/'),
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            'u' => {
                                let hex: String =
                                    c.get(*i..*i + 4).ok_or("short \\u")?.iter().collect();
                                *i += 4;
                                s.push(
                                    char::from_u32(
                                        u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?,
                                    )
                                    .ok_or("bad \\u")?,
                                );
                            }
                            other => return Err(format!("bad escape \\{other}")),
                        }
                    }
                    c if (c as u32) < 0x20 => return Err("raw control character in string".into()),
                    c => s.push(c),
                }
            }
            Err("unterminated string".into())
        }
        Some('t')
            if c.get(*i..*i + 4).map(|s| s.iter().collect::<String>()) == Some("true".into()) =>
        {
            *i += 4;
            Ok(V::Bool(true))
        }
        Some('f')
            if c.get(*i..*i + 5).map(|s| s.iter().collect::<String>()) == Some("false".into()) =>
        {
            *i += 5;
            Ok(V::Bool(false))
        }
        Some('n')
            if c.get(*i..*i + 4).map(|s| s.iter().collect::<String>()) == Some("null".into()) =>
        {
            *i += 4;
            Ok(V::Null)
        }
        Some(ch) if ch.is_ascii_digit() || *ch == '-' => {
            let start = *i;
            while c
                .get(*i)
                .is_some_and(|x| x.is_ascii_digit() || matches!(x, '-' | '+' | '.' | 'e' | 'E'))
            {
                *i += 1;
            }
            let t: String = c[start..*i].iter().collect();
            t.parse::<f64>()
                .map(V::Num)
                .map_err(|_| format!("bad number {t}"))
        }
        other => Err(format!("unexpected {other:?} at {i}")),
    }
}

/// Checks a successful `dimvexa.convert/2` document against the documented schema.
pub fn check_convert_schema(v: &V) {
    for key in [
        "schema",
        "ok",
        "input",
        "output",
        "exact",
        "exactness",
        "dimension",
        "kind",
        "quantity_kind",
        "packs",
        "data_version",
        "sources",
        "config",
        "density",
    ] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.convert/2");
    assert!(v.get("ok").unwrap().as_bool());
    for (obj, keys) in [
        ("input", &["value", "unit"][..]),
        ("output", &["value", "unit", "f64", "truncated"]),
        ("exact", &["value", "factor", "offset"]),
    ] {
        for k in keys {
            assert!(v.get(obj).unwrap().get(k).is_some(), "missing {obj}.{k}");
        }
    }
    assert!(
        v.get("profile").is_none(),
        "profile was removed in convert/2"
    );
    assert_eq!(v.get("dimension").unwrap().as_array().len(), 7);
    assert!(["defined", "measured"].contains(&v.get("exactness").unwrap().as_str()));
    assert!(["linear", "affine"].contains(&v.get("kind").unwrap().as_str()));
    check_config_array(v);
    match v.get("density").expect("missing density") {
        V::Null => {}
        b => check_density(b),
    }
    assert_ne!(v.get("sources").unwrap().as_array(), []);
    assert!(
        v.get("exact")
            .unwrap()
            .get("value")
            .unwrap()
            .as_str()
            .contains('/')
    );
}

/// Checks a `dimvexa.units/3` document against the documented schema.
pub fn check_units_schema(v: &V) {
    if v.get("unit").is_some() {
        return check_unit_detail_schema(v);
    }
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/3");
    assert!(v.get("ok").unwrap().as_bool());
    assert!(v.get("data_version").unwrap().as_str().starts_with('v'));
    assert!(v.get("group").is_some(), "missing group");
    check_config_array(v);
    for b in v.get("densities").expect("missing densities").as_array() {
        check_density(b);
    }
    for m in v.get("mappings").unwrap().as_array() {
        for k in ["name", "unit", "file", "line"] {
            assert!(m.get(k).is_some(), "missing mappings[].{k}");
        }
    }
    let groups = v.get("groups").unwrap().as_array();
    assert_ne!(groups, []);
    for g in groups {
        assert_ne!(g.get("name").unwrap().as_str(), "");
        assert!(g.get("units").unwrap().as_f64() >= 1.0);
    }
    let units = v.get("units").unwrap().as_array();
    for u in units {
        for k in [
            "id",
            "symbol",
            "name",
            "group",
            "aliases",
            "dimension",
            "quantity_kind",
            "exactness",
            "prefixes",
            "namespace",
            "source",
            "origin",
        ] {
            assert!(u.get(k).is_some(), "missing units[].{k}");
        }
        assert_ne!(u.get("id").unwrap().as_str(), "");
        assert_ne!(u.get("symbol").unwrap().as_str(), "");
        assert_ne!(u.get("name").unwrap().as_str(), "");
        assert_eq!(u.get("dimension").unwrap().as_array().len(), 7);
        assert!(
            [
                "plain",
                "absolute_temperature",
                "temperature_difference",
                "gauge_pressure",
                "normal_volume",
                "standard_volume",
            ]
            .contains(&u.get("quantity_kind").unwrap().as_str())
        );
        assert!(["defined", "measured"].contains(&u.get("exactness").unwrap().as_str()));
        assert!(["none", "si", "si_binary"].contains(&u.get("prefixes").unwrap().as_str()));
        assert_ne!(u.get("source").unwrap().as_str(), "");
        let _ = u.get("aliases").unwrap().as_array();
    }
    for a in v.get("ambiguous").unwrap().as_array() {
        for k in ["symbol", "candidates", "planned", "note", "resolved_by"] {
            assert!(a.get(k).is_some(), "missing ambiguous[].{k}");
        }
        // A symbol may have only planned candidates until they enter the table.
        assert!(
            !a.get("candidates").unwrap().as_array().is_empty()
                || !a.get("planned").unwrap().as_array().is_empty()
        );
    }
}

pub fn check_unit_detail_schema(v: &V) {
    assert_eq!(v.get("schema").unwrap().as_str(), "dimvexa.units/3");
    assert!(v.get("ok").unwrap().as_bool());
    let u = v.get("unit").unwrap();
    for k in [
        "id",
        "symbol",
        "name",
        "prefix",
        "base",
        "group",
        "aliases",
        "dimension",
        "quantity_kind",
        "exactness",
        "prefixes",
        "namespace",
        "factor",
        "offset",
        "sources",
        "note",
        "origin",
        "mapping",
    ] {
        assert!(u.get(k).is_some(), "missing unit.{k}");
    }
    assert_eq!(u.get("dimension").unwrap().as_array().len(), 7);
    assert!(u.get("factor").unwrap().as_str().contains('/'));
    assert!(u.get("offset").unwrap().as_str().contains('/'));
}

/// The `config` array of `dimvexa.convert/2` and `dimvexa.units/3`: `[{file, hash}]`.
fn check_config_array(v: &V) {
    for c in v.get("config").expect("missing config").as_array() {
        assert_ne!(c.get("file").unwrap().as_str(), "");
        let hash = c.get("hash").unwrap().as_str();
        assert!(
            hash.starts_with("fnv1a64:") && hash.len() == 8 + 16,
            "{hash}"
        );
    }
}

/// A density object of `dimvexa.convert/2` and `dimvexa.units/3`.
fn check_density(b: &V) {
    for k in [
        "name",
        "kind",
        "value",
        "unit",
        "conditions",
        "source",
        "file",
        "line",
    ] {
        assert!(b.get(k).is_some(), "missing density.{k}");
    }
    assert_eq!(b.get("kind").unwrap().as_str(), "density");
    assert_eq!(b.get("unit").unwrap().as_str(), "kg/m3");
    assert_ne!(b.get("conditions").unwrap().as_str(), "");
    assert_ne!(b.get("source").unwrap().as_str(), "");
}

/// A small JSON Schema checker for the files in `docs/schemas/`: `type`, `const`, `enum`,
/// `required`, `properties`, `items`, `oneOf`, `minItems`, `maxItems` and `minimum`. Enough to
/// catch a CLI output that drifts from the documented schema. Returns the first mismatch.
pub fn schema_mismatch(schema: &V, value: &V, path: &str) -> Option<String> {
    if let Some(V::Arr(options)) = schema.get("oneOf") {
        let matching = options
            .iter()
            .filter(|o| schema_mismatch(o, value, path).is_none())
            .count();
        return (matching == 0).then(|| format!("{path}: matches none of the oneOf branches"));
    }
    if let Some(c) = schema.get("const") {
        if c != value {
            return Some(format!("{path}: expected const {c:?}, got {value:?}"));
        }
    }
    if let Some(V::Arr(options)) = schema.get("enum") {
        if !options.contains(value) {
            return Some(format!("{path}: {value:?} is not one of {options:?}"));
        }
    }
    if let Some(t) = schema.get("type") {
        let names: Vec<&str> = match t {
            V::Str(s) => vec![s.as_str()],
            V::Arr(a) => a.iter().map(V::as_str).collect(),
            _ => vec![],
        };
        let ok = names.iter().any(|n| match (*n, value) {
            ("null", V::Null)
            | ("boolean", V::Bool(_))
            | ("string", V::Str(_))
            | ("array", V::Arr(_))
            | ("object", V::Obj(_))
            | ("number", V::Num(_)) => true,
            ("integer", V::Num(x)) => x.fract() == 0.0,
            _ => false,
        });
        if !names.is_empty() && !ok {
            return Some(format!("{path}: expected type {names:?}, got {value:?}"));
        }
    }
    if let V::Obj(map) = value {
        if let Some(V::Arr(req)) = schema.get("required") {
            for k in req {
                if !map.contains_key(k.as_str()) {
                    return Some(format!("{path}: missing required key '{}'", k.as_str()));
                }
            }
        }
        if let Some(V::Obj(props)) = schema.get("properties") {
            for (k, sub) in props {
                if let Some(v) = map.get(k) {
                    if let Some(m) = schema_mismatch(sub, v, &format!("{path}.{k}")) {
                        return Some(m);
                    }
                }
            }
        }
    }
    if let V::Arr(items) = value {
        if let Some(V::Num(n)) = schema.get("minItems") {
            if f64::from(u32::try_from(items.len()).unwrap()) < *n {
                return Some(format!("{path}: fewer than {n} items"));
            }
        }
        if let Some(V::Num(n)) = schema.get("maxItems") {
            if f64::from(u32::try_from(items.len()).unwrap()) > *n {
                return Some(format!("{path}: more than {n} items"));
            }
        }
        if let Some(sub) = schema.get("items") {
            for (i, v) in items.iter().enumerate() {
                if let Some(m) = schema_mismatch(sub, v, &format!("{path}[{i}]")) {
                    return Some(m);
                }
            }
        }
    }
    if let (Some(V::Num(min)), V::Num(x)) = (schema.get("minimum"), value) {
        if x < min {
            return Some(format!("{path}: {x} is below the minimum {min}"));
        }
    }
    None
}
