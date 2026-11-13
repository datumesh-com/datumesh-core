//! Key normalisation (16.4, `spec-0.5.0`): arbitrary text checked under each rule of the closed
//! list, chosen by the first byte. Must never panic. A value accepted is non-empty and NFC under
//! every rule but `none`, and checking it twice gives the same answer (the check is pure).
#![no_main]

use datumesh_core::KeyNormalisation;
use libfuzzer_sys::fuzz_target;
use unicode_normalization::is_nfc;

fuzz_target!(|data: &[u8]| {
    let Some((&which, rest)) = data.split_first() else { return };
    let rules = KeyNormalisation::ALL;
    let rule = rules[usize::from(which) % rules.len()];
    let text = String::from_utf8_lossy(rest);
    let first = rule.check(&text);
    assert_eq!(first, rule.check(&text));
    if first.is_ok() && rule != KeyNormalisation::NoRule {
        assert!(!text.is_empty());
        assert!(is_nfc(&text));
    }
});
