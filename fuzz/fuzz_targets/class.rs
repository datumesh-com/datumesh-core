//! Class descriptors (16.1, 16.2, `spec-0.5.0`): arbitrary bytes as a class descriptor, through
//! `ClassDescriptor::from_json` and as the descriptor of `define_class` under an empty
//! environment. Must never panic. A descriptor accepted reads back from its canonical bytes as
//! itself, its class hash is 64 lowercase hex digits, and defining it under the namespace of its
//! name never panics.
#![no_main]

use datumesh_core::{define_class, ClassDescriptor, ClassEnv};
use libfuzzer_sys::fuzz_target;

const NAME: &str = "rates.example.org/fx_pair";

fuzz_target!(|data: &[u8]| {
    if let Ok(d) = ClassDescriptor::from_json(data) {
        let canonical = d.canonical();
        let back = ClassDescriptor::from_json(&canonical).expect("the canonical bytes are a descriptor");
        assert_eq!(back, d);
        assert_eq!(back.canonical(), canonical);
        let hash = d.hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    }
    let env = ClassEnv::new("rates.example.org");
    if let Ok(flat) = define_class(NAME, data, &env) {
        assert_eq!(ClassDescriptor::from_json(&flat.canonical()).as_ref(), Ok(&flat));
    }
});
