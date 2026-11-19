//! spec-0.6.1 (f78773a): every new vector of the kinds this library runs is run through the public
//! API and compared with its expected result: `cell`, `uri` (valid and invalid), `jcs`, `function`,
//! `define`, `describe` (with `publisher`). The `receipt`, `envelope` and `class` vectors of the
//! release are run by the all-vector tests of those kinds (`spec_0_3_0`, `spec_0_5_0_classes`), which
//! read every vector file, so they are checked there too. `formula` and `formula-eval` are the
//! formula library's and stay skipped; the same holds for `call`.

mod common;

use std::collections::BTreeMap;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{
    check_define, check_describe, check_describe_with, format_reference, https_url, parse_reference, sha256_hex, Cell,
    ClassDescriptor, Error, FunctionAnswer, FunctionDescriptor,
};

/// The 142 vectors spec-0.6.1 added, as `(kind, id)`, by `git diff --name-only --diff-filter=A
/// 941d9b5 spec-0.6.1 -- vectors` (every one of them is new; no vector of 941d9b5 changed).
const NEW: [(&str, &str); 142] = [
    ("cell", "cell/dns-segment-host-hyphen"),
    ("cell", "cell/dns-segment-hyphen"),
    ("cell", "cell/dns-segment-max-253"),
    ("cell", "cell/dns-segment-max-512"),
    ("cell", "cell/dns-segment-punycode"),
    ("cell", "cell/dns-segment-relay-ecb"),
    ("cell", "cell/dns-segment-three-labels"),
    ("cell", "cell/dns-segment-two-dns-segments"),
    ("cell", "invalid/cell-dns-segment-all-digit-last-label"),
    ("cell", "invalid/cell-dns-segment-empty-label"),
    ("cell", "invalid/cell-dns-segment-hyphen-without-dot"),
    ("cell", "invalid/cell-dns-segment-inner-trailing-hyphen"),
    ("cell", "invalid/cell-dns-segment-label-64"),
    ("cell", "invalid/cell-dns-segment-last-label-hyphen"),
    ("cell", "invalid/cell-dns-segment-last-segment"),
    ("cell", "invalid/cell-dns-segment-leading-dot"),
    ("cell", "invalid/cell-dns-segment-leading-hyphen"),
    ("cell", "invalid/cell-dns-segment-name-65"),
    ("cell", "invalid/cell-dns-segment-segment-254"),
    ("cell", "invalid/cell-dns-segment-too-long"),
    ("cell", "invalid/cell-dns-segment-trailing-dot"),
    ("cell", "invalid/cell-dns-segment-u-label"),
    ("cell", "invalid/cell-dns-segment-underscore"),
    ("cell", "invalid/cell-dns-segment-uppercase"),
    ("class", "class/dns-segment-name"),
    ("class", "class/dns-segment-steward"),
    ("class", "invalid/class-dns-segment-last"),
    ("class", "invalid/class-dns-segment-method-name"),
    ("class", "invalid/class-dns-segment-steward-uppercase"),
    ("define", "define/dns-segment-host-hyphen"),
    ("define", "define/dns-segment-hyphen"),
    ("define", "define/dns-segment-invalid-segment-rooted-reads-rooted"),
    ("define", "define/dns-segment-invalid-segment-unrooted-reads-rooted"),
    ("define", "define/dns-segment-max-253"),
    ("define", "define/dns-segment-max-512"),
    ("define", "define/dns-segment-punycode"),
    ("define", "define/dns-segment-relay-ecb"),
    ("define", "define/dns-segment-three-labels"),
    ("define", "define/dns-segment-two-dns-segments"),
    ("define", "invalid/define-dns-segment-all-digit-last-label"),
    ("define", "invalid/define-dns-segment-empty-label"),
    ("define", "invalid/define-dns-segment-hyphen-without-dot"),
    ("define", "invalid/define-dns-segment-inner-trailing-hyphen"),
    ("define", "invalid/define-dns-segment-invalid-segment-reads-unrooted"),
    ("define", "invalid/define-dns-segment-label-64"),
    ("define", "invalid/define-dns-segment-last-label-hyphen"),
    ("define", "invalid/define-dns-segment-last-segment"),
    ("define", "invalid/define-dns-segment-leading-dot"),
    ("define", "invalid/define-dns-segment-leading-hyphen"),
    ("define", "invalid/define-dns-segment-name-65"),
    ("define", "invalid/define-dns-segment-rooted-reads-unrooted-host"),
    ("define", "invalid/define-dns-segment-segment-254"),
    ("define", "invalid/define-dns-segment-too-long"),
    ("define", "invalid/define-dns-segment-trailing-dot"),
    ("define", "invalid/define-dns-segment-u-label"),
    ("define", "invalid/define-dns-segment-underscore"),
    ("define", "invalid/define-dns-segment-uppercase"),
    ("describe", "describe/class-dns-segment-publisher"),
    ("describe", "describe/class-dns-segment-without-publisher"),
    ("describe", "describe/class-publisher"),
    ("describe", "describe/class-without-publisher"),
    ("describe", "describe/function-counterexamples"),
    ("describe", "describe/function-publisher"),
    ("describe", "describe/function-relay-publisher-source"),
    ("describe", "describe/function-relay-without-publisher"),
    ("describe", "describe/function-without-publisher"),
    ("describe", "invalid/describe-class-publisher-other-host"),
    ("describe", "invalid/describe-class-publisher-segment"),
    ("describe", "invalid/describe-function-name-dns-last"),
    ("describe", "invalid/describe-function-name-uppercase-segment"),
    ("describe", "invalid/describe-object-publisher"),
    ("describe", "invalid/describe-publisher-is-segment"),
    ("describe", "invalid/describe-publisher-namespace"),
    ("describe", "invalid/describe-publisher-not-string"),
    ("describe", "invalid/describe-publisher-other-host"),
    ("describe", "invalid/describe-publisher-uppercase"),
    ("envelope", "envelope/dns-segment-label-only"),
    ("envelope", "envelope/dns-segment-primary"),
    ("envelope", "envelope/dns-segment-relayed"),
    ("envelope", "envelope/test-case-counterexample"),
    ("envelope", "envelope/test-case-primary"),
    ("envelope", "invalid/envelope-dns-segment-host-not-primary"),
    ("envelope", "invalid/envelope-dns-segment-label-primary"),
    ("envelope", "invalid/envelope-dns-segment-relayed-as-primary"),
    ("envelope", "invalid/envelope-test-case-computed"),
    ("formula", "formula/dns-segment-digit-hyphen-host"),
    ("formula", "formula/dns-segment-divide"),
    ("formula", "formula/dns-segment-method-call"),
    ("formula", "formula/dns-segment-minus"),
    ("formula", "formula/dns-segment-minus-left"),
    ("formula", "formula/dns-segment-minus-spaced"),
    ("formula", "formula/dns-segment-punycode"),
    ("formula", "formula/dns-segment-two-segments"),
    ("formula", "invalid/formula-dns-segment-all-digit-last-label"),
    ("formula", "invalid/formula-dns-segment-digit-hyphen-host-unbound"),
    ("formula", "invalid/formula-dns-segment-empty-label"),
    ("formula", "invalid/formula-dns-segment-hyphen-without-dot"),
    ("formula", "invalid/formula-dns-segment-last-before-paren"),
    ("formula", "invalid/formula-dns-segment-leading-hyphen"),
    ("formula", "invalid/formula-dns-segment-trailing-dot"),
    ("formula", "invalid/formula-dns-segment-underscore"),
    ("formula", "invalid/formula-dns-segment-uppercase"),
    ("formula-eval", "formula-eval/dns-segment-divide"),
    ("formula-eval", "formula-eval/dns-segment-minus"),
    ("formula-eval", "formula-eval/dns-segment-minus-left"),
    ("function", "function/inputs-dns-segment"),
    ("function", "function/test-case"),
    ("function", "invalid/function-inputs-dns-segment-last"),
    ("function", "invalid/function-inputs-dns-segment-uppercase"),
    ("jcs", "jcs/test-case-id"),
    ("receipt", "invalid/receipt-dns-segment-publisher-trailing-dot"),
    ("receipt", "invalid/receipt-dns-segment-uppercase"),
    ("receipt", "receipt/dns-segment-invalid-rooted"),
    ("receipt", "receipt/dns-segment-relay"),
    ("receipt", "receipt/dns-segment-stored"),
    ("receipt", "receipt/test-case-counterexample"),
    ("receipt", "receipt/test-case-true"),
    ("uri", "invalid/uri-dns-segment-all-digit-last-label"),
    ("uri", "invalid/uri-dns-segment-empty-label"),
    ("uri", "invalid/uri-dns-segment-hyphen-without-dot"),
    ("uri", "invalid/uri-dns-segment-inner-trailing-hyphen"),
    ("uri", "invalid/uri-dns-segment-label-64"),
    ("uri", "invalid/uri-dns-segment-last-label-hyphen"),
    ("uri", "invalid/uri-dns-segment-last-segment"),
    ("uri", "invalid/uri-dns-segment-leading-dot"),
    ("uri", "invalid/uri-dns-segment-leading-hyphen"),
    ("uri", "invalid/uri-dns-segment-name-65"),
    ("uri", "invalid/uri-dns-segment-segment-254"),
    ("uri", "invalid/uri-dns-segment-too-long"),
    ("uri", "invalid/uri-dns-segment-trailing-dot"),
    ("uri", "invalid/uri-dns-segment-u-label"),
    ("uri", "invalid/uri-dns-segment-underscore"),
    ("uri", "invalid/uri-dns-segment-uppercase"),
    ("uri", "uri/dns-segment-host-hyphen"),
    ("uri", "uri/dns-segment-hyphen"),
    ("uri", "uri/dns-segment-max-253"),
    ("uri", "uri/dns-segment-max-512"),
    ("uri", "uri/dns-segment-punycode"),
    ("uri", "uri/dns-segment-relay-ecb"),
    ("uri", "uri/dns-segment-three-labels"),
    ("uri", "uri/dns-segment-two-dns-segments"),
    ("uri", "uri/test-case"),
];

fn is_new(v: &Value) -> bool {
    NEW.iter().any(|(_, id)| *id == str_of(v, "id"))
}

/// The new vectors of a kind: the valid ones of its directory and the invalid ones of `invalid/`.
fn new_vectors(kind: &str) -> Vec<Value> {
    let all: Vec<Value> = vectors_of_kind(kind).into_iter().filter(is_new).collect();
    let want = NEW.iter().filter(|(k, _)| *k == kind).count();
    assert_eq!(all.len(), want, "the new {kind} vectors are all there (spec submodule at f78773a)");
    all
}

fn is_valid_vector(v: &Value) -> bool {
    v.get("valid") == Some(&Value::Bool(true))
}

fn assert_all(kind: &str, problems: Vec<String>) {
    assert!(problems.is_empty(), "{} new {kind} vectors fail:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn the_new_vectors_are_all_there() {
    assert_eq!(NEW.len(), 142);
    let mut ids: Vec<&str> = NEW.iter().map(|(_, id)| *id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 142, "no id twice");
    let skipped = NEW.iter().filter(|(k, _)| matches!(*k, "formula" | "formula-eval")).count();
    assert_eq!(skipped, 20, "17 formula and 3 formula-eval vectors are the formula library's");
}

#[test]
fn the_new_cell_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("cell") {
        let id = str_of(&v, "id");
        let got = Cell::from_json(str_of(&v, "input").as_bytes());
        match (is_valid_vector(&v), got) {
            (true, Ok(c)) => {
                if String::from_utf8_lossy(&c.canonical()) != str_of(&v, "canonical") {
                    problems.push(format!("{id}: canonical differs"));
                }
                if c.id() != str_of(&v, "sha256") {
                    problems.push(format!("{id}: sha256 differs"));
                }
            }
            (true, Err(e)) => problems.push(format!("{id}: expected success, got {e}")),
            (false, Err(e)) if e.code_str() == str_of(&v, "error") => {}
            (false, other) => problems.push(format!("{id}: expected {}, got {other:?}", str_of(&v, "error"))),
        }
    }
    assert_all("cell", problems);
}

#[test]
fn the_new_uri_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("uri") {
        let id = str_of(&v, "id");
        let sig = v.get("signature").cloned().unwrap_or(Value::Null);
        let got = parse_reference(str_of(&v, "input"), &sig);
        match (is_valid_vector(&v), got) {
            (true, Ok(r)) => {
                if format_reference(&r) != str_of(&v, "input") {
                    problems.push(format!("{id}: does not format back"));
                }
                let https = https_url("mesh.example", str_of(&v, "input"));
                if https.ok().as_deref() != Some(str_of(&v, "https")) {
                    problems.push(format!("{id}: https differs"));
                }
            }
            (true, Err(e)) => problems.push(format!("{id}: expected success, got {e}")),
            (false, Err(e)) if e.code_str() == str_of(&v, "error") => {}
            (false, other) => problems.push(format!("{id}: expected {}, got {:?}", str_of(&v, "error"), other.err())),
        }
    }
    assert_all("uri", problems);
}

#[test]
fn the_new_jcs_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("jcs") {
        let id = str_of(&v, "id");
        let parsed = json::parse(str_of(&v, "input").as_bytes()).unwrap();
        let canonical = parsed.to_jcs();
        if String::from_utf8_lossy(&canonical) != str_of(&v, "canonical") {
            problems.push(format!("{id}: canonical differs"));
        }
        if sha256_hex(&canonical) != str_of(&v, "sha256") {
            problems.push(format!("{id}: sha256 differs"));
        }
    }
    assert_all("jcs", problems);
}

#[test]
fn the_new_function_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("function") {
        let id = str_of(&v, "id");
        match (is_valid_vector(&v), FunctionDescriptor::from_json(str_of(&v, "input").as_bytes())) {
            (true, Ok(f)) => {
                if String::from_utf8_lossy(&f.canonical()) != str_of(&v, "canonical") {
                    problems.push(format!("{id}: canonical differs"));
                }
                if f.hash() != str_of(&v, "sha256") {
                    problems.push(format!("{id}: sha256 differs"));
                }
            }
            (true, Err(e)) => problems.push(format!("{id}: expected success, got {e}")),
            (false, Err(e)) if e.code_str() == str_of(&v, "error") => {}
            (false, other) => problems.push(format!("{id}: expected {}, got {:?}", str_of(&v, "error"), other.err())),
        }
    }
    assert_all("function", problems);
}

#[test]
fn the_new_define_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("define") {
        let id = str_of(&v, "id");
        match (is_valid_vector(&v), check_define(str_of(&v, "name"), str_of(&v, "input").as_bytes())) {
            (true, Ok(d)) => {
                if String::from_utf8_lossy(&d.canonical()) != str_of(&v, "canonical") {
                    problems.push(format!("{id}: canonical differs"));
                }
                if d.hash() != str_of(&v, "sha256") {
                    problems.push(format!("{id}: sha256 differs"));
                }
            }
            (true, Err(e)) => problems.push(format!("{id}: expected success, got {e}")),
            (false, Err(e)) if e.code_str() == str_of(&v, "error") => {}
            (false, other) => problems.push(format!("{id}: expected {}, got {:?}", str_of(&v, "error"), other.err())),
        }
    }
    assert_all("define", problems);
}

fn held_of(v: &Value) -> BTreeMap<String, ClassDescriptor> {
    let mut held = BTreeMap::new();
    for (name, d) in v.get("classes").and_then(Value::as_object).unwrap_or(&[]) {
        held.insert(name.clone(), ClassDescriptor::from_value(d).unwrap());
    }
    held
}

#[test]
fn the_new_describe_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("describe") {
        let functions: Vec<FunctionDescriptor> = match v.get("functions") {
            Some(Value::Array(a)) => a.iter().map(|f| FunctionDescriptor::from_value(f).unwrap()).collect(),
            _ => Vec::new(),
        };
        let input = str_of(&v, "input").as_bytes();
        let got: Result<(), Error> = check_describe_with(input, &held_of(&v), &functions).map(|_| ());
        let id = str_of(&v, "id");
        match (is_valid_vector(&v), got) {
            (true, Ok(())) => {}
            (true, Err(e)) => problems.push(format!("{id}: expected success, got {e}")),
            (false, Err(e)) if e.code_str() == str_of(&v, "error") => {}
            (false, other) => problems.push(format!("{id}: expected {}, got {other:?}", str_of(&v, "error"))),
        }
    }
    assert_all("describe", problems);
}

#[test]
fn a_function_answer_carries_its_publisher() {
    // `publisher` is the host of the name's namespace, exactly (16.8); a domain of a later segment is not.
    let v = vector_by_id("function/inputs-dns-segment");
    let d = FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
    let name = "datumesh.com/relay/ecb.europa.eu/fx_reference";
    let answer = FunctionAnswer::new(name, &d).unwrap();
    assert!(answer.to_value().get("publisher").is_none(), "publisher is optional");
    for bad in ["ecb.europa.eu", "datumesh.com/relay", "Datumesh.com", "datumesh.com."] {
        assert_eq!(answer.clone().with_publisher(bad).unwrap_err().code_str(), "E_DESCRIBE", "{bad}");
    }
    let ok = answer.with_publisher("datumesh.com").unwrap();
    assert_eq!(ok.to_value().get("publisher").and_then(Value::as_str), Some("datumesh.com"));
    let got = check_describe(&ok.to_json(), &BTreeMap::new()).unwrap();
    assert_eq!(got.publisher.as_deref(), Some("datumesh.com"));
    assert_eq!(got.function.as_deref(), Some(name));
}
