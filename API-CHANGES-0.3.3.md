# Public API changes, datumesh-core 0.3.2 to 0.3.3

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.2` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.2` (cargo-public-api 0.52.0,
nightly-tools image) against the 0.3.2 release on crates.io. Additions only, all behind feature
`testkit`: nothing was removed or changed; the default-features diff is empty.
`public-api.txt` is unchanged; `public-api-all-features.txt` gains the lines below.

| Item | What |
|---|---|
| `testkit::AnswerRule` (`#[non_exhaustive]`; `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`) | When a version of a stored cell answers as of a time `A` for a resolver declared time-aware. `TxAndAsof` (the `Default`): its `tx <= A` and its receipt's `asof <= A` (SPEC 5.5 step 5, D31 revised). `TxOnly`: its `tx <= A`, the 0.3.2 kit's rule. |
| `testkit::Kit::answer_rule(self, AnswerRule) -> Kit` | Sets the rule, before or after `time_aware`. |

Behaviour change (test kit only, no signature change): a `Kit` now holds a time-aware resolver
to `TxAndAsof` unless `answer_rule(AnswerRule::TxOnly)` is given. Under `TxOnly`,
`check_asof` is the 0.3.2 check exactly (same points, same expected values, same report lines).
Under `TxAndAsof` its direct points are those of `TxOnly` except that the EUR/CHF version 1
(`tx` 2024-03-15T16:00, `asof` 2024-03-16T09:00) gives `None` at its `tx`, and the points just
before its `asof` (`None`) and at its `asof` (version 1) are added. Recomputation through
`verify` is checked unchanged.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.2
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
(none)


$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.2
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+#[non_exhaustive] pub enum datumesh_core::testkit::AnswerRule
+pub datumesh_core::testkit::AnswerRule::TxAndAsof
+pub datumesh_core::testkit::AnswerRule::TxOnly
+impl core::clone::Clone for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::AnswerRule::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::AnswerRule
+impl core::cmp::PartialEq for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::AnswerRule::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::AnswerRule::default() -> Self
+impl core::fmt::Debug for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::AnswerRule::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::AnswerRule::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::testkit::AnswerRule
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::AnswerRule
+pub fn datumesh_core::testkit::Kit::answer_rule(self, datumesh_core::testkit::AnswerRule) -> datumesh_core::testkit::Kit

```
