//! Licences of computed receipts (`spec-0.4.3`): a publisher's declarations (9.2), licence
//! inheritance (8.5) and the entitlement check (10.3).
//!
//! Inheritance is the rule the engine applies when it signs a computed receipt; the
//! entitlement check is the same rule run by a verifier on a receipt it has verified. Both are
//! pure functions: no network, no clock, no verification.

use std::collections::BTreeMap;

use crate::error::{err, index, ErrorCode, Result};
use crate::json::Value;
use crate::keys::KeyDocument;
use crate::lexical::{self, limits};
use crate::receipt::Receipt;
use crate::schema;

const MAX_NAMES: usize = limits::DECLARATIONS;
const MAX_ENTRIES: usize = limits::DECLARATION_ENTRIES;

/// One dated entry of a publisher's declarations (9.2, `spec-0.4.3`): its function may be
/// computed over inputs under `licence` while the entry is open, the result carrying that
/// licence (8.5). Entries are dated as keys are, and never removed, only closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Declaration {
    /// The declared licence, by the grammar and limits of 8.1, compared byte for byte.
    pub licence: String,
    /// Start of the entry's window (inclusive), an instant.
    pub valid_from: String,
    /// End of the window (exclusive), later than `valid_from`; `None` is open-ended.
    pub valid_to: Option<String>,
}

impl Declaration {
    /// An entry, checked as 9.2 checks one in a key document: `licence` by 8.1, `valid_from` an
    /// instant, `valid_to` an instant later than it. Every failure is `E_KEY_DOC`.
    pub fn new(licence: &str, valid_from: &str, valid_to: Option<&str>) -> Result<Declaration> {
        entry_fields(licence, valid_from, valid_to, "$")
    }

    /// Whether the entry is open at `issued`: `valid_from <= issued`, and `issued < valid_to`
    /// when `valid_to` is present (the window of a key, 9.5).
    pub fn open_at(&self, issued: &str) -> bool {
        self.valid_from.as_str() <= issued && self.valid_to.as_deref().is_none_or(|to| issued < to)
    }

    fn to_value(&self) -> Value {
        let s = |x: &str| Value::String(x.into());
        let mut m = vec![("licence".into(), s(&self.licence)), ("valid_from".into(), s(&self.valid_from))];
        if let Some(t) = &self.valid_to {
            m.push(("valid_to".into(), s(t)));
        }
        Value::Object(m)
    }
}

fn entry_fields(licence: &str, from: &str, to: Option<&str>, p: &str) -> Result<Declaration> {
    let kd = ErrorCode::KeyDoc;
    lexical::check_licence(licence, &format!("{p}.licence")).map_err(|e| e.recode(kd))?;
    if !lexical::is_instant(from) {
        return err(kd, &format!("{p}.valid_from"), "valid_from must be an instant");
    }
    if let Some(t) = to {
        if !lexical::is_instant(t) {
            return err(kd, &format!("{p}.valid_to"), "valid_to must be an instant");
        }
        if t <= from {
            return err(kd, &format!("{p}.valid_to"), "valid_to must be after valid_from");
        }
    }
    Ok(Declaration { licence: licence.into(), valid_from: from.into(), valid_to: to.map(String::from) })
}

/// A publisher's declarations (9.2, `spec-0.4.3`): per full function name (2.3), the dated
/// entries naming the restricted licences the publisher permits as inputs of that function.
/// Read from the key document of the host of the function's namespace ([`KeyDocument`]).
/// Names are kept in ascending order; each name's entries in document order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Declarations {
    names: BTreeMap<String, Vec<Declaration>>,
}

impl Declarations {
    /// No declarations.
    pub fn new() -> Declarations {
        Declarations::default()
    }

    /// Declarations from the JSON value of a `declarations` member: an object of at most 64 full
    /// function names, each to an array of 1 to 16 entries with exactly `licence`, `valid_from`
    /// and optionally `valid_to`, all strings, checked as in 9.2 (`E_KEY_DOC`). Two rules are
    /// the key document's and not checked here, because the value alone cannot tell them: that
    /// every name is under the document's host, and that the object is not empty (`{}` gives no
    /// declarations, as the `licence` vectors write none). [`KeyDocument::from_json`] checks
    /// both.
    pub fn from_value(v: &Value) -> Result<Declarations> {
        parse(v, "$")
    }

    /// Adds an entry for the function `name`, a full function name (2.3), keeping the limits
    /// of 4.5 (64 names, 16 entries for one name). Every failure is `E_KEY_DOC`.
    pub fn insert(&mut self, name: &str, entry: Declaration) -> Result<()> {
        lexical::check_function_name(name, "$").map_err(|e| e.recode(ErrorCode::KeyDoc))?;
        if !self.names.contains_key(name) && self.names.len() >= MAX_NAMES {
            return err(ErrorCode::KeyDoc, "$", "at most 64 declared names");
        }
        let entries = self.names.entry(name.into()).or_default();
        if entries.len() >= MAX_ENTRIES {
            return err(ErrorCode::KeyDoc, "$", "at most 16 entries for one name");
        }
        entries.push(entry);
        Ok(())
    }

    /// The entries for the function `name` (`cell.ns + "/" + cell.fn`); empty when none.
    pub fn get(&self, name: &str) -> &[Declaration] {
        self.names.get(name).map_or(&[], Vec::as_slice)
    }

    /// The declared function names, in ascending order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(String::as_str)
    }

    /// Each declared name with its entries, names in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[Declaration])> {
        self.names.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// The number of declared names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether nothing is declared.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The JSON form of the `declarations` member (members in ascending name order).
    pub fn to_value(&self) -> Value {
        Value::Object(
            self.names
                .iter()
                .map(|(k, es)| (k.clone(), Value::Array(es.iter().map(Declaration::to_value).collect())))
                .collect(),
        )
    }

    /// 9.2: not empty, and every name's namespace's host is `host` exactly (a subdomain is
    /// another host).
    pub(crate) fn check_host(&self, host: &str, path: &str) -> Result<()> {
        if self.names.is_empty() {
            return err(ErrorCode::KeyDoc, path, "declarations holds 1 to 64 names");
        }
        for name in self.names.keys() {
            if lexical::host_of(name) != host {
                return err(
                    ErrorCode::KeyDoc,
                    &format!("{path}.{name}"),
                    format!("a declared name is under the document's own host {host}"),
                );
            }
        }
        Ok(())
    }
}

fn parse(v: &Value, path: &str) -> Result<Declarations> {
    let kd = ErrorCode::KeyDoc;
    let Value::Object(m) = v else {
        return err(kd, path, "declarations must be an object of names");
    };
    if m.len() > MAX_NAMES {
        return err(kd, path, "declarations holds at most 64 names");
    }
    let mut names = BTreeMap::new();
    for (name, entries) in m {
        let p = format!("{path}.{name}");
        lexical::check_function_name(name, &p).map_err(|e| e.recode(kd))?;
        let items = match entries {
            Value::Array(a) if !a.is_empty() && a.len() <= MAX_ENTRIES => a,
            _ => return err(kd, &p, "a declared name has 1 to 16 entries"),
        };
        let mut out = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let ep = index(&p, i);
            let em = schema::object_or(item, &ep, kd, "an entry")?;
            schema::exact(em, &["licence", "valid_from"], &["valid_to"], &ep, kd)?;
            let licence = schema::string_or(em, "licence", &ep, kd)?;
            let from = schema::string_or(em, "valid_from", &ep, kd)?;
            let to = match schema::opt(em, "valid_to") {
                None => None,
                Some(_) => Some(schema::string_or(em, "valid_to", &ep, kd)?),
            };
            out.push(entry_fields(licence, from, to, &ep)?);
        }
        names.insert(name.clone(), out);
    }
    Ok(Declarations { names })
}

/// The `declarations` member of a key document fetched from `host` (9.2).
pub(crate) fn declarations_member(v: &Value, host: &str) -> Result<Declarations> {
    let d = parse(v, "$.declarations")?;
    d.check_host(host, "$.declarations")?;
    Ok(d)
}

/// The named outcome of licence inheritance (8.5) and of the entitlement check (10.3): a
/// string, not an error code. Three reasons give a licence (entitled), five refuse (not
/// entitled).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LicenceReason {
    /// No input receipt: the mesh's own licence (a stored receipt, a constant, only empty
    /// slices).
    NoInputs,
    /// Open inputs only: the highest tier, and within it the identifier listed last.
    Open,
    /// One restricted licence, declared for the function and open at `issued`.
    Declared,
    /// A share-alike licence with a different share-alike licence or with any restricted one.
    ShareAlikeConflict,
    /// Two or more different restricted licences.
    RestrictedConflict,
    /// One restricted licence, with no declaration entry for the function naming it.
    NotDeclared,
    /// One restricted licence, declared for the function, but no such entry open at `issued`.
    DeclarationNotOpen,
    /// The entitlement check only: the rule gives a licence other than the receipt's.
    LicenceMismatch,
}

impl LicenceReason {
    /// Every reason, in the order of SPEC 10.3.
    pub const ALL: &'static [LicenceReason] = &[
        LicenceReason::NoInputs,
        LicenceReason::Open,
        LicenceReason::Declared,
        LicenceReason::ShareAlikeConflict,
        LicenceReason::RestrictedConflict,
        LicenceReason::NotDeclared,
        LicenceReason::DeclarationNotOpen,
        LicenceReason::LicenceMismatch,
    ];

    /// The reason as written, e.g. `"share_alike_conflict"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            LicenceReason::NoInputs => "no_inputs",
            LicenceReason::Open => "open",
            LicenceReason::Declared => "declared",
            LicenceReason::ShareAlikeConflict => "share_alike_conflict",
            LicenceReason::RestrictedConflict => "restricted_conflict",
            LicenceReason::NotDeclared => "not_declared",
            LicenceReason::DeclarationNotOpen => "declaration_not_open",
            LicenceReason::LicenceMismatch => "licence_mismatch",
        }
    }

    /// Looks a reason up by its name.
    pub fn from_name(text: &str) -> Option<LicenceReason> {
        LicenceReason::ALL.iter().copied().find(|r| r.as_str() == text)
    }

    /// Whether the reason is one of an entitled receipt (`no_inputs`, `open`, `declared`): the
    /// rule gave a licence.
    pub const fn entitled(self) -> bool {
        matches!(self, LicenceReason::NoInputs | LicenceReason::Open | LicenceReason::Declared)
    }
}

impl core::fmt::Display for LicenceReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The result of licence inheritance (8.5): the licence of the receipt to be issued, or a
/// refusal (`licence` `None`: the mesh answers `unlicensed` and signs nothing), with the reason.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Inheritance {
    /// The result's licence; `None` when the mesh may not compute the cell.
    pub licence: Option<String>,
    /// Why: `no_inputs`, `open` or `declared` with a licence; `share_alike_conflict`,
    /// `restricted_conflict`, `not_declared` or `declaration_not_open` without.
    pub reason: LicenceReason,
}

impl Inheritance {
    /// Whether the mesh may not compute the cell (it answers `unlicensed`, 5.5).
    pub fn refused(&self) -> bool {
        self.licence.is_none()
    }
}

/// The open tiers of 8.5, least to most strict, each in its listed order.
const TIERS: [&[&str]; 3] =
    [&["CC0-1.0", "PDDL-1.0"], &["MIT", "Apache-2.0", "CC-BY-4.0", "OGL-UK-3.0"], &["CC-BY-SA-4.0", "ODbL-1.0"]];

/// The tier of a licence (0 public domain, 1 attribution, 2 share-alike) and its place in it;
/// `None` for a restricted licence. Byte for byte: `cc0-1.0` is restricted.
fn open_rank(licence: &str) -> Option<(usize, usize)> {
    TIERS.iter().enumerate().find_map(|(t, ids)| ids.iter().position(|x| *x == licence).map(|i| (t, i)))
}

/// Licence inheritance (SPEC 8.5, `spec-0.4.3`): the licence of a computed receipt for the
/// function `function` (its full name, `cell.ns + "/" + cell.fn`) to be issued at `issued`,
/// from `licences`, the licences of its input receipts (every receipt hash in `inputs` and
/// every member of each slice entry's manifest, one per receipt, repeats allowed),
/// `declarations`, those of the function's publisher (the key document of the host of
/// `cell.ns`, whatever host signs; `None` or empty when it has none), and `mesh_licence`, the
/// mesh's own licence for a cell with no input receipt.
///
/// The five steps of 8.5, in order: no inputs (`no_inputs`); a share-alike licence with a
/// different share-alike one or with any restricted one (`share_alike_conflict`, declared or
/// not); two different restricted licences (`restricted_conflict`); one restricted licence,
/// which needs an entry for `function` naming it exactly and open at `issued` (`declared`,
/// else `not_declared` or `declaration_not_open`); open inputs only, the highest tier present
/// and within it the identifier listed last (`open`). Licences are compared byte for byte and
/// not checked against 8.1's grammar: any string not among the eight open identifiers is
/// restricted. `issued` is compared as text, as instants order.
///
/// It lives in this crate because the engine applies it when it signs (and refuses with
/// `unlicensed`), while [`check_entitlement`] applies it on the verifier's side; both must give
/// the same answer for the same receipt.
pub fn inherit_licence<I, S>(
    licences: I,
    declarations: Option<&Declarations>,
    function: &str,
    issued: &str,
    mesh_licence: &str,
) -> Inheritance
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let all: Vec<S> = licences.into_iter().collect();
    let give = |l: &str, reason| Inheritance { licence: Some(l.into()), reason };
    let refuse = |reason| Inheritance { licence: None, reason };
    // 1.
    if all.is_empty() {
        return give(mesh_licence, LicenceReason::NoInputs);
    }
    let mut share_alike: Vec<&str> = Vec::new();
    let mut restricted: Vec<&str> = Vec::new();
    let mut best: Option<(usize, usize)> = None;
    for l in &all {
        let l = l.as_ref();
        match open_rank(l) {
            Some((2, _)) if !share_alike.contains(&l) => share_alike.push(l),
            None if !restricted.contains(&l) => restricted.push(l),
            _ => {}
        }
        if let Some(r) = open_rank(l) {
            best = best.max(Some(r));
        }
    }
    // 2.
    if share_alike.len() > 1 || (!share_alike.is_empty() && !restricted.is_empty()) {
        return refuse(LicenceReason::ShareAlikeConflict);
    }
    // 3.
    if restricted.len() > 1 {
        return refuse(LicenceReason::RestrictedConflict);
    }
    // 4.
    if let Some(r) = restricted.first() {
        let entries: Vec<&Declaration> =
            declarations.map_or(&[][..], |d| d.get(function)).iter().filter(|e| e.licence == *r).collect();
        if entries.is_empty() {
            return refuse(LicenceReason::NotDeclared);
        }
        if !entries.iter().any(|e| e.open_at(issued)) {
            return refuse(LicenceReason::DeclarationNotOpen);
        }
        return give(r, LicenceReason::Declared);
    }
    // 5. Open inputs only; (tier, place) orders by tier first, then by the place listed.
    match best {
        Some((t, i)) => give(TIERS[t][i], LicenceReason::Open),
        None => refuse(LicenceReason::NotDeclared), // unreachable: an input is open or restricted
    }
}

/// The answer of the entitlement check (10.3): whether the receipt's licence is the one the
/// inheritance rule gives, with the reason. It is not a verification result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Entitlement {
    /// Entitled or not.
    pub entitled: bool,
    /// One of the eight reasons; [`LicenceReason::entitled`] agrees with `entitled`.
    pub reason: LicenceReason,
}

/// The key document given to [`check_entitlement`] is not that of the receipt's publisher (the
/// host of `cell.ns`): the caller's error (10.3), not a reason.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WrongPublisher {
    expected: String,
    got: String,
}

impl WrongPublisher {
    /// The publisher's host: the host of the receipt's `cell.ns`.
    pub fn expected(&self) -> &str {
        &self.expected
    }

    /// The host of the key document given.
    pub fn got(&self) -> &str {
        &self.got
    }
}

impl core::fmt::Display for WrongPublisher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "the entitlement check needs the key document of {}, not of {}", self.expected, self.got)
    }
}

impl std::error::Error for WrongPublisher {}

/// The entitlement check (SPEC 10.3, `spec-0.4.3`), a pure function of R, L and K: `receipt`
/// (R, which the caller has verified through step 6 of 10.1), `input_licences` (L, the licences
/// of R's input receipts, slice members included, one per receipt, as step 7.5 obtains them)
/// and `key_document` (K, the checked key document of R's publisher, the host of
/// `R.cell.ns`, even when `computed_by` names another host).
///
/// Applies [`inherit_licence`] to L, K's declarations, `R.cell.ns + "/" + R.cell.fn` and
/// `R.issued`, with `R.licence` as the mesh's own licence: a refusal is not entitled with the
/// refusal's reason; a licence other than `R.licence` (byte for byte) is not entitled,
/// `licence_mismatch`; otherwise entitled with the rule's reason. A K of another host is
/// [`WrongPublisher`].
pub fn check_entitlement<I, S>(
    receipt: &Receipt,
    input_licences: I,
    key_document: &KeyDocument,
) -> core::result::Result<Entitlement, WrongPublisher>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let cell = receipt.cell();
    let publisher = lexical::host_of(&cell.ns);
    if key_document.host() != publisher {
        return Err(WrongPublisher { expected: publisher.into(), got: key_document.host().into() });
    }
    let function = format!("{}/{}", cell.ns, cell.fn_name);
    let rule = inherit_licence(
        input_licences,
        key_document.declarations.as_ref(),
        &function,
        receipt.issued(),
        receipt.licence(),
    );
    Ok(match rule.licence {
        None => Entitlement { entitled: false, reason: rule.reason },
        Some(l) if l != receipt.licence() => Entitlement { entitled: false, reason: LicenceReason::LicenceMismatch },
        Some(_) => Entitlement { entitled: true, reason: rule.reason },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_ranks_order_by_tier_then_place() {
        assert_eq!(open_rank("CC0-1.0"), Some((0, 0)));
        assert_eq!(open_rank("OGL-UK-3.0"), Some((1, 3)));
        assert_eq!(open_rank("ODbL-1.0"), Some((2, 1)));
        assert_eq!(open_rank("cc0-1.0"), None);
        assert!(Some((1, 0)) > Some((0, 1)));
    }
}
