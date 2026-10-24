import json, os, sys, copy
sys.path.insert(0, os.path.dirname(__file__))
from world import *
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
V = []
D_AGG = desc({}, {"type": "number", "unit": "1"}, inputs=[f"{HOST}/daily", f"{HOST}/grid"], lang="expr", body="x")
H_AGG = sha(jcs(D_AGG))
D_NODECL = desc({}, {"type": "number", "unit": "1"}, inputs=[f"{HOST}/other"], lang="expr", body="x")
rng = lambda lo, hi, t="date": {"hi": hi, "lo": lo, "type": t}

def rv(vid, inputs, members_texts, manifests, value, tx=None, asof=None, lineage=None, d=D_AGG, raw_manifests=None, extra_inputs=None, rvalue="6"):
    """A computed receipt citing `inputs` entries; recompute with manifests/receipts."""
    c = {"args": {}, "fn": "agg", "fn_hash": sha(jcs(d)), "ns": HOST}
    r, h = receipt(c, rvalue, tx, asof, inputs=inputs, valid={})
    rc = {"descriptor": d, "inputs": dict(members_texts), "manifests": {}}
    for m in manifests:
        rc["manifests"][entry(m)] = json.dumps(m)
    if raw_manifests:
        rc["manifests"].update(raw_manifests)
    if extra_inputs:
        rc["inputs"].update(extra_inputs)
    if value is not None:
        rc["value"] = num(value)
    if lineage is not None:
        rc["lineage"] = lineage
    payload = jcs({k: v for k, v in r.items() if k != "sig"})
    V.append({"id": f"p/{vid}", "kind": "receipt", "description": vid, "valid": True, "input": json.dumps(r),
              "keys": KEYS, "recompute": rc, "canonical": payload.decode(), "sha256": h, "sig": r["sig"],
              "level": "PROBE", "code": "PROBE"})

d1, h1 = daily("2026-03-01", "1")
d2, h2 = daily("2026-03-02", "2")
d3, h3 = daily("2026-03-03", "3")
T = {h1: json.dumps(d1), h2: json.dumps(d2), h3: json.dumps(d3)}
S = cell("daily", H_DAILY, {"date": rng("2026-03-01", "2026-03-05")})
M = manifest(S, [h1, h2, h3])
tx = "2026-03-03T18:00:00.000000Z"
rv("r-ok", [entry(M)], T, [M], "6", tx)
rv("r-ok-no-value", [entry(M)], T, [M], None, tx)
rv("r-mismatch", [entry(M)], T, [M], "7", tx)
rv("r-tx-wrong", [entry(M)], T, [M], "6", "2026-03-02T18:00:00.000000Z", tx)
rv("r-asof-later-member", [entry(M)], T, [M], "6", tx, tx, )
rv("r-manifest-missing", [entry(M)], T, [], "6", tx)
rv("r-manifest-not-json", [entry(M)], T, [], "6", tx, raw_manifests={entry(M): "{"})
rv("r-manifest-dup-key", [entry(M)], T, [], "6", tx, raw_manifests={entry(M): '{"members":[],"members":[],"slice":{}}'})
rv("r-manifest-number", [entry(M)], T, [], "6", tx, raw_manifests={entry(M): "1"})
M_other = manifest(cell("daily", H_DAILY, {"date": rng("2026-03-01", "2026-03-06")}), [h1, h2, h3])
rv("r-manifest-other-slice", [entry(M)], T, [], "6", tx, raw_manifests={entry(M): json.dumps(M_other)})
M_fewer = manifest(S, [h1, h2])
rv("r-manifest-other-members", [entry(M)], T, [], "6", tx, raw_manifests={entry(M): json.dumps(M_fewer)})
# undeclared slice function + hash ok
D_O = desc({"date": {"type": "date", "valid": True}}, {"type": "number", "unit": "1"})
Mo = manifest({"args": {"date": "*"}, "fn": "other2", "fn_hash": H_DAILY, "ns": HOST}, [h1])
rv("r-undeclared", [entry(Mo)], T, [Mo], "1", "2026-03-01T18:00:00.000000Z")
Mo2 = manifest({"args": {"date": "*"}, "fn": "daily", "fn_hash": H_DAILY, "ns": "other.example.org"}, [h1])
rv("r-undeclared-ns", [entry(Mo2)], T, [Mo2], "1", "2026-03-01T18:00:00.000000Z")
# member outside the range and later asof
d9, h9 = daily("2026-03-09", "9", asof="2026-03-20T00:00:00.000000Z")
Mout = manifest(S, [h1, h9])
rv("r-member-outside-later-asof", [entry(Mout)], {h1: T[h1], h9: json.dumps(d9)}, [Mout], "10", "2026-03-09T18:00:00.000000Z", "2026-03-10T00:00:00.000000Z")
d9b, h9b = daily("2026-03-04", "9", asof="2026-03-20T00:00:00.000000Z")
Mlate = manifest(S, [h1, h9b])
rv("r-member-later-asof", [entry(Mlate)], {h1: T[h1], h9b: json.dumps(d9b)}, [Mlate], "10", "2026-03-04T18:00:00.000000Z", "2026-03-10T00:00:00.000000Z")
# duplicate cell (two versions of 03-01) with the second later asof
d1b, h1b = daily("2026-03-01", "5", tx="2026-03-02T00:00:00.000000Z")
Mdup = manifest(S, [h1, h1b])
rv("r-dup-cell", [entry(Mdup)], {h1: T[h1], h1b: json.dumps(d1b)}, [Mdup], "6", "2026-03-02T00:00:00.000000Z")
d1c, h1c = daily("2026-03-01", "5", tx="2026-03-02T00:00:00.000000Z", asof="2026-03-30T00:00:00.000000Z")
Mdup2 = manifest(S, [h1, h1c])
rv("r-dup-cell-later-asof", [entry(Mdup2)], {h1: T[h1], h1c: json.dumps(d1c)}, [Mdup2], "6", "2026-03-02T00:00:00.000000Z", "2026-03-03T00:00:00.000000Z")
# member of another fn_hash
dx, hx = receipt(cell("daily", "0" * 64, {"date": date("2026-03-01")}), "1", "2026-03-01T18:00:00.000000Z")
Mx = manifest(S, [hx])
rv("r-member-other-fnhash", [entry(Mx)], {hx: json.dumps(dx)}, [Mx], "1", "2026-03-01T18:00:00.000000Z")
# member invalid signature (unknown key) and outside
dk, hk = daily("2026-03-09", "1", key=SK2)
Mk = manifest(S, [hk])
rv("r-member-badsig-outside", [entry(Mk)], {hk: json.dumps(dk)}, [Mk], "1", "2026-03-09T18:00:00.000000Z")
# member text under another hash
Mh = manifest(S, [h1])
rv("r-member-wrong-hash", [entry(Mh)], {h1: T[h2]}, [Mh], "1", "2026-03-01T18:00:00.000000Z")
# grid: k number range numerically
def grid(k, s, v, tx="2026-03-01T00:00:00.000000Z", kty="number"):
    return receipt(cell("grid", H_GRID, {"k": {"type": kty, "value": k}, "s": string(s)}), v, tx, valid={})
g10, hg10 = grid("10", "x", "1")
g95, hg95 = grid("9.5", "x", "1")
g11, hg11 = grid("11", "x", "1")
gs, hgs = grid("10", "x", "1", kty="string")
gy, hgy = grid("10", "y", "1")
Sg = cell("grid", H_GRID, {"k": rng("9", "10.5", "number"), "s": string("x")})
GT = {hg10: json.dumps(g10), hg95: json.dumps(g95), hg11: json.dumps(g11), hgs: json.dumps(gs), hgy: json.dumps(gy)}
gtx = "2026-03-01T00:00:00.000000Z"
for name, mem in [("g-numeric", [hg10, hg95]), ("g-outside", [hg11]), ("g-type", [hgs]), ("g-bound", [hgy])]:
    m = manifest(Sg, mem)
    rv(name, [entry(m)], GT, [m], "2", gtx)
Sg_open = cell("grid", H_GRID, {"k": "*", "s": string("x")})
m = manifest(Sg_open, [hgs, hg10]); rv("g-open-any-type", [entry(m)], GT, [m], "2", gtx)
m = manifest(Sg_open, [hg10, hgs]); rv("g-open-dup-cell-diff-type", [entry(m)], GT, [m], "2", gtx)
Sg_s = cell("grid", H_GRID, {"k": "*", "s": rng("x", "z", "string")})
m = manifest(Sg_s, [hg10, hgy]); rv("g-string-range", [entry(m)], GT, [m], "2", gtx)
Sg_d = cell("grid", H_GRID, {"k": "*", "s": rng("2026-01-01", "2026-01-02", "date")})
m = manifest(Sg_d, [hg10]); rv("g-range-type-vs-member-type", [entry(m)], GT, [m], "2", gtx)
# extra / missing argument
dxa, hxa = receipt(cell("daily", H_DAILY, {"date": date("2026-03-01"), "zz": string("a")}), "1", "2026-03-01T18:00:00.000000Z")
m = manifest(S, [hxa]); rv("r-member-extra-arg", [entry(m)], {hxa: json.dumps(dxa)}, [m], "1", "2026-03-01T18:00:00.000000Z")
# version slice
pv1, hpv1 = daily("2026-03-01", "1", tx="2026-03-01T18:00:00.000000Z")
pv2, hpv2 = daily("2026-03-01", "2", tx="2026-03-05T18:00:00.000000Z")
pv3, hpv3 = daily("2026-03-02", "2", tx="2026-03-05T18:00:00.000000Z")
PV = {hpv1: json.dumps(pv1), hpv2: json.dumps(pv2), hpv3: json.dumps(pv3)}
Sv = {"cell": cell("daily", H_DAILY, {"date": date("2026-03-01")}), "from": "2026-03-01T00:00:00.000000Z", "to": "2026-03-10T00:00:00.000000Z"}
m = manifest(Sv, [hpv1, hpv2]); rv("v-ok", [entry(m)], PV, [m], "2", "2026-03-05T18:00:00.000000Z")
m = manifest(Sv, [hpv1, hpv3]); rv("v-other-cell", [entry(m)], PV, [m], "2", "2026-03-05T18:00:00.000000Z")
Sv2 = dict(Sv, to="2026-03-05T18:00:00.000000Z")
m = manifest(Sv2, [hpv2]); rv("v-to-inclusive", [entry(m)], PV, [m], "2", "2026-03-05T18:00:00.000000Z")
Sv3 = dict(Sv, **{"from": "2026-03-05T18:00:00.000000Z"})
m = manifest(Sv3, [hpv2]); rv("v-from-inclusive", [entry(m)], PV, [m], "2", "2026-03-05T18:00:00.000000Z")
# empty slice only: no tx constraint
me = manifest(S, [])
rv("e-only", [entry(me)], {}, [me], "0", "0001-01-01T00:00:00.000000Z", "2026-03-10T00:00:00.000000Z")
rv("e-only-tx-any", [entry(me)], {}, [me], "0", "2026-03-01T00:00:00.000000Z", "2026-03-10T00:00:00.000000Z")
# mixed hash and slice, both
rv("mix", sorted([h3, entry(manifest(S, [h1, h2]))]), T, [manifest(S, [h1, h2])], "6", tx)
# supersession
p1, hp1 = daily("2026-03-01", "1", tx="2026-03-01T18:00:00.000000Z")
p1l, hp1l = daily("2026-03-01", "5", tx="2026-03-01T20:00:00.000000Z", asof="2026-03-04T00:00:00.000000Z")
p1e, hp1e = daily("2026-03-01", "5", tx="2026-03-01T10:00:00.000000Z", asof="2026-03-04T00:00:00.000000Z")
p1q, hp1q = daily("2026-03-01", "5", tx="2026-03-01T18:00:00.000000Z", asof="2026-03-04T00:00:00.000000Z")
p1f, hp1f = daily("2026-03-01", "5", tx="2026-03-09T18:00:00.000000Z")
p1k, hp1k = daily("2026-03-01", "5", tx="2026-03-01T20:00:00.000000Z", key=SK2)
p2l, hp2l = daily("2026-03-02", "5", tx="2026-03-02T20:00:00.000000Z")
P = {hp1: json.dumps(p1), hp1l: json.dumps(p1l), hp1e: json.dumps(p1e), hp1q: json.dumps(p1q), hp1f: json.dumps(p1f), hp1k: json.dumps(p1k), hp2l: json.dumps(p2l)}
rtx = "2026-03-01T18:00:00.000000Z"; rasof = "2026-03-02T00:00:00.000000Z"
def sx(name, lineage, value="5", inputs_map=P, rvalue="1", ins=None):
    rv(name, ins or [hp1], inputs_map, [], value, rtx, rasof, lineage=lineage, rvalue=rvalue)
sx("x-later", [hp1l])
sx("x-earlier", [hp1e])
sx("x-equal-tx", [hp1q])
sx("x-tx-after-receipt", [hp1f])
sx("x-unavailable", [hp1l], inputs_map={hp1: P[hp1]})
sx("x-bad-sig", [hp1k])
sx("x-wrong-hash", [hp1l], inputs_map={hp1: P[hp1], hp1l: P[hp1e]})
sx("x-other-cell", [hp2l])
sx("x-both", sorted([hp1, hp1l]))
sx("x-same", [hp1])
sx("x-empty", [])
sx("x-equal-value", [hp1l], value="1")
sx("x-no-value", [hp1l], value=None)
sx("x-later-and-other", sorted([hp1l, hp2l]))
sx("x-unsorted", [hp2l, hp1l])
sx("x-not-hex", ["zz"])
# slices
Mb = manifest(S, [h1, h2]); Mc = manifest(S, [h1, h2, h3])
def ss(name, lineage, manifests, inputs_map=T, value="9"):
    rv(name, [entry(Mb)], inputs_map, [Mb] + manifests, value, "2026-03-02T18:00:00.000000Z", "2026-03-04T00:00:00.000000Z", lineage=lineage, rvalue="3")
ss("y-other-members", [entry(Mc)], [Mc])
ss("y-other-members-no-manifest", [entry(Mc)], [])
ss("y-other-slice", [entry(manifest(cell("daily", H_DAILY, {"date": "*"}), [h1, h2, h3]))], [manifest(cell("daily", H_DAILY, {"date": "*"}), [h1, h2, h3])])
ss("y-both", sorted([entry(Mb), entry(Mc)]), [Mc])
ss("y-fake-entry", ["slice:" + entry(Mb).split(":")[1] + ":" + "0" * 64], [])
ss("y-malformed-rest", ["slice:" + entry(Mb).split(":")[1] + ":zz"], [])
ss("y-prefix-only", ["slice:" + entry(Mb).split(":")[1]], [])
ss("y-non-string-item", [1, None, {"a": 1}, entry(Mc)], [Mc])
ss("y-only-non-string", [1, None], [])
sx("x-non-string-item", [7, hp1l])
for v in V:
    json.dump(v, open(os.path.join(out, v["id"].split("/")[1] + ".json"), "w"), ensure_ascii=False)
