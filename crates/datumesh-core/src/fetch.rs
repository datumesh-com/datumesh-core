//! Key retrieval over HTTPS (9.3), only with the `fetch` feature. [`crate::verify`] never
//! calls this: retrieval is a separate, explicit operation (9.4).

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
/// network, DNS, TLS or timeout failure is `E_KEY_UNAVAILABLE`. `host` must be a DNS name
/// (`E_CANON_NAME`).
pub fn fetch_keys(host: &str) -> Result<KeyDoc> {
    lexical::check_dns_name(host, "host")?;
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
