# Public API changes, datumesh-core 0.3.7 to 0.3.8

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.7` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.7` (cargo-public-api 0.52.0)
against the 0.3.7 release on crates.io. **Additions only: nothing was removed or changed** in
either diff ("Removed items" and "Changed items" are empty). Each diff adds 301 lines (300 at the implementation commit, plus `check_describe_with`, added by the
spec-0.5.0 re-pin), and `public-api.txt` and `public-api-all-features.txt` gain exactly the added lines.

The types that gain members are `#[non_exhaustive]`, so a new variant or field is an addition
for the callers that already exist: `ErrorCode`, `Fault`, `Answer` (`resolved`, `shadows`),
`FunctionDescriptor` (`arg_extras`, `returns_classes`). `ErrorCode::ALL` and `Fault::ALL` are
longer (63 and 8 entries; the same signature).

| Item | What |
|---|---|
| `ErrorCode` | Fourteen codes after `ForeignUnavailable`, in 10.2's order: `UnrootedInput`, `NamespaceSuspended`, `ClassMember` to `ClassOwner` (8), `KeyNormalisation`, `KeyEncoding`, `ShortIdAmbiguous`, `Describe`. |
| `Fault` | `Key` (`E_FORMULA_KEY`), `DispatchKey` (`E_FORMULA_DISPATCH_KEY`). |
| `Answer` | `resolved: Option<Resolved>`, `shadows: Option<Shadows>`, `with_resolved`, `with_shadows`. `with_error` takes `E_NAMESPACE_SUSPENDED`. |
| `Resolved`, `Resolution`, `ResolutionKind`, `Shadows` | The envelope members of 5.5 (`resolved`, `shadows`), with their builders. `ResolutionKind` is `#[non_exhaustive]`. |
| `ClassDescriptor`, `ClassKey`, `Steward`, `ClassExtent`, `ClassEnv`, `define_class` | Classes (16.1, 16.2). `ClassDescriptor` and `ClassKey` are `#[non_exhaustive]`. |
| `KeyNormalisation` | The closed list of 16.4 (`#[non_exhaustive]`), `ALL`, `as_str`, `from_name`, `check`. |
| `ObjectKey`, `KeyValue`, `parse_object_key`, `format_object_key`, `short_id` | Key encodings and object ids (16.5, 16.8). |
| `check_describe`, `check_describe_with`, `DescribeKind`, `Described` | describe answers (16.8). `check_describe_with` is the six-step check with the method functions the client holds (step 6; an empty slice is `check_describe`). Added by the spec-0.5.0 re-pin (6d5633e). |
| `check_define` | The define-time check (2.5). |
| `FunctionDescriptor::arg_extras`, `ArgExtras`, `FunctionDescriptor::returns_classes` | `classes`, `default` and `key_normalisation` (7). `ArgDecl` and `Returns` are unchanged. |
| `Resolution::lookup`, `Resolved::with_aliases_tx`, `Shadows::new`, and the others above | The constructors of the envelope members. |
| `try_check_entitlement`, `EntitlementError` | The caller's errors of 10.3, additive. `check_entitlement` is unchanged. |
| `is_unrooted_namespace` | Namespace-checked 2.5, additive. `is_unrooted` is unchanged. |
| `ResolvedMesh`, `MeshDoc` | Unchanged in shape: loopback URLs are accepted by `resolve_mesh`, which is a behaviour change, not an API one. |

Added lines of `cargo public-api -p datumesh-core -ss diff 0.3.7`:

```
+pub fn datumesh_core::lexical::is_unrooted_namespace(&str) -> datumesh_core::Result<bool>
+#[non_exhaustive] pub enum datumesh_core::ClassKey
+pub datumesh_core::ClassKey::Class(alloc::string::String)
+pub datumesh_core::ClassKey::Scalar
+pub datumesh_core::ClassKey::Scalar::key_normalisation: core::option::Option<datumesh_core::KeyNormalisation>
+pub datumesh_core::ClassKey::Scalar::ty: datumesh_core::Type
+pub datumesh_core::ClassKey::Scalar::unit: core::option::Option<datumesh_core::Unit>
+impl core::clone::Clone for datumesh_core::ClassKey
+pub fn datumesh_core::ClassKey::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ClassKey
+impl core::cmp::PartialEq for datumesh_core::ClassKey
+pub fn datumesh_core::ClassKey::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::ClassKey
+pub fn datumesh_core::ClassKey::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::ClassKey
+#[non_exhaustive] pub enum datumesh_core::DescribeKind
+pub datumesh_core::DescribeKind::Class
+pub datumesh_core::DescribeKind::Object
+impl core::clone::Clone for datumesh_core::DescribeKind
+pub fn datumesh_core::DescribeKind::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::DescribeKind
+impl core::cmp::PartialEq for datumesh_core::DescribeKind
+pub fn datumesh_core::DescribeKind::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::DescribeKind
+pub fn datumesh_core::DescribeKind::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::DescribeKind
+pub fn datumesh_core::DescribeKind::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::DescribeKind
+impl core::marker::StructuralPartialEq for datumesh_core::DescribeKind
+#[non_exhaustive] pub enum datumesh_core::EntitlementError
+pub datumesh_core::EntitlementError::LicencesWithoutInputs
+pub datumesh_core::EntitlementError::WrongPublisher(datumesh_core::WrongPublisher)
+impl core::clone::Clone for datumesh_core::EntitlementError
+pub fn datumesh_core::EntitlementError::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::EntitlementError
+impl core::cmp::PartialEq for datumesh_core::EntitlementError
+pub fn datumesh_core::EntitlementError::eq(&self, &Self) -> bool
+impl core::error::Error for datumesh_core::EntitlementError
+impl core::fmt::Debug for datumesh_core::EntitlementError
+pub fn datumesh_core::EntitlementError::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::EntitlementError::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::EntitlementError
+impl core::hash::Hash for datumesh_core::EntitlementError
+pub fn datumesh_core::EntitlementError::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::EntitlementError
+pub datumesh_core::ErrorCode::ClassFinal
+pub datumesh_core::ErrorCode::ClassInherit
+pub datumesh_core::ErrorCode::ClassInterface
+pub datumesh_core::ErrorCode::ClassMember
+pub datumesh_core::ErrorCode::ClassMethod
+pub datumesh_core::ErrorCode::ClassOverride
+pub datumesh_core::ErrorCode::ClassOwner
+pub datumesh_core::ErrorCode::ClassParent
+pub datumesh_core::ErrorCode::Describe
+pub datumesh_core::ErrorCode::KeyEncoding
+pub datumesh_core::ErrorCode::KeyNormalisation
+pub datumesh_core::ErrorCode::NamespaceSuspended
+pub datumesh_core::ErrorCode::ShortIdAmbiguous
+pub datumesh_core::ErrorCode::UnrootedInput
+pub datumesh_core::Fault::DispatchKey
+pub datumesh_core::Fault::Key
+#[non_exhaustive] pub enum datumesh_core::KeyNormalisation
+pub datumesh_core::KeyNormalisation::ArabicSkeleton
+pub datumesh_core::KeyNormalisation::GreekUnaccented
+pub datumesh_core::KeyNormalisation::HebrewConsonantal
+pub datumesh_core::KeyNormalisation::NfcStrict
+pub datumesh_core::KeyNormalisation::NoRule
+pub datumesh_core::KeyNormalisation::UpperAscii
+impl datumesh_core::KeyNormalisation
+pub const datumesh_core::KeyNormalisation::ALL: &'static [datumesh_core::KeyNormalisation]
+pub const fn datumesh_core::KeyNormalisation::as_str(self) -> &'static str
+pub fn datumesh_core::KeyNormalisation::check(self, &str) -> datumesh_core::Result<()>
+pub fn datumesh_core::KeyNormalisation::from_name(&str) -> core::option::Option<datumesh_core::KeyNormalisation>
+impl core::clone::Clone for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::KeyNormalisation
+impl core::cmp::Ord for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::KeyNormalisation::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::KeyNormalisation
+impl core::hash::Hash for datumesh_core::KeyNormalisation
+pub fn datumesh_core::KeyNormalisation::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::KeyNormalisation
+impl core::marker::StructuralPartialEq for datumesh_core::KeyNormalisation
+#[non_exhaustive] pub enum datumesh_core::KeyValue
+pub datumesh_core::KeyValue::Object(datumesh_core::ObjectKey)
+pub datumesh_core::KeyValue::Scalar(datumesh_core::Scalar)
+impl core::clone::Clone for datumesh_core::KeyValue
+pub fn datumesh_core::KeyValue::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::KeyValue
+impl core::cmp::PartialEq for datumesh_core::KeyValue
+pub fn datumesh_core::KeyValue::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::KeyValue
+pub fn datumesh_core::KeyValue::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::KeyValue
+#[non_exhaustive] pub enum datumesh_core::ResolutionKind
+pub datumesh_core::ResolutionKind::Alias
+pub datumesh_core::ResolutionKind::Default
+pub datumesh_core::ResolutionKind::Lookup
+pub datumesh_core::ResolutionKind::Method
+impl datumesh_core::ResolutionKind
+pub const datumesh_core::ResolutionKind::ALL: &'static [datumesh_core::ResolutionKind]
+pub const fn datumesh_core::ResolutionKind::as_str(self) -> &'static str
+pub fn datumesh_core::ResolutionKind::from_name(&str) -> core::option::Option<datumesh_core::ResolutionKind>
+impl core::clone::Clone for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ResolutionKind
+impl core::cmp::Ord for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::ResolutionKind
+pub fn datumesh_core::ResolutionKind::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::ResolutionKind
+impl core::marker::StructuralPartialEq for datumesh_core::ResolutionKind
+pub datumesh_core::Answer::resolved: core::option::Option<datumesh_core::Resolved>
+pub datumesh_core::Answer::shadows: core::option::Option<datumesh_core::Shadows>
+pub fn datumesh_core::Answer::with_resolved(self, datumesh_core::Resolved) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::with_shadows(self, datumesh_core::Shadows) -> datumesh_core::Result<datumesh_core::Answer>
+#[non_exhaustive] pub struct datumesh_core::ArgExtras
+pub datumesh_core::ArgExtras::classes: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub datumesh_core::ArgExtras::default_asof: bool
+pub datumesh_core::ArgExtras::key_normalisation: core::option::Option<datumesh_core::KeyNormalisation>
+impl core::clone::Clone for datumesh_core::ArgExtras
+pub fn datumesh_core::ArgExtras::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ArgExtras
+impl core::cmp::PartialEq for datumesh_core::ArgExtras
+pub fn datumesh_core::ArgExtras::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::ArgExtras
+pub fn datumesh_core::ArgExtras::default() -> Self
+impl core::fmt::Debug for datumesh_core::ArgExtras
+pub fn datumesh_core::ArgExtras::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::ArgExtras
+pub fn datumesh_core::ArgExtras::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::ArgExtras
+#[non_exhaustive] pub struct datumesh_core::ClassDescriptor
+pub datumesh_core::ClassDescriptor::extent: core::option::Option<datumesh_core::ClassExtent>
+pub datumesh_core::ClassDescriptor::implements: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::ClassDescriptor::inverse: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::ClassDescriptor::is_final: bool
+pub datumesh_core::ClassDescriptor::keys: alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::ClassKey>
+pub datumesh_core::ClassDescriptor::lookup: core::option::Option<alloc::string::String>
+pub datumesh_core::ClassDescriptor::maps: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::ClassDescriptor::methods: alloc::collections::btree::map::BTreeMap<alloc::string::String, alloc::string::String>
+pub datumesh_core::ClassDescriptor::open: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::ClassDescriptor::order: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::ClassDescriptor::parent: core::option::Option<alloc::string::String>
+pub datumesh_core::ClassDescriptor::steward: datumesh_core::Steward
+impl datumesh_core::ClassDescriptor
+pub fn datumesh_core::ClassDescriptor::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::ClassDescriptor::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::ClassDescriptor>
+pub fn datumesh_core::ClassDescriptor::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::ClassDescriptor>
+pub fn datumesh_core::ClassDescriptor::hash(&self) -> alloc::string::String
+pub fn datumesh_core::ClassDescriptor::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::ClassDescriptor
+pub fn datumesh_core::ClassDescriptor::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ClassDescriptor
+impl core::cmp::PartialEq for datumesh_core::ClassDescriptor
+pub fn datumesh_core::ClassDescriptor::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::ClassDescriptor
+pub fn datumesh_core::ClassDescriptor::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::ClassDescriptor
+pub struct datumesh_core::ClassEnv
+impl datumesh_core::ClassEnv
+pub fn datumesh_core::ClassEnv::class(&self, &str) -> core::option::Option<&datumesh_core::ClassDescriptor>
+pub fn datumesh_core::ClassEnv::definer(&self) -> &str
+pub fn datumesh_core::ClassEnv::function(&self, &str) -> core::option::Option<&datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::ClassEnv::insert_class(&mut self, datumesh_core::ClassDescriptor)
+pub fn datumesh_core::ClassEnv::insert_function(&mut self, datumesh_core::FunctionDescriptor)
+pub fn datumesh_core::ClassEnv::new(&str) -> datumesh_core::ClassEnv
+impl core::clone::Clone for datumesh_core::ClassEnv
+pub fn datumesh_core::ClassEnv::clone(&self) -> Self
+impl core::default::Default for datumesh_core::ClassEnv
+pub fn datumesh_core::ClassEnv::default() -> Self
+impl core::fmt::Debug for datumesh_core::ClassEnv
+pub fn datumesh_core::ClassEnv::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+#[non_exhaustive] pub struct datumesh_core::ClassExtent
+pub datumesh_core::ClassExtent::exists: core::option::Option<alloc::string::String>
+pub datumesh_core::ClassExtent::keys: core::option::Option<alloc::string::String>
+impl core::clone::Clone for datumesh_core::ClassExtent
+pub fn datumesh_core::ClassExtent::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ClassExtent
+impl core::cmp::PartialEq for datumesh_core::ClassExtent
+pub fn datumesh_core::ClassExtent::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::ClassExtent
+pub fn datumesh_core::ClassExtent::default() -> Self
+impl core::fmt::Debug for datumesh_core::ClassExtent
+pub fn datumesh_core::ClassExtent::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::ClassExtent
+#[non_exhaustive] pub struct datumesh_core::Described
+pub datumesh_core::Described::class: alloc::string::String
+pub datumesh_core::Described::class_hash: alloc::string::String
+pub datumesh_core::Described::kind: datumesh_core::DescribeKind
+pub datumesh_core::Described::object_id: core::option::Option<alloc::string::String>
+impl core::clone::Clone for datumesh_core::Described
+pub fn datumesh_core::Described::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Described
+impl core::cmp::PartialEq for datumesh_core::Described
+pub fn datumesh_core::Described::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Described
+pub fn datumesh_core::Described::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Described
+pub datumesh_core::FunctionDescriptor::arg_extras: alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::ArgExtras>
+pub datumesh_core::FunctionDescriptor::returns_classes: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+#[non_exhaustive] pub struct datumesh_core::ObjectKey
+pub datumesh_core::ObjectKey::class: alloc::string::String
+pub datumesh_core::ObjectKey::keys: alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::KeyValue>
+impl datumesh_core::ObjectKey
+pub fn datumesh_core::ObjectKey::call_form(&self) -> alloc::string::String
+pub fn datumesh_core::ObjectKey::new(&str, impl core::iter::traits::collect::IntoIterator<Item = (alloc::string::String, datumesh_core::KeyValue)>) -> datumesh_core::Result<datumesh_core::ObjectKey>
+pub fn datumesh_core::ObjectKey::object_id(&self, &str) -> datumesh_core::Result<alloc::string::String>
+pub fn datumesh_core::ObjectKey::object_value(&self, &str) -> datumesh_core::Result<datumesh_core::json::Value>
+pub fn datumesh_core::ObjectKey::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::ObjectKey
+pub fn datumesh_core::ObjectKey::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ObjectKey
+impl core::cmp::PartialEq for datumesh_core::ObjectKey
+pub fn datumesh_core::ObjectKey::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::ObjectKey
+pub fn datumesh_core::ObjectKey::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::ObjectKey
+#[non_exhaustive] pub struct datumesh_core::Resolution
+pub datumesh_core::Resolution::asof: core::option::Option<alloc::string::String>
+pub datumesh_core::Resolution::from: alloc::string::String
+pub datumesh_core::Resolution::kind: datumesh_core::ResolutionKind
+pub datumesh_core::Resolution::to: alloc::string::String
+impl datumesh_core::Resolution
+pub fn datumesh_core::Resolution::lookup(&str, &str, &str) -> datumesh_core::Result<datumesh_core::Resolution>
+pub fn datumesh_core::Resolution::new(&str, datumesh_core::ResolutionKind, &str) -> datumesh_core::Result<datumesh_core::Resolution>
+impl core::clone::Clone for datumesh_core::Resolution
+pub fn datumesh_core::Resolution::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Resolution
+impl core::cmp::PartialEq for datumesh_core::Resolution
+pub fn datumesh_core::Resolution::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Resolution
+pub fn datumesh_core::Resolution::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Resolution
+pub fn datumesh_core::Resolution::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Resolution
+#[non_exhaustive] pub struct datumesh_core::Resolved
+pub datumesh_core::Resolved::aliases_tx: core::option::Option<alloc::string::String>
+pub datumesh_core::Resolved::call: alloc::string::String
+pub datumesh_core::Resolved::resolutions: alloc::vec::Vec<datumesh_core::Resolution>
+impl datumesh_core::Resolved
+pub fn datumesh_core::Resolved::new(&str, alloc::vec::Vec<datumesh_core::Resolution>) -> datumesh_core::Result<datumesh_core::Resolved>
+pub fn datumesh_core::Resolved::with_aliases_tx(self, &str) -> datumesh_core::Result<datumesh_core::Resolved>
+impl core::clone::Clone for datumesh_core::Resolved
+pub fn datumesh_core::Resolved::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Resolved
+impl core::cmp::PartialEq for datumesh_core::Resolved
+pub fn datumesh_core::Resolved::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Resolved
+pub fn datumesh_core::Resolved::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Resolved
+pub fn datumesh_core::Resolved::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Resolved
+#[non_exhaustive] pub struct datumesh_core::Shadows
+pub datumesh_core::Shadows::class: alloc::string::String
+pub datumesh_core::Shadows::reference: alloc::string::String
+impl datumesh_core::Shadows
+pub fn datumesh_core::Shadows::new(&str, &str) -> datumesh_core::Result<datumesh_core::Shadows>
+impl core::clone::Clone for datumesh_core::Shadows
+pub fn datumesh_core::Shadows::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Shadows
+impl core::cmp::PartialEq for datumesh_core::Shadows
+pub fn datumesh_core::Shadows::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Shadows
+pub fn datumesh_core::Shadows::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Shadows
+pub fn datumesh_core::Shadows::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Shadows
+#[non_exhaustive] pub struct datumesh_core::Steward
+pub datumesh_core::Steward::namespace: alloc::string::String
+pub datumesh_core::Steward::process: alloc::string::String
+impl core::clone::Clone for datumesh_core::Steward
+pub fn datumesh_core::Steward::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Steward
+impl core::cmp::PartialEq for datumesh_core::Steward
+pub fn datumesh_core::Steward::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Steward
+pub fn datumesh_core::Steward::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Steward
+pub fn datumesh_core::check_define(&str, &[u8]) -> datumesh_core::Result<datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::check_describe(&[u8], &alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::ClassDescriptor>) -> datumesh_core::Result<datumesh_core::Described>
+pub fn datumesh_core::check_describe_with(&[u8], &alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::ClassDescriptor>, &[datumesh_core::FunctionDescriptor]) -> datumesh_core::Result<datumesh_core::Described>
+pub fn datumesh_core::define_class(&str, &[u8], &datumesh_core::ClassEnv) -> datumesh_core::Result<datumesh_core::ClassDescriptor>
+pub fn datumesh_core::format_object_key(&datumesh_core::ObjectKey) -> alloc::string::String
+pub fn datumesh_core::is_unrooted_namespace(&str) -> datumesh_core::Result<bool>
+pub fn datumesh_core::parse_object_key(&str) -> datumesh_core::Result<datumesh_core::ObjectKey>
+pub fn datumesh_core::short_id(&str) -> datumesh_core::Result<alloc::string::String>
+pub fn datumesh_core::try_check_entitlement<I, S>(&datumesh_core::Receipt, I, &datumesh_core::KeyDocument) -> core::result::Result<datumesh_core::Entitlement, datumesh_core::EntitlementError> where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::AsRef<str>
```
