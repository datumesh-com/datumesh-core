//! Key retrieval over HTTPS (9.3), only with the `fetch` feature. [`crate::verify`] never
//! calls this: retrieval is a separate, explicit operation (9.4).
//!
//! Deprecated since 0.3.1 and removed at 1.0: an HTTP client is the application's choice
//! (its TLS roots, proxies, timeouts, async runtime), so the crate keeps only the part that is
//! protocol, [`keys_from_response`], which turns any client's response into keys.

use std::time::Duration;

use crate::error::{err, ErrorCode, Result};
use crate::keys::{keys_from_response, KeyDoc};
use crate::lexical;

/// The largest key document read: 64 keys fit in far less.
const MAX_BODY: u64 = 256 * 1024;

/// Fetches `https://<host>/.well-known/datumesh.json` and checks it (9.2, 9.3).
///
/// Certificates are validated (Web PKI roots), no redirect is followed, and the request
/// times out after 10 seconds. A 200 with a valid document gives the keys; a 200 with an
/// invalid document is `E_KEY_DOC`; every other status (redirects included) and every
/// network, DNS, TLS or timeout failure is `E_KEY_UNAVAILABLE`. A host that is not a DNS
/// name is never fetched: it has no keys (`E_KEY_UNAVAILABLE`, 9.3).
///
/// Deprecated since 0.3.1; the `fetch` feature and this function are removed at 1.0. Fetch
/// `https://<host>/.well-known/datumesh.json` with your own HTTP client (no redirects,
/// validated certificates, a bounded body) and pass the status and body to
/// [`keys_from_response`], or record a network failure with
/// [`Keys::insert_unavailable`](crate::Keys::insert_unavailable).
#[deprecated(
    since = "0.3.1",
    note = "removed at 1.0: fetch the key document with your own HTTP client and pass the response to `keys_from_response`"
)]
pub fn fetch_keys(host: &str) -> Result<KeyDoc> {
    if lexical::check_dns_name(host, "host").is_err() {
        return err(ErrorCode::KeyUnavailable, "$", format!("{host:?} is not a DNS name: no keys are resolved for it"));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let url = format!("https://{host}/.well-known/datumesh.json");
    let mut resp = match agent.get(&url).call() {
        Ok(r) => r,
        Err(e) => return err(ErrorCode::KeyUnavailable, "$", format!("{url}: {e}")),
    };
    let status = resp.status().as_u16();
    if status != 200 {
        return keys_from_response(host, status, None);
    }
    match resp.body_mut().with_config().limit(MAX_BODY).read_to_vec() {
        Ok(body) => keys_from_response(host, status, Some(&body)),
        Err(e) => err(ErrorCode::KeyUnavailable, "$", format!("{url}: {e}")),
    }
}
