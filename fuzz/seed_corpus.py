#!/usr/bin/env python3
"""Seed fuzz/corpus/<target> from the spec's vectors. Run from the repository root."""
import glob, json, os
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
def put(target, name, data):
    d = os.path.join(root, "fuzz", "corpus", target); os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, name), "wb") as f: f.write(data)
for path in sorted(glob.glob(os.path.join(root, "spec", "vectors", "*", "*.json"))):
    v = json.load(open(path, encoding="utf-8")); name = v["id"].replace("/", "_")
    inp = v.get("input")
    raw = inp.encode("utf-8", "surrogatepass") if isinstance(inp, str) else bytes.fromhex(v.get("input_hex", ""))
    if v["kind"] == "receipt":
        for mode in range(32): put("receipt", f"{name}_{mode}", bytes([mode]) + raw)
        try:
            r = json.loads(raw)
            date = "date" if "date" in r.get("valid", {}) else ""
            fields = [json.dumps(r["cell"]), r["type"], str(r["value"]).lower() if isinstance(r["value"], bool) else r["value"],
                      date, r["asof"], r["tx"], r["issued"], r["publisher"], r["computed_by"].split("#")[0],
                      r["licence"], r["summary"], ",".join(r["inputs"])]
            put("sign", name, "\0".join(fields).encode("utf-8", "surrogatepass"))
        except Exception:
            pass
    elif v["kind"] == "uri":
        put("uri", name, raw); put("uri", name + "_sig", raw + b"\n" + json.dumps(v["signature"]).encode())
    elif v["kind"] == "envelope":
        put("envelope", name, v["status"].to_bytes(2, "little") + raw)
    if raw:
        put("json", name, raw)
