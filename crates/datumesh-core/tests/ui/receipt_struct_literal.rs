use datumesh_core::{Cell, Receipt, Scalar};

fn main() {
    let cell = Cell::from_json(b"{}").unwrap();
    let _ = Receipt {
        asof: String::new(),
        cell,
        computed_by: String::new(),
        inputs: Vec::new(),
        issued: String::new(),
        licence: String::new(),
        publisher: String::new(),
        sig: String::new(),
        summary: String::new(),
        tx: String::new(),
        value: Scalar::Boolean(true),
        valid: Default::default(),
    };
}
