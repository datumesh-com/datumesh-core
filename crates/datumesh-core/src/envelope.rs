//! Response envelopes (5.5): the mesh's unsigned statement around a receipt.

use crate::error::{err, ErrorCode, Result};
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

/// What failed in reaching a foreign mesh (SPEC 5.5, `spec-0.4.3`): the `detail` member, required
/// with `E_FOREIGN_UNAVAILABLE` ([`ErrorCode::ForeignUnavailable`]) and forbidden in every other
/// envelope. The list is closed; it is non-exhaustive only so that later revisions can add to
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ForeignDetail {
    /// `offline`: the asked party has no network, or is configured not to use it.
    Offline,
    /// `network`: any other network failure.
    Network,
    /// `dns`: the foreign host's name did not resolve.
    Dns,
    /// `tls`: the TLS handshake or the certificate failed.
    Tls,
    /// `timeout`.
    Timeout,
    /// `refused`: the connection was refused.
    Refused,
}

impl ForeignDetail {
    /// Every detail, in the order of SPEC 5.5.
    pub const ALL: &'static [ForeignDetail] = &[
        ForeignDetail::Offline,
        ForeignDetail::Network,
        ForeignDetail::Dns,
        ForeignDetail::Tls,
        ForeignDetail::Timeout,
        ForeignDetail::Refused,
    ];

    /// The detail as written in JSON, e.g. `"offline"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            ForeignDetail::Offline => "offline",
            ForeignDetail::Network => "network",
            ForeignDetail::Dns => "dns",
            ForeignDetail::Tls => "tls",
            ForeignDetail::Timeout => "timeout",
            ForeignDetail::Refused => "refused",
        }
    }

    /// Looks a detail up by its JSON name; `None` for anything else (case included).
    pub fn from_name(text: &str) -> Option<ForeignDetail> {
        ForeignDetail::ALL.iter().copied().find(|d| d.as_str() == text)
    }
}

impl core::fmt::Display for ForeignDetail {
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
    /// The function the answer is of: the echo of a pin, or the hash the name resolved to
    /// (`spec-0.4.2`).
    pub fn_hash: Option<String>,
    /// What failed in reaching a foreign mesh, with `E_FOREIGN_UNAVAILABLE` (`spec-0.4.3`).
    pub detail: Option<ForeignDetail>,
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
    pub(crate) cell_id: String,
    pub(crate) asof: Option<String>,
    pub(crate) job: Option<String>,
    pub(crate) pin: Option<String>,
    pub(crate) mesh: Option<String>,
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
        Ok(Request { cell_id: cell_id.into(), asof: asof.map(String::from), job: None, pin: None, mesh: None })
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

    /// The same request, pinned to the function `fn_hash` (64 lowercase hex digits; otherwise
    /// [`InvalidRequest`]): the answer must echo it (5.5, `spec-0.4.2`). `cell_id` is then the
    /// id of the cell of that function.
    pub fn with_pin(self, fn_hash: &str) -> core::result::Result<Request, InvalidRequest> {
        if lexical::check_hex(fn_hash, 64, "").is_err() {
            return Err(InvalidRequest(format!("fn_hash must be 64 lowercase hex digits: {fn_hash:?}")));
        }
        Ok(Request { pin: Some(fn_hash.into()), ..self })
    }

    /// The same request, naming the host it was sent to (a DNS name; otherwise
    /// [`InvalidRequest`]): `computed` is then checked against the receipt's signer, and
    /// `poll` against the host (5.5, `spec-0.4.2`). Without it those checks are the caller's.
    pub fn with_mesh(self, host: &str) -> core::result::Result<Request, InvalidRequest> {
        if lexical::check_dns_name(host, "").is_err() {
            return Err(InvalidRequest(format!("mesh must be a DNS name: {host:?}")));
        }
        Ok(Request { mesh: Some(host.into()), ..self })
    }

    /// The pin, for a pinned request ([`Request::with_pin`]).
    pub fn fn_hash(&self) -> Option<&str> {
        self.pin.as_deref()
    }

    /// The host the request was sent to, when given ([`Request::with_mesh`]).
    pub fn mesh(&self) -> Option<&str> {
        self.mesh.as_deref()
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

pub(crate) const BAD_REQUEST_CODES: [ErrorCode; 15] = [
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

pub(crate) fn env_err<T>(path: &str, detail: impl Into<String>) -> Result<T> {
    err(ErrorCode::Envelope, path, detail)
}

/// `0` or `[1-9][0-9]*`, at most 2^53 - 1, judged on the number's text.
pub(crate) fn integer_member(v: &Value, path: &str) -> Result<u64> {
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
/// in 0.3.4; [`check_envelope_any`] accepts it. Nor has its [`Trust`] `computed` (`spec-0.4.2`):
/// an envelope with it is `E_ENVELOPE` here and in [`check_envelope_any`], as in 0.3.5;
/// [`crate::check_answer`] accepts it. Everything else of `spec-0.4.2` (the `fn_hash` echo, in
/// [`EnvelopeExtras`]; `not_covered`'s `error`; the new codes) is checked as there, and so is
/// `spec-0.4.3`'s `E_FOREIGN_UNAVAILABLE` (502, with its `detail` in [`EnvelopeExtras`]).
pub fn check_envelope_extras(
    status: u16,
    body: &[u8],
    keys: &Keys,
    request: &Request,
) -> Result<(Envelope, EnvelopeExtras)> {
    crate::answer::check_answer(status, body, keys, request)?.into_envelope()
}

/// Checks a response envelope (5.5) of any of the nine outcomes, `pending` included
/// (`spec-0.4.1`), in the order of its procedure, and binds it to `request`: a `pending`
/// envelope's echoed request to the request made (and its `job` to the job polled, when
/// `request` has one), any other outcome to the original request's cell id and `asof`,
/// whether it answers the `/cell` request or the job URL. `E_JOB_UNKNOWN` is accepted only in a
/// `bad_request` answering a job URL (`request` with a job). Never touches the network.
pub fn check_envelope_any(status: u16, body: &[u8], keys: &Keys, request: &Request) -> Result<AnyEnvelope> {
    let a = crate::answer::check_answer(status, body, keys, request)?;
    if let Some(p) = a.pending {
        return Ok(AnyEnvelope::Pending(p));
    }
    a.into_envelope().map(|(e, x)| AnyEnvelope::Envelope(e, x))
}

/// `[0-9a-f]{32,64}`.
pub(crate) fn is_job(s: &str) -> bool {
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
    /// The original request, echoed: its `cell_id`, `asof` and pin (never a job or a mesh).
    pub request: Request,
}

impl Pending {
    /// The HTTP status of a `pending` envelope.
    pub const STATUS: u16 = 202;

    /// A `pending` envelope answering `request` (its `cell_id`, `asof` and pin are echoed; a
    /// job or mesh in it is not), checked by the rules of 5.5: `job` 32 to 64 lowercase hex digits,
    /// `estimate_seconds` at most 2^53 - 1, `poll` an `https://` URL of printable ASCII of at
    /// most 512 characters whose host (up to the first `/`) is a DNS name (2.1), ending
    /// `/jobs/<job>`. That `poll` names the mesh's own host is the caller's to ensure. A failure
    /// is `E_ENVELOPE`, at the member's path.
    pub fn new(job: &str, estimate_seconds: u64, poll: &str, request: Request) -> Result<Pending> {
        check_job_and_poll(job, poll)?;
        if estimate_seconds > limits::BLOB_LENGTH {
            return env_err("$.estimate_seconds", "estimate_seconds is at most 2^53 - 1");
        }
        let request = Request { job: None, mesh: None, ..request };
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
        if let Some(p) = &self.request.pin {
            out.push_str(",\"fn_hash\":");
            json::write_string(p, &mut out);
        }
        out.push_str("}}");
        out.into_bytes()
    }
}

pub(crate) fn check_job_and_poll(job: &str, poll: &str) -> Result<()> {
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

pub(crate) fn quote(q: &Value) -> Result<Quote> {
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
