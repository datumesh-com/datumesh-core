//! The embedded unit list and Unicode 15.1 table equal the spec's files
//! (regenerate with `python3 tools/gen_tables.py`).

mod common;

use datumesh_core::lexical::{is_assigned_15_1, is_currency, is_literal_unit};

fn fenced_blocks(text: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line == format!("```{tag}") {
            inside = true;
        } else if inside && line.starts_with("```") {
            inside = false;
        } else if inside {
            out.extend(line.split_whitespace().map(String::from));
        }
    }
    out
}

#[test]
fn units_match_units_md() {
    let text = std::fs::read_to_string(common::spec("units.md")).unwrap();
    let currencies = fenced_blocks(&text, "units-currency");
    let physical = fenced_blocks(&text, "units-physical");
    let ratio = fenced_blocks(&text, "units-ratio");
    assert!(currencies.len() > 150);
    for c in &currencies {
        assert!(is_currency(c) && is_literal_unit(c), "{c}");
    }
    for u in physical.iter().chain(&ratio) {
        assert!(is_literal_unit(u) && !is_currency(u), "{u}");
    }
    // And nothing else: the generated file holds exactly these lists, in file order.
    let generated = include_str!("../src/tables.rs");
    let listed = |name: &str| -> Vec<String> {
        let start = generated.find(&format!("const {name}: &[&str] = &[")).unwrap();
        let end = start + generated[start..].find("];").unwrap();
        generated[start..end].lines().skip(1).map(|l| l.trim().trim_end_matches(',').trim_matches('"').to_string()).collect()
    };
    assert_eq!(listed("CURRENCIES"), currencies);
    assert_eq!(listed("PHYSICAL"), physical);
    assert_eq!(listed("RATIOS"), ratio);
    for not_unit in ["usd", "°C", "W/m²", "USD/EUR", "XXX", "ABC", ""] {
        assert!(!is_literal_unit(not_unit), "{not_unit}");
    }
}

#[test]
fn assigned_table_matches_unicode_file() {
    let text = std::fs::read_to_string(common::spec("tools/unicode-15.1-assigned.txt")).unwrap();
    let mut ranges = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let (a, b) = line.split_once("..").unwrap();
        ranges.push((u32::from_str_radix(a, 16).unwrap(), u32::from_str_radix(b, 16).unwrap()));
    }
    let mut assigned = vec![false; 0x110000];
    for (a, b) in &ranges {
        for cp in *a..=*b {
            assigned[cp as usize] = true;
        }
    }
    for cp in 0..0x110000u32 {
        if let Some(c) = char::from_u32(cp) {
            assert_eq!(is_assigned_15_1(c), assigned[cp as usize], "U+{cp:04X}");
        }
    }
    // Independent of the toolchain: U+1C89 (Unicode 16.0) is not assigned in 15.1.
    assert!(!is_assigned_15_1('\u{1C89}'));
}
