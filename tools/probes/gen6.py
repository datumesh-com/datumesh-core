"""Probes for spec-0.4.2 at fc870ab: step 3a's order, cursors, a page's order of checks."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from world import *
out = sys.argv[1]; os.makedirs(out, exist_ok=True); V = []
own, hown = daily("2026-03-01", "1")
cid_own = sha(jcs(own["cell"]))
bad = dict(own, sig="00" * 64)
def env_(vid, body, status=200, req=None):
    v = vector(vid, "envelope", input=json.dumps(body), status=status, keys=KEYS); v["request"] = req; V.append(v)
okv = lambda r, trust="primary", **kw: dict({"outcome": "ok", "receipt": r, "trust": trust}, **kw)
PIN = "a" * 64
env_("q3a-pin-missing-bad-sig", okv(bad), req={"cell_id": cid_own, "fn_hash": H_DAILY})
env_("q3a-pin-other-bad-sig", okv(bad, fn_hash=PIN), req={"cell_id": cid_own, "fn_hash": H_DAILY})
env_("q3a-unpinned-echo-other-bad-sig", okv(bad, fn_hash=PIN), req={"cell_id": cid_own})
env_("q3a-unpinned-echo-other-other-cell", okv(own, fn_hash=PIN), req={"cell_id": "0" * 64})
SL = {"args": {"date": "*"}, "fn": "daily", "fn_hash": H_DAILY, "ns": HOST}
SID = sha(jcs(SL))
m1, _ = daily("2026-03-01", "1"); m2, _ = daily("2026-03-02", "2"); m3, _ = daily("2026-03-03", "3")
mstr, _ = receipt(cell("daily", H_DAILY, {"date": string("2026-03-04")}), "4", "2026-03-04T18:00:00.000000Z", valid={})
mo, _ = receipt(cell("daily", "0" * 64, {"date": date("2026-03-04")}), "4", "2026-03-04T18:00:00.000000Z")
page = lambda members, **kw: dict({"outcome": "ok", "slice": SL, "members": members}, **kw)
mem = lambda r, **kw: dict({"outcome": "ok", "receipt": r, "trust": "primary"}, **kw)
S = {"slice_id": SID}
env_("qs-cursor-extra-name", page([mem(m3)]), req=dict(S, after={"date": date("2026-03-02"), "x": string("y")}))
env_("qs-cursor-missing", page([mem(m3)]), req=dict(S, after={"x": string("y")}))
env_("qs-cursor-other-type", page([mem(m3)]), req=dict(S, after={"date": string("2026-03-02")}))
env_("qs-members-differing-types", page([mem(m1), mem(mstr)]), req=S)
env_("qs-bad-sig-then-bad-shape", page([mem(dict(m1, sig="00" * 64)), dict(mem(m2), x=1)]), req=S)
env_("qs-bad-sig-then-stray", page([mem(dict(m1, sig="00" * 64)), mem(mo)]), req=S)
env_("qs-shape-then-bad-sig", page([dict(mem(m1), x=1), mem(dict(m2, sig="00" * 64))]), req=S)
env_("qs-member-echo-other-then-bad-sig", page([mem(m1, fn_hash=PIN), mem(dict(m2, sig="00" * 64))]), req=S)
env_("qs-page-fn-hash-other", page([mem(m1)], fn_hash=PIN), req=S)
env_("qs-slice-other-and-member-bad-sig", dict(page([mem(dict(m1, sig="00" * 64))]), slice=dict(SL, fn="d2")), req=S)
V.append(vector("qb-item-status-true", "batch", input=json.dumps([{"envelope": okv(own), "status": True}]), status=200, keys=KEYS, requests=[{"cell_id": cid_own}]))
V.append(vector("qb-item-status-exp", "batch", input=json.dumps([{"envelope": okv(own), "status": 200}]).replace("200", "2e2"), status=200, keys=KEYS, requests=[{"cell_id": cid_own}]))
for v in V:
    json.dump(v, open(os.path.join(out, v["id"].split("/")[1] + ".json"), "w"), ensure_ascii=False)
