//! Key documents with declarations (spec-0.4.3): arbitrary bytes as a key document of a fixed
//! host, through KeyDocument and the 0.3.x KeyDoc, and as a bare declarations value. Must never
//! panic; a document accepted with declarations names only functions under its host, has 1 to
//! 64 names of 1 to 16 entries, round-trips through to_value, and its keys are KeyDoc's.
#![no_main]

use datumesh_core::json;
use datumesh_core::{is_unrooted, Declarations, KeyDoc, KeyDocument};
use libfuzzer_sys::fuzz_target;

const HOST: &str = "rates.example.org";

fuzz_target!(|data: &[u8]| {
    let doc = KeyDocument::from_json(HOST, data);
    let old = KeyDoc::from_json(HOST, data);
    assert_eq!(doc.as_ref().ok().map(|d| &d.keys), old.as_ref().ok());
    if let Ok(d) = &doc {
        assert_eq!(d.host(), HOST);
        if let Some(decl) = &d.declarations {
            assert!((1..=64).contains(&decl.len()));
            for (name, entries) in decl.iter() {
                assert!(name.starts_with("rates.example.org/"));
                assert!((1..=16).contains(&entries.len()));
                for e in entries {
                    assert!(e.valid_to.as_deref().is_none_or(|t| t > e.valid_from.as_str()));
                }
            }
            assert_eq!(Declarations::from_value(&decl.to_value()).as_ref(), Ok(decl));
            assert_eq!(KeyDocument::new(d.keys.clone(), Some(decl.clone())).as_ref(), Ok(d));
        }
    }
    if let Ok(v) = json::parse(data) {
        if let Ok(decl) = Declarations::from_value(&v) {
            assert_eq!(Declarations::from_value(&decl.to_value()), Ok(decl));
        }
    }
    if let Ok(s) = core::str::from_utf8(data) {
        let _ = is_unrooted(s);
    }
});
