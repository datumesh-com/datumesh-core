#!/bin/sh
# Packaging test: cargo package datumesh-core, unpack the .crate in a fresh container, build a
# new project that depends on it by path, and run the README example on
# spec/vectors/receipt/stored-number.json. Usage: tools/package_test.sh [toolchain tag]
set -e
tag="${1:-1.99}"
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/crates/datumesh-core/Cargo.toml" | head -n 1)"
"$root/tools/cargo.sh" "$tag" package -p datumesh-core
work="$(mktemp -d)"
cp "$root/target-$tag/package/datumesh-core-$version.crate" "$work/"
mkdir -p "$work/app/src"
cp "$root/spec/vectors/receipt/stored-number.json" "$work/app/"
# The README's first ```rust block is the example.
awk '/^```rust$/{f=1;next} /^```$/{if(f){exit}} f' "$root/README.md" > "$work/app/src/main.rs"
printf '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\ndatumesh-core = { path = "../datumesh-core-%s" }\n' "$version" > "$work/app/Cargo.toml"
docker run --rm -v "$work:/p" -w /p "rust:$tag" sh -c "tar xzf datumesh-core-$version.crate && cd app && cargo run -q"
