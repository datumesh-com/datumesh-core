"""Probes for spec-0.4.3 at 99123b2: declarations in the key document (corners the vectors leave),
licence inheritance (mixes and declaration windows), and the entitlement check's inputs."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from world import *
out = sys.argv[1]; os.makedirs(out, exist_ok=True); V = []
R = "https://probe.example.org/licences/restricted"
FN = f"{HOST}/daily"
E = lambda lic=R, f="2020-01-01T00:00:00.000000Z", t=None: dict({"licence": lic, "valid_from": f}, **({"valid_to": t} if t else {}))

def keydoc(vid, decl, raw=None, host=HOST):
    doc = dict(KEYDOC, host=host)
    if decl is not None:
        doc["declarations"] = decl
    body = raw if raw is not None else json.dumps(doc)
    V.append(vector(vid, "keys", host=host, env={"https": {"status": 200, "body": body}}))

keydoc("kd-null", None, raw=json.dumps(dict(KEYDOC, declarations=None)))
keydoc("kd-path-namespace", {f"{HOST}/stats/m/mean": [E()]})
keydoc("kd-host-only", {HOST: [E()]})
keydoc("kd-uppercase-fn", {f"{HOST}/Daily": [E()]})
keydoc("kd-bad-segment", {f"{HOST}/a__b/daily": [E()]})
keydoc("kd-name-513", {f"{HOST}/" + "a" * 64 + "/" + "/".join(["b" * 64] * 6) + "/" + "c" * 40: [E()]})
keydoc("kd-name-512", {f"{HOST}/" + "a" * 64 + "/" + "/".join(["b" * 64] * 6) + "/" + "c" * 39: [E()]})
keydoc("kd-spdx-64", {FN: [E("A" * 64)]})
keydoc("kd-spdx-65", {FN: [E("A" * 65)]})
keydoc("kd-url-512", {FN: [E("https://" + "x" * 504)]})
keydoc("kd-url-513", {FN: [E("https://" + "x" * 505)]})
keydoc("kd-url-bare", {FN: [E("https://")]})
keydoc("kd-spdx-leading-dot", {FN: [E(".MIT")]})
keydoc("kd-from-no-micros", {FN: [E(f="2020-01-01T00:00:00Z")]})
keydoc("kd-to-null", {FN: [dict(E(), valid_to=None)]})
keydoc("kd-entry-string", {FN: ["CC-BY-NC-4.0"]})
keydoc("kd-value-object", {FN: E()})
keydoc("kd-duplicate-entries", {FN: [E(), E()]})
keydoc("kd-open-licence", {FN: [E("CC0-1.0")]})
keydoc("kd-overlapping", {FN: [E(t="2025-01-01T00:00:00.000000Z"), E(f="2024-01-01T00:00:00.000000Z")]})
keydoc("kd-other-host-suffix", {"probe.example.org.evil.example/daily": [E()]})
keydoc("kd-parent-host", {"example.org/daily": [E()]})
keydoc("kd-unrooted-host", {"local.invalid/daily": [E()]}, host="local.invalid")
keydoc("kd-dup-name", None, raw=json.dumps(KEYDOC)[:-1] + ', "declarations": {"%s": [%s], "%s": [%s]}}' % (FN, json.dumps(E()), FN, json.dumps(E())))
keydoc("kd-from-feb-30", {FN: [E(f="2020-02-30T00:00:00.000000Z")]})

def lic(vid, licences, decl=None, issued="2024-03-16T09:00:00.000000Z", mesh="CC-BY-4.0", fn=FN):
    V.append(vector(vid, "licence", licences=licences, declarations=decl or {}, fn=fn, issued=issued, mesh_licence=mesh))
    V[-1]["valid"] = True; V[-1].pop("error"); V[-1]["licence"] = "PROBE"; V[-1]["reason"] = "PROBE"

C2 = "https://probe.example.org/licences/other"
lic("l-two-restricted-one-sa", [R, C2, "ODbL-1.0"])
lic("l-restricted-twice-declared-pd", [R, "CC0-1.0", R], {FN: [E()]})
lic("l-sa-twice-pd-attr", ["ODbL-1.0", "MIT", "ODbL-1.0", "PDDL-1.0"])
lic("l-declared-other-licence-and-closed", [R], {FN: [E(C2), E(t="2021-01-01T00:00:00.000000Z")]})
lic("l-declared-other-licence-open-only", [R], {FN: [E(C2)]})
lic("l-no-inputs-with-declarations", [], {FN: [E()]})
lic("l-declared-for-other-host-fn", [R], {"other.example.org/daily": [E()]})
lic("l-open-at-micro-before-to", [R], {FN: [E(t="2024-03-16T09:00:00.000001Z")]})
lic("l-pd-attr-sa-restricted-declared", ["CC0-1.0", "MIT", "CC-BY-SA-4.0", R], {FN: [E()]})
lic("l-attr-order-ogl-apache", ["OGL-UK-3.0", "Apache-2.0"])
lic("l-cc-by-sa-lower", ["cc-by-sa-4.0", "CC-BY-SA-4.0"])
lic("l-closed-then-open-other-licence", [R, C2], {FN: [E(), E(C2)]})

# Entitlement: a computed receipt over daily receipts, K with declarations.
D_SUM = desc({}, {"type": "number", "unit": "1"}, inputs=[f"{HOST}/daily"], lang="expr", body="x")
H_SUM = sha(jcs(D_SUM))
KD = dict(KEYDOC, declarations={f"{HOST}/sum": [E()]})
KEYS_D = {HOST: {"https": {"status": 200, "body": json.dumps(KD)}}}
i1, h1 = daily("2026-03-01", "1", licence="CC-BY-4.0")
i2, h2 = daily("2026-03-02", "2", licence=R)
SLICE = cell("daily", H_DAILY, {"date": "*"})
def summ(inputs, licence, **kw):
    return receipt(cell("sum", H_SUM, {}), "3", "2026-03-02T18:00:00.000000Z", inputs=inputs, licence=licence, valid={}, **kw)
def ent(vid, r, receipts, manifests=None, keys=KEYS_D):
    v = vector(vid, "entitlement", input=json.dumps(r), keys=keys, receipts={h: json.dumps(x) for h, x in receipts.items()}, manifests={e: json.dumps(m) for e, m in (manifests or {}).items()})
    V.append(v)
r, _ = summ([h1, h2], R)
ent("e-declared", r, {h1: i1, h2: i2})
ent("e-input-bad-sig", r, {h1: i1, h2: dict(i2, sig="00" * 64)})
ent("e-receipt-bad-sig", dict(r, sig="00" * 64), {h1: i1, h2: i2})
ent("e-no-keys-for-host", r, {h1: i1, h2: i2}, keys={})
ent("e-keydoc-without-declarations", r, {h1: i1, h2: i2}, keys=KEYS)
m_empty = manifest(SLICE, [])
r, _ = summ([entry(m_empty)], "PDDL-1.0")
ent("e-empty-slice-only", r, {}, {entry(m_empty): m_empty})
m12 = manifest(SLICE, [h1, h2])
r, _ = summ([entry(m12), h1], R)
ent("e-slice-and-direct-input", r, {h1: i1, h2: i2}, {entry(m12): m12})
r, _ = summ([h1], "CC-BY-4.0", issued="2026-03-02T18:00:00.000000Z")
ent("e-open-attr", r, {h1: i1})
for v in V:
    json.dump(v, open(os.path.join(out, v["id"].split("/")[1] + ".json"), "w"), ensure_ascii=False)
