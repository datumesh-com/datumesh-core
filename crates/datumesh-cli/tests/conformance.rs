//! Test 1: every vector in spec/vectors passes (valid: all outputs equal; invalid: exact code).

use std::path::{Path, PathBuf};

use datumesh_core::json;

fn vectors() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    let mut files = Vec::new();
    datumesh_cli::collect(&dir, &mut files).expect("spec/vectors is readable (git submodule update --init)");
    files.sort();
    files
}

#[test]
fn every_vector_passes() {
    let files = vectors();
    assert!(files.len() >= 365, "expected at least 365 vectors, found {}", files.len());
    let mut failures = Vec::new();
    for f in &files {
        let v = json::parse(&std::fs::read(f).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let problems = datumesh_cli::check_vector(&v);
        if !problems.is_empty() {
            failures.push(format!("{}: {}", f.display(), problems.join("; ")));
        }
    }
    assert!(failures.is_empty(), "{} of {} vectors fail:\n{}", failures.len(), files.len(), failures.join("\n"));
}

#[test]
fn run_line_output_shape() {
    // Exactly the SPEC 11 output fields: id, error, then the kind's outputs.
    let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/keyid/rates-2020.json");
    let v = json::parse(&std::fs::read(f).unwrap()).unwrap();
    let out = datumesh_cli::run(&v).unwrap();
    let names: Vec<&str> = out.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["key_id"]);
}
