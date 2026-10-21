//! The public surface of `json`: `parse`, and `Value` with `get`, `as_str`, `as_object`,
//! `as_number_text` and `to_jcs`.

use datumesh_core::json::{self, Value};

#[test]
fn numbers_are_read_as_their_text() {
    let v = json::parse(br#"{"a":[1.50,200,-0,1E3],"b":"200","c":null}"#).unwrap();
    let Some(Value::Array(items)) = v.get("a") else { panic!() };
    let texts: Vec<Option<&str>> = items.iter().map(Value::as_number_text).collect();
    assert_eq!(texts, [Some("1.50"), Some("200"), Some("-0"), Some("1E3")]);
    assert_eq!(v.get("b").and_then(Value::as_number_text), None);
    assert_eq!(v.get("c").and_then(Value::as_number_text), None);
    assert!(matches!(&items[0], Value::Number(_)));
    // JCS formats the double, not the text.
    assert_eq!(v.to_jcs(), br#"{"a":[1.5,200,0,1000],"b":"200","c":null}"#);
}

#[test]
fn numbers_can_be_moved_between_values() {
    let v = json::parse(b"[7]").unwrap();
    let Value::Array(mut items) = v else { panic!() };
    let n = items.pop().unwrap();
    let wrapped = Value::Object(vec![("n".into(), n)]);
    assert_eq!(wrapped.to_jcs(), br#"{"n":7}"#);
    assert_eq!(wrapped.get("n").and_then(Value::as_number_text), Some("7"));
}
