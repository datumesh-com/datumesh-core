//! Strict JSON parser and JCS: never panics; JCS output parses back to the same JCS.
#![no_main]

use datumesh_core::json;
use datumesh_core::{canonicalize_cell, canonicalize_descriptor, canonicalize_ref, canonicalize_scalar};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = json::parse(data) {
        let c = v.to_jcs();
        let again = json::parse(&c).expect("JCS output is strict JSON");
        assert_eq!(again.to_jcs(), c, "JCS is idempotent");
    }
    for f in [canonicalize_scalar, canonicalize_cell, canonicalize_ref, canonicalize_descriptor] {
        if let Ok(c) = f(data) {
            assert_eq!(f(&c).as_ref(), Ok(&c), "canonical form is a fixed point");
        }
    }
});
