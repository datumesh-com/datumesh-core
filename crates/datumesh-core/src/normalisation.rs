//! Key normalisation (16.4, `spec-0.5.0`): the closed list of rules a string key or argument may
//! be canonical under, and the pure check of a value against one.

use crate::error::Result;

/// One rule of the closed list of 16.4. The list grows only through revisions of the
/// specification, so the enum is `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum KeyNormalisation {
    /// `none`: any string (3.5). Named `NoRule` so that it is not mistaken for `Option::None`.
    NoRule,
    /// `upper_ascii`: printable ASCII only (U+0021 to U+007E), no lowercase `a`-`z`.
    UpperAscii,
    /// `nfc_strict`: no format character (Cf), no line or paragraph separator, no space other
    /// than U+0020, and no leading, trailing or repeated U+0020.
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

    /// Checks that `text` is canonical under this rule (16.4): a pure function. A value that is
    /// not canonical is `E_KEY_NORMALISATION`, never repaired. Every rule but `none` rejects the
    /// empty string and a value that is not NFC.
    pub fn check(self, text: &str) -> Result<()> {
        let _ = (self, text);
        // STUB (0.3.8 tests first): the implementation lives here.
        todo!("16.4: check a value against a key normalisation rule")
    }
}

impl core::fmt::Display for KeyNormalisation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
