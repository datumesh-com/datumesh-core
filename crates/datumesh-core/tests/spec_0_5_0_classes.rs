//! spec-0.5.0 through the library API, part two (additions of 0.3.8). Contract items of
//! `CONTRACT-0.5.0-libs.md`:
//!
//! * item 5, classes (16.1, 16.2): [`classes`];
//! * item 6, key normalisation (16.4): [`normalisation`];
//! * item 7, object keys (16.5): [`object_keys`];
//! * item 8, describe (16.8): [`describe`];
//! * item 9, the define-time check (2.5): [`define`].
//!
//! Items 1 to 4 are in `spec_0_5_0.rs`. Every test here fails until the behaviour is implemented
//! (the new modules are `todo!()` stubs; see `IMPL-NOTES-0.3.8.md`). Cases come from the vectors
//! (all of them run here, named in the assertion messages) and from the text of the sections
//! named in each test.

mod common;

use std::collections::BTreeMap;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{
    check_define, check_describe, check_describe_with, define_class, format_object_key, parse_object_key, sha256_hex,
    short_id, ClassDescriptor, ClassEnv, DescribeKind, ErrorCode, FunctionDescriptor, KeyNormalisation, KeyValue,
    ObjectKey, Scalar,
};

fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    match v.get(k) {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

/// The definer's knowledge in a `class` vector's `env` (SPEC 11): method functions, flattened
/// classes, the definer.
fn env_of(v: &Value) -> ClassEnv {
    let env = v.get("env").unwrap();
    let mut out = ClassEnv::new(str_of(env, "definer"));
    for f in array(env, "functions") {
        out.insert_function(FunctionDescriptor::from_value(f).unwrap_or_else(|e| panic!("{}: {e}", str_of(v, "id"))));
    }
    for c in array(env, "classes") {
        out.insert_class(ClassDescriptor::from_value(c).unwrap_or_else(|e| panic!("{}: {e}", str_of(v, "id"))));
    }
    out
}

fn held_of(v: &Value) -> BTreeMap<String, ClassDescriptor> {
    let mut held = BTreeMap::new();
    for (name, d) in v.get("classes").and_then(Value::as_object).unwrap() {
        held.insert(name.clone(), ClassDescriptor::from_value(d).unwrap());
    }
    held
}

// ---------------------------------------------------------------------------------------------
// Item 5: classes (16.1, 16.2)
// ---------------------------------------------------------------------------------------------

mod classes {
    use super::*;

    fn fx() -> Value {
        vector_by_id("class/fx-pair")
    }

    /// Defines `descriptor` (a JSON value) under `name` in the environment of the `class/fx-pair`
    /// vector (definer `rates.example.org`; the method functions of the vectors are known).
    fn define_fx(name: &str, descriptor: &Value) -> datumesh_core::Result<ClassDescriptor> {
        define_class(name, &descriptor.to_jcs(), &env_of(&fx()))
    }

    fn fx_input() -> Value {
        parse(str_of(&fx(), "input"))
    }

    fn parse_class(v: &Value) -> datumesh_core::Result<ClassDescriptor> {
        ClassDescriptor::from_json(&v.to_jcs())
    }

    #[test]
    fn every_class_vector_defines_or_fails_with_its_code() {
        let all = vectors_of_kind("class");
        // 12 valid, 32 invalid (every class code of 10.2 among them); spec-0.6.1 adds 5 (2 valid, 3 invalid).
        assert_eq!(all.len(), 50);
        for v in &all {
            let id = str_of(v, "id");
            let got = define_class(str_of(v, "name"), str_of(v, "input").as_bytes(), &env_of(v));
            if is_valid(v) {
                let flat = got.unwrap_or_else(|e| panic!("{id}: {e}"));
                // The flattened descriptor is the class descriptor, and its hash is the class hash.
                assert_eq!(String::from_utf8(flat.canonical()).unwrap(), str_of(v, "flattened"), "{id}");
                assert_eq!(flat.hash(), str_of(v, "class_hash"), "{id}");
                assert_eq!(sha256_hex(str_of(v, "flattened").as_bytes()), str_of(v, "class_hash"), "{id}");
                // A flattened descriptor is itself a descriptor: it passes 16.1 and reads back as it was.
                let back = ClassDescriptor::from_json(str_of(v, "flattened").as_bytes())
                    .unwrap_or_else(|e| panic!("{id}: the flattened descriptor does not parse: {e}"));
                assert_eq!(back, flat, "{id}");
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(v, "error"), "{id}");
            }
        }
    }

    #[test]
    fn the_vectors_cover_every_class_code() {
        let codes: std::collections::BTreeSet<String> =
            vectors_of_kind("class").iter().filter(|v| !is_valid(v)).map(|v| str_of(v, "error").to_string()).collect();
        for c in [
            "E_CLASS_MEMBER",
            "E_CLASS_PARENT",
            "E_CLASS_INHERIT",
            "E_CLASS_OVERRIDE",
            "E_CLASS_FINAL",
            "E_CLASS_METHOD",
            "E_CLASS_INTERFACE",
            "E_CLASS_OWNER",
        ] {
            assert!(codes.contains(c), "no class vector for {c}");
            assert!(ErrorCode::from_code(c).is_some());
        }
    }

    #[test]
    fn the_class_hash_is_the_hash_of_the_flattened_canonical_bytes_and_not_of_the_name() {
        let flat = define_fx("rates.example.org/fx_pair", &fx_input()).unwrap();
        assert_eq!(flat.hash(), str_of(&fx(), "class_hash"));
        // 16.1: "As for functions, the class name is not part of it."
        let other = define_fx("rates.example.org/fx_pair_copy", &fx_input()).unwrap();
        assert_eq!(other.hash(), flat.hash());
        // Member order in the source text is not part of it (the vector class/fx-pair-reordered).
        let re = vector_by_id("class/fx-pair-reordered");
        assert_eq!(str_of(&re, "class_hash"), str_of(&fx(), "class_hash"));
        assert_ne!(str_of(&re, "input"), str_of(&fx(), "input"));
        let flat2 = define_class(str_of(&re, "name"), str_of(&re, "input").as_bytes(), &env_of(&re)).unwrap();
        assert_eq!(flat2.canonical(), flat.canonical());
        // A different descriptor is a different class.
        let more = edit(&fx_input(), &["maps"], Some(Value::Array(vec![text("https://schema.example.org/FxPair")])));
        assert_ne!(define_fx("rates.example.org/fx_pair", &more).unwrap().hash(), flat.hash());
    }

    #[test]
    fn a_subclass_is_flattened_onto_its_parent() {
        // 16.2 step 3, the vector class/subclass-flattened: the parent's keys, then the class's own;
        // `order` the parent's followed by the class's own; `methods` the parent's overlaid by the
        // class's own; the other members the class's own; `parent` stays.
        let v = vector_by_id("class/subclass-flattened");
        let flat = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v)).unwrap();
        assert_eq!(flat.order, ["base", "quote", "fixing"]);
        assert_eq!(flat.keys.keys().map(String::as_str).collect::<Vec<_>>(), ["base", "fixing", "quote"]);
        assert_eq!(flat.methods.keys().map(String::as_str).collect::<Vec<_>>(), ["fixing_date", "rate"]);
        let parent = parse(str_of(&v, "input")).get("parent").and_then(Value::as_str).unwrap().to_string();
        assert_eq!(flat.parent.as_deref(), Some(parent.as_str()));
        assert_eq!(flat.steward.namespace, "calc.example.net");
        // Its hash is not the parent's, and the parent's own descriptor is untouched.
        assert_ne!(flat.hash(), parent);
        let env = env_of(&v);
        assert_eq!(env.class(&parent).unwrap().hash(), parent);
        assert_eq!(env.class(&parent).unwrap().order, ["base", "quote"]);
        // Defining it twice gives the same bytes.
        let again = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env).unwrap();
        assert_eq!(again.canonical(), flat.canonical());
    }

    #[test]
    fn final_and_open_control_overrides() {
        // 16.2 step 4: a subclass of a final class adds methods, and overrides only what `open` lists.
        for (id, ok) in [
            ("class/subclass-of-final-adds", true),
            ("class/subclass-of-final-overrides-open", true),
            ("invalid/class-override-final", false),
        ] {
            let v = vector_by_id(id);
            let got = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v));
            assert_eq!(got.is_ok(), ok, "{id}");
            if !ok {
                assert_eq!(got.unwrap_err().code(), ErrorCode::ClassFinal, "{id}");
            }
        }
        // The flattened descriptor of a subclass of a final class says what its parent said.
        let v = vector_by_id("class/subclass-of-final-adds");
        let flat = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v)).unwrap();
        assert!(!flat.is_final, "`final` is the class's own member, not inherited");
    }

    #[test]
    fn define_checks_in_the_order_of_16_2() {
        let valid = fx_input();
        // 1. The class name (E_CANON_NAME), then its owner (E_CLASS_OWNER), before the descriptor.
        let broken = edit(&valid, &["colour"], Some(text("red"))); // E_SCHEMA
        assert_eq!(define_fx("fx_pair", &broken).unwrap_err().code(), ErrorCode::CanonName);
        assert_eq!(define_fx("Rates.example.org/fx_pair", &broken).unwrap_err().code(), ErrorCode::CanonName);
        assert_eq!(define_fx("calc.example.net/fx_pair", &valid).unwrap_err().code(), ErrorCode::ClassOwner);
        assert_eq!(define_fx("calc.example.net/fx_pair", &broken).unwrap_err().code(), ErrorCode::ClassOwner);
        assert_eq!(define_fx("rates.example.org/fx_pair", &broken).unwrap_err().code(), ErrorCode::Schema);
        // 2. The descriptor as written (16.1) before the parent.
        let child = edit(&valid, &["parent"], Some(text(&"ab".repeat(32))));
        assert_eq!(define_fx("rates.example.org/sub", &child).unwrap_err().code(), ErrorCode::ClassParent);
        let child_bad = edit(&child, &["colour"], Some(text("red")));
        assert_eq!(define_fx("rates.example.org/sub", &child_bad).unwrap_err().code(), ErrorCode::Schema);
        // 3. The parent is known before anything about methods: a parent no one knows, and a method
        //    no one knows.
        let child_unknown_method =
            edit(&child, &["methods"], Some(parse(&format!(r#"{{"rate":"{}"}}"#, "cd".repeat(32)))));
        assert_eq!(
            define_fx("rates.example.org/sub", &child_unknown_method).unwrap_err().code(),
            ErrorCode::ClassParent
        );
        // 6. A method with no function the definer knows is E_CLASS_METHOD.
        assert_eq!(
            define_fx("rates.example.org/fx_pair", &child_unknown_method.clone()).unwrap_err().code(),
            ErrorCode::ClassParent,
            "a parent member makes it a subclass"
        );
        let own_unknown = edit(&valid, &["methods"], Some(parse(&format!(r#"{{"rate":"{}"}}"#, "cd".repeat(32)))));
        assert_eq!(define_fx("rates.example.org/fx_pair", &own_unknown).unwrap_err().code(), ErrorCode::ClassMethod);
    }

    #[test]
    fn a_subclass_may_not_declare_a_parent_key_again() {
        // 16.2 step 3: keys are inherited, never removed or retyped (E_CLASS_INHERIT).
        let v = vector_by_id("invalid/class-inherit-redeclared");
        let got = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v));
        assert_eq!(got.unwrap_err().code(), ErrorCode::ClassInherit);
        // Even the same declaration, word for word, is a redeclaration.
        let ok = vector_by_id("class/subclass-flattened");
        let d = parse(str_of(&ok, "input"));
        let key = parse(r#"{"key_normalisation":"upper_ascii","type":"string"}"#);
        let d = edit(&d, &["keys", "base"], Some(key));
        let d = edit(&d, &["order"], Some(parse(r#"["base","fixing"]"#)));
        assert_eq!(
            define_class(str_of(&ok, "name"), &d.to_jcs(), &env_of(&ok)).unwrap_err().code(),
            ErrorCode::ClassInherit
        );
    }

    // ---- 16.1: the descriptor alone ----

    #[test]
    fn a_descriptor_as_written_is_checked_alone() {
        // Parent-free descriptors need no environment. The all-members vector has every optional
        // member: it reads back and writes the same canonical bytes.
        let v = vector_by_id("class/all-members");
        let d = ClassDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
        assert_eq!(String::from_utf8(d.canonical()).unwrap(), str_of(&v, "flattened"));
        assert!(d.is_final && !d.open.is_empty() && !d.maps.is_empty() && !d.inverse.is_empty());
        assert!(d.extent.is_some() && d.parent.is_none());
        // The keyless interface: no keys, an empty order, methods only.
        let v = vector_by_id("class/interface");
        let d = ClassDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
        assert!(d.keys.is_empty() && d.order.is_empty() && !d.methods.is_empty());
        // A lookup method (D93), in a class that names it.
        let v = vector_by_id("class/lookup");
        let d = ClassDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
        assert_eq!(d.lookup.as_deref(), Some("named"));
        assert!(d.methods.contains_key("named"));
        // Not JSON, or not an object.
        assert_eq!(ClassDescriptor::from_json(b"{").unwrap_err().code(), ErrorCode::ParseJson);
        assert_eq!(ClassDescriptor::from_json(b"[]").unwrap_err().code(), ErrorCode::Schema);
    }

    #[test]
    fn structure_is_checked_over_the_whole_descriptor_first() {
        // 16.1: `keys`, `methods`, `order` and `steward` are required; any other member is E_SCHEMA;
        // an optional member is absent rather than empty; `final` is `true` or absent.
        let base = fx_input();
        assert!(parse_class(&base).is_ok());
        for required in ["keys", "methods", "order", "steward"] {
            assert_eq!(
                parse_class(&edit(&base, &[required], None)).unwrap_err().code(),
                ErrorCode::Schema,
                "{required}"
            );
        }
        assert_eq!(parse_class(&edit(&base, &["colour"], Some(text("red")))).unwrap_err().code(), ErrorCode::Schema);
        for (member, empty) in [
            ("open", "[]"),
            ("maps", "[]"),
            ("inverse", "[]"),
            ("implements", "[]"),
            ("extent", "{}"),
            ("final", "false"),
            ("final", "\"true\""),
        ] {
            let d = edit(&base, &[member], Some(parse(empty)));
            assert_eq!(parse_class(&d).unwrap_err().code(), ErrorCode::Schema, "{member}: {empty}");
        }
        // Structure first: an E_SCHEMA anywhere wins over an earlier-member canonical error.
        let d = edit(&edit(&base, &["colour"], Some(text("red"))), &["order"], Some(parse(r#"["base"]"#)));
        assert_eq!(parse_class(&d).unwrap_err().code(), ErrorCode::Schema);
        // A key declaration is {"type", ...} or {"class": name}, nothing else: E_SCHEMA.
        let d = edit(&base, &["keys", "base"], Some(parse(r#"{"type":"string","class":"a.example.org/c"}"#)));
        assert_eq!(parse_class(&d).unwrap_err().code(), ErrorCode::Schema);
    }

    #[test]
    fn members_are_checked_in_the_order_of_16_1() {
        // keys, order, methods, steward, parent, open, maps, inverse, lookup, extent, implements.
        let base = fx_input();
        let bad_key = |d: &Value| edit(d, &["keys", "Bad"], Some(parse(r#"{"type":"date"}"#))); // E_CANON_ARG_NAME
        let bad_order = |d: &Value| edit(d, &["order"], Some(parse(r#"["base"]"#))); // E_CANON_SET
        let bad_method = |d: &Value| {
            edit(d, &["methods"], Some(parse(&format!(r#"{{"rate":"{}"}}"#, "AB".repeat(32)))))
            // E_CANON_HEX
        };
        let bad_steward = |d: &Value| edit(d, &["steward", "process"], Some(text("http://rates.example.org/c"))); // MEMBER
        let bad_parent = |d: &Value| edit(d, &["parent"], Some(text("xyz"))); // E_CANON_HEX
        let open_without_final = |d: &Value| edit(d, &["open"], Some(parse(r#"["rate"]"#))); // MEMBER
        let bad_maps = |d: &Value| edit(d, &["maps"], Some(parse(r#"["no scheme"]"#))); // MEMBER
                                                                                        // (earlier, later, code of the earlier): the earlier member's failure is the one reported.
        let cases: Vec<(&str, Value, ErrorCode)> = vec![
            ("keys before order", bad_order(&bad_key(&base)), ErrorCode::CanonArgName),
            ("keys before methods", bad_method(&bad_key(&base)), ErrorCode::CanonArgName),
            ("order before methods", bad_method(&bad_order(&base)), ErrorCode::CanonSet),
            ("methods before steward", bad_steward(&bad_method(&base)), ErrorCode::CanonHex),
            ("steward before parent", bad_parent(&bad_steward(&base)), ErrorCode::ClassMember),
            ("parent before open", open_without_final(&bad_parent(&base)), ErrorCode::CanonHex),
            ("open before maps", bad_maps(&open_without_final(&base)), ErrorCode::ClassMember),
        ];
        for (what, d, want) in cases {
            assert_eq!(parse_class(&d).unwrap_err().code(), want, "{what}");
        }
    }

    #[test]
    fn keys_are_checked_by_16_1() {
        let base = fx_input();
        let key = |decl: &str| edit(&base, &["keys", "base"], Some(parse(decl)));
        let code = |d: &Value| parse_class(d).unwrap_err().code();
        // Valid: every scalar type but blob; a unit on a number; a class-typed key.
        for decl in [
            r#"{"type":"string","key_normalisation":"none"}"#,
            r#"{"type":"string","key_normalisation":"nfc_strict"}"#,
            r#"{"type":"date"}"#,
            r#"{"type":"instant"}"#,
            r#"{"type":"boolean"}"#,
            r#"{"type":"number","unit":"1"}"#,
            r#"{"class":"iso.example.org/currency"}"#,
        ] {
            assert!(parse_class(&key(decl)).is_ok(), "{decl}");
        }
        assert_eq!(code(&key(r#"{"type":"blob"}"#)), ErrorCode::CanonType);
        assert_eq!(code(&key(r#"{"type":"colour"}"#)), ErrorCode::CanonType);
        // A number key has a unit and no other key does (E_CANON_UNIT).
        assert_eq!(code(&key(r#"{"type":"number"}"#)), ErrorCode::CanonUnit);
        assert_eq!(code(&key(r#"{"type":"date","unit":"1"}"#)), ErrorCode::CanonUnit);
        assert_eq!(code(&key(r#"{"type":"number","unit":"furlong"}"#)), ErrorCode::CanonUnit);
        // A string key has key_normalisation, one of the rules, and no other key does (E_CLASS_MEMBER).
        assert_eq!(code(&key(r#"{"type":"string"}"#)), ErrorCode::ClassMember);
        assert_eq!(code(&key(r#"{"type":"string","key_normalisation":"lowercase"}"#)), ErrorCode::ClassMember);
        assert_eq!(code(&key(r#"{"type":"date","key_normalisation":"none"}"#)), ErrorCode::ClassMember);
        assert_eq!(code(&key(r#"{"type":"number","unit":"1","key_normalisation":"none"}"#)), ErrorCode::ClassMember);
        // A class-typed key names a class by a full name (2.3).
        assert_eq!(code(&key(r#"{"class":"currency"}"#)), ErrorCode::CanonName);
        // A key name is an argument name (2.4), and at most 32 keys.
        assert_eq!(code(&edit(&base, &["keys", "Quote"], Some(parse(r#"{"type":"date"}"#)))), ErrorCode::CanonArgName);
        let many: Vec<String> = (0..33).map(|i| format!(r#""k{i:02}":{{"type":"date"}}"#)).collect();
        let order: Vec<String> = (0..33).map(|i| format!(r#""k{i:02}""#)).collect();
        let d = parse(&format!(
            r#"{{"keys":{{{}}},"methods":{{}},"order":[{}],"steward":{{"namespace":"a.example.org","process":"https://a.example.org/p"}}}}"#,
            many.join(","),
            order.join(",")
        ));
        assert_eq!(code(&d), ErrorCode::CanonLimit);
        // `order` holds every key name exactly once (E_CANON_SET).
        for order in [r#"["base"]"#, r#"["base","base","quote"]"#, r#"["base","quote","date"]"#] {
            assert_eq!(code(&edit(&base, &["order"], Some(parse(order)))), ErrorCode::CanonSet, "{order}");
        }
        // Order is the order of the key tuple: any permutation is canonical.
        assert!(parse_class(&edit(&base, &["order"], Some(parse(r#"["quote","base"]"#)))).is_ok());
    }

    #[test]
    fn methods_steward_and_the_other_members_are_checked_by_16_1() {
        let base = fx_input();
        let code = |d: &Value| parse_class(d).unwrap_err().code();
        let rate = "530bf86365ff4afb5a70196542d45f293fad5909026e015eddcfd866290f85b8";
        let methods = |m: &str| edit(&base, &["methods"], Some(parse(m)));
        assert_eq!(code(&methods(&format!(r#"{{"Rate":"{rate}"}}"#))), ErrorCode::CanonName);
        assert_eq!(code(&methods(&format!(r#"{{"rate":"{}"}}"#, "ab".repeat(31)))), ErrorCode::CanonHex);
        assert_eq!(code(&methods(r#"{"rate":1}"#)), ErrorCode::Schema);
        // steward: namespace (2.3) and process, an https URL by the licence URL rule (8.1).
        let steward = |s: &str| edit(&base, &["steward"], Some(parse(s)));
        assert_eq!(
            code(&steward(r#"{"namespace":"Rates","process":"https://a.example.org/p"}"#)),
            ErrorCode::CanonName
        );
        assert_eq!(
            code(&steward(r#"{"namespace":"a.example.org","process":"http://a.example.org/p"}"#)),
            ErrorCode::ClassMember
        );
        assert_eq!(
            code(&steward(r#"{"namespace":"a.example.org","process":"https://a.example.org/p q"}"#)),
            ErrorCode::ClassMember
        );
        assert_eq!(code(&steward(r#"{"namespace":"a.example.org"}"#)), ErrorCode::Schema);
        // parent: a class hash (E_CANON_HEX).
        assert_eq!(code(&edit(&base, &["parent"], Some(text(&"AB".repeat(32))))), ErrorCode::CanonHex);
        // open: only with final (E_CLASS_MEMBER), naming methods of the class, ascending and unique (E_CANON_SET).
        let finalised = edit(&base, &["final"], Some(Value::Bool(true)));
        assert!(parse_class(&edit(&finalised, &["open"], Some(parse(r#"["rate"]"#)))).is_ok());
        assert_eq!(code(&edit(&base, &["open"], Some(parse(r#"["rate"]"#)))), ErrorCode::ClassMember);
        assert_eq!(code(&edit(&finalised, &["open"], Some(parse(r#"["nothing"]"#)))), ErrorCode::ClassMember);
        let two = edit(&finalised, &["methods"], Some(parse(&format!(r#"{{"a":"{rate}","b":"{rate}"}}"#))));
        assert_eq!(code(&edit(&two, &["open"], Some(parse(r#"["b","a"]"#)))), ErrorCode::CanonSet);
        assert_eq!(code(&edit(&two, &["open"], Some(parse(r#"["a","a"]"#)))), ErrorCode::CanonSet);
        // maps: URIs, a scheme and printable ASCII (E_CLASS_MEMBER).
        assert!(parse_class(&edit(&base, &["maps"], Some(parse(r#"["https://schema.example.org/FxPair"]"#)))).is_ok());
        for bad in [r#"["FxPair"]"#, r#"["https://a.example.org/\u00e9"]"#, r#"["https://a b"]"#] {
            assert_eq!(code(&edit(&base, &["maps"], Some(parse(bad)))), ErrorCode::ClassMember, "{bad}");
        }
        // inverse: each attribute has a method by_<attribute> (E_CLASS_MEMBER).
        assert_eq!(code(&edit(&base, &["inverse"], Some(parse(r#"["isin"]"#)))), ErrorCode::ClassMember);
        let with_by = edit(&base, &["methods"], Some(parse(&format!(r#"{{"by_isin":"{rate}","rate":"{rate}"}}"#))));
        assert!(parse_class(&edit(&with_by, &["inverse"], Some(parse(r#"["isin"]"#)))).is_ok());
        // lookup names a method of the class (E_CLASS_MEMBER).
        assert_eq!(code(&edit(&base, &["lookup"], Some(text("named")))), ErrorCode::ClassMember);
        assert!(parse_class(&edit(&base, &["lookup"], Some(text("rate")))).is_ok(), "its signature is define's check");
    }

    #[test]
    fn the_function_checks_of_define_use_the_functions_the_definer_knows() {
        // 16.2 step 6: each method's function descriptor is known (E_CLASS_METHOD) and takes the keys
        // first, with their types and, for a string key, exactly its key_normalisation; the lookup
        // method takes one string `label` and returns a key of this class (E_CLASS_MEMBER).
        for id in [
            "invalid/class-method-unknown",
            "invalid/class-method-keys",
            "invalid/class-method-normalisation",
            "invalid/class-same-as-self",
        ] {
            let v = vector_by_id(id);
            assert_eq!(str_of(&v, "error"), "E_CLASS_METHOD", "{id}");
            let got = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v));
            assert_eq!(got.unwrap_err().code(), ErrorCode::ClassMethod, "{id}");
        }
        for id in ["invalid/class-lookup-signature", "invalid/class-lookup-returns", "invalid/class-extent-signature"] {
            let v = vector_by_id(id);
            let got = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v));
            assert_eq!(got.unwrap_err().code(), ErrorCode::ClassMember, "{id}");
        }
        // Absent key_normalisation on a method's key argument means none, so it matches a `none` key
        // (and only that): class/keyed-none-ruleless-methods is valid, class-method-normalisation not.
        let v = vector_by_id("class/keyed-none-ruleless-methods");
        assert!(define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v)).is_ok());
    }

    #[test]
    fn interfaces_are_keyless_classes() {
        // 16.2 step 8: `implements` names keyless classes the define knows (E_CLASS_INTERFACE), and
        // the class has each of their methods with that signature once the class keys are taken away.
        for id in ["invalid/class-implements-keyed", "invalid/class-implements-missing-method"] {
            let v = vector_by_id(id);
            let got = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v));
            assert_eq!(got.unwrap_err().code(), ErrorCode::ClassInterface, "{id}");
        }
        let v = vector_by_id("class/implements-interface");
        let flat = define_class(str_of(&v, "name"), str_of(&v, "input").as_bytes(), &env_of(&v)).unwrap();
        assert_eq!(flat.implements.len(), 1);
        assert_eq!(flat.hash(), str_of(&v, "class_hash"));
    }
}

// ---------------------------------------------------------------------------------------------
// Item 6: key normalisation (16.4)
// ---------------------------------------------------------------------------------------------

mod normalisation {
    use super::*;
    use KeyNormalisation::*;

    fn ok(rule: KeyNormalisation, s: &str) {
        rule.check(s).unwrap_or_else(|e| panic!("{rule} should accept {s:?}: {e}"));
    }

    fn bad(rule: KeyNormalisation, s: &str) {
        match rule.check(s) {
            Ok(()) => panic!("{rule} should reject {s:?}"),
            Err(e) => assert_eq!(e.code(), ErrorCode::KeyNormalisation, "{rule} {s:?}"),
        }
    }

    fn bad_string(rule: KeyNormalisation, s: &str) {
        match rule.check(s) {
            Ok(()) => panic!("{rule} should reject {s:?}"),
            Err(e) => assert_eq!(e.code(), ErrorCode::CanonString, "{rule} {s:?}"),
        }
    }

    #[test]
    fn the_rules_are_the_closed_list() {
        let names: Vec<&str> = KeyNormalisation::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            names,
            ["none", "upper_ascii", "nfc_strict", "arabic_skeleton", "hebrew_consonantal", "greek_unaccented"]
        );
        for r in KeyNormalisation::ALL {
            assert_eq!(KeyNormalisation::from_name(r.as_str()), Some(*r));
            assert_eq!(r.to_string(), r.as_str());
        }
        for not_a_rule in ["", "None", "UPPER_ASCII", "lowercase", "nfc", "upper_ascii "] {
            assert_eq!(KeyNormalisation::from_name(not_a_rule), None, "{not_a_rule:?}");
        }
        // The names of the table in 16.4 are the names of the schema's enum.
        assert_eq!(KeyNormalisation::NoRule.as_str(), "none");
    }

    #[test]
    fn every_normalisation_vector() {
        let all = vectors_of_kind("normalisation");
        assert_eq!(all.len(), 24); // 9 valid, 15 invalid
        for v in &all {
            let id = str_of(v, "id");
            let rule = KeyNormalisation::from_name(str_of(v, "rule")).unwrap_or_else(|| panic!("{id}: rule"));
            let got = rule.check(str_of(v, "text"));
            if is_valid(v) {
                got.unwrap_or_else(|e| panic!("{id}: {e}"));
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(v, "error"), "{id}");
            }
        }
    }

    #[test]
    fn none_accepts_any_string() {
        // 16.4 (6d5633e): `none` is any string valid by 3.5, the empty string included. A value that is
        // not NFC is no string value at all (E_CANON_STRING), under `none` too: see
        // `a_key_value_is_first_a_string_value_by_3_5`.
        for s in ["", "anything at all", "  Mixed  CASE  ", "ΑΒΓ", "a\u{200D}b", "a\u{00A0}b"] {
            ok(NoRule, s);
        }
    }

    #[test]
    fn every_rule_but_none_rejects_the_empty_string() {
        // (Before 6d5633e this test also had a value that is not NFC here with E_KEY_NORMALISATION;
        // that is now E_CANON_STRING under every rule, 3.5 first.)
        for rule in KeyNormalisation::ALL.iter().filter(|r| **r != NoRule) {
            bad(*rule, "");
        }
    }

    #[test]
    fn upper_ascii() {
        for s in ["EUR", "1M", "XAMS:ASML", "!", "~", "A-B_C.D", "0"] {
            ok(UpperAscii, s); // printable ASCII U+0021 to U+007E, no lowercase letter
        }
        for s in ["eur", "Eur", "E R", " EUR", "EUR ", "\u{00C9}", "\u{FF21}", "a", "z"] {
            bad(UpperAscii, s);
        }
        // 6d5633e (16.4): a control or a non-NFC value fails as a 3.5 string value first.
        for s in ["\u{7F}", "\t", "EU\u{0301}R"] {
            bad_string(UpperAscii, s);
        }
        // The boundaries of the lowercase range: '@' and '`' flank A-Z and a-z, '[' and '{' follow.
        for s in ["@", "[", "`", "{", "A", "Z"] {
            ok(UpperAscii, s);
        }
        bad(UpperAscii, "a");
        bad(UpperAscii, "z");
    }

    #[test]
    fn nfc_strict() {
        for s in ["Zürich", "New York", "a", "\u{00E9}", "日本語", "a b c"] {
            ok(NfcStrict, s);
        }
        for s in [
            " a",         // leading space
            "a ",         // trailing space
            "a  b",       // repeated space
            "a\u{00A0}b", // a space other than U+0020 (NBSP)
            "a\u{2003}b", // em space
            "a\u{3000}b", // ideographic space
            "a\u{200D}b", // ZWJ, a format character (Cf)
            "a\u{200B}b", // zero width space (Cf)
            "a\u{FEFF}b", // BOM (Cf)
            "a\u{2028}b", // line separator
            "a\u{2029}b", // paragraph separator
            " ",
        ] {
            bad(NfcStrict, s);
        }
    }

    #[test]
    fn arabic_skeleton() {
        // Letters and the Arabic-Indic digits U+0660-U+0669 are fine; marks and tatweel are not.
        for s in [
            "\u{0627}\u{0644}\u{0644}\u{0647}",
            "بسم ٱلله ٱلرحمن ٱلرحيم",
            "\u{0660}\u{0669}",
            "\u{060F}",
            "\u{061B}",
            "\u{06DD}",
            "\u{06E9}",
            "\u{06EE}",
        ] {
            ok(ArabicSkeleton, s);
        }
        // Each range of 16.4, at both ends: U+0610-U+061A, U+064B-U+065F, U+0670, U+06D6-U+06DC,
        // U+06DF-U+06E8, U+06EA-U+06ED; and tatweel U+0640.
        for c in [
            0x0610, 0x0615, 0x061A, 0x064B, 0x0650, 0x065F, 0x0670, 0x06D6, 0x06DA, 0x06DC, 0x06DF, 0x06E4, 0x06E8,
            0x06EA, 0x06ED, 0x0640,
        ] {
            let s = format!("\u{0627}{}", char::from_u32(c).unwrap());
            bad(ArabicSkeleton, &s);
        }
    }

    #[test]
    fn hebrew_consonantal() {
        // Letters and the four exceptions of U+0591-U+05C7: maqaf U+05BE, paseq U+05C0, sof pasuq
        // U+05C3, nun hafukha U+05C6.
        for s in [
            "בראשית ברא אלהים",
            "את־השמים",
            "\u{05D0}\u{05BE}\u{05D1}",
            "\u{05D0}\u{05C0}",
            "\u{05D0}\u{05C3}",
            "\u{05D0}\u{05C6}",
        ] {
            ok(HebrewConsonantal, s);
        }
        // Points and accents: both ends of the range, and some inside, including the vector's.
        for c in
            [0x0591, 0x05A0, 0x05AF, 0x05B0, 0x05B4, 0x05BD, 0x05BF, 0x05C1, 0x05C2, 0x05C4, 0x05C5, 0x05C7, 0x05BC]
        {
            let s = format!("\u{05D0}{}", char::from_u32(c).unwrap());
            bad(HebrewConsonantal, &s);
        }
        bad(HebrewConsonantal, "בְּרֵאשִׁית בָּרָא אֱלֹהִים");
    }

    #[test]
    fn greek_unaccented() {
        for s in ["λογος", "Εν αρχη ην ο λογος", "ς", "αβγ δεζ"] {
            ok(GreekUnaccented, s);
        }
        // "no combining mark U+0300-U+036F in its NFD form": precomposed accented letters,
        // breathings, iota subscript, dialytika, and any Latin letter with an accent too.
        for s in ["λόγος", "ὁ", "Ἐν", "ᾳ", "ϊ", "\u{1F00}", "é", "λ\u{0301}"] {
            bad(GreekUnaccented, s);
        }
        // 6d5633e (16.4): U+03B1 U+0345 is not NFC (it composes to U+1FB3), so 3.5 refuses it first.
        bad_string(GreekUnaccented, "\u{03B1}\u{0345}");
    }

    #[test]
    fn the_check_is_pure() {
        // Same answer twice; the value is never repaired (it is not even returned).
        for _ in 0..2 {
            bad(UpperAscii, "eur");
            ok(UpperAscii, "EUR");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Item 7: object keys (16.5)
// ---------------------------------------------------------------------------------------------

mod object_keys {
    use super::*;

    const VERSE: &str = r#"scripture.example.org/verse(book:"Gen",chapter:1,corpus:"tanakh",verse:1)"#;

    #[test]
    fn every_key_vector() {
        let all = vectors_of_kind("key");
        assert_eq!(all.len(), 17); // 7 valid, 10 invalid
        for v in &all {
            let id = str_of(v, "id");
            let got = parse_object_key(str_of(v, "input"));
            if is_valid(v) {
                let k = got.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(jcs(&k.to_value()), jcs(v.get("decoded").unwrap()), "{id}");
                // One spelling: the encoding of what it decodes to is the input itself.
                assert_eq!(format_object_key(&k), str_of(v, "input"), "{id}");
                assert_eq!(parse_object_key(&format_object_key(&k)).unwrap(), k, "{id}");
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(v, "error"), "{id}");
            }
        }
    }

    #[test]
    fn decoding_gives_typed_scalars_and_nested_objects() {
        let k = parse_object_key(VERSE).unwrap();
        assert_eq!(k.class, "scripture.example.org/verse");
        assert_eq!(k.keys.keys().map(String::as_str).collect::<Vec<_>>(), ["book", "chapter", "corpus", "verse"]);
        assert_eq!(k.keys["book"], KeyValue::Scalar(Scalar::String("Gen".into())));
        assert_eq!(k.keys["chapter"], KeyValue::Scalar(Scalar::Number("1".into())));
        let all = parse_object_key(
            r#"a.example.org/k(b:true,d:2024-03-15,f:false,i:2024-03-15T16:00:00.000000Z,n:-1.5,s:"say ""hi""")"#,
        )
        .unwrap();
        assert_eq!(all.keys["b"], KeyValue::Scalar(Scalar::Boolean(true)));
        assert_eq!(all.keys["f"], KeyValue::Scalar(Scalar::Boolean(false)));
        assert_eq!(all.keys["d"], KeyValue::Scalar(Scalar::Date("2024-03-15".into())));
        assert_eq!(all.keys["i"], KeyValue::Scalar(Scalar::Instant("2024-03-15T16:00:00.000000Z".into())));
        assert_eq!(all.keys["n"], KeyValue::Scalar(Scalar::Number("-1.5".into())));
        assert_eq!(all.keys["s"], KeyValue::Scalar(Scalar::String("say \"hi\"".into())));
        // A class-typed key is its own encoding, unquoted, to any depth.
        let deep = parse_object_key("a.example.org/o(x:b.example.org/m(y:c.example.org/leaf(z:1)))").unwrap();
        let KeyValue::Object(m) = &deep.keys["x"] else { panic!("nested object") };
        assert_eq!(m.class, "b.example.org/m");
        let KeyValue::Object(leaf) = &m.keys["y"] else { panic!("nested object") };
        assert_eq!(leaf.keys["z"], KeyValue::Scalar(Scalar::Number("1".into())));
        assert_eq!(format_object_key(&deep), "a.example.org/o(x:b.example.org/m(y:c.example.org/leaf(z:1)))");
        // A quoted class-key is a string: the encoding alone cannot tell (a class is checked by the caller).
        let quoted = parse_object_key(r#"a.example.org/o(x:"b.example.org/m()")"#).unwrap();
        assert_eq!(quoted.keys["x"], KeyValue::Scalar(Scalar::String("b.example.org/m()".into())));
        // A keyless object.
        let none = parse_object_key("a.example.org/singleton()").unwrap();
        assert!(none.keys.is_empty());
        assert_eq!(format_object_key(&none), "a.example.org/singleton()");
    }

    #[test]
    fn anything_that_is_not_exactly_the_encoding_is_e_key_encoding() {
        // 16.5: "anything that is not exactly the encoding of what it decodes to".
        for bad in [
            "",
            "(",
            "a.example.org/k",
            "a.example.org/k(",
            "a.example.org/k)",
            "a.example.org/k()x",
            " a.example.org/k()",
            "a.example.org/k() ",
            "a.example.org/k( )",
            "currency(code:\"EUR\")",                  // class name not in full (vector)
            "Currency.example.org/c()",                // not a name (2.3)
            "iso.example.org/currency(code: \"EUR\")", // a space (vector)
            "iso.example.org/currency(code :\"EUR\")",
            "a.example.org/k(a:1, b:2)",
            "a.example.org/k(a:1,)",
            "a.example.org/k(,a:1)",
            "a.example.org/k(a:1,,b:2)",
            "a.example.org/k(z:1,a:2)", // not ascending (vector)
            "a.example.org/k(a:1,a:2)", // repeated
            "a.example.org/k(A:1)",     // not an argument name
            "a.example.org/k(a)",
            "a.example.org/k(a:)",
            "a.example.org/k(:1)",
            "iso.example.org/currency(code:EUR)", // an unquoted string (vector)
            "a.example.org/k(a:\"x)",             // unterminated
            "a.example.org/k(a:\"x\"y)",
            "a.example.org/k(a:'x')",
            "a.example.org/k(a:TRUE)",
            "a.example.org/k(a:True)",
            "a.example.org/k(a:null)",
            "a.example.org/k(n:01)", // number form (vector)
            "a.example.org/k(n:+1)",
            "a.example.org/k(n:.5)",
            "a.example.org/k(n:1e3)",
            "a.example.org/k(d:2024-3-15)",
            "a.example.org/k(d:2024-03-15T16:00:00Z)",
            "a.example.org/k(a:b.example.org/c)", // a nested encoding needs its parentheses
            "a.example.org/k(a:b.example.org/c(x:1)",
        ] {
            assert_eq!(parse_object_key(bad).expect_err(bad).code(), ErrorCode::KeyEncoding, "{bad:?}");
        }
    }

    #[test]
    fn format_writes_the_one_canonical_spelling() {
        // Keys sorted by name whatever the order they came in; strings quoted with `"` doubled.
        let pairs = vec![
            ("verse".to_string(), KeyValue::Scalar(Scalar::from_lexical(datumesh_core::Type::Number, "1").unwrap())),
            ("book".to_string(), KeyValue::Scalar(Scalar::String("Gen".into()))),
            ("note".to_string(), KeyValue::Scalar(Scalar::String("say \"hi\"".into()))),
        ];
        let k = ObjectKey::new("scripture.example.org/verse", pairs).unwrap();
        assert_eq!(format_object_key(&k), r#"scripture.example.org/verse(book:"Gen",note:"say ""hi""",verse:1)"#);
        // Nested: the inner object unquoted.
        let inner = ObjectKey::new(
            "iso.example.org/currency",
            vec![("code".to_string(), KeyValue::Scalar(Scalar::String("EUR".into())))],
        )
        .unwrap();
        let outer =
            ObjectKey::new("rates.example.org/fx_pair", vec![("base".to_string(), KeyValue::Object(inner))]).unwrap();
        assert_eq!(
            format_object_key(&outer),
            r#"rates.example.org/fx_pair(base:iso.example.org/currency(code:"EUR"))"#
        );
        // A class name and key names are checked; a key is never a blob; no key twice (E_CANON_ARG_DUPLICATE).
        let s = |v: &str| KeyValue::Scalar(Scalar::String(v.into()));
        assert_eq!(ObjectKey::new("verse", vec![]).unwrap_err().code(), ErrorCode::CanonName);
        assert_eq!(
            ObjectKey::new("a.example.org/k", vec![("Code".into(), s("x"))]).unwrap_err().code(),
            ErrorCode::CanonArgName
        );
        let blob = KeyValue::Scalar(Scalar::Blob(format!("{}:3:text/plain", "a".repeat(64))));
        assert_eq!(
            ObjectKey::new("a.example.org/k", vec![("b".into(), blob)]).unwrap_err().code(),
            ErrorCode::CanonType
        );
        assert_eq!(
            ObjectKey::new("a.example.org/k", vec![("a".into(), s("x")), ("a".into(), s("y"))]).unwrap_err().code(),
            ErrorCode::CanonArgDuplicate
        );
    }

    #[test]
    fn the_object_and_its_id() {
        // The vector describe/object: object_id 2318527365.. is SHA-256 of the canonical bytes of
        // the object JSON {"class", "class_hash", "keys"} (16.5).
        let v = vector_by_id("describe/object");
        let answer = parse(str_of(&v, "input"));
        let object = answer.get("object").unwrap();
        let class_hash = str_of(object, "class_hash");
        let k = parse_object_key(VERSE).unwrap();
        assert_eq!(jcs(&k.object_value(class_hash).unwrap()), jcs(object));
        let id = k.object_id(class_hash).unwrap();
        assert_eq!(id, str_of(&answer, "object_id"));
        assert_eq!(id, sha256_hex(&object.to_jcs()));
        // The class hash is part of the object: another hash, another id.
        assert_ne!(k.object_id(&"ab".repeat(32)).unwrap(), id);
        // Keys in the encoding's order or not, one object.
        let again = parse_object_key(&format_object_key(&k)).unwrap();
        assert_eq!(again.object_id(class_hash).unwrap(), id);
        // A class hash is 64 lowercase hex digits.
        for bad in ["", "abc", &"AB".repeat(32), &"zz".repeat(32)] {
            assert_eq!(k.object_value(bad).unwrap_err().code(), ErrorCode::CanonHex, "{bad:?}");
            assert_eq!(k.object_id(bad).unwrap_err().code(), ErrorCode::CanonHex, "{bad:?}");
        }
    }

    #[test]
    fn the_call_form_of_an_object() {
        // 16.8: the class name and every key labelled in ascending order, `name: <literal>`.
        let v = vector_by_id("describe/object");
        let answer = parse(str_of(&v, "input"));
        assert_eq!(parse_object_key(VERSE).unwrap().call_form(), str_of(&answer, "call"));
        let k = parse_object_key(r#"a.example.org/k(b:true,d:2024-03-15,n:-1.5,s:"say ""hi""")"#).unwrap();
        assert_eq!(k.call_form(), r#"a.example.org/k(b: true, d: 2024-03-15, n: -1.5, s: "say ""hi""")"#);
        assert_eq!(parse_object_key("a.example.org/singleton()").unwrap().call_form(), "a.example.org/singleton()");
        // A class-typed key is a string holding its encoding (16.5), quoted in the call form
        // (the reference tool's describe check, as a black box).
        let nested = parse_object_key("a.example.org/o(x:b.example.org/m(y:1))").unwrap();
        assert_eq!(nested.call_form(), r#"a.example.org/o(x: "b.example.org/m(y:1)")"#);
    }

    #[test]
    fn a_class_typed_key_is_a_string_in_the_object_json() {
        // 16.5: "When a key is class-typed ... the object travels as one canonical string". In the
        // object JSON its value is therefore the typed string scalar holding the encoding.
        let nested = parse_object_key("a.example.org/o(x:b.example.org/m(y:1))").unwrap();
        let hash = "ab".repeat(32);
        let v = nested.object_value(&hash).unwrap();
        let want = parse(&format!(
            r#"{{"class":"a.example.org/o","class_hash":"{hash}","keys":{{"x":{{"type":"string","value":"b.example.org/m(y:1)"}}}}}}"#
        ));
        assert_eq!(jcs(&v), jcs(&want));
    }

    #[test]
    fn short_ids_are_the_first_12_digits() {
        let id = "23185273656a260e628a19e624304212504b065267b4cf551c961639fc18556c";
        assert_eq!(short_id(id).unwrap(), "23185273656a");
        // A cell id works the same way (16.8).
        let cell = vector_by_id("envelope/not-covered-fault-key");
        let cell_id = str_of(cell.get("request").unwrap(), "cell_id");
        assert_eq!(short_id(cell_id).unwrap(), &cell_id[..12]);
        for bad in ["", "23185273656a", &id.to_uppercase(), &id[..63], &format!("{id}0"), "x"] {
            assert_eq!(short_id(bad).unwrap_err().code(), ErrorCode::CanonHex, "{bad:?}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Item 8: describe (16.8)
// ---------------------------------------------------------------------------------------------

mod describe {
    use super::*;

    fn class_vector() -> Value {
        vector_by_id("describe/class")
    }

    fn object_vector() -> Value {
        vector_by_id("describe/object")
    }

    fn check(v: &Value, answer: &Value) -> datumesh_core::Result<datumesh_core::Described> {
        check_describe(&answer.to_jcs(), &held_of(v))
    }

    fn code(v: &Value, answer: &Value) -> ErrorCode {
        check(v, answer).unwrap_err().code()
    }

    #[test]
    fn every_describe_vector() {
        let all = vectors_of_kind("describe");
        assert_eq!(all.len(), 65); // 12 valid, 34 invalid (spec-0.6.0 added 6 valid, 16 invalid); spec-0.6.1 adds 19 (9 valid, 10 invalid)
        for v in &all {
            let id = str_of(v, "id");
            // 6d5633e (16.8 step 6): a vector's held function descriptors go to check_describe_with
            let functions: Vec<FunctionDescriptor> =
                array(v, "functions").iter().map(|f| FunctionDescriptor::from_value(f).unwrap()).collect();
            let got = if functions.is_empty() {
                check_describe(str_of(v, "input").as_bytes(), &held_of(v))
            } else {
                check_describe_with(str_of(v, "input").as_bytes(), &held_of(v), &functions)
            };
            if is_valid(v) {
                got.unwrap_or_else(|e| panic!("{id}: {e}"));
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(v, "error"), "{id}");
            }
        }
    }

    #[test]
    fn a_class_answer_is_read_back() {
        let v = class_vector();
        let a = parse(str_of(&v, "input"));
        let d = check(&v, &a).unwrap();
        assert_eq!(d.kind, DescribeKind::Class);
        assert_eq!(d.class, "rates.example.org/fx_pair");
        assert_eq!(d.class_hash, str_of(&a, "class_hash"));
        assert_eq!(d.object_id, None);
        // class_hash is the hash of the descriptor.
        assert_eq!(d.class_hash, held_of(&v)["rates.example.org/fx_pair"].hash());
    }

    #[test]
    fn an_object_answer_is_read_back() {
        let v = object_vector();
        let a = parse(str_of(&v, "input"));
        let d = check(&v, &a).unwrap();
        assert_eq!(d.kind, DescribeKind::Object);
        assert_eq!(d.class, "scripture.example.org/verse");
        assert_eq!(d.class_hash, held_of(&v)["scripture.example.org/verse"].hash());
        assert_eq!(d.object_id.as_deref(), Some(str_of(&a, "object_id")));
        // same_as is optional.
        let without = edit(&a, &["same_as"], None);
        assert!(check(&v, &without).is_ok());
    }

    #[test]
    fn a_class_answer_has_exactly_the_members_of_16_8() {
        let v = class_vector();
        let a = parse(str_of(&v, "input"));
        for required in ["kind", "class", "class_hash", "descriptor", "methods"] {
            assert_eq!(code(&v, &edit(&a, &[required], None)), ErrorCode::Describe, "without {required}");
        }
        assert_eq!(code(&v, &edit(&a, &["x"], Some(text("y")))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["kind"], Some(text("Class")))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["kind"], Some(text("instance")))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["class"], Some(Value::Bool(true)))), ErrorCode::Describe);
        // class_hash: the descriptor's hash, 64 lowercase hex digits.
        for h in [text(&"ab".repeat(32)), text(&"AB".repeat(32)), text("abab"), Value::Null] {
            assert_eq!(code(&v, &edit(&a, &["class_hash"], Some(h))), ErrorCode::Describe);
        }
        // The answer is not an object, or not JSON text of a describe answer.
        assert_eq!(check_describe(b"[]", &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
        assert_eq!(check_describe(b"\"class\"", &held_of(&v)).unwrap_err().code(), ErrorCode::Describe);
    }

    #[test]
    fn the_method_table_of_a_class_is_the_descriptors() {
        let v = class_vector();
        let a = parse(str_of(&v, "input"));
        // Each method entry: exactly fn, fn_hash, args, returns, and optionally description.
        let set = |name: &str, member: &str, new: Option<Value>| edit(&a, &["methods", name, member], new);
        for member in ["fn", "fn_hash", "args", "returns"] {
            assert_eq!(code(&v, &set("rate", member, None)), ErrorCode::Describe, "without {member}");
        }
        assert_eq!(code(&v, &set("rate", "note", Some(text("x")))), ErrorCode::Describe);
        // `rate` has a description in the vector; another may be given one (3.5, at most 1024 bytes).
        assert!(check(&v, &set("spot", "description", Some(text("A spot rate.")))).is_ok());
        assert!(check(&v, &set("spot", "description", Some(text(&"a".repeat(1024))))).is_ok());
        assert_eq!(code(&v, &set("spot", "description", Some(text(&"a".repeat(1025))))), ErrorCode::Describe);
        assert_eq!(code(&v, &set("spot", "description", Some(Value::Bool(true)))), ErrorCode::Describe);
        // fn is `<class>/<method>`; fn_hash is the descriptor's.
        assert_eq!(code(&v, &set("rate", "fn", Some(text("rates.example.org/fx_pair/spot")))), ErrorCode::Describe);
        assert_eq!(code(&v, &set("rate", "fn", Some(text("rate")))), ErrorCode::Describe);
        assert_eq!(code(&v, &set("rate", "fn_hash", Some(text(&"ab".repeat(32))))), ErrorCode::Describe);
        assert_eq!(code(&v, &set("rate", "fn_hash", Some(text(&"AB".repeat(32))))), ErrorCode::Describe);
        // "from each method name of the descriptor, and no other": a method missing, or an extra one
        // (the vector invalid/describe-class-methods is the first).
        let methods = a.get("methods").unwrap();
        let Value::Object(entries) = methods else { panic!() };
        let without: Vec<(String, Value)> = entries.iter().filter(|(k, _)| k != "cross").cloned().collect();
        assert_eq!(code(&v, &edit(&a, &["methods"], Some(Value::Object(without)))), ErrorCode::Describe);
        let mut with = entries.clone();
        with.push(("zzz".to_string(), entries[0].1.clone()));
        assert_eq!(code(&v, &edit(&a, &["methods"], Some(Value::Object(with)))), ErrorCode::Describe);
    }

    #[test]
    fn the_descriptor_in_a_class_answer_is_the_class_held() {
        let v = class_vector();
        let a = parse(str_of(&v, "input"));
        // A descriptor that is not the held one: its hash is not the class_hash the answer states,
        // or, with class_hash following it, not the class the caller holds.
        let changed = edit(&a, &["descriptor", "maps"], Some(Value::Array(vec![text("https://x.example.org/FxPair")])));
        assert_eq!(code(&v, &changed), ErrorCode::Describe, "class_hash no longer follows from the descriptor");
        assert_eq!(code(&v, &edit(&a, &["descriptor"], None)), ErrorCode::Describe);
    }

    #[test]
    fn an_object_answer_has_exactly_the_members_of_16_8() {
        let v = object_vector();
        let a = parse(str_of(&v, "input"));
        for required in ["kind", "object", "object_id", "short_id", "call", "methods"] {
            assert_eq!(code(&v, &edit(&a, &[required], None)), ErrorCode::Describe, "without {required}");
        }
        assert_eq!(code(&v, &edit(&a, &["x"], Some(text("y")))), ErrorCode::Describe);
        // object_id is the object's id; short_id its first 12 digits (the vectors: other id, 11 digits).
        let id = str_of(&a, "object_id");
        assert_eq!(code(&v, &edit(&a, &["object_id"], Some(text(&"ab".repeat(32))))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["object_id"], Some(text(&id.to_uppercase())))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["short_id"], Some(text(&id[..11])))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["short_id"], Some(text(&id[1..13])))), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["short_id"], Some(text(&id[..13])))), ErrorCode::Describe);
        // call is the canonical call form of the object.
        let call = r#"scripture.example.org/verse(book: "Gen", chapter: 1, corpus: "tanakh", verse: 2)"#;
        assert_eq!(code(&v, &edit(&a, &["call"], Some(text(call)))), ErrorCode::Describe);
        let compact = r#"scripture.example.org/verse(book:"Gen",chapter:1,corpus:"tanakh",verse:1)"#;
        assert_eq!(code(&v, &edit(&a, &["call"], Some(text(compact)))), ErrorCode::Describe);
        // methods: the class's method names, ascending, and no other (the vector: only `text`).
        for m in [
            r#"["next","consonantal","text"]"#,
            r#"["consonantal","next"]"#,
            r#"["consonantal","next","text","x"]"#,
            "[]",
        ] {
            assert_eq!(code(&v, &edit(&a, &["methods"], Some(parse(m)))), ErrorCode::Describe, "{m}");
        }
        // same_as: object keys (16.5): well-formed encodings.
        assert!(check(&v, &edit(&a, &["same_as"], Some(parse(r#"["bible.example.org/passage(osis:\"Gen.1.1\")"]"#))))
            .is_ok());
        assert_eq!(
            code(&v, &edit(&a, &["same_as"], Some(parse(r#"["bible.example.org/passage(osis:Gen.1.1)"]"#)))),
            ErrorCode::Describe
        );
        assert_eq!(code(&v, &edit(&a, &["same_as"], Some(text("x")))), ErrorCode::Describe);
    }

    #[test]
    fn the_object_of_an_answer_is_one_of_the_held_class() {
        let v = object_vector();
        let a = parse(str_of(&v, "input"));
        // object.class_hash is the held class's hash; the keys are the class's keys, of its types.
        let other = edit(&a, &["object", "class_hash"], Some(text(&"ab".repeat(32))));
        assert_eq!(code(&v, &other), ErrorCode::Describe);
        let missing = edit(&a, &["object", "keys", "verse"], None);
        assert_eq!(code(&v, &missing), ErrorCode::Describe);
        let extra = edit(&a, &["object", "keys", "colour"], Some(parse(r#"{"type":"string","value":"red"}"#)));
        assert_eq!(code(&v, &extra), ErrorCode::Describe);
        let wrong_type = edit(&a, &["object", "keys", "chapter"], Some(parse(r#"{"type":"string","value":"1"}"#)));
        assert_eq!(code(&v, &wrong_type), ErrorCode::Describe);
        // object_id follows from the object: changing a key without changing the id.
        let moved = edit(&a, &["object", "keys", "verse", "value"], Some(text("2")));
        assert_eq!(code(&v, &moved), ErrorCode::Describe);
        assert_eq!(code(&v, &edit(&a, &["object"], None)), ErrorCode::Describe);
    }
}

// ---------------------------------------------------------------------------------------------
// Item 9: the define-time check (2.5)
// ---------------------------------------------------------------------------------------------

mod define {
    use super::*;

    fn descriptor(inputs: &[&str]) -> Value {
        let list: Vec<String> = inputs.iter().map(|i| format!("{i:?}")).collect();
        parse(&format!(
            r#"{{"signature":{{"returns":{{"unit":"1","type":"number"}},"args":{{}}}},"inputs":[{}],"code":{{"lang":"expr","body":""}}}}"#,
            list.join(",")
        ))
    }

    fn run(name: &str, d: &Value) -> datumesh_core::Result<FunctionDescriptor> {
        check_define(name, &d.to_jcs())
    }

    #[test]
    fn every_define_vector() {
        let all = vectors_of_kind("define");
        assert_eq!(all.len(), 35); // 3 valid, 4 invalid; spec-0.6.1 adds 28 (10 valid, 18 invalid)
        for v in &all {
            let id = str_of(v, "id");
            let got = check_define(str_of(v, "name"), str_of(v, "input").as_bytes());
            if is_valid(v) {
                let d = got.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(String::from_utf8(d.canonical()).unwrap(), str_of(v, "canonical"), "{id}");
                assert_eq!(d.hash(), str_of(v, "sha256"), "{id}");
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(v, "error"), "{id}");
            }
        }
    }

    #[test]
    fn a_rooted_function_may_not_read_an_unrooted_one() {
        // 2.5: any host under .invalid is unrooted, path segments too.
        for input in ["local.invalid/fx", "local.invalid/notes/fx", "agent-7.local.invalid/notes/fx", "x.invalid/f"] {
            let e = run("calc.example.net/total", &descriptor(&[input])).unwrap_err();
            assert_eq!(e.code(), ErrorCode::UnrootedInput, "{input}");
        }
        // Also when the unrooted input is not the first (inputs are ascending: after the rooted ones).
        let e = run("calc.example.net/total", &descriptor(&["calc.example.net/a", "local.invalid/b"])).unwrap_err();
        assert_eq!(e.code(), ErrorCode::UnrootedInput);
        // A rooted function reading rooted ones, and an unrooted one reading either, are fine.
        assert!(
            run("calc.example.net/total", &descriptor(&["rates.example.org/fx_rate", "rates.example.org/z"])).is_ok()
        );
        assert!(run("local.invalid/total", &descriptor(&["local.invalid/fx"])).is_ok());
        assert!(run("local.invalid/total", &descriptor(&["rates.example.org/fx_rate"])).is_ok());
        assert!(run(
            "agent-7.local.invalid/notes/total",
            &descriptor(&["local.invalid/notes/fx", "rates.example.org/fx"])
        )
        .is_ok());
        // A rooted function with no inputs, and one whose name merely contains "invalid".
        assert!(run("calc.example.net/total", &descriptor(&[])).is_ok());
        assert!(run("invalid.example.net/total", &descriptor(&["rates.example.org/fx"])).is_ok());
        assert!(run("calc.example.net/total", &descriptor(&["invalid.example.net/fx"])).is_ok());
        assert!(run("calc.example.net/invalid", &descriptor(&["calc.example.net/invalid"])).is_ok());
    }

    #[test]
    fn define_checks_the_name_then_the_descriptor_then_the_rule() {
        let unrooted = descriptor(&["local.invalid/fx"]);
        // Name first (2.3): the descriptor is never looked at.
        for name in
            ["total", "calc.example.net/", "/total", "calc.example.net/Total", "LOCAL.INVALID/f", "invalid/total", ""]
        {
            assert_eq!(run(name, &unrooted).unwrap_err().code(), ErrorCode::CanonName, "{name:?}");
        }
        // The descriptor next (section 7), before the rule of 2.5: unsorted inputs are E_CANON_SET.
        let unsorted = descriptor(&["z.example/b", "local.invalid/a"]);
        assert_eq!(run("calc.example.net/total", &unsorted).unwrap_err().code(), ErrorCode::CanonSet);
        assert_eq!(check_define("calc.example.net/total", b"{").unwrap_err().code(), ErrorCode::ParseJson);
        assert_eq!(check_define("calc.example.net/total", b"[]").unwrap_err().code(), ErrorCode::Schema);
        // Then the rule.
        assert_eq!(run("calc.example.net/total", &unrooted).unwrap_err().code(), ErrorCode::UnrootedInput);
    }

    #[test]
    fn it_is_a_define_code_and_never_a_verification_result() {
        // 10.2: E_UNROOTED_INPUT is define's. The descriptor itself is a valid descriptor: parsing it
        // anywhere else (a verifier reading a stored function, FunctionDescriptor) does not apply the rule.
        let d = descriptor(&["local.invalid/fx"]);
        assert!(FunctionDescriptor::from_json(&d.to_jcs()).is_ok());
        assert_eq!(datumesh_core::function_hash(&d.to_jcs()).unwrap().len(), 64);
        assert_eq!(run("calc.example.net/total", &d).unwrap_err().code().as_str(), "E_UNROOTED_INPUT");
    }
}
