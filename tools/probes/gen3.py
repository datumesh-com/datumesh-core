import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from world import *
out = sys.argv[1]; os.makedirs(out, exist_ok=True); V = []
def fn(vid, ext, ty="number"):
    a = {"type": ty, "valid": False, "extent": ext}
    if ty == "number": a["unit"] = "1"
    V.append(vector(vid, "function", input=json.dumps(desc({"a": a}, "date"))))
fn("hi-dec-lo-noncanon", {"hi": "1.5", "lo": "01"})
fn("hi-ok-lo-dec", {"hi": "5", "lo": "1.5"})
fn("reversed-int", {"hi": "1", "lo": "5"})
fn("reversed-date", {"hi": "2026-01-01", "lo": "2026-01-05"}, "date")
fn("neg-reversed", {"hi": "-10", "lo": "-1"})
fn("big", {"hi": "100000000000000000000000000000000000000000", "lo": "-100000000000000000000000000000000000000000"})
fn("values-number-bad-then-unsorted", ["2", "1", "01"])
fn("values-dec", ["-1.5", "-1", "0.25", "3"])
fn("values-neg-order", ["-1", "-1.5"])
fn("string-nfd", ["Zürich"], "string")
fn("string-empty", [""], "string")
fn("blob-values", [], "blob")
fn("number-range-string-hi", {"hi": "a", "lo": "1"})
for v in V:
    json.dump(v, open(os.path.join(out, v["id"].split("/")[1] + ".json"), "w"), ensure_ascii=False)
