//! Key normalisation (16.4, `spec-0.5.0`): the closed list of rules a string key or argument may
//! be canonical under, and the pure check of a value against one.

use crate::error::{ErrorCode, Result};
use crate::lexical;

/// One rule of the closed list of 16.4. The list grows only through revisions of the
/// specification, so the enum is `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum KeyNormalisation {
    /// `none`: any string (3.5). Named `NoRule` so that it is not mistaken for `Option::None`.
    NoRule,
    /// `upper_ascii`: printable ASCII only (U+0021 to U+007E), no lowercase `a`-`z`.
    UpperAscii,
    /// `nfc_strict`: no format character (Cf), no White_Space character (Unicode 15.1) other than
    /// U+0020, and no leading, trailing or repeated U+0020. The White_Space characters that 3.5
    /// does not already refuse are U+00A0, U+1680, U+2002-U+200A, U+2028, U+2029, U+202F, U+205F
    /// and U+3000 (U+0009-U+000D and U+0085 are controls; U+2000 and U+2001 are not NFC).
    NfcStrict,
    /// `arabic_skeleton`: no Arabic mark (U+0610-U+061A, U+064B-U+065F, U+0670, U+06D6-U+06DC,
    /// U+06DF-U+06E8, U+06EA-U+06ED) and no tatweel U+0640.
    ArabicSkeleton,
    /// `hebrew_consonantal`: no Hebrew point or accent U+0591-U+05C7 except maqaf U+05BE, paseq
    /// U+05C0, sof pasuq U+05C3 and nun hafukha U+05C6.
    HebrewConsonantal,
    /// `greek_unaccented`: no combining mark U+0300-U+036F in the NFD form.
    GreekUnaccented,
}

impl KeyNormalisation {
    /// Every rule, in the order of the table of 16.4.
    pub const ALL: &'static [KeyNormalisation] = &[
        KeyNormalisation::NoRule,
        KeyNormalisation::UpperAscii,
        KeyNormalisation::NfcStrict,
        KeyNormalisation::ArabicSkeleton,
        KeyNormalisation::HebrewConsonantal,
        KeyNormalisation::GreekUnaccented,
    ];

    /// The rule as written in a descriptor: `none`, `upper_ascii`, `nfc_strict`,
    /// `arabic_skeleton`, `hebrew_consonantal`, `greek_unaccented`.
    pub const fn as_str(self) -> &'static str {
        match self {
            KeyNormalisation::NoRule => "none",
            KeyNormalisation::UpperAscii => "upper_ascii",
            KeyNormalisation::NfcStrict => "nfc_strict",
            KeyNormalisation::ArabicSkeleton => "arabic_skeleton",
            KeyNormalisation::HebrewConsonantal => "hebrew_consonantal",
            KeyNormalisation::GreekUnaccented => "greek_unaccented",
        }
    }

    /// Looks a rule up by its text (exact, lowercase); `None` for anything not in the list.
    pub fn from_name(text: &str) -> Option<KeyNormalisation> {
        KeyNormalisation::ALL.iter().copied().find(|r| r.as_str() == text)
    }

    /// Checks that `text` is canonical under this rule (16.4): a pure function. The text is first
    /// a string value by 3.5 under every rule, with 3.5's codes (`E_CANON_STRING`, `E_CANON_LIMIT`,
    /// never `E_KEY_NORMALISATION`). Then the rule: `none` accepts what 3.5 accepts, the empty
    /// string included; every other rule rejects the empty string, and a value the rule forbids
    /// is `E_KEY_NORMALISATION`, never repaired.
    pub fn check(self, text: &str) -> Result<()> {
        lexical::check_string(text, "$")?;
        if self == KeyNormalisation::NoRule {
            return Ok(());
        }
        let fail = |why: &str| {
            Err(crate::error::Error::new(
                ErrorCode::KeyNormalisation,
                "$",
                format!("not canonical under {}: {why}", self.as_str()),
            ))
        };
        if text.is_empty() {
            return fail("the empty string");
        }
        if !unicode_normalization::is_nfc(text) {
            return fail("not in Normalization Form C");
        }
        let forbidden = |c: char| match self {
            KeyNormalisation::NoRule => false,
            KeyNormalisation::UpperAscii => !(('!'..='~').contains(&c) && !c.is_ascii_lowercase()),
            KeyNormalisation::NfcStrict => is_format_character(c) || (is_white_space(c) && c != ' '),
            KeyNormalisation::ArabicSkeleton => ARABIC_MARKS.iter().any(|r| r.contains(&(c as u32))),
            KeyNormalisation::HebrewConsonantal => {
                let cp = c as u32;
                (0x0591..=0x05C7).contains(&cp) && !matches!(cp, 0x05BE | 0x05C0 | 0x05C3 | 0x05C6)
            }
            KeyNormalisation::GreekUnaccented => false, // checked on the NFD form below
        };
        if let Some(c) = text.chars().find(|&c| forbidden(c)) {
            return fail(&format!("forbidden character U+{:04X}", c as u32));
        }
        if self == KeyNormalisation::NfcStrict && (text.starts_with(' ') || text.ends_with(' ') || text.contains("  "))
        {
            return fail("a leading, trailing or repeated space");
        }
        if self == KeyNormalisation::GreekUnaccented {
            use unicode_normalization::UnicodeNormalization;
            if text.nfd().any(|c| ('\u{0300}'..='\u{036F}').contains(&c)) {
                return fail("a combining mark in its decomposed form");
            }
        }
        Ok(())
    }
}

impl core::fmt::Display for KeyNormalisation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The Arabic marks of 16.4 (`arabic_skeleton`): the vowel signs, the Quranic annotation signs
/// and tatweel.
const ARABIC_MARKS: &[core::ops::RangeInclusive<u32>] = &[
    0x0610..=0x061A,
    0x064B..=0x065F,
    0x0670..=0x0670,
    0x06D6..=0x06DC,
    0x06DF..=0x06E8,
    0x06EA..=0x06ED,
    0x0640..=0x0640,
];

/// Format characters (general category Cf) of Unicode 15.1, as 16.4's `nfc_strict` names them.
/// The ranges are those of the Unicode database, checked against Python 3.13's `unicodedata`
/// (Unicode 15.1.0): every Cf code point, and no other.
const FORMAT_CHARACTERS: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// The White_Space property of Unicode 15.1 (the list of 16.4 `nfc_strict`).
fn is_white_space(c: char) -> bool {
    matches!(
        c as u32,
        0x0009..=0x000D
            | 0x0020
            | 0x0085
            | 0x00A0
            | 0x1680
            | 0x2000..=0x200A
            | 0x2028
            | 0x2029
            | 0x202F
            | 0x205F
            | 0x3000
    )
}

fn is_format_character(c: char) -> bool {
    let cp = c as u32;
    FORMAT_CHARACTERS.iter().any(|&(lo, hi)| (lo..=hi).contains(&cp))
}
