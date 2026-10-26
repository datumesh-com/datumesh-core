"""fill.py <dir> <results>: replace the PROBE outputs of valid envelope and batch probes with
what the reference tool gave ("<field>: expected 'PROBE', got <python repr>"); a field it
gave as None is dropped."""
import ast, json, os, re, sys
d, res = sys.argv[1:3]
FIELDS = ("outcome", "trust", "receipt_hash", "results", "object", "canonical", "sha256", "https")
got = {}
for line in open(res, encoding="utf-8"):
    name, _, r = line.rstrip("\n").partition(": ")
    m = re.match(r"(outcome|trust|receipt_hash|results|object|canonical|sha256|https): expected 'PROBE', got (.*)$", r)
    if m:
        got.setdefault(name, {})[m.group(1)] = ast.literal_eval(m.group(2))
for f in os.listdir(d):
    p = os.path.join(d, f)
    v = json.load(open(p, encoding="utf-8"))
    if not any(v.get(k) == "PROBE" for k in FIELDS):
        continue
    g = got.get(f[:-5], {})
    for k in FIELDS:
        if v.get(k) == "PROBE":
            if g.get(k) is None:
                del v[k]
            else:
                v[k] = g[k]
    json.dump(v, open(p, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
