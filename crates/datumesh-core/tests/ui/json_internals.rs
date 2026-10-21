use datumesh_core::json;

fn main() {
    let _ = json::MAX_DEPTH;
    let _ = json::format_number(1.5);
    let _ = json::canonicalize_json(b"[]");
    let v = json::parse(b"[1]").unwrap();
    let _ = json::check_value_depth(&v);
    let _ = v.to_jcs_string();
}
