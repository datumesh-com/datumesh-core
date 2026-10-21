//! Response envelopes (5.5): the mesh's unsigned statement around a receipt.

use crate::error::{err, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::Keys;
use crate::lexical::{self, limits};
use crate::receipt::Receipt;
use crate::schema;

/// The eight outcomes of 5.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// 200: the receipt answers the cell as of the requested `asof`.
    Ok,
    /// 200: the last receipt the mesh knows.
    Stale,
    /// 404: no receipt for this cell.
    NotCovered,
    /// 403: the requester may not receive this cell.
    Unlicensed,
    /// 402: available for the quoted price.
    PaymentRequired,
    /// 429: too many requests.
    RateLimited,
    /// 400: the cell URI does not parse.
    BadRequest,
    /// 5xx: the mesh failed.
    Error,
}

impl Outcome {
    /// The outcome as written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Stale => "stale",
            Outcome::NotCovered => "not_covered",
            Outcome::Unlicensed => "unlicensed",
            Outcome::PaymentRequired => "payment_required",
            Outcome::RateLimited => "rate_limited",
            Outcome::BadRequest => "bad_request",
            Outcome::Error => "error",
        }
    }

    fn from_name(s: &str) -> Option<Outcome> {
        Some(match s {
            "ok" => Outcome::Ok,
            "stale" => Outcome::Stale,
            "not_covered" => Outcome::NotCovered,
            "unlicensed" => Outcome::Unlicensed,
            "payment_required" => Outcome::PaymentRequired,
            "rate_limited" => Outcome::RateLimited,
            "bad_request" => Outcome::BadRequest,
            "error" => Outcome::Error,
            _ => return None,
        })
    }

    /// Required and optional members besides `outcome`.
    fn members(self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            Outcome::Ok => (&["receipt", "trust"], &["superseded_by", "reason"]),
            Outcome::Stale => (&["receipt", "trust", "age_seconds"], &["superseded_by", "reason"]),
            Outcome::NotCovered | Outcome::Unlicensed => (&[], &[]),
            Outcome::PaymentRequired => (&["quote"], &[]),
            Outcome::RateLimited => (&["retry_after_seconds"], &[]),
            Outcome::BadRequest | Outcome::Error => (&["error"], &[]),
        }
    }

    fn status_ok(self, status: u16) -> bool {
        match self {
            Outcome::Ok | Outcome::Stale => status == 200,
            Outcome::NotCovered => status == 404,
            Outcome::Unlicensed => status == 403,
            Outcome::PaymentRequired => status == 402,
            Outcome::RateLimited => status == 429,
            Outcome::BadRequest => status == 400,
            Outcome::Error => (500..=599).contains(&status),
        }
    }
}

/// The mesh's trust label (5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trust {
    /// Signed by the namespace's host, with no inputs.
    Primary,
    /// The mesh says it recomputed the receipt.
    Verified,
    /// Neither.
    Unverified,
}

impl Trust {
    /// The label as written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Trust::Primary => "primary",
            Trust::Verified => "verified",
            Trust::Unverified => "unverified",
        }
    }
}

/// A price quote (`payment_required`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Quote {
    /// A canonical decimal greater than zero.
    pub amount: String,
    /// A currency code from `units.md`.
    pub currency: String,
    /// Where to pay: an https URL.
    pub authorise: String,
}

/// What the client asked for, for the binding of 5.5 step 5.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request {
    cell_id: String,
    asof: Option<String>,
}

/// A malformed request description (5.5 step 5): the caller's mistake, not an `E_*` code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvalidRequest(String);

impl core::fmt::Display for InvalidRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid request: {}", self.0)
    }
}

impl std::error::Error for InvalidRequest {}

impl Request {
    /// A request for the cell with id `cell_id` (64 lowercase hex digits) as of `asof` (an
    /// instant) or now. Anything else is [`InvalidRequest`]: SPEC 5.5 makes a malformed
    /// request the caller's error, reported outside the `E_*` codes.
    pub fn new(cell_id: &str, asof: Option<&str>) -> core::result::Result<Request, InvalidRequest> {
        if lexical::check_hex(cell_id, 64, "").is_err() {
            return Err(InvalidRequest(format!("cell_id must be 64 lowercase hex digits: {cell_id:?}")));
        }
        if let Some(a) = asof {
            if !lexical::is_instant(a) {
                return Err(InvalidRequest(format!("asof must be an instant: {a:?}")));
            }
        }
        Ok(Request { cell_id: cell_id.into(), asof: asof.map(String::from) })
    }

    /// The requested cell id.
    pub fn cell_id(&self) -> &str {
        &self.cell_id
    }

    /// The requested `asof`, if any.
    pub fn asof(&self) -> Option<&str> {
        self.asof.as_deref()
    }
}

/// A checked envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// The outcome.
    pub outcome: Outcome,
    /// The receipt (`ok`, `stale`), signature-valid and bound to the request.
    pub receipt: Option<Receipt>,
    /// The trust label (`ok`, `stale`).
    pub trust: Option<Trust>,
    /// Seconds since the mesh last confirmed the receipt (`stale`).
    pub age_seconds: Option<u64>,
    /// Hash of a newer receipt for the same cell.
    pub superseded_by: Option<String>,
    /// Why the receipt was superseded.
    pub reason: Option<String>,
    /// The quote (`payment_required`).
    pub quote: Option<Quote>,
    /// Seconds to wait (`rate_limited`).
    pub retry_after_seconds: Option<u64>,
    /// The failure code (`bad_request`, `error`).
    pub error: Option<ErrorCode>,
}

const BAD_REQUEST_CODES: [ErrorCode; 15] = [
    ErrorCode::ParseUri,
    ErrorCode::CanonName,
    ErrorCode::CanonArgName,
    ErrorCode::CanonArgDuplicate,
    ErrorCode::CanonArgOrder,
    ErrorCode::CanonPercent,
    ErrorCode::CanonType,
    ErrorCode::CanonNumber,
    ErrorCode::CanonDate,
    ErrorCode::CanonInstant,
    ErrorCode::CanonString,
    ErrorCode::CanonBlob,
    ErrorCode::CanonLimit,
    ErrorCode::ArgUnknown,
    ErrorCode::ArgMissing,
];

fn env_err<T>(path: &str, detail: impl Into<String>) -> Result<T> {
    err(ErrorCode::Envelope, path, detail)
}

/// `0` or `[1-9][0-9]*`, at most 2^53 - 1, judged on the number's text.
fn integer_member(v: &Value, path: &str) -> Result<u64> {
    let Value::Number(n) = v else {
        return env_err(path, "must be a non-negative integer");
    };
    let t = n.text();
    let ok =
        !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()) && (t == "0" || !t.starts_with('0')) && t.len() <= 16;
    match (ok, t.parse::<u64>()) {
        (true, Ok(x)) if x <= limits::BLOB_LENGTH => Ok(x),
        _ => env_err(path, format!("must be written 0|[1-9][0-9]* and at most 2^53 - 1, not {t}")),
    }
}

/// Checks a response envelope (5.5) in the order of its procedure: strict JSON, members,
/// HTTP status, the receipt through step 6 of 10.1, the binding to `request`, then `trust`
/// and `superseded_by`. Never touches the network.
pub fn check_envelope(status: u16, body: &[u8], keys: &Keys, request: &Request) -> Result<Envelope> {
    // 1.
    let v = json::parse(body)?;
    // 2.
    let m = match &v {
        Value::Object(m) if matches!(schema::opt(m, "outcome"), Some(Value::String(_))) => m,
        _ => return env_err("$", "an envelope is an object with a string outcome"),
    };
    let oname = schema::get(m, "outcome").as_str().unwrap_or("");
    let Some(outcome) = Outcome::from_name(oname) else {
        return env_err("$.outcome", format!("unknown outcome {oname:?}"));
    };
    let (req, optional) = outcome.members();
    let mut allowed: Vec<&str> = vec!["outcome"];
    allowed.extend_from_slice(req);
    schema::exact(m, &allowed, optional, "$", ErrorCode::Envelope)?;
    let mut env = Envelope {
        outcome,
        receipt: None,
        trust: None,
        age_seconds: None,
        superseded_by: None,
        reason: None,
        quote: None,
        retry_after_seconds: None,
        error: None,
    };
    let receipt_v = schema::opt(m, "receipt");
    if let Some(r) = receipt_v {
        if !matches!(r, Value::Object(_)) {
            return env_err("$.receipt", "receipt is a receipt object");
        }
    }
    if let Some(t) = schema::opt(m, "trust") {
        env.trust = Some(match t.as_str() {
            Some("primary") => Trust::Primary,
            Some("verified") => Trust::Verified,
            Some("unverified") => Trust::Unverified,
            _ => return env_err("$.trust", "trust is primary, verified or unverified"),
        });
    }
    if let Some(a) = schema::opt(m, "age_seconds") {
        env.age_seconds = Some(integer_member(a, "$.age_seconds")?);
    }
    if let Some(a) = schema::opt(m, "retry_after_seconds") {
        env.retry_after_seconds = Some(integer_member(a, "$.retry_after_seconds")?);
    }
    if let Some(s) = schema::opt(m, "superseded_by") {
        let s = s.as_str().unwrap_or("");
        lexical::check_hex(s, 64, "$.superseded_by").map_err(|e| e.recode(ErrorCode::Envelope))?;
        env.superseded_by = Some(s.into());
    }
    if let Some(r) = schema::opt(m, "reason") {
        let Some(r) = r.as_str() else { return env_err("$.reason", "reason must be a string") };
        if schema::opt(m, "superseded_by").is_none() {
            return env_err("$.reason", "reason is only present with superseded_by");
        }
        lexical::check_text(r, limits::SUMMARY, false, "$.reason").map_err(|e| e.recode(ErrorCode::Envelope))?;
        if r.is_empty() {
            return env_err("$.reason", "reason must not be empty");
        }
        env.reason = Some(r.into());
    }
    if let Some(q) = schema::opt(m, "quote") {
        env.quote = Some(quote(q)?);
    }
    if let Some(e) = schema::opt(m, "error") {
        let code = e.as_str().and_then(ErrorCode::from_code);
        let ok = match (outcome, code) {
            (Outcome::BadRequest, Some(c)) => BAD_REQUEST_CODES.contains(&c),
            // E_EVAL_UNAVAILABLE is not a code of spec-0.2.1's 10.2 (RS-28).
            (_, Some(c)) => c != ErrorCode::Envelope && c != ErrorCode::EvalUnavailable,
            (_, None) => false,
        };
        if !ok {
            return env_err("$.error", format!("{e:?} is not a code this outcome can carry"));
        }
        env.error = code;
    }
    // 3.
    if !outcome.status_ok(status) {
        return env_err("$.outcome", format!("status {status} does not match outcome {}", outcome.as_str()));
    }
    let Some(rv) = receipt_v else { return Ok(env) };
    // 4.
    let r = crate::verify::signature_valid_value(rv, keys).map_err(|(e, _)| {
        let inner = e.path().strip_prefix('$').unwrap_or(e.path());
        Error::new(e.code(), format!("$.receipt{inner}"), e.detail())
    })?;
    // 5.
    if r.cell.id() != request.cell_id {
        return env_err("$.receipt.cell", "the receipt is for another cell than the one requested");
    }
    if let Some(asked) = &request.asof {
        let bound = match outcome {
            Outcome::Stale => r.asof <= *asked,
            _ => r.asof == *asked,
        };
        if !bound {
            return env_err("$.receipt.asof", "the receipt does not answer the requested asof");
        }
    }
    // 6.
    let primary = r.signer_host() == lexical::host_of(&r.cell.ns) && r.inputs.is_empty();
    if (env.trust == Some(Trust::Primary)) != primary {
        return env_err(
            "$.trust",
            if primary { "a primary receipt is labelled primary" } else { "the receipt is not primary" },
        );
    }
    if env.superseded_by.as_deref() == Some(r.hash().as_str()) {
        return env_err("$.superseded_by", "a receipt cannot supersede itself");
    }
    env.receipt = Some(r);
    Ok(env)
}

fn quote(q: &Value) -> Result<Quote> {
    let m = schema::object_or(q, "$.quote", ErrorCode::Envelope, "quote")?;
    schema::exact(m, &["amount", "authorise", "currency"], &[], "$.quote", ErrorCode::Envelope)?;
    let amount = schema::string_or(m, "amount", "$.quote", ErrorCode::Envelope)?;
    let authorise = schema::string_or(m, "authorise", "$.quote", ErrorCode::Envelope)?;
    let currency = schema::string_or(m, "currency", "$.quote", ErrorCode::Envelope)?;
    if lexical::check_number(amount, "").is_err() || amount.starts_with('-') || amount == "0" {
        return env_err("$.quote.amount", "amount is a canonical decimal greater than zero");
    }
    let url_ok = authorise
        .strip_prefix("https://")
        .is_some_and(|r| !r.is_empty() && r.bytes().all(|c| (0x21..=0x7E).contains(&c)))
        && authorise.len() <= limits::LICENCE_URL;
    if !url_ok {
        return env_err("$.quote.authorise", "authorise is an https URL");
    }
    if !lexical::is_currency(currency) {
        return env_err("$.quote.currency", "currency is a currency code from units.md");
    }
    Ok(Quote { amount: amount.into(), currency: currency.into(), authorise: authorise.into() })
}
