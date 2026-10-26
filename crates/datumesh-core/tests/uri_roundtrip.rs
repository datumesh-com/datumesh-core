//! Test 4: format(parse(uri)) == uri for every valid URI vector, and the HTTPS mapping is
//! reversible.

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{cell_uri_from_https, format_reference, format_uri, https_url, parse_reference, parse_uri};

#[test]
fn every_valid_uri_round_trips() {
    let mut n = 0;
    for v in vectors("uri") {
        let uri = str_of(&v, "input");
        let sig = v.get("signature").cloned().unwrap_or(Value::Null);
        // spec-0.4.2: a pinned URI reads as a Reference (parse_uri's CellRef has no pin).
        let r = parse_reference(uri, &sig).unwrap_or_else(|e| panic!("{uri}: {e}"));
        let again = format_reference(&r);
        assert_eq!(again, uri);
        assert_eq!(parse_reference(&again, &sig).unwrap(), r);
        if let Some(c) = r.to_cell_ref() {
            assert_eq!(format_uri(&c), uri);
            assert_eq!(parse_uri(uri, &sig).unwrap(), c);
        }
        let https = https_url("mesh.example", uri).unwrap();
        assert_eq!(https, str_of(&v, "https"));
        assert_eq!(cell_uri_from_https("mesh.example", &https).as_deref(), Some(uri));
        n += 1;
    }
    assert!(n >= 11);
}
