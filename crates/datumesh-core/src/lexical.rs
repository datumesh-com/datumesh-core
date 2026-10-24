//! Lexical forms: names (2), scalars (3), hex, licences, key references (9.1) and units (7.1).
//!
//! Every check takes the path of the value it checks, for the error it may return. Each value
//! is checked against its lexical form first and its length limit second (SPEC 4.5).

use crate::error::{err, ErrorCode, Result};
use crate::tables;

/// Limits of SPEC 4.5.
pub mod limits {
    /// Characters in a function name, path segment, argument name or code language.
    pub const NAME: usize = 64;
    /// Characters in `<namespace>/<function>`.
    pub const FUNCTION_NAME: usize = 512;
    /// Arguments per function or cell.
    pub const ARGS: usize = 32;
    /// Characters in a number.
    pub const NUMBER: usize = 128;
    /// UTF-8 bytes in a string value.
    pub const STRING: usize = 1024;
    /// UTF-8 bytes in a receipt summary.
    pub const SUMMARY: usize = 1024;
    /// UTF-8 bytes in a function body.
    pub const BODY: usize = 1_048_576;
    /// Entries in a receipt's or descriptor's `inputs`.
    pub const INPUTS: usize = 4096;
    /// Characters in an SPDX licence identifier.
    pub const LICENCE_SPDX: usize = 64;
    /// Characters in a licence URL.
    pub const LICENCE_URL: usize = 512;
    /// Keys in one key publication.
    pub const KEYS: usize = 64;
    /// Characters in a blob media type (3.7).
    pub const MEDIA_TYPE: usize = 128;
    /// Characters in an index unit (`units.md` 2.4).
    pub const INDEX_UNIT: usize = 64;
    /// The largest blob length, 2^53 - 1.
    pub const BLOB_LENGTH: u64 = 9_007_199_254_740_991;
    /// Values in an argument's extent (7; `spec-0.4.0`).
    pub const EXTENT: usize = 4096;
    /// Member receipt hashes in a slice manifest (15.13; `spec-0.4.0`).
    pub const MEMBERS: usize = 1_048_576;
}

fn is_lower_digit(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

/// The grammar of 2.2 without its length limit.
pub(crate) fn is_name_grammar(s: &str) -> bool {
    let b = s.as_bytes();
    match b.first() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    let mut prev_underscore = false;
    for &c in &b[1..] {
        if c == b'_' {
            if prev_underscore {
                return false;
            }
            prev_underscore = true;
        } else if is_lower_digit(c) {
            prev_underscore = false;
        } else {
            return false;
        }
    }
    !prev_underscore
}

/// A name (2.2): function names, path segments and code languages. `E_CANON_NAME`, then `E_CANON_LIMIT`.
pub fn check_name(s: &str, path: &str) -> Result<()> {
    if !is_name_grammar(s) {
        return err(ErrorCode::CanonName, path, format!("not a lowercase snake_case name: {s:?}"));
    }
    if s.len() > limits::NAME {
        return err(ErrorCode::CanonLimit, path, "name longer than 64 characters");
    }
    Ok(())
}

/// An argument name (2.4): grammar (`E_CANON_ARG_NAME`), length (`E_CANON_LIMIT`), then reserved names.
pub fn check_arg_name(s: &str, path: &str) -> Result<()> {
    if !is_name_grammar(s) {
        return err(ErrorCode::CanonArgName, path, format!("not a lowercase snake_case name: {s:?}"));
    }
    if s.len() > limits::NAME {
        return err(ErrorCode::CanonLimit, path, "name longer than 64 characters");
    }
    if s == "asof" || s.starts_with("dm_") {
        return err(ErrorCode::CanonArgName, path, format!("reserved argument name {s:?}"));
    }
    Ok(())
}

/// A DNS name (2.1). Every failure is `E_CANON_NAME`.
pub fn check_dns_name(s: &str, path: &str) -> Result<()> {
    if s.len() > 253 {
        return err(ErrorCode::CanonName, path, "DNS name longer than 253 characters");
    }
    let labels: Vec<&str> = s.split('.').collect();
    if labels.len() < 2 {
        return err(ErrorCode::CanonName, path, format!("DNS name needs at least two labels: {s:?}"));
    }
    for l in &labels {
        let b = l.as_bytes();
        let ok = !b.is_empty()
            && b.len() <= 63
            && b.iter().all(|&c| is_lower_digit(c) || c == b'-')
            && b[0] != b'-'
            && b[b.len() - 1] != b'-';
        if !ok {
            return err(ErrorCode::CanonName, path, format!("bad DNS label {l:?}"));
        }
    }
    if labels.last().is_some_and(|l| l.bytes().all(|c| c.is_ascii_digit())) {
        return err(ErrorCode::CanonName, path, "top-level label is numeric");
    }
    Ok(())
}

/// A namespace (2.3): a DNS name, then path segments that are names.
pub fn check_namespace(s: &str, path: &str) -> Result<()> {
    let mut parts = s.split('/');
    check_dns_name(parts.next().unwrap_or(""), path)?;
    for seg in parts {
        check_name(seg, path)?;
    }
    Ok(())
}

/// The host of a namespace: its DNS name.
pub(crate) fn host_of(namespace: &str) -> &str {
    namespace.split('/').next().unwrap_or(namespace)
}

/// A fully qualified function name (2.3): namespace, `/`, name; at most 512 characters.
pub fn check_function_name(s: &str, path: &str) -> Result<()> {
    let Some((ns, f)) = s.rsplit_once('/') else {
        return err(ErrorCode::CanonName, path, format!("function name has no namespace: {s:?}"));
    };
    check_namespace(ns, path)?;
    check_name(f, path)?;
    if s.len() > limits::FUNCTION_NAME {
        return err(ErrorCode::CanonLimit, path, "function name longer than 512 characters");
    }
    Ok(())
}

/// Lowercase hex of exactly `digits` digits (`E_CANON_HEX`).
pub fn check_hex(s: &str, digits: usize, path: &str) -> Result<()> {
    if s.len() != digits || !s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return err(ErrorCode::CanonHex, path, format!("expected {digits} lowercase hex digits"));
    }
    Ok(())
}

/// A key reference `<host>#<key id>` (9.1).
pub fn check_key_ref(s: &str, path: &str) -> Result<()> {
    let Some((host, id)) = s.split_once('#') else {
        return err(ErrorCode::CanonName, path, "a key reference is <host>#<key id>");
    };
    check_dns_name(host, path)?;
    check_hex(id, 16, path)
}

/// The number grammar of 3.1 without the length limit.
pub(crate) fn is_number_grammar(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if b.first() == Some(&b'-') {
        i = 1;
    }
    match b.get(i) {
        Some(b'0') => i += 1,
        Some(b'1'..=b'9') => {
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
        _ => return false,
    }
    if b.get(i) == Some(&b'.') {
        let frac = &b[i + 1..];
        if frac.is_empty() || !frac.iter().all(u8::is_ascii_digit) || frac[frac.len() - 1] == b'0' {
            return false;
        }
        i = b.len();
    }
    i == b.len() && s != "-0"
}

/// Compares two numbers in canonical form (3.1) numerically.
pub(crate) fn cmp_numbers(a: &str, b: &str) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    let (an, a) = a.strip_prefix('-').map_or((false, a), |x| (true, x));
    let (bn, b) = b.strip_prefix('-').map_or((false, b), |x| (true, x));
    // Canonical: no "-0", so a negative number is below every other.
    let magnitude = || {
        let (ai, af) = a.split_once('.').unwrap_or((a, ""));
        let (bi, bf) = b.split_once('.').unwrap_or((b, ""));
        ai.len().cmp(&bi.len()).then_with(|| ai.cmp(bi)).then_with(|| af.cmp(bf))
    };
    match (an, bn) {
        (false, false) => magnitude(),
        (true, true) => magnitude().reverse(),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

/// Whether a canonical number is an integer.
pub(crate) fn is_integer(s: &str) -> bool {
    !s.contains('.')
}

/// Whether `s` is an `inputs` entry of the slice form `slice:<64 hex>:<64 hex>` (8.1).
pub(crate) fn split_slice_entry(s: &str) -> Option<(&str, &str)> {
    let rest = s.strip_prefix("slice:")?;
    let (id, members) = rest.split_once(':')?;
    let hex = |h: &str| h.len() == 64 && h.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
    (hex(id) && hex(members)).then_some((id, members))
}

/// An `inputs` entry (8.1, 10.1 step 3): 64 lowercase hex digits, or since `spec-0.4.0` a
/// slice entry `slice:<slice id>:<members hash>` (135 characters); `E_CANON_HEX` otherwise.
pub fn check_input_entry(s: &str, path: &str) -> Result<()> {
    if s.starts_with("slice:") {
        if split_slice_entry(s).is_none() {
            return err(ErrorCode::CanonHex, path, format!("not a slice entry slice:<64 hex>:<64 hex>: {s:?}"));
        }
        return Ok(());
    }
    check_hex(s, 64, path)
}

/// A number (3.1): `E_CANON_NUMBER`, then `E_CANON_LIMIT`.
pub fn check_number(s: &str, path: &str) -> Result<()> {
    if !is_number_grammar(s) {
        return err(ErrorCode::CanonNumber, path, format!("not a canonical decimal: {s:?}"));
    }
    if s.len() > limits::NUMBER {
        return err(ErrorCode::CanonLimit, path, "number longer than 128 characters");
    }
    Ok(())
}

fn digits(b: &[u8]) -> Option<u32> {
    if b.is_empty() || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(b.iter().fold(0u32, |a, &c| a * 10 + u32::from(c - b'0')))
}

fn days_in_month(y: u32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400) => 29,
        2 => 28,
        _ => 0,
    }
}

/// `YYYY-MM-DD`, a real day in years 0001-9999.
pub(crate) fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let (Some(y), Some(m), Some(d)) = (digits(&b[0..4]), digits(&b[5..7]), digits(&b[8..10])) else {
        return false;
    };
    (1..=9999).contains(&y) && (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)
}

/// A date (3.2), `E_CANON_DATE`.
pub fn check_date(s: &str, path: &str) -> Result<()> {
    if !is_date(s) {
        return err(ErrorCode::CanonDate, path, format!("not a YYYY-MM-DD calendar date: {s:?}"));
    }
    Ok(())
}

/// `YYYY-MM-DDTHH:MM:SS.ffffffZ`.
pub(crate) fn is_instant(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 27 || !s.is_char_boundary(10) || !is_date(&s[..10]) {
        return false;
    }
    if b[10] != b'T' || b[13] != b':' || b[16] != b':' || b[19] != b'.' || b[26] != b'Z' {
        return false;
    }
    let (Some(h), Some(mi), Some(se), Some(_)) =
        (digits(&b[11..13]), digits(&b[14..16]), digits(&b[17..19]), digits(&b[20..26]))
    else {
        return false;
    };
    h < 24 && mi < 60 && se < 60
}

/// An instant (3.3), `E_CANON_INSTANT`.
pub fn check_instant(s: &str, path: &str) -> Result<()> {
    if !is_instant(s) {
        return err(ErrorCode::CanonInstant, path, format!("not YYYY-MM-DDTHH:MM:SS.ffffffZ: {s:?}"));
    }
    Ok(())
}

fn is_restricted_name(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && is_lower_digit(b[0]) && b.iter().all(|&c| is_lower_digit(c) || b"!#$&-^_.+".contains(&c))
}

/// A blob (3.7): its sha256, length and media type. Every failure is `E_CANON_BLOB`.
pub(crate) fn parse_blob(s: &str) -> core::result::Result<([u8; 32], u64, &str), String> {
    let mut parts = s.split(':');
    let (Some(h), Some(len), Some(mt), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err("a blob is <sha256>:<length>:<media type>".into());
    };
    let hash = match decode_hex_lower::<32>(h) {
        Some(x) => x,
        None => return Err("the hash is 64 lowercase hex digits".into()),
    };
    let lb = len.as_bytes();
    if lb.is_empty() || !lb.iter().all(u8::is_ascii_digit) || (lb.len() > 1 && lb[0] == b'0') || lb.len() > 16 {
        return Err("the length is a canonical non-negative integer".into());
    }
    let n: u64 = len.parse().map_err(|_| String::from("bad length"))?;
    if n > limits::BLOB_LENGTH {
        return Err("length above 9007199254740991".into());
    }
    let ok_mt = match mt.split_once('/') {
        Some((t, st)) => is_restricted_name(t) && is_restricted_name(st),
        None => false,
    };
    if !ok_mt {
        return Err(format!("not a lowercase type/subtype media type: {mt:?}"));
    }
    if mt.len() > limits::MEDIA_TYPE {
        return Err("media type longer than 128 characters".into());
    }
    Ok((hash, n, mt))
}

/// A blob (3.7), `E_CANON_BLOB`.
pub fn check_blob(s: &str, path: &str) -> Result<()> {
    match parse_blob(s) {
        Ok(_) => Ok(()),
        Err(d) => err(ErrorCode::CanonBlob, path, d),
    }
}

/// Decodes exactly `N` bytes of lowercase hex.
pub(crate) fn decode_hex_lower<const N: usize>(s: &str) -> Option<[u8; N]> {
    let b = s.as_bytes();
    if b.len() != 2 * N {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        let hi = hexval(b[2 * i])?;
        let lo = hexval(b[2 * i + 1])?;
        *o = (hi << 4) | lo;
    }
    Some(out)
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Lowercase hex of bytes.
pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(char::from(HEX[usize::from(b >> 4)]));
        s.push(char::from(HEX[usize::from(b & 0xF)]));
    }
    s
}

/// Whether a code point is assigned in Unicode 15.1, by the spec's own table.
pub fn is_assigned_15_1(c: char) -> bool {
    let cp = c as u32;
    tables::ASSIGNED
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                core::cmp::Ordering::Less
            } else if lo > cp {
                core::cmp::Ordering::Greater
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The string rules of 3.5 in order: characters, NFC, then the byte limit.
/// `allow_tab_lf` relaxes the control rule for function bodies (7).
pub(crate) fn check_text(s: &str, limit: usize, allow_tab_lf: bool, path: &str) -> Result<()> {
    for c in s.chars() {
        let cp = c as u32;
        if (cp < 0x20 && !(allow_tab_lf && (c == '\t' || c == '\n'))) || (0x7F..=0x9F).contains(&cp) {
            return err(ErrorCode::CanonString, path, format!("control character U+{cp:04X}"));
        }
        if !is_assigned_15_1(c) {
            return err(ErrorCode::CanonString, path, format!("U+{cp:04X} is not assigned in Unicode 15.1"));
        }
    }
    if !unicode_normalization::is_nfc(s) {
        return err(ErrorCode::CanonString, path, "not in Normalization Form C");
    }
    if s.len() > limit {
        return err(ErrorCode::CanonLimit, path, format!("longer than {limit} UTF-8 bytes"));
    }
    Ok(())
}

/// A string value (3.5).
pub fn check_string(s: &str, path: &str) -> Result<()> {
    check_text(s, limits::STRING, false, path)
}

/// A licence (8.1): grammar (`E_CANON_LICENCE`), then length (`E_CANON_LIMIT`).
pub fn check_licence(s: &str, path: &str) -> Result<()> {
    if let Some(rest) = s.strip_prefix("https://") {
        if rest.is_empty() || !rest.bytes().all(|c| (0x21..=0x7E).contains(&c)) {
            return err(ErrorCode::CanonLicence, path, "a licence URL is https:// and printable ASCII");
        }
        if s.len() > limits::LICENCE_URL {
            return err(ErrorCode::CanonLimit, path, "licence URL longer than 512 characters");
        }
        return Ok(());
    }
    let b = s.as_bytes();
    let ok = !b.is_empty()
        && b[0].is_ascii_alphanumeric()
        && b.iter().all(|&c| c.is_ascii_alphanumeric() || c == b'.' || c == b'+' || c == b'-');
    if !ok {
        return err(ErrorCode::CanonLicence, path, format!("not an SPDX identifier or https URL: {s:?}"));
    }
    if s.len() > limits::LICENCE_SPDX {
        return err(ErrorCode::CanonLimit, path, "licence identifier longer than 64 characters");
    }
    Ok(())
}

/// Whether `s` is a currency code of `units.md` 2.1 (withdrawn codes included).
pub fn is_currency(s: &str) -> bool {
    tables::CURRENCIES.contains(&s)
}

/// Whether `s` is an index unit (`units.md` 2.4).
pub fn is_index_unit(s: &str) -> bool {
    if s.len() > limits::INDEX_UNIT {
        return false;
    }
    let Some(rest) = s.strip_prefix("index:") else { return false };
    let Some((period, value)) = rest.split_once('=') else { return false };
    let period_ok = match period.len() {
        4 => is_date(&format!("{period}-01-01")),
        7 => is_date(&format!("{period}-01")),
        10 => is_date(period),
        _ => false,
    };
    // Shapes like "2015-1-01" must not slip through the padding above.
    let shape_ok =
        period.bytes().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == b'-' } else { c.is_ascii_digit() });
    period_ok && shape_ok && is_number_grammar(value) && !value.starts_with('-') && value != "0"
}

/// Whether `s` is a literal unit: listed in `units.md` 2.1-2.3, or an index unit.
pub fn is_literal_unit(s: &str) -> bool {
    is_currency(s) || tables::PHYSICAL.contains(&s) || tables::RATIOS.contains(&s) || is_index_unit(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        for ok in ["fx_rate", "base", "a01", "v2_final"] {
            assert!(is_name_grammar(ok), "{ok}");
        }
        for bad in ["FxRate", "_x", "x_", "a__b", "1st", ""] {
            assert!(!is_name_grammar(bad), "{bad}");
        }
    }

    #[test]
    fn numbers() {
        for ok in ["0", "-1", "1.5", "0.001", "10"] {
            assert!(is_number_grammar(ok), "{ok}");
        }
        for bad in ["-0", "01", "1.", ".5", "1.50", "+1", "1e3", "", "-", "0.0"] {
            assert!(!is_number_grammar(bad), "{bad}");
        }
    }

    #[test]
    fn index_units() {
        assert!(is_index_unit("index:2015=100"));
        assert!(is_index_unit("index:1984-01-03=1000"));
        assert!(is_index_unit("index:2016-02-29=0.5"));
        for bad in [
            "index:0000=1",
            "index:2015=0",
            "index:2015=-1",
            "index:2015-13=1",
            "index:2015-02-29=1",
            "index:2015=1.0",
            "index:15=1",
        ] {
            assert!(!is_index_unit(bad), "{bad}");
        }
    }

    #[test]
    fn instants() {
        assert!(is_instant("2024-03-15T16:00:00.000000Z"));
        assert!(!is_instant("2024-03-15T24:00:00.000000Z"));
        assert!(!is_instant("2024-03-15T23:59:60.000000Z"));
        assert!(!is_instant("2023-02-29T00:00:00.000000Z"));
    }
}
