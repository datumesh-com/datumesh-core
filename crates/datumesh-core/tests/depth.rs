//! SPEC 6.1: the 64-level nesting limit, for texts and for values accepted already parsed.

use datumesh_core::json::{self, check_value_depth, Value};
use datumesh_core::{parse_uri, resolved_unit_value, Cell, Descriptor, ErrorCode, Receipt, Request};

fn nested(depth: usize) -> Value {
    let mut v = Value::Null;
    for _ in 0..depth {
        v = Value::Array(vec![v]);
    }
    v
}

#[test]
fn texts_and_values_share_the_limit() {
    for d in [64, 65] {
        let text = format!("{}{}", "[".repeat(d), "]".repeat(d));
        assert_eq!(json::parse(text.as_bytes()).is_ok(), d <= 64);
    }
    assert!(check_value_depth(&nested(64)).is_ok());
    assert_eq!(check_value_depth(&nested(65)).unwrap_err().code(), ErrorCode::ParseJson);
}

#[test]
fn parsed_value_entry_points_reject_deep_values() {
    let deep = Value::Object(vec![("x".into(), nested(100))]);
    assert_eq!(Cell::from_value(&deep).unwrap_err().code(), ErrorCode::ParseJson);
    assert_eq!(Descriptor::from_value(&deep).unwrap_err().code(), ErrorCode::ParseJson);
    assert_eq!(Receipt::from_value(&deep).unwrap_err().code(), ErrorCode::ParseJson);
    assert_eq!(resolved_unit_value(&deep, &deep).unwrap_err().code(), ErrorCode::ParseJson);
    assert_eq!(parse_uri("cell://a.example/f", &deep).unwrap_err().code(), ErrorCode::ParseJson);
}

#[test]
fn jcs_of_a_very_deep_value_does_not_overflow_the_stack() {
    let mut v = nested(200_000);
    assert_eq!(v.to_jcs().len(), 2 * 200_000 + 4);
    // Unwind iteratively too, so that dropping does not recurse 200,000 levels.
    while let Value::Array(mut items) = v {
        v = items.pop().unwrap_or(Value::Null);
    }
}

#[test]
fn malformed_request_is_not_an_error_code() {
    assert!(Request::new("XYZ", None).is_err());
    assert!(Request::new(&"a".repeat(64), Some("2024-03-16")).is_err());
    assert!(Request::new(&"a".repeat(64), Some("2024-03-16T09:00:00.000000Z")).is_ok());
}
