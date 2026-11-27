"""Check barcodes on the direct-printing path: the core renders each type
(example data) to a bitmap with resvg at a printer resolution, and zxing-cpp
must decode it. Needs the dev server: cargo run -p labelforge-dev

Usage: pip install zxing-cpp pillow; python3 tools/verify_raster.py [dpi ...]   (default 203 300 600)"""
import base64, io, json, sys, urllib.request
from PIL import Image
import zxingcpp
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from decode import UNSUPPORTED, DECODED, norm  # noqa: E402

API = "http://127.0.0.1:8765/api/"

def post(cmd, body):
    req = urllib.request.Request(API + cmd, data=json.dumps(body).encode(), method="POST")
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read())

def label(s):
    square = s["group"] in ("two_d", "hibc") and "39" not in s["key"] and "128" not in s["key"]
    return {"schema": 1, "name": "t", "width_mm": 60, "height_mm": 40 if square else 30, "corner_radius_mm": 0,
            "dpi": DPI, "variables": [],
            "objects": [{"id": "b", "x_mm": 6, "y_mm": 4, "w_mm": 30 if square else 48, "h_mm": 30 if square else 20,
                         "kind": {"type": "barcode", "symbology": s["key"], "data": s["example"], "show_text": True}}]}

syms = post("symbologies", {})
bad = 0
for DPI in [int(a) for a in sys.argv[1:]] or [203, 300, 600]:
    counts = {"PASS": 0, "FAIL": 0, "SKIP": 0}
    for s in syms:
        key = s["key"]
        png = base64.b64decode(post("png", {"label": label(s), "dpi": DPI}))
        got = [r.text for r in zxingcpp.read_barcodes(Image.open(io.BytesIO(png)))]
        w = norm(DECODED.get(key, s["example"]))
        ok = any(norm(g) == w or norm(g).upper() == w.upper() or (key == "codabar" and norm(g).strip("ABCD") == w.strip("ABCD"))
                 or (key.startswith("hibc") and w.upper() in norm(g).upper()) for g in got)
        st = "PASS" if ok else ("SKIP" if key in UNSUPPORTED else "FAIL")
        counts[st] += 1
        if st == "FAIL":
            bad += 1
            print(f"FAIL {DPI}dpi {key}: got {got}")
    print(f"{DPI} dpi: {counts}")
sys.exit(1 if bad else 0)
