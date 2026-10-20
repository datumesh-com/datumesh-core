//! Strict JSON (SPEC 6.1, I-JSON) and the RFC 8785 JSON Canonicalization Scheme (SPEC 6.2).
//!
//! [`parse`] accepts exactly the texts SPEC 6.1 accepts and reports failures in the order it
//! defines. [`Value::to_jcs`] writes the RFC 8785 serialisation, including ECMAScript number
//! formatting, so it is a complete JCS even though Datumesh objects never contain JSON numbers.

use crate::error::{err, ErrorCode, Result};
use core::cmp::Ordering;

/// The deepest nesting of arrays and objects accepted (SPEC 6.1).
pub const MAX_DEPTH: usize = 64;

/// A JSON number: the text it was written as, and its IEEE 754 double value.
#[derive(Clone, Debug, PartialEq)]
pub struct Number {
    text: String,
    value: f64,
}

impl Number {
    /// The number exactly as written in the parsed text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The nearest double. Always finite.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// A number from a finite double; `None` for NaN and infinities.
    pub fn from_f64(value: f64) -> Option<Number> {
        if value.is_finite() {
            Some(Number { text: format_number(value), value })
        } else {
            None
        }
    }
}

/// A parsed JSON value. Object members keep their text order; names are unique.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null`
    Null,
    /// `true` or `false`
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object, members in text order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member named `name` of an object.
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The members, if this is an object.
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    /// The JSON type name, for messages.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "a boolean",
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Array(_) => "an array",
            Value::Object(_) => "an object",
        }
    }

    /// The RFC 8785 canonical serialisation as UTF-8 bytes.
    pub fn to_jcs(&self) -> Vec<u8> {
        let mut out = String::new();
        write_jcs(self, &mut out);
        out.into_bytes()
    }

    /// The RFC 8785 canonical serialisation as a string.
    pub fn to_jcs_string(&self) -> String {
        let mut out = String::new();
        write_jcs(self, &mut out);
        out
    }
}

/// An object's members sorted as JCS sorts them (UTF-16 code units of the names).
pub(crate) fn jcs_sorted(members: &[(String, Value)]) -> Vec<&(String, Value)> {
    let mut v: Vec<&(String, Value)> = members.iter().collect();
    v.sort_by(|a, b| utf16_cmp(&a.0, &b.0));
    v
}

/// Compares two strings by their UTF-16 code units (RFC 8785 3.2.3).
pub(crate) fn utf16_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Parses `bytes` as strict JSON (SPEC 6.1).
///
/// Failures, in the order SPEC 6.1 reports them: a byte order mark, invalid UTF-8, nesting
/// deeper than [`MAX_DEPTH`], then the first in text order of a syntax error, a number that
/// overflows a double, or a duplicate member (detected when its object closes), and last a
/// lone surrogate. All are `E_PARSE_JSON` except duplicates (`E_PARSE_DUPLICATE_KEY`).
pub fn parse(bytes: &[u8]) -> Result<Value> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return err(ErrorCode::ParseJson, "$", "byte order mark");
    }
    let text = match core::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => return err(ErrorCode::ParseJson, "$", format!("invalid UTF-8 at byte {}", e.valid_up_to())),
    };
    check_depth(text.as_bytes())?;
    let mut p = Parser { b: text.as_bytes(), i: 0, lone_surrogate: false };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return p.syntax("extra data after the value");
    }
    if p.lone_surrogate {
        return err(ErrorCode::ParseJson, "$", "lone surrogate");
    }
    Ok(v)
}

/// Parses `bytes` and returns their JCS serialisation.
pub fn canonicalize_json(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(parse(bytes)?.to_jcs())
}

fn check_depth(b: &[u8]) -> Result<()> {
    let mut depth: usize = 0;
    let mut in_str = false;
    let mut esc = false;
    for &c in b {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return err(ErrorCode::ParseJson, "$", format!("nested deeper than {MAX_DEPTH} levels"));
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    lone_surrogate: bool,
}

impl Parser<'_> {
    fn syntax<T>(&self, what: &str) -> Result<T> {
        err(ErrorCode::ParseJson, "$", format!("{what} at byte {}", self.i))
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.i += 1;
        }
    }

    fn lit(&mut self, word: &[u8], v: Value) -> Result<Value> {
        if self.b.get(self.i..self.i + word.len()) == Some(word) {
            self.i += word.len();
            Ok(v)
        } else {
            self.syntax("invalid literal")
        }
    }

    // Depth is bounded by `check_depth`, so recursion is at most MAX_DEPTH deep.
    fn value(&mut self) -> Result<Value> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?.0)),
            Some(b't') => self.lit(b"true", Value::Bool(true)),
            Some(b'f') => self.lit(b"false", Value::Bool(false)),
            Some(b'n') => self.lit(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.syntax("unexpected character"),
            None => self.syntax("unexpected end of text"),
        }
    }

    fn array(&mut self) -> Result<Value> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::Array(items));
                }
                _ => return self.syntax("expected ',' or ']'"),
            }
        }
    }

    fn object(&mut self) -> Result<Value> {
        self.i += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        let mut units: Vec<Vec<u16>> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return self.syntax("expected a member name");
            }
            let (name, name_units) = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return self.syntax("expected ':'");
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            members.push((name, v));
            units.push(name_units);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return self.syntax("expected ',' or '}'"),
            }
        }
        // Duplicates are detected when the object closes (SPEC 6.1), comparing unescaped names.
        let mut order: Vec<usize> = (0..units.len()).collect();
        order.sort_by(|&a, &b| units[a].cmp(&units[b]).then(a.cmp(&b)));
        let mut dup: Option<usize> = None;
        for w in order.windows(2) {
            if units[w[0]] == units[w[1]] {
                let second = w[0].max(w[1]);
                dup = Some(dup.map_or(second, |d| d.min(second)));
            }
        }
        if let Some(d) = dup {
            let name = members.get(d).map(|m| m.0.as_str()).unwrap_or("");
            return err(ErrorCode::ParseDuplicateKey, "$", format!("duplicate member '{name}'"));
        }
        Ok(Value::Object(members))
    }

    fn hex4(&mut self) -> Result<u16> {
        let mut v: u16 = 0;
        for _ in 0..4 {
            let d = match self.peek() {
                Some(c @ b'0'..=b'9') => c - b'0',
                Some(c @ b'a'..=b'f') => c - b'a' + 10,
                Some(c @ b'A'..=b'F') => c - b'A' + 10,
                _ => return self.syntax("invalid \\u escape"),
            };
            v = (v << 4) | u16::from(d);
            self.i += 1;
        }
        Ok(v)
    }

    /// Returns the string and its UTF-16 code units (lone surrogates kept in the units,
    /// replaced by U+FFFD in the string and remembered as an error for later).
    fn string(&mut self) -> Result<(String, Vec<u16>)> {
        self.i += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let c = match self.peek() {
                None => return self.syntax("unterminated string"),
                Some(c) => c,
            };
            match c {
                b'"' => {
                    self.i += 1;
                    break;
                }
                b'\\' => {
                    self.i += 1;
                    let e = match self.peek() {
                        Some(e) => e,
                        None => return self.syntax("unterminated string"),
                    };
                    self.i += 1;
                    let u = match e {
                        b'"' => 0x22,
                        b'\\' => 0x5C,
                        b'/' => 0x2F,
                        b'b' => 0x08,
                        b'f' => 0x0C,
                        b'n' => 0x0A,
                        b'r' => 0x0D,
                        b't' => 0x09,
                        b'u' => self.hex4()?,
                        _ => {
                            self.i -= 1;
                            return self.syntax("invalid escape");
                        }
                    };
                    units.push(u);
                }
                0x00..=0x1F => return self.syntax("control character in string"),
                _ => {
                    // Copy one UTF-8 encoded character (input is valid UTF-8).
                    let len = match c {
                        0x00..=0x7F => 1,
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        _ => 4,
                    };
                    let chunk = self.b.get(self.i..self.i + len).unwrap_or(&[]);
                    match core::str::from_utf8(chunk).ok().and_then(|s| s.chars().next()) {
                        Some(ch) => {
                            let mut buf = [0u16; 2];
                            units.extend_from_slice(ch.encode_utf16(&mut buf));
                        }
                        None => return self.syntax("invalid UTF-8"),
                    }
                    self.i += len;
                }
            }
        }
        let mut s = String::with_capacity(units.len());
        for r in char::decode_utf16(units.iter().copied()) {
            match r {
                Ok(ch) => s.push(ch),
                Err(_) => {
                    self.lone_surrogate = true;
                    s.push('\u{FFFD}');
                }
            }
        }
        Ok((s, units))
    }

    fn number(&mut self) -> Result<Value> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while let Some(b'0'..=b'9') = self.peek() {
                    self.i += 1;
                }
            }
            _ => return self.syntax("invalid number"),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return self.syntax("invalid number");
            }
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            self.i += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return self.syntax("invalid number");
            }
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
            }
        }
        let text = core::str::from_utf8(self.b.get(start..self.i).unwrap_or(&[])).unwrap_or("");
        let value: f64 = match text.parse() {
            Ok(v) => v,
            Err(_) => return self.syntax("invalid number"),
        };
        if !value.is_finite() {
            return err(ErrorCode::ParseJson, "$", format!("number out of IEEE 754 range: {text}"));
        }
        Ok(Value::Number(Number { text: text.to_string(), value }))
    }
}

enum Piece<'a> {
    Value(&'a Value),
    Text(&'static str),
    Key(&'a str),
}

// Iterative, so that a deeply nested value built in memory cannot exhaust the stack.
fn write_jcs(v: &Value, out: &mut String) {
    let mut stack = vec![Piece::Value(v)];
    while let Some(p) = stack.pop() {
        match p {
            Piece::Text(t) => out.push_str(t),
            Piece::Key(k) => {
                write_string(k, out);
                out.push(':');
            }
            Piece::Value(Value::Null) => out.push_str("null"),
            Piece::Value(Value::Bool(true)) => out.push_str("true"),
            Piece::Value(Value::Bool(false)) => out.push_str("false"),
            Piece::Value(Value::Number(n)) => out.push_str(&format_number(n.value)),
            Piece::Value(Value::String(s)) => write_string(s, out),
            Piece::Value(Value::Array(items)) => {
                out.push('[');
                stack.push(Piece::Text("]"));
                for (i, item) in items.iter().enumerate().rev() {
                    stack.push(Piece::Value(item));
                    if i > 0 {
                        stack.push(Piece::Text(","));
                    }
                }
            }
            Piece::Value(Value::Object(members)) => {
                out.push('{');
                stack.push(Piece::Text("}"));
                for (i, (k, val)) in jcs_sorted(members).into_iter().enumerate().rev() {
                    stack.push(Piece::Value(val));
                    stack.push(Piece::Key(k));
                    if i > 0 {
                        stack.push(Piece::Text(","));
                    }
                }
            }
        }
    }
}

/// Checks that an already-parsed value nests at most [`MAX_DEPTH`] arrays and objects
/// (SPEC 6.1 applies the parser's limit to values accepted in parsed form). `E_PARSE_JSON`.
pub fn check_value_depth(v: &Value) -> Result<()> {
    let mut stack: Vec<(&Value, usize)> = vec![(v, 0)];
    while let Some((v, d)) = stack.pop() {
        let children: Box<dyn Iterator<Item = &Value>> = match v {
            Value::Array(items) => Box::new(items.iter()),
            Value::Object(members) => Box::new(members.iter().map(|(_, x)| x)),
            _ => continue,
        };
        if d + 1 > MAX_DEPTH {
            return err(ErrorCode::ParseJson, "$", format!("nested deeper than {MAX_DEPTH} levels"));
        }
        stack.extend(children.map(|c| (c, d + 1)));
    }
    Ok(())
}

/// Writes a JSON string as RFC 8785 does: only `"`, `\` and C0 controls are escaped.
pub(crate) fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{09}' => out.push_str("\\t"),
            '\u{0A}' => out.push_str("\\n"),
            '\u{0C}' => out.push_str("\\f"),
            '\u{0D}' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let n = c as usize;
                out.push_str("\\u00");
                out.push(char::from(HEX[(n >> 4) & 0xF]));
                out.push(char::from(HEX[n & 0xF]));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn split_sci(sci: &str) -> (String, i64) {
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci, "0"));
    (mantissa.chars().filter(char::is_ascii_digit).collect(), exp.parse().unwrap_or(0))
}

/// The ECMAScript digits of a positive finite double: the fewest digits that round-trip
/// and, among those, the closest to the value, ties to even (ECMA-262 Number::toString).
///
/// `{:e}` gives the fewest digits but breaks ties upwards; `{:.N e}` rounds the exact value
/// half to even. The latter is used whenever it round-trips at the same length.
fn shortest_digits(v: f64) -> (String, i64) {
    let shortest = split_sci(&format!("{v:e}"));
    let k = shortest.0.len();
    let exact_text = format!("{:.*e}", k.saturating_sub(1), v);
    if exact_text.parse::<f64>() == Ok(v) {
        let exact = split_sci(&exact_text);
        if exact.0.len() == k {
            return exact;
        }
    }
    shortest
}

/// Formats a finite double as ECMAScript `Number.prototype.toString` does (RFC 8785 3.2.2.3).
///
/// Rust's `{:e}` formatting yields the shortest digit string that round-trips; this lays
/// those digits out by the ECMAScript rules.
pub fn format_number(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        // -0 serialises as 0; non-finite values never reach here from `parse`.
        return "0".to_string();
    }
    let mut out = String::new();
    if v < 0.0 {
        out.push('-');
    }
    let (digits, exp) = shortest_digits(v.abs());
    let k = digits.len() as i64;
    let n = exp + 1;
    if k <= n && n <= 21 {
        out.push_str(&digits);
        for _ in 0..(n - k) {
            out.push('0');
        }
    } else if 0 < n && n <= 21 {
        let (a, b) = digits.split_at(n as usize);
        out.push_str(a);
        out.push('.');
        out.push_str(b);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        for _ in 0..(-n) {
            out.push('0');
        }
        out.push_str(&digits);
    } else {
        let (a, b) = digits.split_at(1);
        out.push_str(a);
        if !b.is_empty() {
            out.push('.');
            out.push_str(b);
        }
        out.push('e');
        let e = n - 1;
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.abs().to_string());
    }
    out
}
