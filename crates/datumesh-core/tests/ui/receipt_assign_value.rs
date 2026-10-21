use datumesh_core::{Receipt, Scalar};

fn main() {
    let mut r = Receipt::from_json(b"{}").unwrap();
    r.value = Scalar::Number("2".into());
}
