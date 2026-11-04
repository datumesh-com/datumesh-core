//! Envelopes of `spec-0.4.2` (5.5-5.7): every outcome and trust label, the `fn_hash` echo, slice
//! pages and batches, checked and built.

use std::collections::BTreeMap;

use crate::descriptor::cmp_lexical;
use crate::envelope::{
    check_job_and_poll, env_err, integer_member, quote, EnvelopeExtras, Fault, ForeignDetail, Outcome, Pending, Quote,
    Request, Trust, BAD_REQUEST_CODES,
};
use crate::error::{Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::Keys;
use crate::lexical::{self, limits};
use crate::model::{Arg, Scalar};
use crate::receipt::Receipt;
use crate::schema;
use crate::slice::Slice;
use crate::Envelope;

/// The nine outcomes of SPEC 5.5 (`spec-0.4.2`), `pending` included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AnswerOutcome {
    /// 200.
    Ok,
    /// 200.
    Stale,
    /// 404.
    NotCovered,
    /// 403.
    Unlicensed,
    /// 402.
    PaymentRequired,
    /// 429.
    RateLimited,
    /// 400.
    BadRequest,
    /// 500-599.
    Error,
    /// 202.
    Pending,
}

impl AnswerOutcome {
    /// The outcome as written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            AnswerOutcome::Ok => "ok",
            AnswerOutcome::Stale => "stale",
            AnswerOutcome::NotCovered => "not_covered",
            AnswerOutcome::Unlicensed => "unlicensed",
            AnswerOutcome::PaymentRequired => "payment_required",
            AnswerOutcome::RateLimited => "rate_limited",
            AnswerOutcome::BadRequest => "bad_request",
            AnswerOutcome::Error => "error",
            AnswerOutcome::Pending => "pending",
        }
    }

    /// The outcome of its JSON name.
    pub fn from_name(s: &str) -> Option<AnswerOutcome> {
        Some(match s {
            "ok" => AnswerOutcome::Ok,
            "stale" => AnswerOutcome::Stale,
            "not_covered" => AnswerOutcome::NotCovered,
            "unlicensed" => AnswerOutcome::Unlicensed,
            "payment_required" => AnswerOutcome::PaymentRequired,
            "rate_limited" => AnswerOutcome::RateLimited,
            "bad_request" => AnswerOutcome::BadRequest,
            "error" => AnswerOutcome::Error,
            "pending" => AnswerOutcome::Pending,
            _ => return None,
        })
    }

    /// The HTTP status a mesh sends with it (500 for `error`, whose exact 5xx is the mesh's).
    pub const fn status(self) -> u16 {
        match self {
            AnswerOutcome::Ok | AnswerOutcome::Stale => 200,
            AnswerOutcome::NotCovered => 404,
            AnswerOutcome::Unlicensed => 403,
            AnswerOutcome::PaymentRequired => 402,
            AnswerOutcome::RateLimited => 429,
            AnswerOutcome::BadRequest => 400,
            AnswerOutcome::Error => 500,
            AnswerOutcome::Pending => 202,
        }
    }

    fn status_ok(self, status: u16) -> bool {
        match self {
            AnswerOutcome::Error => (500..=599).contains(&status),
            other => other.status() == status,
        }
    }

    fn members(self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            AnswerOutcome::Ok => (&["receipt", "trust"], &["superseded_by", "reason", "fn_hash"]),
            AnswerOutcome::Stale => (&["receipt", "trust", "age_seconds"], &["superseded_by", "reason", "fn_hash"]),
            AnswerOutcome::NotCovered => (&[], &["fault", "error", "fn_hash"]),
            AnswerOutcome::Unlicensed => (&[], &["fn_hash"]),
            AnswerOutcome::PaymentRequired => (&["quote"], &["fn_hash"]),
            AnswerOutcome::RateLimited => (&["retry_after_seconds"], &["fn_hash"]),
            AnswerOutcome::BadRequest => (&["error"], &["fn_hash"]),
            // spec-0.4.3: detail, with E_FOREIGN_UNAVAILABLE only.
            AnswerOutcome::Error => (&["error"], &["detail", "fn_hash"]),
            AnswerOutcome::Pending => (&["estimate_seconds", "job", "poll", "request"], &[]),
        }
    }
}

/// The four trust labels of SPEC 5.5 (`spec-0.4.2`), `computed` included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TrustLabel {
    /// The publisher answered from its own records.
    Primary,
    /// Not primary; the mesh asked evaluated the cell and signed the receipt itself.
    Computed,
    /// Not primary, signed by another host, and the mesh recomputed it.
    Verified,
    /// Not primary, signed by another host, not recomputed.
    Unverified,
}

impl TrustLabel {
    /// The label as written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            TrustLabel::Primary => "primary",
            TrustLabel::Computed => "computed",
            TrustLabel::Verified => "verified",
            TrustLabel::Unverified => "unverified",
        }
    }

    fn from_name(s: &str) -> Option<TrustLabel> {
        Some(match s {
            "primary" => TrustLabel::Primary,
            "computed" => TrustLabel::Computed,
            "verified" => TrustLabel::Verified,
            "unverified" => TrustLabel::Unverified,
            _ => return None,
        })
    }
}

/// A slice page (5.6): the `ok` answer of the slice endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SlicePage {
    /// The slice object requested, with the function hash its name resolved to (or the pin).
    pub slice: Slice,
    /// The member envelopes, `ok` or `stale`, in ascending key order.
    pub members: Vec<Answer>,
    /// The URL of the following page; `None` on the last.
    pub next: Option<String>,
}

impl SlicePage {
    /// A page of `members` of `slice`, with `next` when there is a following page. Not checked
    /// until written and checked back.
    pub fn new(slice: Slice, members: Vec<Answer>, next: Option<String>) -> SlicePage {
        SlicePage { slice, members, next }
    }
}

/// An envelope of `spec-0.4.2` (5.5), checked or to be written: every outcome, trust label and
/// member, with a slice page (5.6) for the `ok` answer of the slice endpoint; since 0.3.7
/// (`spec-0.4.3`) also `E_FOREIGN_UNAVAILABLE` with its `detail` ([`Answer::foreign_unavailable`]). As a builder: one
/// constructor per outcome, `with_*` for the optional members, [`Answer::to_json`] for the body
/// and [`Answer::status`] for the HTTP status; check the body back with [`check_answer`] (or
/// [`check_slice_answer`]) to hold it to every rule.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Answer {
    /// The outcome.
    pub outcome: AnswerOutcome,
    /// The receipt (`ok`, `stale`; not a slice page's).
    pub receipt: Option<Receipt>,
    /// The trust label (`ok`, `stale`).
    pub trust: Option<TrustLabel>,
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
    /// The code (`bad_request`, `error`, and `E_NAME_UNBOUND` in `not_covered`).
    pub error: Option<ErrorCode>,
    /// The formula evaluation fault (`not_covered`).
    pub fault: Option<Fault>,
    /// What failed in reaching a foreign mesh: with `E_FOREIGN_UNAVAILABLE` only, and always
    /// with it (`error`, 502; `spec-0.4.3`).
    pub detail: Option<ForeignDetail>,
    /// The function the answer is of: a pin's echo, or the hash the name resolved to.
    pub fn_hash: Option<String>,
    /// The `pending` envelope's members.
    pub pending: Option<Pending>,
    /// The slice page (`ok` at the slice endpoint).
    pub page: Option<SlicePage>,
    /// The HTTP status to send (`error`'s exact 5xx); checked answers carry the one received.
    status: u16,
}

fn bare(outcome: AnswerOutcome) -> Answer {
    Answer {
        outcome,
        receipt: None,
        trust: None,
        age_seconds: None,
        superseded_by: None,
        reason: None,
        quote: None,
        retry_after_seconds: None,
        error: None,
        fault: None,
        detail: None,
        fn_hash: None,
        pending: None,
        page: None,
        status: outcome.status(),
    }
}

fn code_allowed(outcome: AnswerOutcome, code: ErrorCode, job_url: bool) -> bool {
    match outcome {
        AnswerOutcome::BadRequest => {
            BAD_REQUEST_CODES.contains(&code)
                || matches!(code, ErrorCode::CanonHex | ErrorCode::SliceAtCell)
                || (code == ErrorCode::JobUnknown && job_url)
        }
        AnswerOutcome::NotCovered => code == ErrorCode::NameUnbound,
        AnswerOutcome::Error => !matches!(
            code,
            ErrorCode::Envelope | ErrorCode::JobUnknown | ErrorCode::SliceAtCell | ErrorCode::NameUnbound
        ),
        _ => false,
    }
}

impl Answer {
    /// `ok` (200): `receipt`, served with `trust`.
    pub fn ok(receipt: Receipt, trust: TrustLabel) -> Answer {
        Answer { receipt: Some(receipt), trust: Some(trust), ..bare(AnswerOutcome::Ok) }
    }

    /// `stale` (200): the last receipt known, last confirmed `age_seconds` ago.
    pub fn stale(receipt: Receipt, trust: TrustLabel, age_seconds: u64) -> Answer {
        Answer {
            receipt: Some(receipt),
            trust: Some(trust),
            age_seconds: Some(age_seconds),
            ..bare(AnswerOutcome::Stale)
        }
    }

    /// `not_covered` (404).
    pub fn not_covered() -> Answer {
        bare(AnswerOutcome::NotCovered)
    }

    /// `unlicensed` (403).
    pub fn unlicensed() -> Answer {
        bare(AnswerOutcome::Unlicensed)
    }

    /// `payment_required` (402) with its quote.
    pub fn payment_required(quote: Quote) -> Answer {
        Answer { quote: Some(quote), ..bare(AnswerOutcome::PaymentRequired) }
    }

    /// `rate_limited` (429): retry after this many seconds.
    pub fn rate_limited(retry_after_seconds: u64) -> Answer {
        Answer { retry_after_seconds: Some(retry_after_seconds), ..bare(AnswerOutcome::RateLimited) }
    }

    /// `bad_request` (400) with a code it can carry (5.5: a parse code, `E_CANON_HEX`,
    /// `E_SLICE_AT_CELL`, or `E_JOB_UNKNOWN` at a job URL); `E_ENVELOPE` otherwise.
    pub fn bad_request(code: ErrorCode) -> Result<Answer> {
        if !code_allowed(AnswerOutcome::BadRequest, code, true) {
            return env_err("$.error", format!("bad_request cannot carry {code}"));
        }
        Ok(Answer { error: Some(code), ..bare(AnswerOutcome::BadRequest) })
    }

    /// `error` (500; see [`Answer::with_status`]) with a code it can carry (any of 10.2 but
    /// `E_ENVELOPE`, `E_JOB_UNKNOWN`, `E_SLICE_AT_CELL` and `E_NAME_UNBOUND`).
    /// `E_FOREIGN_UNAVAILABLE` needs a `detail` and HTTP 502: build it with
    /// [`Answer::foreign_unavailable`] (here it is `E_ENVELOPE`).
    pub fn error(code: ErrorCode) -> Result<Answer> {
        if !code_allowed(AnswerOutcome::Error, code, false) {
            return env_err("$.error", format!("error cannot carry {code}"));
        }
        if code == ErrorCode::ForeignUnavailable {
            return env_err("$.detail", "E_FOREIGN_UNAVAILABLE needs a detail: Answer::foreign_unavailable");
        }
        Ok(Answer { error: Some(code), ..bare(AnswerOutcome::Error) })
    }

    /// `error` with `E_FOREIGN_UNAVAILABLE` and `detail` (5.5, `spec-0.4.3`), HTTP 502: the cell
    /// belongs to another mesh, which could not be reached. Answered by a resolver, or by a mesh
    /// fetching across meshes, when no HTTP response came back from the foreign host. Its
    /// status must stay 502 ([`Answer::with_status`] to another fails [`check_answer`]).
    pub fn foreign_unavailable(detail: ForeignDetail) -> Answer {
        Answer {
            error: Some(ErrorCode::ForeignUnavailable),
            detail: Some(detail),
            status: 502,
            ..bare(AnswerOutcome::Error)
        }
    }

    /// `pending` (202).
    pub fn pending(p: Pending) -> Answer {
        Answer { pending: Some(p), ..bare(AnswerOutcome::Pending) }
    }

    /// The `ok` answer of the slice endpoint (5.6).
    pub fn page(page: SlicePage) -> Answer {
        Answer { page: Some(page), ..bare(AnswerOutcome::Ok) }
    }

    /// With the `fn_hash` echo (64 lowercase hex digits; not on `pending`); `E_ENVELOPE`
    /// otherwise.
    pub fn with_fn_hash(self, fn_hash: &str) -> Result<Answer> {
        if self.outcome == AnswerOutcome::Pending || lexical::check_hex(fn_hash, 64, "").is_err() {
            return env_err("$.fn_hash", "fn_hash is 64 lowercase hex digits, not on pending");
        }
        Ok(Answer { fn_hash: Some(fn_hash.into()), ..self })
    }

    /// A `not_covered` answer with a formula evaluation fault. Not checked here: a fault with
    /// `error`, or on another outcome, fails [`check_answer`].
    pub fn with_fault(self, fault: Fault) -> Answer {
        Answer { fault: Some(fault), ..self }
    }

    /// A `not_covered` answer with `error` (only `E_NAME_UNBOUND`, and not with a fault);
    /// `E_ENVELOPE` otherwise.
    pub fn with_error(self, code: ErrorCode) -> Result<Answer> {
        if self.outcome != AnswerOutcome::NotCovered || self.fault.is_some() || code != ErrorCode::NameUnbound {
            return env_err("$.error", "not_covered carries E_NAME_UNBOUND only, and not with a fault");
        }
        Ok(Answer { error: Some(code), ..self })
    }

    /// An `ok` or `stale` answer pointing to a newer receipt, with an optional reason.
    pub fn with_superseded(self, superseded_by: &str, reason: Option<&str>) -> Answer {
        Answer { superseded_by: Some(superseded_by.into()), reason: reason.map(String::from), ..self }
    }

    /// The HTTP status to send, for an `error` within 500-599 (other outcomes have one status).
    pub fn with_status(self, status: u16) -> Answer {
        Answer { status, ..self }
    }

    /// The HTTP status: the outcome's, or for a checked answer the one received.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The JSON form of the envelope.
    pub fn to_value(&self) -> Value {
        if let Some(p) = &self.pending {
            return p.to_value();
        }
        let mut m: Vec<(String, Value)> = vec![("outcome".into(), Value::String(self.outcome.as_str().into()))];
        let s = |x: &str| Value::String(x.into());
        let n = |x: u64| json::parse(x.to_string().as_bytes()).unwrap_or(Value::Null);
        if let Some(r) = &self.receipt {
            m.push(("receipt".into(), receipt_value(r)));
        }
        if let Some(t) = self.trust {
            m.push(("trust".into(), s(t.as_str())));
        }
        if let Some(a) = self.age_seconds {
            m.push(("age_seconds".into(), n(a)));
        }
        if let Some(x) = &self.superseded_by {
            m.push(("superseded_by".into(), s(x)));
        }
        if let Some(x) = &self.reason {
            m.push(("reason".into(), s(x)));
        }
        if let Some(q) = &self.quote {
            m.push((
                "quote".into(),
                Value::Object(vec![
                    ("amount".into(), s(&q.amount)),
                    ("authorise".into(), s(&q.authorise)),
                    ("currency".into(), s(&q.currency)),
                ]),
            ));
        }
        if let Some(a) = self.retry_after_seconds {
            m.push(("retry_after_seconds".into(), n(a)));
        }
        if let Some(e) = self.error {
            m.push(("error".into(), s(e.as_str())));
        }
        if let Some(f) = self.fault {
            m.push(("fault".into(), s(f.as_str())));
        }
        if let Some(d) = self.detail {
            m.push(("detail".into(), s(d.as_str())));
        }
        if let Some(h) = &self.fn_hash {
            m.push(("fn_hash".into(), s(h)));
        }
        if let Some(p) = &self.page {
            m.push(("slice".into(), p.slice.to_value()));
            m.push(("members".into(), Value::Array(p.members.iter().map(Answer::to_value).collect())));
            if let Some(x) = &p.next {
                m.push(("next".into(), s(x)));
            }
        }
        Value::Object(m)
    }

    /// The envelope's body: its canonical JSON text (JCS).
    pub fn to_json(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The `spec-0.4.1` form, for the 0.3.x functions: not `pending`, not `computed`.
    pub(crate) fn into_envelope(self) -> Result<(Envelope, EnvelopeExtras)> {
        let outcome = match self.outcome {
            AnswerOutcome::Ok => Outcome::Ok,
            AnswerOutcome::Stale => Outcome::Stale,
            AnswerOutcome::NotCovered => Outcome::NotCovered,
            AnswerOutcome::Unlicensed => Outcome::Unlicensed,
            AnswerOutcome::PaymentRequired => Outcome::PaymentRequired,
            AnswerOutcome::RateLimited => Outcome::RateLimited,
            AnswerOutcome::BadRequest => Outcome::BadRequest,
            AnswerOutcome::Error => Outcome::Error,
            AnswerOutcome::Pending => return env_err("$.outcome", "unknown outcome \"pending\""),
        };
        let trust = match self.trust {
            None => None,
            Some(TrustLabel::Primary) => Some(Trust::Primary),
            Some(TrustLabel::Verified) => Some(Trust::Verified),
            Some(TrustLabel::Unverified) => Some(Trust::Unverified),
            Some(_) => return env_err("$.trust", "trust is primary, verified or unverified"),
        };
        let env = Envelope {
            outcome,
            receipt: self.receipt,
            trust,
            age_seconds: self.age_seconds,
            superseded_by: self.superseded_by,
            reason: self.reason,
            quote: self.quote,
            retry_after_seconds: self.retry_after_seconds,
            error: self.error,
        };
        Ok((env, EnvelopeExtras { fault: self.fault, fn_hash: self.fn_hash, detail: self.detail }))
    }
}

fn receipt_value(r: &Receipt) -> Value {
    let mut members = json::parse(r.payload()).ok().and_then(|v| v.as_object().map(<[_]>::to_vec)).unwrap_or_default();
    members.push(("sig".into(), Value::String(r.sig().into())));
    Value::Object(members)
}

/// What an envelope is checked against: a cell request, the slice endpoint's request, or
/// nothing (a batch's refusal).
enum Ctx<'a> {
    Cell(&'a Request),
    Slice(&'a SliceRequest),
    None,
}

impl Ctx<'_> {
    fn pin(&self) -> Option<&str> {
        match self {
            Ctx::Cell(r) => r.pin.as_deref(),
            Ctx::Slice(s) => s.pin.as_deref(),
            Ctx::None => None,
        }
    }

    fn mesh(&self) -> Option<&str> {
        match self {
            Ctx::Cell(r) => r.mesh.as_deref(),
            Ctx::Slice(s) => s.mesh.as_deref(),
            Ctx::None => None,
        }
    }

    fn job_url(&self) -> bool {
        matches!(self, Ctx::Cell(r) if r.job.is_some())
    }
}

/// Checks an envelope (5.5) answering a `/cell` request or a job URL, of any outcome and trust
/// label, in the order of 5.5: strict JSON, members, HTTP status, the receipt through step 6 of
/// 10.1, the binding to `request` (cell id, `asof`, the pin and its echo, the echoed request
/// and job of `pending`), then `trust` (with `computed` against the signer and `poll` against
/// the host when `request` names the mesh asked) and `superseded_by`. Never touches the
/// network.
pub fn check_answer(status: u16, body: &[u8], keys: &Keys, request: &Request) -> Result<Answer> {
    let v = json::parse(body)?;
    check_value(status, &v, keys, &Ctx::Cell(request), "$")
}

/// A slice request (5.6): the slice id asked for, the `asof`, the pin, the host asked, and for
/// a following page the key of the last member received (`after`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SliceRequest {
    slice_id: String,
    asof: Option<String>,
    pin: Option<String>,
    mesh: Option<String>,
    after: Option<BTreeMap<String, Scalar>>,
}

impl SliceRequest {
    /// A request for the slice with id `slice_id` (64 lowercase hex digits) as of `asof` (an
    /// instant) or now; anything else is [`crate::InvalidRequest`].
    pub fn new(slice_id: &str, asof: Option<&str>) -> core::result::Result<SliceRequest, crate::InvalidRequest> {
        let r = Request::new(slice_id, asof)?;
        Ok(SliceRequest { slice_id: r.cell_id, asof: r.asof, pin: None, mesh: None, after: None })
    }

    /// Pinned to `fn_hash` (64 lowercase hex digits).
    pub fn with_pin(self, fn_hash: &str) -> core::result::Result<SliceRequest, crate::InvalidRequest> {
        let r = Request::new(&self.slice_id, None)?.with_pin(fn_hash)?;
        Ok(SliceRequest { pin: r.pin, ..self })
    }

    /// Naming the host asked (a DNS name): `next` and `computed` are then checked against it.
    pub fn with_mesh(self, host: &str) -> core::result::Result<SliceRequest, crate::InvalidRequest> {
        let r = Request::new(&self.slice_id, None)?.with_mesh(host)?;
        Ok(SliceRequest { mesh: r.mesh, ..self })
    }

    /// For a following page: the key of the last member received, from each open argument's
    /// name to its value. Other names are ignored; a cursor lacking an open argument, or whose
    /// value has another type than the members' key, cannot be ordered against (`E_ENVELOPE`
    /// from [`check_slice_answer`]).
    pub fn with_after(self, after: BTreeMap<String, Scalar>) -> SliceRequest {
        SliceRequest { after: Some(after), ..self }
    }

    /// The slice id asked for.
    pub fn slice_id(&self) -> &str {
        &self.slice_id
    }

    /// The `asof` asked for.
    pub fn asof(&self) -> Option<&str> {
        self.asof.as_deref()
    }

    /// The pin.
    pub fn fn_hash(&self) -> Option<&str> {
        self.pin.as_deref()
    }

    /// The host asked.
    pub fn mesh(&self) -> Option<&str> {
        self.mesh.as_deref()
    }

    /// The key after which this page's members come.
    pub fn after(&self) -> Option<&BTreeMap<String, Scalar>> {
        self.after.as_ref()
    }
}

/// Checks an answer of the slice endpoint (5.6) against `request`: `stale` and `pending` are
/// not answers there; `ok` is a page, checked as 5.6 says (its members, the slice id, `next`,
/// then each member's receipt through step 6 of 10.1, membership, `asof`, trust and key order
/// after `after`); every other outcome as in 5.5.
pub fn check_slice_answer(status: u16, body: &[u8], keys: &Keys, request: &SliceRequest) -> Result<Answer> {
    let v = json::parse(body)?;
    check_value(status, &v, keys, &Ctx::Slice(request), "$")
}

/// The answer to a batch (5.7).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Batch {
    /// 200: one result per request, in request order; a failing item is its error, in place.
    Items(Vec<Result<Answer>>),
    /// The whole batch refused: one envelope (`unlicensed`, `payment_required`, `rate_limited`,
    /// `bad_request` or `error`).
    Refused(Box<Answer>),
}

/// Checks the answer to a batch (5.7) of `requests`: for 200, an array with one
/// `{envelope, status}` per request (`status` an integer 100-599), else `E_ENVELOPE` for the
/// whole; then each item as an envelope (5.5) with its status against its own request, a
/// failing item reported in its place. Any other status is one envelope of a refusal outcome,
/// with nothing to bind.
pub fn check_batch(status: u16, body: &[u8], keys: &Keys, requests: &[Request]) -> Result<Batch> {
    let v = json::parse(body)?;
    if status != 200 {
        if !matches!(v, Value::Object(_)) {
            return env_err("$", "a refused batch is one envelope");
        }
        let a = check_value(status, &v, keys, &Ctx::None, "$")?;
        if !matches!(
            a.outcome,
            AnswerOutcome::Unlicensed
                | AnswerOutcome::PaymentRequired
                | AnswerOutcome::RateLimited
                | AnswerOutcome::BadRequest
                | AnswerOutcome::Error
        ) {
            return env_err(
                "$.outcome",
                "a batch is refused with unlicensed, payment_required, rate_limited, bad_request or error",
            );
        }
        return Ok(Batch::Refused(Box::new(a)));
    }
    let Value::Array(items) = &v else { return env_err("$", "a batch answer is an array") };
    if items.len() != requests.len() {
        return env_err("$", format!("{} items for {} requests", items.len(), requests.len()));
    }
    let mut parts = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let p = format!("$[{i}]");
        let m = schema::object_or(item, &p, ErrorCode::Envelope, "an item")?;
        schema::exact(m, &["envelope", "status"], &[], &p, ErrorCode::Envelope)?;
        let st = integer_member(schema::get(m, "status"), &format!("{p}.status"))?;
        if !(100..=599).contains(&st) {
            return env_err(&format!("{p}.status"), "an item's status is 100-599");
        }
        parts.push((schema::get(m, "envelope"), st as u16));
    }
    Ok(Batch::Items(
        parts
            .into_iter()
            .zip(requests)
            .enumerate()
            .map(|(i, ((env, st), req))| check_value(st, env, keys, &Ctx::Cell(req), &format!("$[{i}].envelope")))
            .collect(),
    ))
}

/// A batch answer's body (5.7): one `{envelope, status}` per answer, in order.
pub fn batch_body(items: &[(u16, Answer)]) -> Vec<u8> {
    let n = |x: u16| json::parse(x.to_string().as_bytes()).unwrap_or(Value::Null);
    Value::Array(
        items
            .iter()
            .map(|(st, a)| Value::Object(vec![("envelope".into(), a.to_value()), ("status".into(), n(*st))]))
            .collect(),
    )
    .to_jcs()
}

fn nested(e: Error, prefix: &str) -> Error {
    if prefix == "$" {
        return e;
    }
    let inner = e.path().strip_prefix('$').unwrap_or(e.path());
    Error::new(e.code(), format!("{prefix}{inner}"), e.detail())
}

fn check_value(status: u16, v: &Value, keys: &Keys, ctx: &Ctx<'_>, at: &str) -> Result<Answer> {
    check_inner(status, v, keys, ctx).map_err(|e| nested(e, at))
}

/// A URL `https://<DNS name>/<path>` of printable ASCII, at most 512 characters.
fn url_host(u: &str) -> Option<&str> {
    let rest = u.strip_prefix("https://")?;
    if u.len() > limits::LICENCE_URL || !rest.bytes().all(|c| (0x21..=0x7E).contains(&c)) {
        return None;
    }
    let (host, _) = rest.split_once('/')?;
    lexical::check_dns_name(host, "").ok().map(|()| host)
}

fn check_inner(status: u16, v: &Value, keys: &Keys, ctx: &Ctx<'_>) -> Result<Answer> {
    // 2.
    let m = match v {
        Value::Object(m) if matches!(schema::opt(m, "outcome"), Some(Value::String(_))) => m,
        _ => return env_err("$", "an envelope is an object with a string outcome"),
    };
    let oname = schema::get(m, "outcome").as_str().unwrap_or("");
    let Some(outcome) = AnswerOutcome::from_name(oname) else {
        return env_err("$.outcome", format!("unknown outcome {oname:?}"));
    };
    let slice_ctx = matches!(ctx, Ctx::Slice(_));
    if slice_ctx && matches!(outcome, AnswerOutcome::Stale | AnswerOutcome::Pending) {
        return env_err("$.outcome", "stale and pending are not answers of the slice endpoint");
    }
    if outcome == AnswerOutcome::Pending {
        return check_pending(status, m, ctx);
    }
    let is_page = slice_ctx && outcome == AnswerOutcome::Ok;
    let mut a = check_members(m, outcome, ctx, is_page)?;
    // 3.
    if !outcome.status_ok(status) {
        return env_err("$.outcome", format!("status {status} does not match outcome {oname}"));
    }
    // spec-0.4.3: E_FOREIGN_UNAVAILABLE is answered with 502 exactly.
    if a.error == Some(ErrorCode::ForeignUnavailable) && status != 502 {
        return env_err("$.outcome", format!("E_FOREIGN_UNAVAILABLE is answered with 502, not {status}"));
    }
    a.status = status;
    // 3a. What the request decides without the receipt: the pin echo (E_JOB_UNKNOWN only at
    // a job URL was checked with the members; pending is checked apart).
    pin_echo(&a, ctx.pin(), "$.fn_hash")?;
    if is_page {
        let Ctx::Slice(req) = ctx else { return env_err("$", "a page answers a slice request") };
        return check_page(m, a, keys, req);
    }
    // 4.
    if let Some(rv) = schema::opt(m, "receipt") {
        a.receipt = Some(member_receipt(rv, keys, "$.receipt")?);
    }
    // 5.
    if let Ctx::Cell(req) = ctx {
        bind_cell(&a, req)?;
    }
    // 6.
    if let Some(r) = &a.receipt {
        trust_check(&a, r, ctx.mesh(), "$")?;
    }
    Ok(a)
}

fn member_receipt(rv: &Value, keys: &Keys, path: &str) -> Result<Receipt> {
    crate::verify::signature_valid_value(rv, keys).map_err(|(e, _)| {
        let inner = e.path().strip_prefix('$').unwrap_or(e.path());
        Error::new(e.code(), format!("{path}{inner}"), e.detail())
    })
}

/// 5.5 step 2 for every outcome but `pending`; for a page, its own members.
fn check_members(m: &[(String, Value)], outcome: AnswerOutcome, ctx: &Ctx<'_>, is_page: bool) -> Result<Answer> {
    let (req, opt): (&[&str], &[&str]) =
        if is_page { (&["members", "slice"], &["next", "fn_hash"]) } else { outcome.members() };
    let mut allowed: Vec<&str> = vec!["outcome"];
    allowed.extend_from_slice(req);
    schema::exact(m, &allowed, opt, "$", ErrorCode::Envelope)?;
    let mut a = bare(outcome);
    if let Some(r) = schema::opt(m, "receipt") {
        if !matches!(r, Value::Object(_)) {
            return env_err("$.receipt", "receipt is a receipt object");
        }
    }
    if let Some(t) = schema::opt(m, "trust") {
        a.trust = match t.as_str().and_then(TrustLabel::from_name) {
            Some(t) => Some(t),
            None => return env_err("$.trust", "trust is primary, computed, verified or unverified"),
        };
    }
    if let Some(x) = schema::opt(m, "age_seconds") {
        a.age_seconds = Some(integer_member(x, "$.age_seconds")?);
    }
    if let Some(x) = schema::opt(m, "retry_after_seconds") {
        a.retry_after_seconds = Some(integer_member(x, "$.retry_after_seconds")?);
    }
    if let Some(s) = schema::opt(m, "superseded_by") {
        let s = s.as_str().unwrap_or("");
        lexical::check_hex(s, 64, "$.superseded_by").map_err(|e| e.recode(ErrorCode::Envelope))?;
        a.superseded_by = Some(s.into());
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
        a.reason = Some(r.into());
    }
    if let Some(q) = schema::opt(m, "quote") {
        a.quote = Some(quote(q)?);
    }
    if let Some(e) = schema::opt(m, "error") {
        let code = e.as_str().and_then(ErrorCode::from_code);
        match code {
            Some(c) if code_allowed(outcome, c, ctx.job_url()) => a.error = Some(c),
            _ => return env_err("$.error", format!("{e:?} is not a code this outcome can carry")),
        }
    }
    // spec-0.4.3: detail, one of the closed list, present exactly with E_FOREIGN_UNAVAILABLE.
    if let Some(d) = schema::opt(m, "detail") {
        let Some(detail) = d.as_str().and_then(ForeignDetail::from_name) else {
            return env_err("$.detail", format!("{d:?} is not a detail of E_FOREIGN_UNAVAILABLE"));
        };
        a.detail = Some(detail);
    }
    if (a.error == Some(ErrorCode::ForeignUnavailable)) != a.detail.is_some() {
        return env_err("$.detail", "detail is present exactly when error is E_FOREIGN_UNAVAILABLE");
    }
    if let Some(f) = schema::opt(m, "fault") {
        let Some(fault) = f.as_str().and_then(Fault::from_code) else {
            return env_err("$.fault", format!("{f:?} is not a formula evaluation fault"));
        };
        if a.error.is_some() {
            return env_err("$.fault", "not_covered carries a fault or an error, not both");
        }
        a.fault = Some(fault);
    }
    if let Some(h) = schema::opt(m, "fn_hash") {
        let h = h.as_str().unwrap_or("");
        lexical::check_hex(h, 64, "$.fn_hash").map_err(|e| e.recode(ErrorCode::Envelope))?;
        a.fn_hash = Some(h.into());
    }
    if let Some(n) = schema::opt(m, "next") {
        if n.as_str().and_then(url_host).is_none() {
            return env_err("$.next", "next is https://, a DNS name, then a path, at most 512 characters");
        }
    }
    Ok(a)
}

/// The receipt answers the request (5.5 step 5).
fn bind_cell(a: &Answer, req: &Request) -> Result<()> {
    let Some(r) = &a.receipt else { return Ok(()) };
    if r.cell().id() != req.cell_id {
        return env_err("$.receipt.cell", "the receipt is for another cell than the one requested");
    }
    if let Some(asked) = &req.asof {
        if r.asof() > asked.as_str() {
            return env_err("$.receipt.asof", "the receipt's asof is after the requested asof");
        }
    }
    Ok(())
}

/// The pin echo (5.5 step 3a): with a pin, `fn_hash` required (but on `rate_limited` and
/// `bad_request`) and equal to it.
fn pin_echo(a: &Answer, pin: Option<&str>, path: &str) -> Result<()> {
    if let Some(p) = pin {
        match &a.fn_hash {
            Some(h) if h != p => return env_err(path, "the fn_hash echo is not the pin"),
            None if !matches!(a.outcome, AnswerOutcome::RateLimited | AnswerOutcome::BadRequest) => {
                return env_err(path, "an answer to a pinned request echoes the pin");
            }
            _ => {}
        }
    }
    Ok(())
}

/// 5.5 step 6: trust against the receipt (and the host asked, when known), then `fn_hash`
/// against the receipt's `cell.fn_hash`, then `superseded_by`.
fn trust_check(a: &Answer, r: &Receipt, mesh: Option<&str>, at: &str) -> Result<()> {
    let p = |m: &str| format!("{at}.{m}");
    let primary = r.signer_host() == lexical::host_of(&r.cell().ns) && r.inputs().is_empty();
    let t = a.trust.unwrap_or(TrustLabel::Unverified);
    if (t == TrustLabel::Primary) != primary {
        return env_err(
            &p("trust"),
            if primary { "a primary receipt is labelled primary" } else { "the receipt is not primary" },
        );
    }
    if let Some(host) = mesh {
        match t {
            TrustLabel::Computed if r.signer_host() != host => {
                return env_err(&p("trust"), "computed: the receipt is signed by another host than the one asked");
            }
            TrustLabel::Verified | TrustLabel::Unverified if r.signer_host() == host => {
                return env_err(&p("trust"), "the mesh asked signed this receipt: its trust is computed");
            }
            _ => {}
        }
    }
    if a.fn_hash.as_deref().is_some_and(|h| h != r.cell().fn_hash) {
        return env_err(&p("fn_hash"), "fn_hash differs from the receipt's cell.fn_hash");
    }
    if a.superseded_by.as_deref() == Some(r.hash()) {
        return env_err(&p("superseded_by"), "a receipt cannot supersede itself");
    }
    Ok(())
}

fn check_pending(status: u16, m: &[(String, Value)], ctx: &Ctx<'_>) -> Result<Answer> {
    // 2.
    schema::exact(m, &["estimate_seconds", "job", "outcome", "poll", "request"], &[], "$", ErrorCode::Envelope)?;
    let estimate = integer_member(schema::get(m, "estimate_seconds"), "$.estimate_seconds")?;
    let Some(job) = schema::get(m, "job").as_str() else { return env_err("$.job", "job must be a string") };
    let Some(poll) = schema::get(m, "poll").as_str() else { return env_err("$.poll", "poll must be a string") };
    check_job_and_poll(job, poll)?;
    let rm = schema::object_or(schema::get(m, "request"), "$.request", ErrorCode::Envelope, "request")?;
    schema::exact(rm, &["cell_id"], &["asof", "fn_hash"], "$.request", ErrorCode::Envelope)?;
    let cell_id = schema::string_or(rm, "cell_id", "$.request", ErrorCode::Envelope)?;
    let opt = |name: &str| -> Result<Option<&str>> {
        match schema::opt(rm, name) {
            None => Ok(None),
            Some(_) => schema::string_or(rm, name, "$.request", ErrorCode::Envelope).map(Some),
        }
    };
    let invalid = |e: crate::InvalidRequest| Error::new(ErrorCode::Envelope, "$.request", e.to_string());
    let mut echoed = Request::new(cell_id, opt("asof")?).map_err(invalid)?;
    if let Some(p) = opt("fn_hash")? {
        echoed = echoed.with_pin(p).map_err(invalid)?;
    }
    // 3.
    if status != Pending::STATUS {
        return env_err("$.outcome", format!("status {status} does not match outcome pending"));
    }
    // 5.
    let Ctx::Cell(request) = ctx else { return env_err("$.outcome", "pending answers a cell request") };
    if echoed.cell_id != request.cell_id || echoed.asof != request.asof || echoed.pin != request.pin {
        return env_err("$.request", "the echoed request is not the request made");
    }
    if let Some(polled) = &request.job {
        if job != polled {
            return env_err("$.job", "the envelope is for another job than the one polled");
        }
    }
    // 6. poll on the host asked.
    if let Some(host) = &request.mesh {
        if url_host(poll) != Some(host.as_str()) {
            return env_err("$.poll", "poll is on another host than the one asked");
        }
    }
    let p = Pending::new(job, estimate, poll, echoed)?;
    Ok(Answer { pending: Some(p), ..bare(AnswerOutcome::Pending) })
}

/// The key of a member: its values of the slice's open (and ranged) arguments, by name.
fn key_of(r: &Receipt, names: &[String]) -> Vec<Scalar> {
    names.iter().filter_map(|n| r.cell().args.get(n).and_then(Arg::scalar).cloned()).collect()
}

fn cmp_keys(a: &[Scalar], b: &[Scalar]) -> core::cmp::Ordering {
    for (x, y) in a.iter().zip(b) {
        let o = if x.ty() == y.ty() { cmp_lexical(x.ty(), x.lexical(), y.lexical()) } else { x.ty().cmp(&y.ty()) };
        if o != core::cmp::Ordering::Equal {
            return o;
        }
    }
    a.len().cmp(&b.len())
}

/// 5.6: a page bound to its request, the page first, then each member completely.
fn check_page(m: &[(String, Value)], mut a: Answer, keys: &Keys, req: &SliceRequest) -> Result<Answer> {
    let Value::Array(items) = schema::get(m, "members") else {
        return env_err("$.members", "members is an array of envelopes");
    };
    let slice = Slice::from_value(schema::get(m, "slice")).map_err(|e| e.recode(ErrorCode::Envelope))?;
    let next = schema::opt(m, "next").and_then(Value::as_str).map(String::from);
    // 2. The page: the slice, its fn_hash, next, the cursor.
    if slice.id() != req.slice_id {
        return env_err("$.slice", "the page is of another slice than the one requested");
    }
    if a.fn_hash.as_deref().is_some_and(|h| h != slice.fn_hash()) {
        return env_err("$.fn_hash", "fn_hash differs from the slice's");
    }
    if let (Some(host), Some(n)) = (&req.mesh, &next) {
        if url_host(n) != Some(host.as_str()) {
            return env_err("$.next", "next is on another host than the one asked");
        }
    }
    // The cursor on the open arguments only; other names are ignored.
    let names = slice.key_names();
    let mut last: Option<Vec<Scalar>> = match &req.after {
        None => None,
        Some(after) => {
            let key: Vec<Scalar> = names.iter().filter_map(|n| after.get(n).cloned()).collect();
            if key.len() != names.len() {
                return env_err("$", "the cursor lacks an open argument of the slice");
            }
            Some(key)
        }
    };
    // 3. Each member, completely, in order.
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let p = format!("$.members[{i}]");
        let mm = match item {
            Value::Object(mm) => mm,
            _ => return env_err(&p, "a member is an envelope"),
        };
        let o = match schema::get(mm, "outcome").as_str().and_then(AnswerOutcome::from_name) {
            Some(o @ (AnswerOutcome::Ok | AnswerOutcome::Stale)) => o,
            _ => return env_err(&p, "a member is an ok or stale envelope"),
        };
        let mut ma = check_members(mm, o, &Ctx::None, false).map_err(|e| nested(e, &p))?;
        let r = member_receipt(schema::get(mm, "receipt"), keys, &format!("{p}.receipt"))?;
        if !slice.contains(&r) {
            return env_err(&p, "the member's cell is not a member of the slice");
        }
        if let Some(asked) = &req.asof {
            if r.asof() > asked.as_str() {
                return env_err(&format!("{p}.receipt.asof"), "the member's asof is after the requested asof");
            }
        }
        trust_check(&ma, &r, req.mesh.as_deref(), &p)?;
        let key = key_of(&r, &names);
        if let Some(prev) = &last {
            if key.len() != prev.len() || key.iter().zip(prev).any(|(x, y)| x.ty() != y.ty()) {
                return env_err(&p, "the member's key has other types than the previous key (or the cursor)");
            }
            if cmp_keys(&key, prev) != core::cmp::Ordering::Greater {
                return env_err(&p, "members are in strictly ascending key order, after the cursor");
            }
        }
        last = Some(key);
        ma.receipt = Some(r);
        out.push(ma);
    }
    a.page = Some(SlicePage { slice, members: out, next });
    Ok(a)
}
