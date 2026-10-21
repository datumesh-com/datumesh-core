use datumesh_core::Receipt;

fn main() {
    let mut r = Receipt::from_json(b"{}").unwrap();
    r.sig = String::new();
}
