"""Probes for spec-0.4.1: mesh documents (resolution order, URL rules) and pending envelopes."""
import json, os, sys, copy
sys.path.insert(0, os.path.dirname(__file__))
from world import *
out = sys.argv[1]; os.makedirs(out, exist_ok=True); V = []
H = "mesh.example.org"
KD = {"host": H, "keys": [{"alg": "ed25519", "id": KID, "public": PUB, "valid_from": "2020-01-01T00:00:00.000000Z"}], "proto": "datumesh/1"}
EP = {"cell": f"https://{H}/cell/{{cell}}", "descriptor": f"https://{H}/d/{{hash}}", "jobs": f"https://{H}/jobs/{{job}}",
      "manifest": f"https://{H}/m/{{slice_id}}/{{members_hash}}", "receipt": f"https://{H}/r/{{hash}}"}
MD = {"bindings": f"https://{H}/bindings", "endpoints": EP, "host": H, "proto": "datumesh/1"}
def env(body=None, status=200, raw=None):
    if status is None: return {"https": {"status": "error"}}
    e = {"https": {"status": status}}
    if raw is not None: e["https"]["body"] = raw
    elif body is not None: e["https"]["body"] = json.dumps(body)
    return e
def mesh(vid, keys=env(KD), md=env(MD), host=H, drop=None):
    e = {"keys": keys, "mesh": md}
    if drop: e.pop(drop)
    V.append(vector(vid, "mesh", host=host, env=e))
def md(**kw):
    d = copy.deepcopy(MD)
    for k, v in kw.items():
        if v is None: d.pop(k, None)
        else: d[k] = v
    return env(d)
def ep(**kw):
    e = dict(EP)
    for k, v in kw.items():
        if v is None: e.pop(k, None)
        else: e[k] = v
    return md(endpoints=e)
mesh("md-ok")
mesh("md-no-keys-member", drop="keys")
mesh("md-no-mesh-member", drop="mesh")
mesh("md-both-unavailable", keys=env(status=None), md=env(status=None))
mesh("md-key-404-mesh-bad", keys=env(status=404), md=env(raw="{"))
mesh("md-key-200-nobody", keys=env(status=200))
mesh("md-key-200-nobody-mesh-404", keys=env(status=200), md=env(status=404))
mesh("md-mesh-200-nobody", md=env(status=200))
mesh("md-key-bad-json-mesh-bad-json", keys=env(raw="{"), md=env(raw="{"))
mesh("md-key-bad-json-mesh-404", keys=env(raw="{"), md=env(status=404))
mesh("md-key-not-object", keys=env(raw="[]"))
mesh("md-key-bad-json-mesh-not-object", keys=env(raw="{"), md=env(raw="[]"))
mesh("md-key-host-number", keys=env(dict(KD, host=1)))
mesh("md-key-no-host", keys=env({k: v for k, v in KD.items() if k != "host"}))
mesh("md-key-no-proto", keys=env({k: v for k, v in KD.items() if k != "proto"}))
mesh("md-key-proto-other-both", keys=env(dict(KD, proto="datumesh/2")), md=md(proto="datumesh/2"))
mesh("md-key-bad-keys", keys=env(dict(KD, keys=[])))
mesh("md-key-bad-keys-mesh-bad-bindings", keys=env(dict(KD, keys=[])), md=md(bindings="http://x"))
mesh("md-mesh-host-number", md=md(host=1))
mesh("md-mesh-no-host", md=md(host=None))
mesh("md-mesh-no-proto", md=md(proto=None))
mesh("md-host-uppercase", host="MESH.example.org", keys=env(dict(KD, host="MESH.example.org")), md=md(host="MESH.example.org"))
mesh("md-bindings-missing", md=md(bindings=None))
mesh("md-bindings-number", md=md(bindings=1))
mesh("md-bindings-placeholder", md=md(bindings=f"https://{H}/b/{{x}}"))
mesh("md-bindings-no-path", md=md(bindings=f"https://{H}"))
mesh("md-bindings-query", md=md(bindings=f"https://{H}?a=1"))
mesh("md-bindings-port", md=md(bindings=f"https://{H}:443/b"))
mesh("md-bindings-userinfo", md=md(bindings=f"https://u@{H}/b"))
mesh("md-bindings-long", md=md(bindings=f"https://{H}/" + "a" * 600))
mesh("md-bindings-space", md=md(bindings=f"https://{H}/a b"))
mesh("md-bindings-host-suffix", md=md(bindings=f"https://{H}.evil/b"))
mesh("md-bindings-bad-and-endpoints-missing", md=md(bindings=1, endpoints=None))
mesh("md-endpoints-array", md=md(endpoints=[]))
mesh("md-endpoint-number", md=ep(cell=1))
mesh("md-endpoint-unknown-number", md=ep(x_y=1))
mesh("md-endpoint-unknown-bad-url", md=ep(x_y="ftp://x"))
mesh("md-endpoint-dup-placeholder", md=ep(cell=f"https://{H}/c/{{cell}}/{{cell}}"))
mesh("md-endpoint-brace-close", md=ep(cell=f"https://{H}/c/{{cell}}}}"))
mesh("md-endpoint-unclosed", md=ep(cell=f"https://{H}/c/{{cell}}/{{"))
mesh("md-endpoint-placeholder-in-host", md=ep(cell=f"https://{{cell}}/c"))
mesh("md-endpoint-optional-ok", md=ep(blob=f"https://{H}/b/{{sha256}}", cells=f"https://{H}/cells", find=f"https://{H}/find"))
mesh("md-endpoint-cells-placeholder", md=ep(cells=f"https://{H}/cells/{{cell}}"))
mesh("md-endpoint-order", md=ep(receipt=f"https://{H}/r", cell="http://x/{cell}"))
mesh("md-endpoint-jobs-not-suffix", md=ep(jobs=f"https://{H}/j/{{job}}/x"))
mesh("md-endpoint-root-path", md=ep(cells=f"https://{H}/"))
mesh("md-extra-top", md=dict(json.loads(md()["https"]["body"]), x_a=[1, {"a": None}]) and env(dict(MD, x_a=[1])))
mesh("md-dup-in-unknown", md=env(raw=json.dumps(MD)[:-1] + ', "x": {"a": 1, "a": 2}}'))
mesh("md-key-not-object-mesh-host-other", keys=env(raw="[]"), md=md(host="other.example.org"))
mesh("md-key-proto-number", keys=env(dict(KD, proto=1)))
mesh("md-key-host-other-mesh-host-other", keys=env(dict(KD, host="o.example.org")), md=md(host="o.example.org"))
mesh("md-key-extra-member", keys=env(dict(KD, x=1)))
mesh("md-mesh-proto-number-key-bad", keys=env(dict(KD, keys=[])), md=md(proto=1))
# pending
CID = "f566f2838815a8649fe467b7a41876c5d2b4deca9afb974b8c50caafd4633ca5"
J = "4f9c2a7e1b3d5f60817a9c2e4b6d8f01"
P = {"estimate_seconds": 5, "job": J, "outcome": "pending", "poll": f"https://mesh.example/jobs/{J}", "request": {"asof": "2024-03-16T09:00:00.000000Z", "cell_id": CID}}
REQ = {"asof": "2024-03-16T09:00:00.000000Z", "cell_id": CID}
def pend(vid, body=None, status=202, request=REQ, raw=None):
    V.append(vector(vid, "envelope", input=raw if raw is not None else json.dumps(body), status=status, keys={}, request=request))
def p(**kw):
    d = copy.deepcopy(P)
    for k, v in kw.items():
        if v is None: d.pop(k, None)
        else: d[k] = v
    return d
pend("pe-ok", p())
pend("pe-job-64", p(job="a" * 64, poll="https://mesh.example/jobs/" + "a" * 64))
pend("pe-job-65", p(job="a" * 65, poll="https://mesh.example/jobs/" + "a" * 65))
pend("pe-job-31", p(job="a" * 31, poll="https://mesh.example/jobs/" + "a" * 31))
pend("pe-job-number", p(job=1))
pend("pe-estimate-float", raw=json.dumps(p()).replace('"estimate_seconds": 5', '"estimate_seconds": 5.0'))
pend("pe-estimate-string", p(estimate_seconds="5"))
pend("pe-estimate-big", raw=json.dumps(p()).replace('"estimate_seconds": 5', '"estimate_seconds": 9007199254740992'))
pend("pe-poll-long", p(poll="https://mesh.example/" + "a" * 520 + "/jobs/" + J))
pend("pe-poll-just-jobs", p(poll="https://jobs/" + J))
pend("pe-poll-space", p(poll="https://mesh.example/x y/jobs/" + J))
pend("pe-poll-other-host", p(poll="https://other.example/jobs/" + J))
pend("pe-poll-trailing-slash", p(poll="https://mesh.example/jobs/" + J + "/"))
pend("pe-poll-query", p(poll="https://mesh.example/jobs/" + J + "?x=1"))
pend("pe-request-no-asof-requested", p(request={"cell_id": CID}), request={"cell_id": CID})
pend("pe-request-asof-bad", p(request={"asof": "2024-03-16", "cell_id": CID}))
pend("pe-request-cell-upper", p(request={"asof": REQ["asof"], "cell_id": CID.upper()}))
pend("pe-request-not-object", p(request=[CID]))
pend("pe-request-asof-later", p(request={"asof": "2024-03-17T09:00:00.000000Z", "cell_id": CID}))
pend("pe-request-asof-earlier", p(request={"asof": "2024-03-15T09:00:00.000000Z", "cell_id": CID}))
pend("pe-request-job-member", p(request=dict(REQ, job=J)), request=dict(REQ, job=J))
pend("pe-status-200", p(), status=200)
pend("pe-with-reason", p(reason="x"))
pend("pe-polled-ok", p(), request=dict(REQ, job=J))
pend("pe-polled-other", p(), request=dict(REQ, job="b" * 32))
pend("pe-error-job-unknown-polled", {"error": "E_JOB_UNKNOWN", "outcome": "error"}, status=500, request=dict(REQ, job=J))
pend("pe-bad-request-job-unknown-polled", {"error": "E_JOB_UNKNOWN", "outcome": "bad_request"}, status=400, request=dict(REQ, job=J))
pend("pe-bad-request-parse-polled", {"error": "E_PARSE_URI", "outcome": "bad_request"}, status=400, request=dict(REQ, job=J))
pend("pe-not-covered-polled", {"outcome": "not_covered"}, status=404, request=dict(REQ, job=J))
pend("pe-bad-request-mesh-doc", {"error": "E_MESH_DOC", "outcome": "bad_request"}, status=400)
pend("pe-error-mesh-doc", {"error": "E_MESH_DOC", "outcome": "error"}, status=500)
pend("pe-error-envelope", {"error": "E_ENVELOPE", "outcome": "error"}, status=500)
for v in V:
    json.dump(v, open(os.path.join(out, v["id"].split("/")[1] + ".json"), "w"), ensure_ascii=False)
