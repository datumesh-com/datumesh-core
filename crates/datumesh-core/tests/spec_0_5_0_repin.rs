//! The re-pin of spec-0.5.0 from `7bed002` to `6d5633e` ("clarifications from the verifier
//! libraries", section 14, points 1 to 8 of `CONTRACT-0.5.0-libs-repin.md`), through the library API.
//! 40 vectors are new, none changed. Each module is one point:
//!
//! * [`point1`]: `E_KEY_NORMALISATION` and `E_KEY_ENCODING` are `bad_request` codes (5.5, 5.8);
//! * [`point2`]: the exact list of codes an `error` envelope may not carry (5.5);
//! * [`point3`]: 16.4: 3.5 first, `none` and the empty string, `nfc_strict`'s White_Space list;
//! * [`point4`]: an unknown rule, an empty `classes` (16.1, 7);
//! * [`point5`]: the describe check, six steps (16.8), with optional function descriptors;
//! * [`point6`]: a class-typed key in the object JSON, nested object ids (16.5);
//! * [`point7`]: a key encoding is at most 1024 bytes as a whole, every decoding failure is
//!   `E_KEY_ENCODING` (16.5);
//! * [`point8`]: the short id of a malformed id is `E_CANON_HEX` (16.8).
//!
//! The vector-driven loops of the other test files (`every_*_vector`) also meet the new vectors.
//! Tests that fail on the implementation of `7bed002` are listed in `IMPL-NOTES-0.3.8.md`;
//! the others pin behaviour the re-pin states more exactly. `check_describe_with` is a stub
//! (`todo!()`): the new public function for the optional function descriptors of step 6.

mod common;

use std::collections::BTreeMap;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{
    check_answer, check_describe, check_describe_with, parse_object_key, short_id, Answer, AnswerOutcome,
    ClassDescriptor, DescribeKind, ErrorCode, Fault, FunctionDescriptor, KeyNormalisation, ObjectKey, Request,
};

fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    match v.get(k) {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

fn held_of(v: &Value) -> BTreeMap<String, ClassDescriptor> {
    let mut held = BTreeMap::new();
    for (name, d) in v.get("classes").and_then(Value::as_object).unwrap_or(&[]) {
        held.insert(name.clone(), ClassDescriptor::from_value(d).unwrap());
    }
    held
}

fn functions_of(v: &Value) -> Vec<FunctionDescriptor> {
    array(v, "functions").iter().map(|f| FunctionDescriptor::from_value(f).unwrap()).collect()
}

fn check(status: u16, body: &str) -> datumesh_core::Result<Answer> {
    let request = Request::new(&"f".repeat(64), None).unwrap();
    check_answer(status, body.as_bytes(), &datumesh_core::Keys::new(), &request)
}

fn code_of(c: ErrorCode) -> &'static str {
    c.as_str()
}

// ---------------------------------------------------------------------------------------------
// Point 1: E_KEY_NORMALISATION and E_KEY_ENCODING are bad_request codes
// ---------------------------------------------------------------------------------------------

mod point1 {
    use super::*;

    const KEY_CODES: [ErrorCode; 2] = [ErrorCode::KeyNormalisation, ErrorCode::KeyEncoding];

    #[test]
    fn a_bad_request_carries_them() {
        for c in KEY_CODES {
            let body = format!(r#"{{"error":"{}","outcome":"bad_request"}}"#, code_of(c));
            let a = check(400, &body).unwrap_or_else(|e| panic!("{c}: {e}"));
            assert_eq!((a.outcome, a.error), (AnswerOutcome::BadRequest, Some(c)));
            // The builder writes it; the checker reads the builder's output back.
            let built = Answer::bad_request(c).unwrap_or_else(|e| panic!("{c}: {e}"));
            assert_eq!(built.status(), 400);
            assert_eq!(String::from_utf8(built.to_json()).unwrap(), body);
        }
    }

    #[test]
    fn the_vectors_say_so() {
        for (id, c) in [
            ("envelope/bad-request-key-encoding", ErrorCode::KeyEncoding),
            ("envelope/bad-request-key-normalisation", ErrorCode::KeyNormalisation),
        ] {
            let v = vector_by_id(id);
            let built = Answer::bad_request(c).unwrap();
            assert_eq!(String::from_utf8(built.to_json()).unwrap(), jcs(&parse(str_of(&v, "input"))), "{id}");
            let a = check(400, str_of(&v, "input")).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(a.error, Some(c), "{id}");
        }
    }

    #[test]
    fn no_other_outcome_carries_them() {
        for c in KEY_CODES {
            for (status, outcome) in [(500, "error"), (404, "not_covered"), (403, "unlicensed")] {
                let body = format!(r#"{{"error":"{}","outcome":"{outcome}"}}"#, code_of(c));
                assert_eq!(check(status, &body).unwrap_err().code(), ErrorCode::Envelope, "{c} in {outcome}");
            }
            assert_eq!(Answer::error(c).unwrap_err().code(), ErrorCode::Envelope, "{c}");
            assert!(Answer::not_covered().with_error(c).is_err(), "{c}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Point 2: the exact list of codes an `error` envelope may not carry
// ---------------------------------------------------------------------------------------------

mod point2 {
    use super::*;

    /// 5.5: "any code in 10.2 except exactly these".
    const REFUSED: [&str; 18] = [
        // the client's codes
        "E_ENVELOPE",
        "E_DESCRIBE",
        // the request codes
        "E_JOB_UNKNOWN",
        "E_SLICE_AT_CELL",
        "E_NAME_UNBOUND",
        "E_SHORT_ID_AMBIGUOUS",
        "E_KEY_NORMALISATION",
        "E_KEY_ENCODING",
        // the define and write codes
        "E_UNROOTED_INPUT",
        "E_NAMESPACE_SUSPENDED",
        "E_CLASS_MEMBER",
        "E_CLASS_PARENT",
        "E_CLASS_INHERIT",
        "E_CLASS_OVERRIDE",
        "E_CLASS_FINAL",
        "E_CLASS_METHOD",
        "E_CLASS_INTERFACE",
        "E_CLASS_OWNER",
    ];

    #[test]
    fn an_error_envelope_carries_every_code_of_10_2_but_exactly_these() {
        let mut allowed = 0;
        for c in ErrorCode::ALL {
            let s = code_of(*c);
            if *c == ErrorCode::ForeignUnavailable {
                // 502 with a `detail` of its own form: the vectors of spec-0.4.3 carry it.
                allowed += 1;
                continue;
            }
            let (status, body) = (500, format!(r#"{{"error":"{s}","outcome":"error"}}"#));
            let got = check(status, &body);
            if REFUSED.contains(&s) {
                assert_eq!(got.unwrap_err().code(), ErrorCode::Envelope, "{s} is not an error envelope's");
                if *c != ErrorCode::Envelope {
                    assert_eq!(Answer::error(*c).unwrap_err().code(), ErrorCode::Envelope, "builder: {s}");
                }
            } else {
                allowed += 1;
                let a = got.unwrap_or_else(|e| panic!("{s} is an error envelope's: {e}"));
                assert_eq!(a.error, Some(*c), "{s}");
                assert!(Answer::error(*c).is_ok(), "builder: {s}");
            }
        }
        assert_eq!(allowed, ErrorCode::ALL.len() - REFUSED.len());
    }

    #[test]
    fn the_refused_list_is_in_10_2() {
        for s in REFUSED {
            assert!(ErrorCode::from_code(s).is_some(), "{s}");
        }
    }

    #[test]
    fn the_class_codes_are_ten_with_the_two_key_codes() {
        // 10.2: ten codes are marked "class": the eight E_CLASS_*, E_KEY_NORMALISATION, E_KEY_ENCODING;
        // E_SHORT_ID_AMBIGUOUS comes before E_DESCRIBE.
        let all: Vec<&str> = ErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        let at = |s: &str| all.iter().position(|c| *c == s).unwrap();
        assert_eq!(at("E_CLASS_MEMBER") + 7, at("E_CLASS_OWNER"));
        assert_eq!(at("E_CLASS_OWNER") + 1, at("E_KEY_NORMALISATION"));
        assert_eq!(at("E_KEY_NORMALISATION") + 1, at("E_KEY_ENCODING"));
        assert_eq!(at("E_SHORT_ID_AMBIGUOUS") + 1, at("E_DESCRIBE"));
    }

    #[test]
    fn a_fault_and_a_suspension_stay_a_not_covered_only() {
        assert!(check(404, r#"{"fault":"E_FORMULA_KEY","outcome":"not_covered"}"#).is_ok());
        assert!(Answer::not_covered()
            .with_fault(Fault::DispatchKey)
            .with_error(ErrorCode::NamespaceSuspended)
            .is_err());
        assert_eq!(
            check(500, r#"{"error":"E_NAMESPACE_SUSPENDED","outcome":"error"}"#).unwrap_err().code(),
            ErrorCode::Envelope
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Point 3: 16.4
// ---------------------------------------------------------------------------------------------

mod point3 {
    use super::*;
    use KeyNormalisation::*;

    fn code(rule: KeyNormalisation, s: &str) -> Option<ErrorCode> {
        rule.check(s).err().map(|e| e.code())
    }

    #[test]
    fn every_rule_first_checks_a_string_value_by_3_5() {
        // A tab, a line feed, NUL and the other controls; not NFC; unassigned in Unicode 15.1.
        // 3.5's code, never E_KEY_NORMALISATION, under every rule (the vectors check `none`).
        let not_strings = [
            "a\tb",
            "a\nb",
            "a\rb",
            "a\u{0}b",
            "a\u{B}b",
            "a\u{C}b",
            "a\u{7F}b",
            "a\u{85}b",
            "e\u{0301}",  // NFD
            "\u{2000}",   // not NFC (U+2002)
            "\u{2001}",   // not NFC (U+2003)
            "a\u{0378}b", // unassigned
        ];
        for rule in KeyNormalisation::ALL {
            for s in not_strings {
                let got = code(*rule, s);
                assert_eq!(got, Some(ErrorCode::CanonString), "{rule} {s:?}");
            }
        }
    }

    #[test]
    fn a_string_value_is_at_most_1024_bytes_with_3_5s_code() {
        for rule in KeyNormalisation::ALL {
            let long = "A".repeat(1025);
            assert_eq!(code(*rule, &long), Some(ErrorCode::CanonLimit), "{rule}");
        }
        for rule in [NoRule, UpperAscii, NfcStrict] {
            assert_eq!(code(rule, &"A".repeat(1024)), None, "{rule}");
        }
        // 1024 bytes, not characters: 513 two-byte characters are 1026 bytes.
        assert_eq!(code(NoRule, &"\u{E9}".repeat(513)), Some(ErrorCode::CanonLimit));
        assert_eq!(code(NoRule, &"\u{E9}".repeat(512)), None);
    }

    #[test]
    fn none_accepts_the_empty_string_and_every_other_rule_refuses_it() {
        assert_eq!(code(NoRule, ""), None);
        for rule in KeyNormalisation::ALL.iter().filter(|r| **r != NoRule) {
            assert_eq!(code(*rule, ""), Some(ErrorCode::KeyNormalisation), "{rule}");
        }
        // The vectors of the re-pin.
        assert_eq!(str_of(&vector_by_id("normalisation/none-empty"), "text"), "");
        assert_eq!(str_of(&vector_by_id("invalid/normalisation-upper-ascii-empty"), "error"), "E_KEY_NORMALISATION");
    }

    #[test]
    fn none_accepts_what_3_5_accepts() {
        for s in [" ", "  a  ", "a\u{00A0}b", "a\u{200D}b", "\u{FEFF}", "Mixed Case", "日本語"] {
            assert_eq!(code(NoRule, s), None, "{s:?}");
        }
    }

    #[test]
    fn nfc_strict_refuses_every_white_space_character_but_u0020() {
        // Unicode 15.1 White_Space: U+0009-U+000D, U+0020, U+0085, U+00A0, U+1680, U+2000-U+200A,
        // U+2028, U+2029, U+202F, U+205F, U+3000. The controls, U+2000 and U+2001 are refused by 3.5
        // first; the rest by the rule itself.
        let by_rule: Vec<char> = ['\u{A0}', '\u{1680}']
            .into_iter()
            .chain('\u{2002}'..='\u{200A}')
            .chain(['\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}'])
            .collect();
        assert_eq!(by_rule.len(), 2 + 9 + 5);
        for c in by_rule {
            for s in [format!("a{c}b"), format!("{c}a"), format!("a{c}"), c.to_string()] {
                assert_eq!(code(NfcStrict, &s), Some(ErrorCode::KeyNormalisation), "U+{:04X} in {s:?}", c as u32);
            }
        }
        for c in ['\u{9}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{85}', '\u{2000}', '\u{2001}'] {
            assert_eq!(code(NfcStrict, &format!("a{c}b")), Some(ErrorCode::CanonString), "U+{:04X}", c as u32);
        }
        // U+0020 itself: one space inside is fine; leading, trailing or repeated is not.
        assert_eq!(code(NfcStrict, "a b"), None);
        for s in [" a", "a ", "a  b", " "] {
            assert_eq!(code(NfcStrict, s), Some(ErrorCode::KeyNormalisation), "{s:?}");
        }
        // The vector.
        let v = vector_by_id("invalid/normalisation-nfc-strict-nbsp");
        assert_eq!(code(NfcStrict, str_of(&v, "text")), Some(ErrorCode::KeyNormalisation));
    }

    #[test]
    fn the_new_normalisation_vectors() {
        for id in [
            "normalisation/none-empty",
            "invalid/normalisation-none-tab",
            "invalid/normalisation-none-newline",
            "invalid/normalisation-none-nul",
            "invalid/normalisation-none-nfd",
            "invalid/normalisation-nfc-strict-nbsp",
            "invalid/normalisation-upper-ascii-empty",
        ] {
            let v = vector_by_id(id);
            let rule = KeyNormalisation::from_name(str_of(&v, "rule")).unwrap();
            let got = rule.check(str_of(&v, "text"));
            if is_valid(&v) {
                got.unwrap_or_else(|e| panic!("{id}: {e}"));
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(&v, "error"), "{id}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Point 4: an unknown rule; an empty `classes`
// ---------------------------------------------------------------------------------------------

mod point4 {
    use super::*;

    fn fx_input() -> Value {
        parse(str_of(&vector_by_id("class/fx-pair"), "input"))
    }

    fn function() -> Value {
        parse(str_of(&vector_by_id("function/argument-key-normalisation"), "input"))
    }

    const ISIN: [&str; 4] = ["signature", "args", "isin", "key_normalisation"];

    #[test]
    fn an_unknown_rule_in_a_class_key_is_e_class_member() {
        for rule in ["bogus", "None", "upper", "", "UPPER_ASCII"] {
            let d = edit(
                &fx_input(),
                &["keys", "base"],
                Some(parse(&format!(r#"{{"type":"string","key_normalisation":"{rule}"}}"#))),
            );
            assert_eq!(ClassDescriptor::from_json(&d.to_jcs()).unwrap_err().code(), ErrorCode::ClassMember, "{rule:?}");
        }
        // `none` is a rule there.
        let d = edit(&fx_input(), &["keys", "base"], Some(parse(r#"{"type":"string","key_normalisation":"none"}"#)));
        assert!(ClassDescriptor::from_json(&d.to_jcs()).is_ok());
    }

    #[test]
    fn an_unknown_rule_on_a_function_argument_is_e_schema() {
        for rule in ["bogus", "none", "None", "", "UPPER_ASCII"] {
            let d = edit(&function(), &ISIN, Some(text(rule)));
            assert_eq!(FunctionDescriptor::from_json(&d.to_jcs()).unwrap_err().code(), ErrorCode::Schema, "{rule:?}");
        }
    }

    #[test]
    fn the_structure_is_checked_first_so_a_number_argument_says_e_schema_for_none_and_unknown() {
        let number = |rule: &str| {
            edit(
                &function(),
                &["signature", "args", "date"],
                Some(parse(&format!(r#"{{"type":"number","unit":"1","valid":false,"key_normalisation":"{rule}"}}"#))),
            )
        };
        for rule in ["none", "bogus"] {
            assert_eq!(
                FunctionDescriptor::from_json(&number(rule).to_jcs()).unwrap_err().code(),
                ErrorCode::Schema,
                "{rule}"
            );
        }
        // A real rule of 16.4 on a declaration that may not carry one is E_CANON_TYPE.
        assert_eq!(
            FunctionDescriptor::from_json(&number("upper_ascii").to_jcs()).unwrap_err().code(),
            ErrorCode::CanonType
        );
        let with_classes = edit(
            &function(),
            &["signature", "args", "isin"],
            Some(parse(
                r#"{"type":"string","valid":false,"classes":["a.example.org/c"],"key_normalisation":"upper_ascii"}"#,
            )),
        );
        assert_eq!(FunctionDescriptor::from_json(&with_classes.to_jcs()).unwrap_err().code(), ErrorCode::CanonType);
    }

    #[test]
    fn an_empty_classes_is_e_canon_set_in_an_argument_and_in_a_return() {
        let arg = edit(
            &function(),
            &["signature", "args", "isin"],
            Some(parse(r#"{"type":"string","valid":false,"classes":[]}"#)),
        );
        assert_eq!(FunctionDescriptor::from_json(&arg.to_jcs()).unwrap_err().code(), ErrorCode::CanonSet);
        let ret = edit(&function(), &["signature", "returns"], Some(parse(r#"{"type":"string","classes":[]}"#)));
        assert_eq!(FunctionDescriptor::from_json(&ret.to_jcs()).unwrap_err().code(), ErrorCode::CanonSet);
        let ok = edit(
            &function(),
            &["signature", "returns"],
            Some(parse(r#"{"type":"string","classes":["a.example.org/c"]}"#)),
        );
        assert!(FunctionDescriptor::from_json(&ok.to_jcs()).is_ok());
    }

    #[test]
    fn an_empty_classes_in_a_class_descriptor_is_e_canon_set() {
        // 7 and 16.1: `implements`, `open`, `maps` and the like are sets too; an empty set is E_CANON_SET.
        // (The members whose empty form is allowed are absent when empty.) Pinned by the vectors:
        for id in ["invalid/function-classes-empty", "invalid/function-returns-classes-empty"] {
            let v = vector_by_id(id);
            assert_eq!(
                FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap_err().code().as_str(),
                str_of(&v, "error"),
                "{id}"
            );
        }
        for id in ["invalid/function-key-normalisation-none-number", "invalid/function-key-normalisation-unknown"] {
            let v = vector_by_id(id);
            assert_eq!(
                FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap_err().code().as_str(),
                str_of(&v, "error"),
                "{id}"
            );
        }
        let v = vector_by_id("invalid/class-lookup-keys-only");
        assert_eq!(str_of(&v, "error"), "E_CLASS_MEMBER");
    }
}

// ---------------------------------------------------------------------------------------------
// Point 5: the describe check (16.8), six steps
// ---------------------------------------------------------------------------------------------

mod point5 {
    use super::*;

    fn class_functions() -> Value {
        vector_by_id("describe/class-functions")
    }

    fn answer(v: &Value) -> Value {
        parse(str_of(v, "input"))
    }

    fn with(v: &Value, answer: &Value) -> datumesh_core::Result<datumesh_core::Described> {
        check_describe_with(&answer.to_jcs(), &held_of(v), &functions_of(v))
    }

    fn without(v: &Value, answer: &Value) -> datumesh_core::Result<datumesh_core::Described> {
        check_describe(&answer.to_jcs(), &held_of(v))
    }

    #[test]
    fn step_1_parse_codes_are_their_own() {
        let v = vector_by_id("describe/class");
        for (id, want) in [
            ("invalid/describe-not-json", ErrorCode::ParseJson),
            ("invalid/describe-duplicate-member", ErrorCode::ParseDuplicateKey),
        ] {
            let bad = vector_by_id(id);
            assert_eq!(str_of(&bad, "error"), want.as_str(), "{id}");
            let got = check_describe(str_of(&bad, "input").as_bytes(), &held_of(&bad)).unwrap_err();
            assert_eq!(got.code(), want, "{id}");
        }
        // Not JSON text at all; trailing text; a duplicate member deeper down.
        for (text, want) in [
            ("", ErrorCode::ParseJson),
            ("{", ErrorCode::ParseJson),
            ("{} x", ErrorCode::ParseJson),
            (r#"{"kind":"class","kind":"class"}"#, ErrorCode::ParseDuplicateKey),
        ] {
            assert_eq!(check_describe(text.as_bytes(), &held_of(&v)).unwrap_err().code(), want, "{text:?}");
        }
        // Valid JSON of the wrong shape is E_DESCRIBE.
        assert_eq!(check_describe(b"[]", &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
        // The same through the function with function descriptors (a parse failure comes first).
        assert_eq!(check_describe_with(b"{", &held_of(&v), &[]).unwrap_err().code(), ErrorCode::ParseJson);
    }

    #[test]
    fn step_2_a_description_is_1_to_1024_bytes() {
        let v = vector_by_id("describe/class");
        let a = answer(&v);
        let set = |d: Value| edit(&a, &["methods", "spot", "description"], Some(d));
        assert_eq!(check_describe(&set(text("")).to_jcs(), &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
        assert!(check_describe(&set(text("x")).to_jcs(), &held_of(&v)).is_ok());
        assert!(check_describe(&set(text(&"x".repeat(1024))).to_jcs(), &held_of(&v)).is_ok());
        assert_eq!(
            check_describe(&set(text(&"x".repeat(1025))).to_jcs(), &held_of(&v)).unwrap_err().code(),
            ErrorCode::Describe
        );
        // Valid by 3.5: no control character.
        assert_eq!(check_describe(&set(text("a\tb")).to_jcs(), &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
        let e = vector_by_id("invalid/describe-description-empty");
        assert_eq!(
            check_describe(str_of(&e, "input").as_bytes(), &held_of(&e)).unwrap_err().code(),
            ErrorCode::Describe
        );
    }

    #[test]
    fn step_2_same_as_is_ascending_and_unique() {
        let v = vector_by_id("describe/object");
        let a = answer(&v);
        let one = r#""bible.example.org/passage(osis:\"Gen.1.1\")""#;
        let two = r#""bible.example.org/passage(osis:\"Gen.1.2\")""#;
        let set = |s: &str| edit(&a, &["same_as"], Some(parse(s)));
        assert!(check_describe(&set(&format!("[{one},{two}]")).to_jcs(), &held_of(&v)).is_ok());
        for bad in [format!("[{two},{one}]"), format!("[{one},{one}]"), "[]".to_string()] {
            assert_eq!(
                check_describe(&set(&bad).to_jcs(), &held_of(&v)).unwrap_err().code(),
                ErrorCode::Describe,
                "{bad}"
            );
        }
        for id in ["invalid/describe-same-as-duplicate", "invalid/describe-same-as-unsorted"] {
            let e = vector_by_id(id);
            assert_eq!(
                check_describe(str_of(&e, "input").as_bytes(), &held_of(&e)).unwrap_err().code(),
                ErrorCode::Describe,
                "{id}"
            );
        }
    }

    #[test]
    fn an_objects_methods_are_unique_and_ascending() {
        let v = vector_by_id("describe/object");
        let a = answer(&v);
        let held = held_of(&v);
        for bad in [r#"["text","text"]"#, r#"["text","consonantal"]"#] {
            let m = edit(&a, &["methods"], Some(parse(bad)));
            assert_eq!(check_describe(&m.to_jcs(), &held).unwrap_err().code(), ErrorCode::Describe, "{bad}");
        }
    }

    #[test]
    fn step_4_the_held_class_is_found_by_class_hash_and_never_by_name() {
        let v = vector_by_id("describe/object");
        let held = held_of(&v);
        let name = str_of(answer(&v).get("object").unwrap(), "class").to_string();
        let class = held[&name].clone();
        // The same class held under another name is still the held class: the object is checked
        // against it (invalid/describe-object-key-normalisation fails under any name).
        let bad = vector_by_id("invalid/describe-object-key-normalisation");
        let renamed: BTreeMap<String, ClassDescriptor> = held_of(&bad)
            .into_iter()
            .map(|(n, c)| (format!("renamed.example.org/{}", n.rsplit('/').next().unwrap()), c))
            .collect();
        assert_eq!(check_describe(str_of(&bad, "input").as_bytes(), &renamed).unwrap_err().code(), ErrorCode::Describe);
        // And a different class held under the answer's own name is not the held class: nothing
        // is held for this answer, so only steps 1 to 3 apply (describe/object-not-held).
        let fx = held_of(&vector_by_id("describe/class")).into_values().next().unwrap();
        assert_ne!(fx.hash(), class.hash());
        let impostor = BTreeMap::from([(name, fx)]);
        let got = check_describe(str_of(&v, "input").as_bytes(), &impostor).unwrap();
        assert_eq!(got.kind, DescribeKind::Object);
        assert_eq!(got.class_hash, class.hash());
    }

    #[test]
    fn step_4_a_client_holding_nothing_stops_after_step_3() {
        // describe/object-not-held: valid with the class not held.
        let v = vector_by_id("describe/object-not-held");
        let got = check_describe(str_of(&v, "input").as_bytes(), &held_of(&v)).unwrap();
        assert_eq!(got.kind, DescribeKind::Object);
        let none = BTreeMap::new();
        for id in ["describe/class", "describe/object", "describe/object-class-typed-key", "describe/class-functions"] {
            let v = vector_by_id(id);
            assert!(check_describe(str_of(&v, "input").as_bytes(), &none).is_ok(), "{id}");
        }
        // The consistency of step 3 needs nothing held: an id, a short id, a call form, a class hash
        // or a method table that does not follow is E_DESCRIBE with nothing held too.
        let o = vector_by_id("describe/object");
        let a = answer(&o);
        for bad in [
            edit(&a, &["object_id"], Some(text(&"ab".repeat(32)))),
            edit(&a, &["short_id"], Some(text("abababababab"))),
            edit(&a, &["call"], Some(text("scripture.example.org/verse()"))),
        ] {
            assert_eq!(check_describe(&bad.to_jcs(), &none).unwrap_err().code(), ErrorCode::Describe);
        }
        let e = vector_by_id("invalid/describe-object-not-held-call");
        assert_eq!(
            check_describe(str_of(&e, "input").as_bytes(), &held_of(&e)).unwrap_err().code(),
            ErrorCode::Describe
        );
        let c = vector_by_id("describe/class");
        let ca = answer(&c);
        let bad_hash = edit(&ca, &["class_hash"], Some(text(&"ab".repeat(32))));
        assert_eq!(check_describe(&bad_hash.to_jcs(), &none).unwrap_err().code(), ErrorCode::Describe);
    }

    #[test]
    fn step_5_an_object_is_checked_against_the_held_class() {
        // Key names, types, a string key's normalisation (vectors), a class-typed key's class.
        for id in [
            "invalid/describe-object-key-names",
            "invalid/describe-object-key-type",
            "invalid/describe-object-key-normalisation",
        ] {
            let e = vector_by_id(id);
            assert_eq!(
                check_describe(str_of(&e, "input").as_bytes(), &held_of(&e)).unwrap_err().code(),
                ErrorCode::Describe,
                "{id}"
            );
        }
        let v = vector_by_id("describe/object-class-typed-key");
        let got = check_describe(str_of(&v, "input").as_bytes(), &held_of(&v)).unwrap();
        assert_eq!(got.class, "rates.example.org/ccy_pair");
        assert_eq!(got.object_id.as_deref(), Some("fcb2180650816aab6614bc0fd855fdec3301e41eab5a6bbcea0970f3cb7a590a"));
    }

    #[test]
    fn step_6_args_and_returns_are_checked_only_when_function_descriptors_are_held() {
        // describe/class-args-unchecked: wrong args pass when nothing is held.
        let v = vector_by_id("describe/class-args-unchecked");
        assert!(check_describe(str_of(&v, "input").as_bytes(), &held_of(&v)).is_ok());
        assert!(check_describe_with(str_of(&v, "input").as_bytes(), &held_of(&v), &[]).is_ok());
        // describe/class-functions: held functions, matching args and returns.
        let v = class_functions();
        let fns = functions_of(&v);
        assert!(fns.len() >= 2);
        assert!(with(&v, &answer(&v)).is_ok());
        assert!(without(&v, &answer(&v)).is_ok());
        // A wrong args or returns passes without functions and fails with them (E_DESCRIBE).
        let a = answer(&v);
        let bad_args = edit(&a, &["methods", "rate", "args", "date", "valid"], Some(Value::Bool(false)));
        let extra_arg =
            edit(&a, &["methods", "rate", "args", "tenor"], Some(parse(r#"{"type":"string","valid":false}"#)));
        let missing_arg = edit(&a, &["methods", "rate", "args", "date"], None);
        let bad_returns = edit(&a, &["methods", "rate", "returns", "type"], Some(text("string")));
        for (what, bad) in
            [("args", bad_args), ("extra arg", extra_arg), ("missing arg", missing_arg), ("returns", bad_returns)]
        {
            assert!(without(&v, &bad).is_ok(), "{what}: unchecked when nothing is held");
            assert_eq!(with(&v, &bad).unwrap_err().code(), ErrorCode::Describe, "{what}");
        }
        // The class keys are not among `args`: the method's args are its function's other arguments.
        let keys_in_args =
            edit(&a, &["methods", "rate", "args", "base"], Some(parse(r#"{"type":"string","valid":false}"#)));
        assert_eq!(with(&v, &keys_in_args).unwrap_err().code(), ErrorCode::Describe);
        // The vectors.
        for id in ["invalid/describe-class-args", "invalid/describe-class-returns"] {
            let e = vector_by_id(id);
            assert!(!functions_of(&e).is_empty(), "{id} carries functions");
            let got = check_describe_with(str_of(&e, "input").as_bytes(), &held_of(&e), &functions_of(&e));
            assert_eq!(got.unwrap_err().code(), ErrorCode::Describe, "{id}");
        }
    }

    #[test]
    fn step_6_a_method_whose_function_is_not_held_is_unchecked() {
        let v = class_functions();
        let a = answer(&v);
        let all = functions_of(&v);
        let bad = edit(&a, &["methods", "rate", "args", "date", "valid"], Some(Value::Bool(false)));
        // Hold every function but the one `rate` points to: `rate` is unchecked, the rest are checked.
        let rate_hash = str_of(a.get("methods").unwrap().get("rate").unwrap(), "fn_hash").to_string();
        let others: Vec<FunctionDescriptor> = all.iter().filter(|f| f.hash() != rate_hash).cloned().collect();
        assert!(others.len() < all.len(), "rate's function is among the held ones");
        assert!(check_describe_with(&bad.to_jcs(), &held_of(&v), &others).is_ok());
        let bad_spot = edit(&a, &["methods", "spot", "args", "date", "valid"], Some(Value::Bool(false)));
        assert_eq!(
            check_describe_with(&bad_spot.to_jcs(), &held_of(&v), &others).unwrap_err().code(),
            ErrorCode::Describe
        );
    }

    #[test]
    fn the_steps_report_the_first_failure() {
        // A wrong description (step 2) and a wrong args (step 6) at once: step 2 is reported, and
        // a parse failure (step 1) before either. All E_DESCRIBE but step 1; the stub of
        // check_describe_with must not panic first.
        let v = class_functions();
        let a = answer(&v);
        let both = edit(
            &edit(&a, &["methods", "rate", "description"], Some(text(""))),
            &["methods", "rate", "args", "date", "valid"],
            Some(Value::Bool(false)),
        );
        assert_eq!(with(&v, &both).unwrap_err().code(), ErrorCode::Describe);
        // With the description right, the args alone make it fail; with the args right, it passes.
        let args_only = edit(&a, &["methods", "rate", "args", "date", "valid"], Some(Value::Bool(false)));
        assert_eq!(with(&v, &args_only).unwrap_err().code(), ErrorCode::Describe);
        assert!(with(&v, &a).is_ok());
    }
}

// ---------------------------------------------------------------------------------------------
// Point 6: a class-typed key in the object JSON; nested object ids
// ---------------------------------------------------------------------------------------------

mod point6 {
    use super::*;

    #[test]
    fn a_key_vector_given_class_hash_has_its_object_and_object_id() {
        let mut seen = 0;
        for v in vectors_of_kind("key").into_iter().filter(|v| v.get("class_hash").is_some()) {
            seen += 1;
            let id = str_of(&v, "id");
            let key = parse_object_key(str_of(&v, "input")).unwrap_or_else(|e| panic!("{id}: {e}"));
            let class_hash = str_of(&v, "class_hash");
            if is_valid(&v) {
                let object = key.object_value(class_hash).unwrap();
                assert_eq!(jcs(&object), jcs(v.get("object").unwrap()), "{id}: object");
                assert_eq!(key.object_id(class_hash).unwrap(), str_of(&v, "object_id"), "{id}: object_id");
            }
        }
        assert!(seen >= 1);
    }

    #[test]
    fn nested_object_ids_are_defined() {
        // key/nested-object: the typed string scalar of each nested key's encoding; the id follows.
        let v = vector_by_id("key/nested-object");
        let key = parse_object_key(str_of(&v, "input")).unwrap();
        let hash = str_of(&v, "class_hash");
        let object = key.object_value(hash).unwrap();
        let base = object.get("keys").and_then(|k| k.get("base")).unwrap();
        assert_eq!(jcs(base), r#"{"type":"string","value":"iso.example.org/currency(code:\"EUR\")"}"#);
        assert_eq!(key.object_id(hash).unwrap(), "fcb2180650816aab6614bc0fd855fdec3301e41eab5a6bbcea0970f3cb7a590a");
        // The decoded form keeps the nested objects: it is the parse, not the object.
        assert_eq!(jcs(&key.to_value()), jcs(v.get("decoded").unwrap()));
        // The object's call form quotes the nested encoding, doubling its quotes.
        assert_eq!(
            key.call_form(),
            r#"rates.example.org/ccy_pair(base: "iso.example.org/currency(code:""EUR"")", quote: "iso.example.org/currency(code:""USD"")")"#
        );
    }

    #[test]
    fn the_object_id_depends_on_the_nested_encoding_and_the_class_hash() {
        let a = parse_object_key(r#"a.example.org/o(x:b.example.org/m(y:1))"#).unwrap();
        let b = parse_object_key(r#"a.example.org/o(x:b.example.org/m(y:2))"#).unwrap();
        let (h1, h2) = ("ab".repeat(32), "cd".repeat(32));
        assert_ne!(a.object_id(&h1).unwrap(), b.object_id(&h1).unwrap());
        assert_ne!(a.object_id(&h1).unwrap(), a.object_id(&h2).unwrap());
        // The id is the id of the JSON with the nested key as a typed string.
        let flat = parse_object_key(r#"a.example.org/o(x:"b.example.org/m(y:1)")"#);
        assert!(flat.is_ok(), "a string key holding the same text");
        assert_eq!(
            jcs(&a.object_value(&h1).unwrap()),
            jcs(&flat.unwrap().object_value(&h1).unwrap()),
            "the object JSON does not tell a class-typed key from a string holding its encoding"
        );
    }

    #[test]
    fn a_describe_answer_holds_a_class_typed_key_as_its_encoding() {
        let v = vector_by_id("describe/object-class-typed-key");
        let a = parse(str_of(&v, "input"));
        let base = a.get("object").and_then(|o| o.get("keys")).and_then(|k| k.get("base")).unwrap();
        assert_eq!(str_of(base, "type"), "string");
        assert!(check_describe(&a.to_jcs(), &held_of(&v)).is_ok());
        // A nested object in place of the typed scalar is not the object JSON.
        let nested = edit(
            &a,
            &["object", "keys", "base"],
            Some(parse(r#"{"class":"iso.example.org/currency","keys":{"code":{"type":"string","value":"EUR"}}}"#)),
        );
        assert_eq!(check_describe(&nested.to_jcs(), &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
        // The call form quotes the encoding; the unquoted form is not the call form.
        let unquoted = edit(
            &a,
            &["call"],
            Some(text(
                r#"rates.example.org/ccy_pair(base: iso.example.org/currency(code:"EUR"), quote: iso.example.org/currency(code:"USD"))"#,
            )),
        );
        assert_eq!(check_describe(&unquoted.to_jcs(), &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
    }

    #[test]
    fn object_key_new_still_builds_what_parse_gives() {
        let key = parse_object_key(r#"a.example.org/o(x:b.example.org/m(y:1))"#).unwrap();
        let again = ObjectKey::new(&key.class, key.keys.clone()).unwrap();
        assert_eq!(again, key);
    }
}

// ---------------------------------------------------------------------------------------------
// Point 7: an encoding is one string value, at most 1024 bytes; every decoding failure
// ---------------------------------------------------------------------------------------------

mod point7 {
    use super::*;

    fn encoding_error(s: &str) {
        match parse_object_key(s) {
            Ok(k) => panic!("{s:?} should not decode, got {k:?}"),
            Err(e) => assert_eq!(e.code(), ErrorCode::KeyEncoding, "{s:?}"),
        }
    }

    #[test]
    fn at_most_1024_bytes_as_a_whole() {
        let v = vector_by_id("key/at-limit");
        let at = str_of(&v, "input");
        assert_eq!(at.len(), 1024);
        let key = parse_object_key(at).unwrap();
        assert_eq!(datumesh_core::format_object_key(&key), at, "the canonical spelling");
        assert_eq!(jcs(&key.to_value()), jcs(v.get("decoded").unwrap()));
        // One byte more, by one more byte in the string.
        let over = at.replacen("\"x", "\"xx", 1);
        assert_eq!(over.len(), 1025);
        encoding_error(&over);
        let v = vector_by_id("invalid/key-too-long");
        assert_eq!(str_of(&v, "error"), "E_KEY_ENCODING");
        encoding_error(str_of(&v, "input"));
    }

    #[test]
    fn the_whole_bounds_each_string_inside_it() {
        // Two strings of 600 bytes: each is a valid string value, the whole is 1230 bytes.
        let s = "x".repeat(600);
        encoding_error(&format!(r#"a.example.org/k(a:"{s}",b:"{s}")"#));
        // One string of 600 bytes is fine.
        assert!(parse_object_key(&format!(r#"a.example.org/k(a:"{s}")"#)).is_ok());
        // The bytes are UTF-8 bytes: 2-byte characters count twice.
        let two = "\u{E9}".repeat(520);
        encoding_error(&format!(r#"a.example.org/k(a:"{two}")"#));
        // Nested encodings count in the whole.
        let inner = format!(r#"b.example.org/m(y:"{s}")"#);
        encoding_error(&format!(r#"a.example.org/k(a:{inner},c:"{s}")"#));
    }

    #[test]
    fn every_failure_to_decode_is_e_key_encoding() {
        // A control character, NFD, an unassigned code point: 3.5's checks inside the encoding.
        for bad in [
            "a.example.org/k(s:\"a\tb\")",
            "a.example.org/k(s:\"a\nb\")",
            "a.example.org/k(s:\"a\u{0}b\")",
            "a.example.org/k(s:\"e\u{0301}\")",
            "a.example.org/k(s:\"a\u{0378}b\")",
            // A tab or a line feed outside a string is a syntax failure like any other.
            "a.example.org/k(\ts:1)",
            "a.example.org/k(s:1)\n",
            // A date that is not a calendar day; an instant that is not a real time.
            "a.example.org/k(d:2023-02-29)",
            "a.example.org/k(d:2024-13-01)",
            "a.example.org/k(d:2024-00-10)",
            "a.example.org/k(d:2024-04-31)",
            "a.example.org/k(i:2024-02-30T10:00:00.000000Z)",
            "a.example.org/k(i:2024-03-15T24:00:00.000000Z)",
            "a.example.org/k(i:2024-03-15T16:60:00.000000Z)",
            // Syntax, order, text after the closing parenthesis.
            "a.example.org/k(a:1,a:2)",
            "a.example.org/k(b:1,a:2)",
            "a.example.org/k(a:1)x",
        ] {
            encoding_error(bad);
        }
        for id in ["invalid/key-date-calendar", "invalid/key-string-control", "invalid/key-string-nfd"] {
            let v = vector_by_id(id);
            assert_eq!(str_of(&v, "error"), "E_KEY_ENCODING", "{id}");
            encoding_error(str_of(&v, "input"));
        }
    }

    #[test]
    fn a_leap_day_is_a_calendar_day() {
        assert!(parse_object_key("a.example.org/k(d:2024-02-29)").is_ok());
        assert!(parse_object_key("a.example.org/k(i:2024-02-29T23:59:59.999999Z)").is_ok());
    }

    #[test]
    fn nested_encodings_fail_as_e_key_encoding_too() {
        encoding_error("a.example.org/k(x:b.example.org/m(d:2023-02-29))");
        encoding_error("a.example.org/k(x:b.example.org/m(s:\"e\u{0301}\"))");
        encoding_error("a.example.org/k(x:b.example.org/m(s:\"a\tb\"))");
    }
}

// ---------------------------------------------------------------------------------------------
// Point 8: the short id of a malformed id
// ---------------------------------------------------------------------------------------------

mod point8 {
    use super::*;

    #[test]
    fn the_short_id_of_a_string_that_is_not_64_lowercase_hex_digits_is_e_canon_hex() {
        let id = "fcb2180650816aab6614bc0fd855fdec3301e41eab5a6bbcea0970f3cb7a590a";
        assert_eq!(short_id(id).unwrap(), "fcb218065081");
        let upper = id.to_uppercase();
        let non_hex = format!("g{}", &id[1..]);
        let multibyte = format!("{}\u{E9}", &id[..62]);
        for bad in [
            "",
            "x",
            "fcb218065081",
            &id[..63],
            &format!("{id}0"),
            &upper,
            &non_hex,
            &multibyte,
            " fcb2180650816aab6614bc0fd855fdec3301e41eab5a6bbcea0970f3cb7a590",
        ] {
            assert_eq!(short_id(bad).unwrap_err().code(), ErrorCode::CanonHex, "{bad:?}");
        }
    }

    #[test]
    fn a_cell_id_and_an_object_id_work_the_same_way() {
        let v = vector_by_id("describe/object");
        let a = parse(str_of(&v, "input"));
        assert_eq!(short_id(str_of(&a, "object_id")).unwrap(), str_of(&a, "short_id"));
    }
}
