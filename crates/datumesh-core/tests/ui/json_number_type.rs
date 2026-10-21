use datumesh_core::json::{Number, Value};

fn main() {
    let _ = Value::Number(Number::from_f64(1.0).unwrap());
}
