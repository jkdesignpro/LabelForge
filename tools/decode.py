"""Decode the PNGs written by verify-barcodes.cjs and compare with the input.
Check digits added by the symbology (EAN) are allowed at the end.
Usage: pip install zxing-cpp pillow; python3 tools/decode.py barcode-check/"""
import json, sys, zxingcpp
from PIL import Image

d = sys.argv[1] if len(sys.argv) > 1 else "barcode-check"
exp = json.load(open(f"{d}/expected.json"))
bad = 0
for f, want in exp.items():
    got = [r.text for r in zxingcpp.read_barcodes(Image.open(f"{d}/{f}"))]
    ok = any(g == want or (g.startswith(want) and len(g) == len(want) + 1) for g in got)
    bad += not ok
    print(("PASS" if ok else "FAIL"), f, "want", repr(want), "got", got)
sys.exit(1 if bad else 0)
