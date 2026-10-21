//! Receipt encapsulation: outside the crate, a `Receipt` cannot be changed field by field or
//! built by struct literal, so its cached payload and hash can never drift from its members.
//! The only ways to get one are `Receipt::from_json` / `from_value` (checked) and, with
//! feature `sign`, `ReceiptBuilder::sign`.

#[test]
fn receipt_fields_cannot_be_assigned_or_built() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/receipt_*.rs");
}
