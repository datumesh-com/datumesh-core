//! Error codes (SPEC 10.2) and the error type every fallible function returns.

use core::fmt;

macro_rules! codes {
    ($( $(#[$doc:meta])* $variant:ident = $text:literal, )*) => {
        /// One variant per `E_*` code of SPEC 10.2 (`spec-0.4.0`).
        /// Non-exhaustive: codes added by later revisions of the specification are additions.
        ///
        /// The variants are declared in the order of 10.2, except the codes added within
        /// 0.3.x, which follow in the order they were added so that [`ErrorCode::ALL`] and the
        /// derived ordering only grow: `EvalUnavailable` (0.3.0; 10.2 lists it between
        /// `E_VALUE_MISMATCH` and `E_BLOB_MISMATCH`), then `SliceMember` and `InputSuperseded`
        /// (0.3.4, `spec-0.4.0`; 10.2 lists them after `E_INPUT_UNDECLARED` and after
        /// `E_EVAL_UNAVAILABLE`). The formula codes of SPEC 15.11 are not verification codes
        /// and have no variant; the envelope's `fault` is a [`crate::Fault`].
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum ErrorCode {
            $( $(#[$doc])* $variant, )*
        }

        impl ErrorCode {
            /// Every code, in the order of SPEC 10.2 except the codes added within 0.3.x, which
            /// come last (see [`ErrorCode`]).
            pub const ALL: &'static [ErrorCode] = &[$(ErrorCode::$variant,)*];

            /// The code as written in the specification, e.g. `"E_SIGNATURE"`.
            pub const fn as_str(self) -> &'static str {
                match self { $(ErrorCode::$variant => $text,)* }
            }

            /// Looks a code up by its text; `None` for anything not in [`ErrorCode::ALL`].
            pub fn from_code(text: &str) -> Option<ErrorCode> {
                match text { $($text => Some(ErrorCode::$variant),)* _ => None }
            }
        }
    };
}

codes! {
    /// Not strict JSON (6.1).
    ParseJson = "E_PARSE_JSON",
    /// A member name repeated in one object.
    ParseDuplicateKey = "E_PARSE_DUPLICATE_KEY",
    /// Not a cell URI by the outer grammar (5.1).
    ParseUri = "E_PARSE_URI",
    /// Missing or extra member, wrong JSON type, wrong `proto`, null; content for a non-blob.
    Schema = "E_SCHEMA",
    /// Invalid DNS name, namespace, function name, code language or key reference.
    CanonName = "E_CANON_NAME",
    /// Invalid or reserved argument name.
    CanonArgName = "E_CANON_ARG_NAME",
    /// An argument or `asof` repeated in a URI.
    CanonArgDuplicate = "E_CANON_ARG_DUPLICATE",
    /// URI arguments out of order, or `asof` not last.
    CanonArgOrder = "E_CANON_ARG_ORDER",
    /// Non-canonical percent-encoding or non-UTF-8 bytes.
    CanonPercent = "E_CANON_PERCENT",
    /// Unknown type name, value of the wrong JSON type, or a bad valid-time declaration.
    CanonType = "E_CANON_TYPE",
    /// Number not in canonical decimal form.
    CanonNumber = "E_CANON_NUMBER",
    /// A unit rule of 7.1 or `units.md` broken.
    CanonUnit = "E_CANON_UNIT",
    /// Date not `YYYY-MM-DD` or not a calendar day.
    CanonDate = "E_CANON_DATE",
    /// Instant not `YYYY-MM-DDTHH:MM:SS.ffffffZ` or not a real time.
    CanonInstant = "E_CANON_INSTANT",
    /// Control, unassigned or surrogate code point, not NFC, or empty summary.
    CanonString = "E_CANON_STRING",
    /// Blob not `<sha256>:<length>:<media type>`.
    CanonBlob = "E_CANON_BLOB",
    /// Hash, key id, signature or slice entry not lowercase hex of the right length.
    CanonHex = "E_CANON_HEX",
    /// Licence neither an SPDX identifier nor an `https://` URL.
    CanonLicence = "E_CANON_LICENCE",
    /// `inputs`, an extent or manifest members not strictly ascending; `order` not every
    /// argument exactly once.
    CanonSet = "E_CANON_SET",
    /// A limit from 4.5 exceeded.
    CanonLimit = "E_CANON_LIMIT",
    /// A slice marker where a single cell is required; a slice object without an open or
    /// ranged argument.
    Slice = "E_SLICE",
    /// `publisher` differs from `cell.ns`.
    PublisherMismatch = "E_PUBLISHER_MISMATCH",
    /// `valid` does not echo exactly the valid-time arguments.
    ValidMismatch = "E_VALID_MISMATCH",
    /// A time-order rule broken.
    TimeOrder = "E_TIME_ORDER",
    /// An argument the function does not declare.
    ArgUnknown = "E_ARG_UNKNOWN",
    /// A declared argument not bound.
    ArgMissing = "E_ARG_MISSING",
    /// The key document could not be retrieved.
    KeyUnavailable = "E_KEY_UNAVAILABLE",
    /// The key document is malformed or breaks a rule of 9.2.
    KeyDoc = "E_KEY_DOC",
    /// The key document has no key with that id.
    KeyUnknown = "E_KEY_UNKNOWN",
    /// `issued` is outside the key's validity window.
    KeyNotValid = "E_KEY_NOT_VALID",
    /// The Ed25519 signature does not verify.
    Signature = "E_SIGNATURE",
    /// No descriptor found for `fn_hash`.
    FnUnavailable = "E_FN_UNAVAILABLE",
    /// The descriptor does not hash to `fn_hash`.
    FnHashMismatch = "E_FN_HASH_MISMATCH",
    /// An argument's type differs from its declaration.
    ArgType = "E_ARG_TYPE",
    /// `type` differs from the declared return type.
    ReturnType = "E_RETURN_TYPE",
    /// An input receipt cannot be found, or has another hash.
    InputUnavailable = "E_INPUT_UNAVAILABLE",
    /// An input receipt is not signature-valid.
    InputInvalid = "E_INPUT_INVALID",
    /// An input receipt, or a slice, is of a function not in the descriptor's `inputs`.
    InputUndeclared = "E_INPUT_UNDECLARED",
    /// Recomputation gives a different type or value.
    ValueMismatch = "E_VALUE_MISMATCH",
    /// Blob content has another length or hash than the blob states.
    BlobMismatch = "E_BLOB_MISMATCH",
    /// A response envelope breaks a rule of 5.5.
    Envelope = "E_ENVELOPE",
    /// The verifier could not evaluate the function (10.1 step 7.6): no runtime for its
    /// language, a budget exceeded, or inputs the evaluator cannot obtain. Not a failure: it
    /// is reported with success at level `signed` (see [`crate::VerifyResult`]). A mesh may
    /// carry it in an `error` envelope (5.5). In 10.2 since `spec-0.3.0` (RS-28).
    EvalUnavailable = "E_EVAL_UNAVAILABLE",
    /// A member receipt does not belong to its slice, or two members of a membership slice
    /// are one cell (10.1 step 7.5, `spec-0.4.0`).
    SliceMember = "E_SLICE_MEMBER",
    /// Recomputation read a later version of an input cell, or other members of an input
    /// slice (10.1 step 7.6, `spec-0.4.0`): the receipt is historical, not wrong. Not a
    /// failure: it is reported with success at level `signed`, at path `$.inputs` (see
    /// [`crate::VerifyResult`]).
    InputSuperseded = "E_INPUT_SUPERSEDED",
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A failure: its code, the JSON path (or URI part) where it was found, and a human detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    code: ErrorCode,
    path: String,
    detail: String,
}

impl Error {
    /// Builds an error. `path` is a JSONPath-like location such as `$.cell.args.base`.
    pub fn new(code: ErrorCode, path: impl Into<String>, detail: impl Into<String>) -> Error {
        Error { code, path: path.into(), detail: detail.into() }
    }

    /// The code of SPEC 10.2.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// The code as text, e.g. `"E_SIGNATURE"`.
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    /// Where the failure was found: `$` is the whole input, `$.a.b[0]` a member.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// A human-readable explanation. Not stable; do not match on it.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The same failure reported under another code (e.g. wrapping as `E_KEY_DOC`).
    pub(crate) fn recode(self, code: ErrorCode) -> Error {
        let detail = if self.code == code { self.detail } else { format!("{}: {}", self.code, self.detail) };
        Error { code, path: self.path, detail }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.path, self.detail)
    }
}

impl std::error::Error for Error {}

/// Shorthand used throughout the crate.
pub type Result<T> = core::result::Result<T, Error>;

pub(crate) fn err<T>(code: ErrorCode, path: &str, detail: impl Into<String>) -> Result<T> {
    Err(Error::new(code, path, detail))
}

/// Appends a member name to a path.
pub(crate) fn member(path: &str, name: &str) -> String {
    format!("{path}.{name}")
}

/// Appends an array index to a path.
pub(crate) fn index(path: &str, i: usize) -> String {
    format!("{path}[{i}]")
}
