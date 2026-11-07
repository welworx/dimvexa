// SPDX-License-Identifier: MIT OR Apache-2.0
//! A minimal JSON reader and writer (std only), enough for the corpus and the result files.
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn get(&self, key: &str) -> &Json {
        static NULL: Json = Json::Null;
        match self {
            Json::Obj(m) => m.get(key).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
    pub fn str(&self) -> &str {
        match self {
            Json::Str(s) => s,
            _ => "",
        }
    }
    pub fn num(&self) -> f64 {
        match self {
            Json::Num(n) => *n,
            _ => f64::NAN,
        }
    }
    pub fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => &[],
        }
    }
    pub fn obj() -> Json {
        Json::Obj(BTreeMap::new())
    }
    pub fn set(&mut self, key: &str, v: Json) {
        if let Json::Obj(m) = self {
            m.insert(key.to_string(), v);
        }
    }
    pub fn s(v: &str) -> Json {
        Json::Str(v.to_string())
    }
    pub fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.is_finite() {
                    let _ = write!(out, "{n}");
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => write_str(out, s),
            Json::Arr(a) => {
                if a.iter().all(|x| !matches!(x, Json::Obj(_) | Json::Arr(_))) {
                    out.push('[');
                    for (i, x) in a.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        x.write(out, indent);
                    }
                    out.push(']');
                } else {
                    out.push_str("[\n");
                    for (i, x) in a.iter().enumerate() {
                        out.push_str(&" ".repeat(indent + 1));
                        x.write(out, indent + 1);
                        out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
                    }
                    out.push_str(&" ".repeat(indent));
                    out.push(']');
                }
            }
            Json::Obj(m) => {
                out.push_str("{\n");
                let n = m.len();
                for (i, (k, v)) in m.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 1));
                    write_str(out, k);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                    out.push_str(if i + 1 < n { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(indent));
                out.push('}');
            }
        }
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn parse(text: &str) -> Result<Json, String> {
    let b = text.as_bytes();
    let mut p = Parser { b, i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != b.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.i))
        }
    }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("bad literal at byte {}", self.i))
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.b.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return Err(format!("bad object at byte {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    a.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return Err(format!("bad array at byte {}", self.i)),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(_) => {
                let s = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[s..self.i])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(Json::Num)
                    .ok_or_else(|| format!("bad number at byte {s}"))
            }
            None => Err("unexpected end".to_string()),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("expected string at byte {}", self.i));
        }
        self.i += 1;
        let mut out: Vec<u16> = Vec::new();
        let mut bytes: Vec<u8> = Vec::new();
        let flush = |bytes: &mut Vec<u8>, out: &mut Vec<u16>| {
            if !bytes.is_empty() {
                out.extend(String::from_utf8_lossy(bytes).encode_utf16());
                bytes.clear();
            }
        };
        loop {
            let c = *self.b.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    flush(&mut bytes, &mut out);
                    let e = *self.b.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(0x22),
                        b'\\' => out.push(0x5c),
                        b'/' => out.push(0x2f),
                        b'n' => out.push(0x0a),
                        b'r' => out.push(0x0d),
                        b't' => out.push(0x09),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'u' => {
                            let h = std::str::from_utf8(self.b.get(self.i..self.i + 4).ok_or("bad \\u")?)
                                .map_err(|e| e.to_string())?;
                            out.push(u16::from_str_radix(h, 16).map_err(|e| e.to_string())?);
                            self.i += 4;
                        }
                        _ => return Err("bad escape".to_string()),
                    }
                }
                c => bytes.push(c),
            }
        }
        flush(&mut bytes, &mut out);
        Ok(String::from_utf16_lossy(&out))
    }
}
