//! `datumesh-cli`: runs Datumesh test vectors (SPEC 11) against `datumesh-core`.
//!
//! * `datumesh-cli run-vectors`: one vector object per stdin line; one result line per input:
//!   `{"id": …, "error": <code or null>, …outputs}`.
//! * `datumesh-cli check [DIR]`: checks every vector file under DIR (default `spec/vectors`),
//!   prints `FAIL <file>` lines with details and `<passed>/<total> vectors pass`; exits 1 on
//!   any failure.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use datumesh_cli::{check, run_vectors};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run-vectors") => run_vectors(),
        Some("check") => check(Path::new(args.get(2).map(String::as_str).unwrap_or("spec/vectors"))),
        _ => {
            eprintln!("usage: datumesh-cli run-vectors < vectors.jsonl\n       datumesh-cli check [DIR]");
            ExitCode::from(2)
        }
    }
}
