#!/bin/sh
# Oracle probes for spec-0.4.0 (DEFECTS.md, "How the oracle was used"): generate the probe
# vectors (gen1: slice objects, manifests, descriptors with order and extent; gen2: receipts
# citing slices, membership, E_INPUT_SUPERSEDED; gen3: extent ranges), run them through the
# reference tool as a black box (`canon.py check` on a copy of spec/ whose vectors/ holds only
# the probes), record its results in the probes, check them once more, and write
# crates/datumesh-cli/tests/probes-0.4.0.jsonl. Needs Docker and the datumesh-test image.
# Usage: tools/probes/run.sh <scratch dir>
set -e
root="$(cd "$(dirname "$0")/../.." && pwd)"
w="$1"; [ -n "$w" ] || { echo "usage: $0 <scratch dir>" >&2; exit 2; }
rm -rf "$w/raw" "$w/final" "$w/oracle"; mkdir -p "$w/raw" "$w/oracle/vectors"
cp -r "$root/spec/tools" "$root/spec/units.md" "$root/spec/schemas" "$w/oracle/"
for g in gen1 gen2 gen3 gen4; do
  docker run --rm -v "$root/tools/probes:/p:ro" -v "$w:/w" datumesh-test python -I /p/$g.py /w/raw >/dev/null
done
cp -r "$w/raw" "$w/oracle/vectors/p"
docker run --rm -v "$w/oracle:/o" -w /o datumesh-test python tools/canon.py check 2>&1 |
  awk '/^FAIL/{n=$2; sub(".*/","",n); sub(/\.json$/,"",n); next} /^ /{sub(/^ +/,""); sub("expected PROBE, got ",""); print n": "$0}' > "$w/results.txt"
python3 -I "$root/tools/probes/finalize.py" "$w/raw" "$w/results.txt" "$w/final"
rm -rf "$w/oracle/vectors/p"; cp -r "$w/final" "$w/oracle/vectors/p"
docker run --rm -v "$w/oracle:/o" -w /o datumesh-test python tools/canon.py check 2>&1 | tail -1
python3 -I - "$w/final" "$root/crates/datumesh-cli/tests/probes-0.4.0.jsonl" <<'PY'
import json, os, sys
d, out = sys.argv[1:3]
with open(out, "w", encoding="utf-8") as f:
    for n in sorted(os.listdir(d)):
        f.write(json.dumps(json.load(open(os.path.join(d, n), encoding="utf-8")), ensure_ascii=False, separators=(",", ":")) + "\n")
PY
