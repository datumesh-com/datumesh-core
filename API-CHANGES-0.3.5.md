# Public API changes, datumesh-core 0.3.4 to 0.3.5

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.4` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.4` (cargo-public-api 0.52.0, a
nightly image with cargo-fuzz and cargo-public-api) against the 0.3.4 release on crates.io.
**Additions only: nothing was removed or changed** in either diff. `public-api.txt` and
`public-api-all-features.txt` are regenerated and gain exactly the added lines.

| Item | What |
|---|---|
| `ErrorCode::JobUnknown`, `ErrorCode::MeshDoc` | `E_JOB_UNKNOWN`, `E_MESH_DOC` (10.2), after `InputSuperseded` (`ErrorCode` is `#[non_exhaustive]`). |
| `Request::with_job`, `Request::job` | The request description at a job URL (5.5). `Request` has private fields, so it gains the job without a break. |
| `check_envelope_any`, `AnyEnvelope` (`#[non_exhaustive]`: `Envelope(Envelope, EnvelopeExtras)`, `Pending(Pending)`; `outcome_str`) | Every outcome, `pending` included. `Outcome` is an exhaustive enum and `Envelope` an exhaustive struct, so neither could gain `pending` without a break; `check_envelope` and `check_envelope_extras` keep refusing it (`E_ENVELOPE`), as in 0.3.4. |
| `Pending` (`#[non_exhaustive]`: `job`, `estimate_seconds`, `poll`, `request`; `new`, `to_json`, `to_value`, `STATUS`) | A `pending` envelope, checked or built (for the HTTP crate). |
| `resolve_mesh`, `Retrieval` (`Response { status, body }`, `Failed`), `ResolvedMesh` (`#[non_exhaustive]`: `keys`, `mesh`), `MeshDoc` (`#[non_exhaustive]`: `host`, `bindings`, `endpoints`; `from_json`, `url`) | Mesh documents and cross-mesh resolution (9.6), without a resolver. |

Behaviour changes without a signature change: `check_envelope` and `check_envelope_extras`
accept `E_JOB_UNKNOWN` only in `bad_request` for a request with a job, and never in `error`. Since
`3f79313` (`spec-0.4.1`, no signature change): `poll` must name a DNS host, a lone `}` in a known
endpoint template is `E_MESH_DOC`, and a key document without a string `proto` is `E_KEY_DOC`.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.4
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+#[non_exhaustive] pub enum datumesh_core::AnyEnvelope
+pub datumesh_core::AnyEnvelope::Envelope(datumesh_core::Envelope, datumesh_core::EnvelopeExtras)
+pub datumesh_core::AnyEnvelope::Pending(datumesh_core::Pending)
+impl datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::outcome_str(&self) -> &'static str
+impl core::clone::Clone for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::AnyEnvelope
+impl core::cmp::PartialEq for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::AnyEnvelope
+pub datumesh_core::ErrorCode::JobUnknown
+pub datumesh_core::ErrorCode::MeshDoc
+pub enum datumesh_core::Retrieval<'a>
+pub datumesh_core::Retrieval::Failed
+pub datumesh_core::Retrieval::Response
+pub datumesh_core::Retrieval::Response::body: core::option::Option<&'a [u8]>
+pub datumesh_core::Retrieval::Response::status: u16
+impl<'a> core::clone::Clone for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::clone(&self) -> Self
+impl<'a> core::cmp::Eq for datumesh_core::Retrieval<'a>
+impl<'a> core::cmp::PartialEq for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::eq(&self, &Self) -> bool
+impl<'a> core::fmt::Debug for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl<'a> core::hash::Hash for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl<'a> core::marker::Copy for datumesh_core::Retrieval<'a>
+impl<'a> core::marker::StructuralPartialEq for datumesh_core::Retrieval<'a>
+#[non_exhaustive] pub struct datumesh_core::MeshDoc
+pub datumesh_core::MeshDoc::bindings: alloc::string::String
+pub datumesh_core::MeshDoc::endpoints: alloc::collections::btree::map::BTreeMap<alloc::string::String, alloc::string::String>
+pub datumesh_core::MeshDoc::host: alloc::string::String
+impl datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::from_json(&str, &[u8]) -> datumesh_core::Result<datumesh_core::MeshDoc>
+pub fn datumesh_core::MeshDoc::url(&self, &str, &[(&str, &str)]) -> core::option::Option<alloc::string::String>
+impl core::clone::Clone for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::MeshDoc
+impl core::cmp::PartialEq for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::MeshDoc
+#[non_exhaustive] pub struct datumesh_core::Pending
+pub datumesh_core::Pending::estimate_seconds: u64
+pub datumesh_core::Pending::job: alloc::string::String
+pub datumesh_core::Pending::poll: alloc::string::String
+pub datumesh_core::Pending::request: datumesh_core::Request
+impl datumesh_core::Pending
+pub const datumesh_core::Pending::STATUS: u16
+pub fn datumesh_core::Pending::new(&str, u64, &str, datumesh_core::Request) -> datumesh_core::Result<datumesh_core::Pending>
+pub fn datumesh_core::Pending::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Pending::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Pending
+pub fn datumesh_core::Pending::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Pending
+impl core::cmp::PartialEq for datumesh_core::Pending
+pub fn datumesh_core::Pending::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Pending
+pub fn datumesh_core::Pending::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Pending
+pub fn datumesh_core::Pending::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Pending
+pub fn datumesh_core::Request::job(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_job(&str, core::option::Option<&str>, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::ResolvedMesh
+pub datumesh_core::ResolvedMesh::keys: datumesh_core::KeyDoc
+pub datumesh_core::ResolvedMesh::mesh: datumesh_core::MeshDoc
+impl core::clone::Clone for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ResolvedMesh
+impl core::cmp::PartialEq for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::ResolvedMesh
+pub fn datumesh_core::check_envelope_any(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<datumesh_core::AnyEnvelope>
+pub fn datumesh_core::resolve_mesh(&str, datumesh_core::Retrieval<'_>, datumesh_core::Retrieval<'_>) -> datumesh_core::Result<datumesh_core::ResolvedMesh>


$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.4
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+#[non_exhaustive] pub enum datumesh_core::AnyEnvelope
+pub datumesh_core::AnyEnvelope::Envelope(datumesh_core::Envelope, datumesh_core::EnvelopeExtras)
+pub datumesh_core::AnyEnvelope::Pending(datumesh_core::Pending)
+impl datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::outcome_str(&self) -> &'static str
+impl core::clone::Clone for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::AnyEnvelope
+impl core::cmp::PartialEq for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::AnyEnvelope
+pub fn datumesh_core::AnyEnvelope::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::AnyEnvelope
+pub datumesh_core::ErrorCode::JobUnknown
+pub datumesh_core::ErrorCode::MeshDoc
+pub enum datumesh_core::Retrieval<'a>
+pub datumesh_core::Retrieval::Failed
+pub datumesh_core::Retrieval::Response
+pub datumesh_core::Retrieval::Response::body: core::option::Option<&'a [u8]>
+pub datumesh_core::Retrieval::Response::status: u16
+impl<'a> core::clone::Clone for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::clone(&self) -> Self
+impl<'a> core::cmp::Eq for datumesh_core::Retrieval<'a>
+impl<'a> core::cmp::PartialEq for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::eq(&self, &Self) -> bool
+impl<'a> core::fmt::Debug for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl<'a> core::hash::Hash for datumesh_core::Retrieval<'a>
+pub fn datumesh_core::Retrieval<'a>::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl<'a> core::marker::Copy for datumesh_core::Retrieval<'a>
+impl<'a> core::marker::StructuralPartialEq for datumesh_core::Retrieval<'a>
+#[non_exhaustive] pub struct datumesh_core::MeshDoc
+pub datumesh_core::MeshDoc::bindings: alloc::string::String
+pub datumesh_core::MeshDoc::endpoints: alloc::collections::btree::map::BTreeMap<alloc::string::String, alloc::string::String>
+pub datumesh_core::MeshDoc::host: alloc::string::String
+impl datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::from_json(&str, &[u8]) -> datumesh_core::Result<datumesh_core::MeshDoc>
+pub fn datumesh_core::MeshDoc::url(&self, &str, &[(&str, &str)]) -> core::option::Option<alloc::string::String>
+impl core::clone::Clone for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::MeshDoc
+impl core::cmp::PartialEq for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::MeshDoc
+pub fn datumesh_core::MeshDoc::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::MeshDoc
+#[non_exhaustive] pub struct datumesh_core::Pending
+pub datumesh_core::Pending::estimate_seconds: u64
+pub datumesh_core::Pending::job: alloc::string::String
+pub datumesh_core::Pending::poll: alloc::string::String
+pub datumesh_core::Pending::request: datumesh_core::Request
+impl datumesh_core::Pending
+pub const datumesh_core::Pending::STATUS: u16
+pub fn datumesh_core::Pending::new(&str, u64, &str, datumesh_core::Request) -> datumesh_core::Result<datumesh_core::Pending>
+pub fn datumesh_core::Pending::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Pending::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Pending
+pub fn datumesh_core::Pending::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Pending
+impl core::cmp::PartialEq for datumesh_core::Pending
+pub fn datumesh_core::Pending::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Pending
+pub fn datumesh_core::Pending::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Pending
+pub fn datumesh_core::Pending::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Pending
+pub fn datumesh_core::Request::job(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_job(&str, core::option::Option<&str>, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::ResolvedMesh
+pub datumesh_core::ResolvedMesh::keys: datumesh_core::KeyDoc
+pub datumesh_core::ResolvedMesh::mesh: datumesh_core::MeshDoc
+impl core::clone::Clone for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ResolvedMesh
+impl core::cmp::PartialEq for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::ResolvedMesh
+pub fn datumesh_core::ResolvedMesh::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::ResolvedMesh
+pub fn datumesh_core::check_envelope_any(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<datumesh_core::AnyEnvelope>
+pub fn datumesh_core::resolve_mesh(&str, datumesh_core::Retrieval<'_>, datumesh_core::Retrieval<'_>) -> datumesh_core::Result<datumesh_core::ResolvedMesh>

```
