# Public API changes, datumesh-core 0.3.8 to 0.3.9

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.8` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.8` (cargo-public-api 0.52.0)
against the 0.3.8 release on crates.io. **Additions only: nothing was removed or changed** in
either diff ("Removed items" and "Changed items" are empty). The default diff adds 152 lines and the
all-features diff 153. `public-api.txt` and
`public-api-all-features.txt` gain exactly the added lines.

## Default features

```
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub datumesh_core::AnswerOutcome::Scenario
+#[non_exhaustive] pub enum datumesh_core::CycleFix
+pub datumesh_core::CycleFix::Asof
+pub datumesh_core::CycleFix::Lag
+pub datumesh_core::CycleFix::Solver
+pub datumesh_core::CycleFix::Step
+impl datumesh_core::CycleFix
+pub const datumesh_core::CycleFix::ALL: &'static [datumesh_core::CycleFix]
+pub const fn datumesh_core::CycleFix::as_str(self) -> &'static str
+pub fn datumesh_core::CycleFix::from_name(&str) -> core::option::Option<datumesh_core::CycleFix>
+impl core::clone::Clone for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::CycleFix
+impl core::cmp::Ord for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::CycleFix
+impl core::marker::StructuralPartialEq for datumesh_core::CycleFix
+pub datumesh_core::DescribeKind::Function
+pub datumesh_core::ErrorCode::WhatifNoSolution
+pub datumesh_core::ErrorCode::WhatifUnsupported
+pub datumesh_core::Fault::WhatifNoSolution
+pub datumesh_core::TrustLabel::Relayed
+pub datumesh_core::Answer::cycle: core::option::Option<datumesh_core::Cycle>
+pub datumesh_core::Answer::scenario: core::option::Option<datumesh_core::Scenario>
+pub fn datumesh_core::Answer::scenario(datumesh_core::Scenario) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_cycle(self, datumesh_core::Cycle) -> datumesh_core::Result<datumesh_core::Answer>
+#[non_exhaustive] pub struct datumesh_core::Cycle
+impl datumesh_core::Cycle
+pub fn datumesh_core::Cycle::fixes(&self) -> &[datumesh_core::CycleFix]
+pub fn datumesh_core::Cycle::new(alloc::vec::Vec<alloc::string::String>, alloc::vec::Vec<datumesh_core::CycleFix>) -> datumesh_core::Result<datumesh_core::Cycle>
+pub fn datumesh_core::Cycle::path(&self) -> &[alloc::string::String]
+impl core::clone::Clone for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Cycle
+impl core::cmp::PartialEq for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Cycle
+pub struct datumesh_core::DescribeSource
+pub datumesh_core::DescribeSource::name: alloc::string::String
+pub datumesh_core::DescribeSource::relayed_by: alloc::string::String
+pub datumesh_core::DescribeSource::takeover: alloc::string::String
+impl core::clone::Clone for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::DescribeSource
+impl core::cmp::PartialEq for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::DescribeSource
+pub struct datumesh_core::DescribeTests
+pub datumesh_core::DescribeTests::branches: u64
+pub datumesh_core::DescribeTests::count: u64
+pub datumesh_core::DescribeTests::counterexamples: u64
+pub datumesh_core::DescribeTests::covered: u64
+impl core::clone::Clone for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::DescribeTests
+impl core::cmp::PartialEq for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::DescribeTests
+impl core::marker::StructuralPartialEq for datumesh_core::DescribeTests
+pub datumesh_core::Described::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Described::function: core::option::Option<alloc::string::String>
+pub datumesh_core::Described::source: core::option::Option<datumesh_core::DescribeSource>
+pub datumesh_core::Described::tests: core::option::Option<datumesh_core::DescribeTests>
+pub struct datumesh_core::FunctionAnswer
+impl datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::new(&str, &datumesh_core::FunctionDescriptor) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::FunctionAnswer::to_value(&self) -> datumesh_core::json::Value
+pub fn datumesh_core::FunctionAnswer::with_description(self, &str) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::with_source(self, datumesh_core::DescribeSource) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::with_tests(self, datumesh_core::DescribeTests) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+impl core::clone::Clone for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::clone(&self) -> Self
+impl core::cmp::PartialEq for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::FunctionAnswer
+pub fn datumesh_core::Receipt::source(&self) -> core::option::Option<&datumesh_core::Source>
+pub fn datumesh_core::Request::overrides(&self) -> core::option::Option<&datumesh_core::json::Value>
+pub fn datumesh_core::Request::solve(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_whatif(self, &datumesh_core::json::Value, core::option::Option<&str>) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::Scenario
+impl datumesh_core::Scenario
+pub fn datumesh_core::Scenario::asof(&self) -> &str
+pub fn datumesh_core::Scenario::cell(&self) -> &datumesh_core::Cell
+pub fn datumesh_core::Scenario::new(datumesh_core::Cell, datumesh_core::Scalar, &str, &datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Scenario>
+pub fn datumesh_core::Scenario::overrides(&self) -> &datumesh_core::json::Value
+pub fn datumesh_core::Scenario::solved(&self) -> core::option::Option<&datumesh_core::Solved>
+pub fn datumesh_core::Scenario::ty(&self) -> datumesh_core::Type
+pub fn datumesh_core::Scenario::value(&self) -> &datumesh_core::Scalar
+pub fn datumesh_core::Scenario::with_solved(self, datumesh_core::Cell, datumesh_core::Scalar) -> datumesh_core::Result<datumesh_core::Scenario>
+impl core::clone::Clone for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Scenario
+impl core::cmp::PartialEq for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Scenario
+#[non_exhaustive] pub struct datumesh_core::Solved
+impl datumesh_core::Solved
+pub fn datumesh_core::Solved::value(&self) -> &datumesh_core::Scalar
+pub fn datumesh_core::Solved::vary(&self) -> &datumesh_core::Cell
+impl core::clone::Clone for datumesh_core::Solved
+pub fn datumesh_core::Solved::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Solved
+impl core::cmp::PartialEq for datumesh_core::Solved
+pub fn datumesh_core::Solved::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Solved
+pub fn datumesh_core::Solved::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Solved
+pub struct datumesh_core::Source
+impl datumesh_core::Source
+pub fn datumesh_core::Source::name(&self) -> &str
+pub fn datumesh_core::Source::new(&str, &str, &str) -> datumesh_core::Source
+pub fn datumesh_core::Source::retrieved(&self) -> &str
+pub fn datumesh_core::Source::sha256(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Source::takeover(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Source::url(&self) -> &str
+pub fn datumesh_core::Source::with_sha256(self, &str) -> datumesh_core::Source
+pub fn datumesh_core::Source::with_takeover(self, &str) -> datumesh_core::Source
+impl core::clone::Clone for datumesh_core::Source
+pub fn datumesh_core::Source::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Source
+impl core::cmp::PartialEq for datumesh_core::Source
+pub fn datumesh_core::Source::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Source
+pub fn datumesh_core::Source::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Source
+pub fn datumesh_core::Source::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Source

```

## All features

```
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub datumesh_core::AnswerOutcome::Scenario
+#[non_exhaustive] pub enum datumesh_core::CycleFix
+pub datumesh_core::CycleFix::Asof
+pub datumesh_core::CycleFix::Lag
+pub datumesh_core::CycleFix::Solver
+pub datumesh_core::CycleFix::Step
+impl datumesh_core::CycleFix
+pub const datumesh_core::CycleFix::ALL: &'static [datumesh_core::CycleFix]
+pub const fn datumesh_core::CycleFix::as_str(self) -> &'static str
+pub fn datumesh_core::CycleFix::from_name(&str) -> core::option::Option<datumesh_core::CycleFix>
+impl core::clone::Clone for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::CycleFix
+impl core::cmp::Ord for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::CycleFix
+pub fn datumesh_core::CycleFix::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::CycleFix
+impl core::marker::StructuralPartialEq for datumesh_core::CycleFix
+pub datumesh_core::DescribeKind::Function
+pub datumesh_core::ErrorCode::WhatifNoSolution
+pub datumesh_core::ErrorCode::WhatifUnsupported
+pub datumesh_core::Fault::WhatifNoSolution
+pub datumesh_core::TrustLabel::Relayed
+pub datumesh_core::Answer::cycle: core::option::Option<datumesh_core::Cycle>
+pub datumesh_core::Answer::scenario: core::option::Option<datumesh_core::Scenario>
+pub fn datumesh_core::Answer::scenario(datumesh_core::Scenario) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_cycle(self, datumesh_core::Cycle) -> datumesh_core::Result<datumesh_core::Answer>
+#[non_exhaustive] pub struct datumesh_core::Cycle
+impl datumesh_core::Cycle
+pub fn datumesh_core::Cycle::fixes(&self) -> &[datumesh_core::CycleFix]
+pub fn datumesh_core::Cycle::new(alloc::vec::Vec<alloc::string::String>, alloc::vec::Vec<datumesh_core::CycleFix>) -> datumesh_core::Result<datumesh_core::Cycle>
+pub fn datumesh_core::Cycle::path(&self) -> &[alloc::string::String]
+impl core::clone::Clone for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Cycle
+impl core::cmp::PartialEq for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Cycle
+pub fn datumesh_core::Cycle::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Cycle
+pub struct datumesh_core::DescribeSource
+pub datumesh_core::DescribeSource::name: alloc::string::String
+pub datumesh_core::DescribeSource::relayed_by: alloc::string::String
+pub datumesh_core::DescribeSource::takeover: alloc::string::String
+impl core::clone::Clone for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::DescribeSource
+impl core::cmp::PartialEq for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::DescribeSource
+pub fn datumesh_core::DescribeSource::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::DescribeSource
+pub struct datumesh_core::DescribeTests
+pub datumesh_core::DescribeTests::branches: u64
+pub datumesh_core::DescribeTests::count: u64
+pub datumesh_core::DescribeTests::counterexamples: u64
+pub datumesh_core::DescribeTests::covered: u64
+impl core::clone::Clone for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::DescribeTests
+impl core::cmp::PartialEq for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::DescribeTests
+pub fn datumesh_core::DescribeTests::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::DescribeTests
+impl core::marker::StructuralPartialEq for datumesh_core::DescribeTests
+pub datumesh_core::Described::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Described::function: core::option::Option<alloc::string::String>
+pub datumesh_core::Described::source: core::option::Option<datumesh_core::DescribeSource>
+pub datumesh_core::Described::tests: core::option::Option<datumesh_core::DescribeTests>
+pub struct datumesh_core::FunctionAnswer
+impl datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::new(&str, &datumesh_core::FunctionDescriptor) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::FunctionAnswer::to_value(&self) -> datumesh_core::json::Value
+pub fn datumesh_core::FunctionAnswer::with_description(self, &str) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::with_source(self, datumesh_core::DescribeSource) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+pub fn datumesh_core::FunctionAnswer::with_tests(self, datumesh_core::DescribeTests) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
+impl core::clone::Clone for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::clone(&self) -> Self
+impl core::cmp::PartialEq for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::FunctionAnswer
+pub fn datumesh_core::FunctionAnswer::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::FunctionAnswer
+pub fn datumesh_core::Receipt::source(&self) -> core::option::Option<&datumesh_core::Source>
+pub fn datumesh_core::ReceiptBuilder::source(self, datumesh_core::Source) -> Self
+pub fn datumesh_core::Request::overrides(&self) -> core::option::Option<&datumesh_core::json::Value>
+pub fn datumesh_core::Request::solve(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_whatif(self, &datumesh_core::json::Value, core::option::Option<&str>) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::Scenario
+impl datumesh_core::Scenario
+pub fn datumesh_core::Scenario::asof(&self) -> &str
+pub fn datumesh_core::Scenario::cell(&self) -> &datumesh_core::Cell
+pub fn datumesh_core::Scenario::new(datumesh_core::Cell, datumesh_core::Scalar, &str, &datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Scenario>
+pub fn datumesh_core::Scenario::overrides(&self) -> &datumesh_core::json::Value
+pub fn datumesh_core::Scenario::solved(&self) -> core::option::Option<&datumesh_core::Solved>
+pub fn datumesh_core::Scenario::ty(&self) -> datumesh_core::Type
+pub fn datumesh_core::Scenario::value(&self) -> &datumesh_core::Scalar
+pub fn datumesh_core::Scenario::with_solved(self, datumesh_core::Cell, datumesh_core::Scalar) -> datumesh_core::Result<datumesh_core::Scenario>
+impl core::clone::Clone for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Scenario
+impl core::cmp::PartialEq for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Scenario
+pub fn datumesh_core::Scenario::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Scenario
+#[non_exhaustive] pub struct datumesh_core::Solved
+impl datumesh_core::Solved
+pub fn datumesh_core::Solved::value(&self) -> &datumesh_core::Scalar
+pub fn datumesh_core::Solved::vary(&self) -> &datumesh_core::Cell
+impl core::clone::Clone for datumesh_core::Solved
+pub fn datumesh_core::Solved::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Solved
+impl core::cmp::PartialEq for datumesh_core::Solved
+pub fn datumesh_core::Solved::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Solved
+pub fn datumesh_core::Solved::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Solved
+pub struct datumesh_core::Source
+impl datumesh_core::Source
+pub fn datumesh_core::Source::name(&self) -> &str
+pub fn datumesh_core::Source::new(&str, &str, &str) -> datumesh_core::Source
+pub fn datumesh_core::Source::retrieved(&self) -> &str
+pub fn datumesh_core::Source::sha256(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Source::takeover(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Source::url(&self) -> &str
+pub fn datumesh_core::Source::with_sha256(self, &str) -> datumesh_core::Source
+pub fn datumesh_core::Source::with_takeover(self, &str) -> datumesh_core::Source
+impl core::clone::Clone for datumesh_core::Source
+pub fn datumesh_core::Source::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Source
+impl core::cmp::PartialEq for datumesh_core::Source
+pub fn datumesh_core::Source::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Source
+pub fn datumesh_core::Source::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Source
+pub fn datumesh_core::Source::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Source

```
