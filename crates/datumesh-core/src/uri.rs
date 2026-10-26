//! Cell URIs (5.1-5.4) and the HTTPS mapping (5.3).

use crate::error::{err, ErrorCode, Result};
use crate::json::{jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::model::{check_lexical, unchecked, Arg, Args, CellRef, Reference, Type};
use crate::schema;

fn is_literal(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~' | b':')
}

fn qpath(name: &str) -> String {
    format!("uri.query.{name}")
}

/// Decodes a parameter value (5.2): literal characters as themselves, everything else as
/// `%` and two uppercase hex digits, and the result UTF-8. Failures are `E_CANON_PERCENT`.
fn percent_decode(raw: &str, path: &str) -> Result<String> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'%' {
            let hex = |x: u8| match x {
                b'0'..=b'9' => Some(x - b'0'),
                b'A'..=b'F' => Some(x - b'A' + 10),
                _ => None,
            };
            let (Some(h), Some(l)) = (b.get(i + 1).and_then(|&x| hex(x)), b.get(i + 2).and_then(|&x| hex(x))) else {
                return err(ErrorCode::CanonPercent, path, "percent-encoding must be % and two uppercase hex digits");
            };
            let byte = (h << 4) | l;
            if is_literal(byte) {
                return err(
                    ErrorCode::CanonPercent,
                    path,
                    format!("{:?} must not be percent-encoded", char::from(byte)),
                );
            }
            out.push(byte);
            i += 3;
        } else if is_literal(c) {
            out.push(c);
            i += 1;
        } else {
            let shown = raw[i..].chars().next().unwrap_or('?');
            return err(ErrorCode::CanonPercent, path, format!("{shown:?} must be percent-encoded"));
        }
    }
    String::from_utf8(out).or_else(|_| err(ErrorCode::CanonPercent, path, "decoded bytes are not UTF-8"))
}

fn percent_encode(s: &str, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &c in s.as_bytes() {
        if is_literal(c) {
            out.push(char::from(c));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(c >> 4)]));
            out.push(char::from(HEX[usize::from(c & 0xF)]));
        }
    }
}

/// Parses a cell URI against a function signature (5.4), reporting the first failure in the
/// order 5.4 defines.
///
/// `signature` is the descriptor's `signature` member as JSON. It is taken as given (a
/// descriptor checked under 7), except that it must be readable: an object with an `args`
/// object whose members are objects with a string `type` (`E_SCHEMA` otherwise).
///
/// Its [`CellRef`] has no pin: a pinned URI (`fn_hash=`, `spec-0.4.2`) is refused with
/// `E_CANON_ARG_NAME` (`fn_hash` is a reserved argument name). [`parse_reference`] reads it.
pub fn parse_uri(uri: &str, signature: &Value) -> Result<CellRef> {
    let r = parse_reference(uri, signature)?;
    if r.fn_hash.is_some() {
        return err(ErrorCode::CanonArgName, &qpath("fn_hash"), "a pinned URI: read it with parse_reference");
    }
    Ok(CellRef { ns: r.ns, fn_name: r.fn_name, args: r.args, asof: r.asof })
}

/// Parses a cell URI, pinned or not, against a function signature (5.4, `spec-0.4.2`),
/// reporting the first failure in the order 5.4 defines: the pin `fn_hash=<64 hex>` comes after
/// the arguments and before `asof`, and is checked (`E_CANON_HEX`) after the arguments' values
/// and before `asof`. For a pinned URI, `signature` is that of the pinned descriptor.
pub fn parse_reference(uri: &str, signature: &Value) -> Result<Reference> {
    // 1. Outer grammar.
    let bad = |d: &str| err(ErrorCode::ParseUri, "$", d.to_string());
    let Some(rest) = uri.strip_prefix("cell://") else {
        return bad("not cell://<namespace>/<function>[?<query>]");
    };
    if rest.contains('#') {
        return bad("a cell URI has no fragment");
    }
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let authority = path.split('/').next().unwrap_or("");
    if authority.contains('@') || authority.contains(':') {
        return bad("a cell URI has no userinfo or port");
    }
    let mut params: Vec<(&str, &str)> = Vec::new();
    if let Some(q) = query {
        if q.is_empty() {
            return bad("empty query");
        }
        for p in q.split('&') {
            match p.split_once('=') {
                Some(kv) => params.push(kv),
                None => return bad(&format!("parameter without '=': {p:?}")),
            }
        }
    }

    // 2. Namespace and function names.
    let Some((ns, f)) = path.rsplit_once('/') else {
        return err(ErrorCode::CanonName, "uri.path", "missing namespace");
    };
    lexical::check_namespace(ns, "uri.path")?;
    lexical::check_name(f, "uri.path")?;
    if path.len() > limits::FUNCTION_NAME {
        return err(ErrorCode::CanonLimit, "uri.path", "function name longer than 512 characters");
    }

    // 3. Parameter names: asof last, then the pin last of the rest.
    let asof_raw = match params.last() {
        Some(&("asof", v)) => {
            params.pop();
            Some(v)
        }
        _ => None,
    };
    let pin_raw = match params.last() {
        Some(&("fn_hash", v)) => {
            params.pop();
            Some(v)
        }
        _ => None,
    };
    let mut prev: Option<&str> = None;
    for &(name, _) in &params {
        let p = qpath(name);
        if name == "asof" {
            let code = if asof_raw.is_some() { ErrorCode::CanonArgDuplicate } else { ErrorCode::CanonArgOrder };
            return err(code, &p, "asof is the last parameter, at most once");
        }
        if name == "fn_hash" {
            let code = if pin_raw.is_some() { ErrorCode::CanonArgDuplicate } else { ErrorCode::CanonArgOrder };
            return err(code, &p, "the pin fn_hash comes after the arguments, before asof, at most once");
        }
        lexical::check_arg_name(name, &p)?;
        if let Some(pr) = prev {
            if name == pr {
                return err(ErrorCode::CanonArgDuplicate, &p, "argument repeated");
            }
            if name < pr {
                return err(ErrorCode::CanonArgOrder, &p, "arguments must be sorted by name");
            }
        }
        prev = Some(name);
    }

    // 4. Count.
    if params.len() > limits::ARGS {
        return err(ErrorCode::CanonLimit, "uri.query", "more than 32 arguments");
    }

    // 5. The signature, then the arguments against it.
    crate::json::check_value_depth(signature)
        .map_err(|e| crate::error::Error::new(e.code(), "signature", e.detail()))?;
    let sig = schema::object(signature, "signature")?;
    let decls = match schema::opt(sig, "args") {
        Some(Value::Object(m)) => m,
        _ => return err(ErrorCode::Schema, "signature.args", "the signature has no args object"),
    };
    for (k, d) in jcs_sorted(decls) {
        let readable = matches!(d.get("type"), Some(Value::String(_))) && d.as_object().is_some();
        if !readable {
            return err(
                ErrorCode::Schema,
                &format!("signature.args.{k}"),
                "a declaration is an object with a string type",
            );
        }
    }
    for &(name, _) in &params {
        if schema::opt(decls, name).is_none() {
            return err(ErrorCode::ArgUnknown, &qpath(name), format!("function declares no argument {name:?}"));
        }
    }
    for (k, _) in jcs_sorted(decls) {
        if !params.iter().any(|&(n, _)| n == k) {
            return err(ErrorCode::ArgMissing, "uri.query", format!("argument {k:?} absent; use {k}=* for a slice"));
        }
    }

    // 6. Values.
    let mut args = Args::new();
    for &(name, raw) in &params {
        let p = qpath(name);
        if raw == "*" {
            args.insert(name.into(), Arg::Slice);
            continue;
        }
        let text = percent_decode(raw, &p)?;
        let tname = schema::get(decls, name).get("type").and_then(Value::as_str).unwrap_or("");
        let ty = Type::parse(tname, &p)?;
        check_lexical(ty, &text, &p)?;
        args.insert(name.into(), Arg::Value(unchecked(ty, &text)));
    }

    // 7. The pin: 64 lowercase hex digits, as written.
    let fn_hash = match pin_raw {
        None => None,
        Some(raw) => {
            lexical::check_hex(raw, 64, &qpath("fn_hash"))?;
            Some(raw.to_string())
        }
    };

    // 8. asof.
    let asof = match asof_raw {
        None => None,
        Some(raw) => {
            let text = percent_decode(raw, "uri.query.asof")?;
            lexical::check_instant(&text, "uri.query.asof")?;
            Some(text)
        }
    };
    Ok(Reference { ns: ns.into(), fn_name: f.into(), args, fn_hash, asof })
}

/// Writes the one URI of a cell reference (5.2). `format_uri(&parse_uri(u, s)?) == u` for
/// every accepted `u`.
pub fn format_uri(r: &CellRef) -> String {
    format_parts(&r.ns, &r.fn_name, &r.args, None, r.asof.as_deref())
}

/// Writes the one URI of a reference (5.2), its pin as `&fn_hash=` before `&asof=`.
/// `format_reference(&parse_reference(u, s)?) == u` for every accepted `u`.
pub fn format_reference(r: &Reference) -> String {
    format_parts(&r.ns, &r.fn_name, &r.args, r.fn_hash.as_deref(), r.asof.as_deref())
}

fn format_parts(ns: &str, f: &str, args: &Args, pin: Option<&str>, asof: Option<&str>) -> String {
    let mut out = format!("cell://{ns}/{f}");
    let mut first = true;
    let mut sep = |out: &mut String| {
        out.push(if first { '?' } else { '&' });
        first = false;
    };
    for (name, arg) in args {
        sep(&mut out);
        out.push_str(name);
        out.push('=');
        match arg {
            Arg::Slice => out.push('*'),
            Arg::Value(s) => percent_encode(s.lexical(), &mut out),
        }
    }
    if let Some(h) = pin {
        sep(&mut out);
        out.push_str("fn_hash=");
        out.push_str(h);
    }
    if let Some(a) = asof {
        sep(&mut out);
        out.push_str("asof=");
        percent_encode(a, &mut out);
    }
    out
}

/// The HTTPS URL of a cell URI on mesh host `mesh` (5.3): `https://<mesh>/cell/<rest>`.
/// `mesh` must be a DNS name (`E_CANON_NAME`), `uri` must start with `cell://` (`E_PARSE_URI`).
pub fn https_url(mesh: &str, uri: &str) -> Result<String> {
    lexical::check_dns_name(mesh, "mesh")?;
    let Some(rest) = uri.strip_prefix("cell://") else {
        return err(ErrorCode::ParseUri, "$", "not a cell URI");
    };
    Ok(format!("https://{mesh}/cell/{rest}"))
}

/// The cell URI served at an HTTPS URL on mesh host `mesh` (5.3), by stripping the prefix.
/// `None` when the URL is not under `https://<mesh>/cell/`.
pub fn cell_uri_from_https(mesh: &str, url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?.strip_prefix(mesh)?.strip_prefix("/cell/")?;
    Some(format!("cell://{rest}"))
}
