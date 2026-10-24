"""finalize.py <probe dir> <oracle results> <out dir>: probes with the reference tool's results."""
import json, os, re, sys, hashlib
sys.path.insert(0, os.path.dirname(__file__))
from world import jcs, sha
src, res, out = sys.argv[1:4]
os.makedirs(out, exist_ok=True)
results = {}
for line in open(res, encoding="utf-8"):
    name, _, r = line.rstrip("\n").partition(": ")
    results.setdefault(name, []).append(r)
for f in sorted(os.listdir(src)):
    v = json.load(open(os.path.join(src, f), encoding="utf-8"))
    name = f[:-5]
    rs = results.get(name.replace(".json", ""), results.get(name))
    if rs is None:
        # receipt names mangled by the awk sub(".json"): try that form
        rs = results[re.sub(r".json", "", f)]
    r0 = rs[0]
    for k in ("error",):
        v.pop(k, None)
    if v["kind"] == "receipt":
        if r0.startswith("expected valid, got "):
            code = r0[len("expected valid, got "):].split(" ")[0]
            for k in ("canonical", "sha256", "sig", "level", "code"):
                v.pop(k, None)
            v["valid"] = False; v["error"] = code
        else:
            d = {}
            for r in rs:
                m = re.match(r"(level|code): expected 'PROBE', got (.*)$", r)
                if m:
                    d[m.group(1)] = None if m.group(2) == "None" else m.group(2).strip("'")
            v["valid"] = True; v["level"] = d["level"]
            if d.get("code"):
                v["code"] = d["code"]
            else:
                v.pop("code", None)
    else:
        if r0 == "success":
            v["valid"] = True
            obj = json.loads(v["input"])
            c = jcs(obj)
            v["canonical"] = c.decode(); v["sha256"] = sha(c)
            if v["kind"] == "slice":
                if "members" in obj:
                    v["slice_id"] = sha(jcs(obj["slice"])); v["members_hash"] = sha(jcs(obj["members"]))
                    v["entry"] = f"slice:{v['slice_id']}:{v['members_hash']}"
                else:
                    v["slice_id"] = v["sha256"]
        else:
            v["valid"] = False; v["error"] = r0.split(" ")[0]
    v["description"] = f"Probe {name}: result of the reference tool (canon.py check) at 18be70f."
    json.dump(v, open(os.path.join(out, f), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
