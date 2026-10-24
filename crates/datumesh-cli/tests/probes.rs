//! Oracle probes for spec-0.4.0: 168 vectors written for this crate (`tools/probes/`) where
//! the text leaves an order of checks or a corner to decide, each with the result of the
//! reference tool (`canon.py check`, run as a black box) at 18be70f: slice objects and
//! manifests (range objects, version slices, check order), descriptors with `order` and
//! `extent` (check order, type orders, integer range ends), receipts citing slices (manifest
//! parsing, membership with numeric ranges and open arguments, duplicate cells, member
//! `asof`, the `tx` rule) and `E_INPUT_SUPERSEDED` (which lineages show a replaced input).
//! Regenerate with `tools/probes/run.sh <scratch dir>`.

use std::path::Path;

use datumesh_core::json;

#[test]
fn every_probe_gives_the_reference_tool_result() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes-0.4.0.jsonl")).unwrap();
    let mut failures = Vec::new();
    let mut n = 0;
    for line in text.lines() {
        n += 1;
        let v = json::parse(line.as_bytes()).unwrap();
        let problems = datumesh_cli::check_vector(&v);
        if !problems.is_empty() {
            let id = v.get("id").and_then(json::Value::as_str).unwrap_or("?").to_string();
            failures.push(format!("{id}: {}", problems.join("; ")));
        }
    }
    assert_eq!(n, 168);
    assert!(failures.is_empty(), "{} of {n} probes differ:\n{}", failures.len(), failures.join("\n"));
}
