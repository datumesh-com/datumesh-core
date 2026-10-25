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
    /// 200: the receipt answers the cell as of the requested `asof`: its `asof` is at or
    /// before the requested one, and the mesh asserts it is the cell's latest version known
    /// as of then.
    Ok,
    /// 200: as `Ok`, except that the mesh cannot assert the receipt is current; it serves the
    /// last receipt it knows.
    Stale,
    /// 404: no receipt for this cell. With a `fault` (`spec-0.4.0`, see
    /// [`check_envelope_extras`]), the cell is a computed cell whose evaluation faulted.
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
            Outcome::NotCovered => (&[], &["fault"]),
            Outcome::Unlicensed => (&[], &[]),
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

/// A formula evaluation fault (SPEC 5.5, 15.11; `spec-0.4.0`): the optional `fault` member
/// of a `not_covered` envelope. The cell has no value, deterministically; this is not
/// missing data. These are formula codes, not verification codes, so they are not
/// [`ErrorCode`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Fault {
    /// `E_FORMULA_DIV_ZERO`
    DivZero,
    /// `E_FORMULA_DOMAIN`
    Domain,
    /// `E_FORMULA_RANGE`
    Range,
    /// `E_FORMULA_EMPTY`
    Empty,
    /// `E_FORMULA_ABSENT`
    Absent,
    /// `E_FORMULA_FUTURE`
    Future,
}

impl Fault {
    /// Every fault an envelope may carry, in the order of SPEC 5.5.
    pub const ALL: &'static [Fault] =
        &[Fault::DivZero, Fault::Domain, Fault::Range, Fault::Empty, Fault::Absent, Fault::Future];

    /// The code as written, e.g. `"E_FORMULA_DIV_ZERO"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Fault::DivZero => "E_FORMULA_DIV_ZERO",
            Fault::Domain => "E_FORMULA_DOMAIN",
            Fault::Range => "E_FORMULA_RANGE",
            Fault::Empty => "E_FORMULA_EMPTY",
            Fault::Absent => "E_FORMULA_ABSENT",
            Fault::Future => "E_FORMULA_FUTURE",
        }
    }

    /// Looks a fault up by its code; `None` for any other text (a budget, a cycle or too deep
    /// an evaluation is not a fault but the mesh's failure, SPEC 5.5).
    pub fn from_code(text: &str) -> Option<Fault> {
        Fault::ALL.iter().copied().find(|f| f.as_str() == text)
    }
}

impl core::fmt::Display for Fault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The members of a checked envelope that [`Envelope`] has no field for, because they were
/// added to SPEC 5.5 after it was frozen (0.3.0): see [`check_envelope_extras`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct EnvelopeExtras {
    /// The formula evaluation fault of a `not_covered` envelope (`spec-0.4.0`).
    pub fault: Option<Fault>,
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
    job: Option<String>,
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
        Ok(Request { cell_id: cell_id.into(), asof: asof.map(String::from), job: None })
    }

    /// The request description for `GET` of a job's `poll` URL (5.5, `spec-0.4.1`): the
    /// original request (`cell_id`, `asof`) and the job polled, 32 to 64 lowercase hex digits
    /// (otherwise [`InvalidRequest`]). With it, an envelope may be `bad_request` with
    /// `E_JOB_UNKNOWN`, and a `pending` one must carry this `job`; any other answer is bound
    /// to the original request, as a `/cell` answer is.
    pub fn with_job(cell_id: &str, asof: Option<&str>, job: &str) -> core::result::Result<Request, InvalidRequest> {
        if !is_job(job) {
            return Err(InvalidRequest(format!("job must be 32 to 64 lowercase hex digits: {job:?}")));
        }
        let mut r = Request::new(cell_id, asof)?;
        r.job = Some(job.into());
        Ok(r)
    }

    /// The job polled, for a request made at a job URL ([`Request::with_job`]).
    pub fn job(&self) -> Option<&str> {
        self.job.as_deref()
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
    let Some(t) = v.as_number_text() else {
        return env_err(path, "must be a non-negative integer");
    };
    let ok =
        !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()) && (t == "0" || !t.starts_with('0')) && t.len() <= 16;
    match (ok, t.parse::<u64>()) {
        (true, Ok(x)) if x <= limits::BLOB_LENGTH => Ok(x),
        _ => env_err(path, format!("must be written 0|[1-9][0-9]* and at most 2^53 - 1, not {t}")),
    }
}

/// Checks a response envelope (5.5) in the order of its procedure: strict JSON, members,
/// HTTP status, the receipt through step 6 of 10.1, the binding to `request`, then `trust`
/// and `superseded_by`. Never touches the network. A `not_covered` envelope's `fault`
/// (`spec-0.4.0`) is checked and accepted; [`check_envelope_extras`] also returns it.
pub fn check_envelope(status: u16, body: &[u8], keys: &Keys, request: &Request) -> Result<Envelope> {
    check_envelope_extras(status, body, keys, request).map(|(env, _)| env)
}

/// [`check_envelope`], also returning the members [`Envelope`] has no field for
/// ([`EnvelopeExtras`]): the `fault` of a `not_covered` envelope, one of the six formula
/// evaluation faults of SPEC 5.5 (`E_ENVELOPE` for any other value, or on another outcome).
///
/// Its [`Envelope`] has no `pending` outcome, so a `pending` envelope is `E_ENVELOPE` here, as
/// in 0.3.4; [`check_envelope_any`] accepts it.
pub fn check_envelope_extras(
    status: u16,
    body: &[u8],
    keys: &Keys,
    request: &Request,
) -> Result<(Envelope, EnvelopeExtras)> {
    // 1.
    let v = json::parse(body)?;
    check_parsed(status, &v, keys, request)
}

/// Checks a response envelope (5.5) of any of the nine outcomes, `pending` included
/// (`spec-0.4.1`), in the order of its procedure, and binds it to `request`: a `pending`
/// envelope's echoed request to the request made (and its `job` to the job polled, when
/// `request` has one), any other outcome to the original request's cell id and `asof`,
/// whether it answers the `/cell` request or the job URL. `E_JOB_UNKNOWN` is accepted only in a
/// `bad_request` answering a job URL (`request` with a job). Never touches the network.
pub fn check_envelope_any(status: u16, body: &[u8], keys: &Keys, request: &Request) -> Result<AnyEnvelope> {
    let v = json::parse(body)?;
    if let Value::Object(m) = &v {
        if schema::opt(m, "outcome").and_then(Value::as_str) == Some("pending") {
            return check_pending(status, m, request).map(AnyEnvelope::Pending);
        }
    }
    check_parsed(status, &v, keys, request).map(|(e, x)| AnyEnvelope::Envelope(e, x))
}

fn check_parsed(status: u16, v: &Value, keys: &Keys, request: &Request) -> Result<(Envelope, EnvelopeExtras)> {
    // 2.
    let m = match v {
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
            // E_JOB_UNKNOWN answers a job URL only (spec-0.4.1).
            (Outcome::BadRequest, Some(ErrorCode::JobUnknown)) => request.job.is_some(),
            (Outcome::BadRequest, Some(c)) => BAD_REQUEST_CODES.contains(&c),
            // Any code of 10.2 but the client's E_ENVELOPE and the job URL's E_JOB_UNKNOWN;
            // E_EVAL_UNAVAILABLE included since spec-0.3.0.
            (_, Some(c)) => c != ErrorCode::Envelope && c != ErrorCode::JobUnknown,
            (_, None) => false,
        };
        if !ok {
            return env_err("$.error", format!("{e:?} is not a code this outcome can carry"));
        }
        env.error = code;
    }
    let mut extras = EnvelopeExtras::default();
    if let Some(f) = schema::opt(m, "fault") {
        let Some(fault) = f.as_str().and_then(Fault::from_code) else {
            return env_err("$.fault", format!("{f:?} is not a formula evaluation fault"));
        };
        extras.fault = Some(fault);
    }
    // 3.
    if !outcome.status_ok(status) {
        return env_err("$.outcome", format!("status {status} does not match outcome {}", outcome.as_str()));
    }
    let Some(rv) = receipt_v else { return Ok((env, extras)) };
    // 4.
    let r = crate::verify::signature_valid_value(rv, keys).map_err(|(e, _)| {
        let inner = e.path().strip_prefix('$').unwrap_or(e.path());
        Error::new(e.code(), format!("$.receipt{inner}"), e.detail())
    })?;
    // 5.
    if r.cell().id() != request.cell_id {
        return env_err("$.receipt.cell", "the receipt is for another cell than the one requested");
    }
    if let Some(asked) = &request.asof {
        // D31 (revised in spec-0.3.0): at or before the requested asof, for ok and stale
        // alike; instants are canonical, so text order is time order.
        if r.asof() > asked.as_str() {
            return env_err("$.receipt.asof", "the receipt's asof is after the requested asof");
        }
    }
    // 6.
    let primary = r.signer_host() == lexical::host_of(&r.cell().ns) && r.inputs().is_empty();
    if (env.trust == Some(Trust::Primary)) != primary {
        return env_err(
            "$.trust",
            if primary { "a primary receipt is labelled primary" } else { "the receipt is not primary" },
        );
    }
    if env.superseded_by.as_deref() == Some(r.hash()) {
        return env_err("$.superseded_by", "a receipt cannot supersede itself");
    }
    env.receipt = Some(r);
    Ok((env, extras))
}

/// `[0-9a-f]{32,64}`.
fn is_job(s: &str) -> bool {
    (32..=64).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// The answer to a request: a [`Pending`] envelope, or one of the eight outcomes an
/// [`Envelope`] holds, with its [`EnvelopeExtras`] (see [`check_envelope_any`]).
// Not boxed: one value per checked response, and matching on it stays plain.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnyEnvelope {
    /// `ok`, `stale`, `not_covered`, `unlicensed`, `payment_required`, `rate_limited`,
    /// `bad_request` or `error`.
    Envelope(Envelope, EnvelopeExtras),
    /// `pending` (202): the mesh is still computing; the answer will be at `poll`.
    Pending(Pending),
}

impl AnyEnvelope {
    /// The outcome as written in JSON, `"pending"` included.
    pub fn outcome_str(&self) -> &'static str {
        match self {
            AnyEnvelope::Envelope(e, _) => e.outcome.as_str(),
            AnyEnvelope::Pending(_) => "pending",
        }
    }
}

/// A `pending` envelope (5.5, `spec-0.4.1`): the job, an estimate, the URL to poll and the
/// original request echoed. Build one with [`Pending::new`] (a mesh), or get one from
/// [`check_envelope_any`] (a client).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Pending {
    /// The job id: 32 to 64 lowercase hex digits, a capability.
    pub job: String,
    /// The mesh's estimate of the seconds until the result is ready; a hint.
    pub estimate_seconds: u64,
    /// The job's URL: `https://` and printable ASCII, at most 512 characters, ending
    /// `/jobs/<job>`.
    pub poll: String,
    /// The original request, echoed: its `cell_id` and `asof` (never a job).
    pub request: Request,
}

impl Pending {
    /// The HTTP status of a `pending` envelope.
    pub const STATUS: u16 = 202;

    /// A `pending` envelope answering `request` (its `cell_id` and `asof` are echoed; a job
    /// in it is not), checked by the rules of 5.5: `job` 32 to 64 lowercase hex digits,
    /// `estimate_seconds` at most 2^53 - 1, `poll` an `https://` URL of printable ASCII of at
    /// most 512 characters whose host (up to the first `/`) is a DNS name (2.1), ending
    /// `/jobs/<job>`. That `poll` names the mesh's own host is the caller's to ensure. A failure is `E_ENVELOPE`, at the member's
    /// path.
    pub fn new(job: &str, estimate_seconds: u64, poll: &str, request: Request) -> Result<Pending> {
        check_job_and_poll(job, poll)?;
        if estimate_seconds > limits::BLOB_LENGTH {
            return env_err("$.estimate_seconds", "estimate_seconds is at most 2^53 - 1");
        }
        let request = Request { job: None, ..request };
        Ok(Pending { job: job.into(), estimate_seconds, poll: poll.into(), request })
    }

    /// The JSON form of the envelope.
    pub fn to_value(&self) -> Value {
        // Value has no public number constructor: the canonical text, parsed.
        json::parse(&self.to_json()).unwrap_or(Value::Null)
    }

    /// The envelope's body: its canonical JSON text (JCS), served with [`Pending::STATUS`].
    pub fn to_json(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str("{\"estimate_seconds\":");
        out.push_str(&self.estimate_seconds.to_string());
        out.push_str(",\"job\":");
        json::write_string(&self.job, &mut out);
        out.push_str(",\"outcome\":\"pending\",\"poll\":");
        json::write_string(&self.poll, &mut out);
        out.push_str(",\"request\":{");
        if let Some(a) = &self.request.asof {
            out.push_str("\"asof\":");
            json::write_string(a, &mut out);
            out.push(',');
        }
        out.push_str("\"cell_id\":");
        json::write_string(&self.request.cell_id, &mut out);
        out.push_str("}}");
        out.into_bytes()
    }
}

fn check_job_and_poll(job: &str, poll: &str) -> Result<()> {
    if !is_job(job) {
        return env_err("$.job", "job is 32 to 64 lowercase hex digits");
    }
    let url_ok =
        poll.strip_prefix("https://").is_some_and(|r| !r.is_empty() && r.bytes().all(|c| (0x21..=0x7E).contains(&c)))
            && poll.len() <= limits::LICENCE_URL;
    if !url_ok {
        return env_err("$.poll", "poll is an https URL of printable ASCII, at most 512 characters");
    }
    // The text between https:// and the next / is a DNS name (2.1; 3f79313).
    let host = poll["https://".len()..].split('/').next().unwrap_or("");
    if !poll["https://".len()..].contains('/') || lexical::check_dns_name(host, "").is_err() {
        return env_err("$.poll", "poll's host is a DNS name");
    }
    if !poll.ends_with(&format!("/jobs/{job}")) {
        return env_err("$.poll", "poll ends /jobs/<job>");
    }
    Ok(())
}

/// A `pending` envelope (5.5 steps 2, 3 and 5; it has no receipt, so step 4 and 6 do not
/// apply).
fn check_pending(status: u16, m: &[(String, Value)], request: &Request) -> Result<Pending> {
    // 2.
    schema::exact(m, &["estimate_seconds", "job", "outcome", "poll", "request"], &[], "$", ErrorCode::Envelope)?;
    let estimate = integer_member(schema::get(m, "estimate_seconds"), "$.estimate_seconds")?;
    let Some(job) = schema::get(m, "job").as_str() else { return env_err("$.job", "job must be a string") };
    let Some(poll) = schema::get(m, "poll").as_str() else { return env_err("$.poll", "poll must be a string") };
    check_job_and_poll(job, poll)?;
    let rm = schema::object_or(schema::get(m, "request"), "$.request", ErrorCode::Envelope, "request")?;
    schema::exact(rm, &["cell_id"], &["asof"], "$.request", ErrorCode::Envelope)?;
    let cell_id = schema::string_or(rm, "cell_id", "$.request", ErrorCode::Envelope)?;
    let asof = match schema::opt(rm, "asof") {
        None => None,
        Some(_) => Some(schema::string_or(rm, "asof", "$.request", ErrorCode::Envelope)?),
    };
    let echoed =
        Request::new(cell_id, asof).map_err(|e| Error::new(ErrorCode::Envelope, "$.request", e.to_string()))?;
    // 3.
    if status != Pending::STATUS {
        return env_err("$.outcome", format!("status {status} does not match outcome pending"));
    }
    // 5.
    if echoed.cell_id != request.cell_id || echoed.asof != request.asof {
        return env_err("$.request", "the echoed request is not the request made");
    }
    if let Some(polled) = &request.job {
        if job != polled {
            return env_err("$.job", "the envelope is for another job than the one polled");
        }
    }
    Ok(Pending { job: job.into(), estimate_seconds: estimate, poll: poll.into(), request: echoed })
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
