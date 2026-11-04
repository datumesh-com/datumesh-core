//! Licence inheritance (spec-0.4.3, 8.5): arbitrary input licences, declarations and issued
//! instants through inherit_licence. Must never panic; the result agrees with the rule's
//! invariants (a licence given is the mesh's for no inputs, else one of the inputs; refusals
//! only with inputs; the reason's kind matches the outcome), and repeating or reordering the
//! inputs changes nothing.
#![no_main]

use datumesh_core::{inherit_licence, Declaration, Declarations, LicenceReason};
use libfuzzer_sys::fuzz_target;

const POOL: [&str; 12] = [
    "CC0-1.0",
    "PDDL-1.0",
    "MIT",
    "Apache-2.0",
    "CC-BY-4.0",
    "OGL-UK-3.0",
    "CC-BY-SA-4.0",
    "ODbL-1.0",
    "CC-BY-NC-4.0",
    "https://r.example.org/l",
    "cc0-1.0",
    "https://r.example.org/m",
];
const FN: &str = "calc.example.net/convert";
const TIMES: [&str; 4] = [
    "2020-01-01T00:00:00.000000Z",
    "2022-01-01T00:00:00.000000Z",
    "2024-01-01T00:00:00.000000Z",
    "2026-01-01T00:00:00.000000Z",
];

fuzz_target!(|data: &[u8]| {
    let Some((&head, rest)) = data.split_first() else { return };
    let ndecl = (head & 7) as usize;
    let issued = TIMES[(head >> 3) as usize % 4];
    let (decl_bytes, lic_bytes) = rest.split_at(rest.len().min(ndecl * 2));
    let mut d = Declarations::new();
    for pair in decl_bytes.chunks(2) {
        let lic = POOL[pair[0] as usize % POOL.len()];
        let b = *pair.get(1).unwrap_or(&0);
        let from = TIMES[b as usize % 4];
        let to = TIMES.iter().copied().find(|t| *t > from && (b >> 2) & 1 == 1);
        let name = if b >> 7 == 1 { "calc.example.net/other" } else { FN };
        let _ = d.insert(name, Declaration::new(lic, from, to).expect("valid entry"));
    }
    let licences: Vec<&str> = lic_bytes.iter().take(64).map(|b| POOL[*b as usize % POOL.len()]).collect();
    let r = inherit_licence(&licences, Some(&d), FN, issued, "MIT");
    assert_eq!(r.refused(), !r.reason.entitled());
    assert_ne!(r.reason, LicenceReason::LicenceMismatch);
    match &r.licence {
        Some(l) if licences.is_empty() => assert_eq!(l, "MIT"),
        Some(l) => assert!(licences.contains(&l.as_str())),
        None => assert!(!licences.is_empty()),
    }
    let mut again: Vec<&str> = licences.iter().rev().copied().collect();
    again.extend(licences.iter().copied());
    assert_eq!(inherit_licence(&again, Some(&d), FN, issued, "MIT"), r);
});
