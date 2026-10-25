//! `datumesh-cli run-vectors` output is byte for byte the expected output for every vector.
//!
//! `run-vectors-0.2.1.jsonl` was captured from the 0.2.1 binary, fed every file under
//! `spec/vectors` at spec-0.2.1 (sorted by path) as one compact JSON line each. For
//! spec-0.3.0, `run-vectors-0.3.0.jsonl` was derived from the vector files' expected results
//! alone (`id`, `error`, and the kind's output members the vector gives, `code` included, as
//! one JCS line); the same derivation reproduces `run-vectors-0.2.1.jsonl` byte for byte from
//! the spec-0.2.1 vectors. The lines of the 339 old vectors are unchanged except
//! `invalid/envelope-wrong-asof`, whose expected result spec-0.3.0 changed. For spec-0.4.0
//! and spec-0.4.1 the files are derived by `tools/gen_golden.py`: a vector of a kind for
//! formula evaluators (`formula`, `formula-eval`, `excel`, `decimal`) gives
//! `{"error":null,"id":…,"skipped":<kind>}`, and a valid `mesh` vector's `bindings` and
//! `endpoints` are derived from its mesh document, unknown endpoints dropped (the vectors do
//! not carry them, DEFECTS.md RS-38). Each revision's file keeps the previous one's lines
//! unchanged. The crosscheck harness depends on this interface, so any change to it must be
//! deliberate.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use datumesh_core::json;

#[test]
fn run_vectors_output_is_the_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    let mut files = Vec::new();
    datumesh_cli::collect(&dir, &mut files).expect("spec/vectors is readable (git submodule update --init)");
    files.sort();
    let mut input = String::new();
    for f in &files {
        let v = json::parse(&std::fs::read(f).unwrap()).unwrap();
        input.push_str(&datumesh_cli::line(&v));
        input.push('\n');
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_datumesh-cli"))
        .arg("run-vectors")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // Written from another thread: the output is read while the input is still being sent,
    // so neither pipe can fill up and block the other.
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    assert!(out.status.success());
    let want = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/run-vectors-0.4.1.jsonl")).unwrap();
    let got = String::from_utf8(out.stdout).unwrap();
    let want = String::from_utf8(want).unwrap();
    assert_eq!(got.lines().count(), files.len());
    for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
        assert_eq!(g, w, "line {} ({}) differs", i + 1, files[i].display());
    }
    assert_eq!(got, want);
}
