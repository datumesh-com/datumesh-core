"""Probe world: my own keys, descriptors and receipts, for black-box runs of canon.py."""
import json, hashlib
from nacl.signing import SigningKey

def jcs(v):
    return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()

def sha(b):
    return hashlib.sha256(b).hexdigest()

HOST = 'probe.example.org'
SK = SigningKey(bytes(range(32)))
PUB = SK.verify_key.encode().hex()
KID = hashlib.sha256(SK.verify_key.encode()).hexdigest()[:16]
SK2 = SigningKey(bytes(range(1, 33)))  # a key not in the document
KEYDOC = {"host": HOST, "keys": [{"alg": "ed25519", "id": KID, "public": PUB, "valid_from": "2020-01-01T00:00:00.000000Z"}], "proto": "datumesh/1"}
KEYS = {HOST: {"https": {"status": 200, "body": json.dumps(KEYDOC)}}}

def desc(args, returns, inputs=(), lang='stored', body='', order=None):
    sig = {"args": args, "returns": returns}
    if order is not None:
        sig["order"] = order
    return {"code": {"body": body, "lang": lang}, "inputs": list(inputs), "signature": sig}

def num(v): return {"type": "number", "value": v}
def date(v): return {"type": "date", "value": v}
def string(v): return {"type": "string", "value": v}

D_DAILY = desc({"date": {"type": "date", "valid": True}}, {"type": "number", "unit": "1"})
H_DAILY = sha(jcs(D_DAILY))
D_GRID = desc({"k": {"type": "number", "unit": "1", "valid": False}, "s": {"type": "string", "valid": False}}, {"type": "number", "unit": "1"})
H_GRID = sha(jcs(D_GRID))

def cell(fn, fn_hash, args):
    return {"args": args, "fn": fn, "fn_hash": fn_hash, "ns": HOST}

def receipt(c, value, tx, asof=None, inputs=(), issued=None, key=SK, ty='number', valid=None):
    asof = asof or tx
    issued = issued or asof
    if valid is None:
        valid = {k: v for k, v in c['args'].items() if k == 'date'}
    r = {"asof": asof, "cell": c, "computed_by": f"{HOST}#{KID}", "inputs": sorted(inputs), "issued": issued,
         "licence": "CC0-1.0", "proto": "datumesh/1", "publisher": HOST, "summary": "probe", "tx": tx,
         "type": ty, "valid": valid, "value": value}
    payload = jcs(r)
    r = dict(r, sig=key.sign(payload).signature.hex())
    return r, sha(payload)

def daily(d, value, tx=None, asof=None, **kw):
    tx = tx or f"{d}T18:00:00.000000Z"
    return receipt(cell("daily", H_DAILY, {"date": date(d)}), value, tx, asof, **kw)

def manifest(slice_obj, members):
    return {"members": sorted(members), "slice": slice_obj}

def entry(m):
    return f"slice:{sha(jcs(m['slice']))}:{sha(jcs(m['members']))}"

def vector(vid, kind, **kw):
    v = {"id": f"p/{vid}", "kind": kind, "description": vid, "valid": False, "error": "PROBE"}
    v.update(kw)
    return v
