# Public API changes, datumesh-core 0.3.9 to 0.3.10

Generated with `cargo public-api -p datumesh-core -ss` and `cargo public-api -p datumesh-core --all-features -ss`
(cargo-public-api 0.52.0, in the `datumesh-core-nightly-tools` image), and compared with the 0.3.9 snapshots
`public-api.txt` and `public-api-all-features.txt` at `d071547`. **Additions only: nothing was removed or changed**
in either diff. The default diff adds 2 lines and the all-features diff 2. The two snapshot files gain exactly
those lines.

## Default features and all features

```
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub datumesh_core::Described::publisher: core::option::Option<alloc::string::String>
+pub fn datumesh_core::FunctionAnswer::with_publisher(self, &str) -> datumesh_core::Result<datumesh_core::FunctionAnswer>
```

`Described` is `#[non_exhaustive]`, so a new field is not a breaking change; `FunctionAnswer` keeps its private
fields and its constructor `FunctionAnswer::new`, which gives no publisher. The optional `publisher` of a class or
function answer is read and checked by `check_describe` and `check_describe_with` (16.8).
