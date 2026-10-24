#!/bin/sh
# Run cargo in a toolchain container: tools/cargo.sh <image-tag> <cargo args...>
# e.g. tools/cargo.sh 1.99 test --workspace ; tools/cargo.sh nightly fuzz ...
set -e
tag="$1"; shift
case "$tag" in nightly) image=rustlang/rust:nightly ;; nightly-tools) image=datumesh-nightly-tools ;; lint) image=datumesh-rust-1.99-lint ;; *) image="rust:$tag" ;; esac
root="$(cd "$(dirname "$0")/.." && pwd)"
exec docker run --rm -i -v "$HOME/.cache/datumesh-cargo:/usr/local/cargo/registry" -v "$root:/w" -w /w \
  -e CARGO_TARGET_DIR="/w/target-$tag" -e CARGO_TERM_COLOR=never -e TRYBUILD -e DATUMESH_TESTKIT_WRITE "$image" cargo "$@"
