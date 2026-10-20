//! Test 3: RFC 8785 Appendix B (all 24 number samples) and the two main examples
//! (3.2.2 and 3.2.3).

use datumesh_core::json::{self, format_number, Number};

const APPENDIX_B: [(u64, &str); 24] = [
    (0x0000000000000000, "0"),
    (0x8000000000000000, "0"),
    (0x0000000000000001, "5e-324"),
    (0x8000000000000001, "-5e-324"),
    (0x7fefffffffffffff, "1.7976931348623157e+308"),
    (0xffefffffffffffff, "-1.7976931348623157e+308"),
    (0x4340000000000000, "9007199254740992"),
    (0xc340000000000000, "-9007199254740992"),
    (0x4430000000000000, "295147905179352830000"),
    (0x44b52d02c7e14af5, "9.999999999999997e+22"),
    (0x44b52d02c7e14af6, "1e+23"),
    (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
    (0x444b1ae4d6e2ef4e, "999999999999999700000"),
    (0x444b1ae4d6e2ef4f, "999999999999999900000"),
    (0x444b1ae4d6e2ef50, "1e+21"),
    (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
    (0x3eb0c6f7a0b5ed8d, "0.000001"),
    (0x41b3de4355555553, "333333333.3333332"),
    (0x41b3de4355555554, "333333333.33333325"),
    (0x41b3de4355555555, "333333333.3333333"),
    (0x41b3de4355555556, "333333333.3333334"),
    (0x41b3de4355555557, "333333333.33333343"),
    (0xbecbf647612f3696, "-0.0000033333333333333333"),
    (0x43143ff3c1cb0959, "1424953923781206.2"),
];

#[test]
fn appendix_b_numbers() {
    for (bits, want) in APPENDIX_B {
        let v = f64::from_bits(bits);
        assert_eq!(format_number(v), want, "{bits:016x}");
        // And through the parser: the shortest text read back serialises the same.
        let text = format!("[{v:e}]");
        assert_eq!(json::canonicalize_json(text.as_bytes()).unwrap(), format!("[{want}]").into_bytes(), "{bits:016x}");
    }
}

#[test]
fn appendix_b_nan_and_infinity_have_no_form() {
    assert!(Number::from_f64(f64::from_bits(0x7fffffffffffffff)).is_none());
    assert!(Number::from_f64(f64::from_bits(0x7ff0000000000000)).is_none());
    assert!(json::parse(b"[NaN]").is_err());
    assert!(json::parse(b"[Infinity]").is_err());
    assert!(json::parse(b"[1e400]").is_err());
}

#[test]
fn main_example_3_2_2() {
    let input = br#"{
  "numbers": [333333333.33333329, 1E30, 4.50,
              2e-3, 0.000000000000000000000000001],
  "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
  "literals": [null, true, false]
}"#;
    let want = "{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}";
    assert_eq!(String::from_utf8(json::canonicalize_json(input).unwrap()).unwrap(), want);
}

#[test]
fn sorting_example_3_2_3() {
    let input = r#"{
  "\u20ac": "Euro Sign",
  "\r": "Carriage Return",
  "\ufb33": "Hebrew Letter Dalet With Dagesh",
  "1": "One",
  "\ud83d\ude00": "Emoji: Grinning Face",
  "\u0080": "Control",
  "\u00f6": "Latin Small Letter O With Diaeresis"
}"#;
    let want = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\",\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\",\"\u{1f600}\":\"Emoji: Grinning Face\",\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}";
    assert_eq!(String::from_utf8(json::canonicalize_json(input.as_bytes()).unwrap()).unwrap(), want);
}
