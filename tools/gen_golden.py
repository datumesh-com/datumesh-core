"""Derive datumesh-cli run-vectors' expected output, one line per vector file under DIR (sorted
by path components, as the Rust test sorts), from the files alone: id, error, and the kind's
output members the vector gives. A valid mesh vector gives none (SPEC 11 names bindings and
endpoints, the vectors do not carry them; DEFECTS.md RS-38): they are derived from the mesh
document in its env, unknown endpoints dropped. Usage: python3 tools/gen_golden.py spec/vectors"""
import json, os, sys
OUT = {"jcs": ["canonical", "sha256"], "scalar": ["canonical", "sha256"], "cell": ["canonical", "sha256"],
       "ref": ["canonical", "sha256"], "function": ["canonical", "sha256"],
       "uri": ["object", "canonical", "sha256", "https"], "keyid": ["key_id"], "keys": ["key_ids"],
       "receipt": ["canonical", "sha256", "sig", "level", "blob_checked", "code"], "unit": ["unit"],
       "envelope": ["outcome", "trust", "receipt_hash"],
       "slice": ["canonical", "sha256", "slice_id", "members_hash", "entry"],
       "mesh": ["bindings", "endpoints"]}
KNOWN = {"blob", "cell", "cells", "descriptor", "explain", "find", "jobs", "manifest", "receipt", "slice", "verify"}
SKIP = {"formula", "formula-eval", "excel", "decimal"}
root = sys.argv[1]
files = []
for d, _, fs in os.walk(root):
    files += [os.path.join(d, f) for f in fs if f.endswith(".json")]
for f in sorted(files, key=lambda p: [os.fsencode(c) for c in p.split(os.sep)]):
    v = json.load(open(f, encoding="utf-8"))
    k = v["kind"]
    if k in SKIP:
        o = {"id": v["id"], "error": None, "skipped": k}
    elif v["valid"]:
        o = {"id": v["id"], "error": None}
        for m in OUT[k]:
            if m in v:
                o[m] = v[m]
        if k == "mesh" and "bindings" not in v:
            d = json.loads(v["env"]["mesh"]["https"]["body"])
            o["bindings"] = d["bindings"]
            o["endpoints"] = {n: u for n, u in d["endpoints"].items() if n in KNOWN}
    else:
        o = {"id": v["id"], "error": v["error"]}
    line = json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    print(line.replace("\u2028", "\\u2028").replace("\u2029", "\\u2029"))
