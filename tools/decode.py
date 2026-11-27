"""Decode the PNGs written by verify-barcodes.cjs and compare with the input.

PASS    decoded and matches (check digits/start-stop added by the symbology allowed)
FAIL    decoded to something else, or not decoded although zxing-cpp supports the type
SKIP    zxing-cpp cannot read this symbology; covered by Zint's own test suite only

Usage: pip install zxing-cpp pillow; python3 tools/decode.py barcode-check/"""
import json, re, sys, zxingcpp
from PIL import Image

# Types zxing-cpp 2.x cannot decode.
UNSUPPORTED = {"code11", "msi", "logmars", "telepen", "pharma", "code39_ext", "dotcode", "hanxin", "codablock_f",
               "code16k", "micro_pdf417", "postnet", "planet", "usps_imail", "rm4scc", "mailmark", "kix", "auspost",
               "japanpost", "upu_s10", "dpd", "pdf417_compact", "databar_stacked", "databar_expanded_stacked",
               "hibc_pdf417"}

# What a scanner returns when it differs from the input (check digits, AIs, expansions).
DECODED = {
    "upca": "0036000291452", "upce": "0012345000065", "ean14": "(01)09501101530003",
    "itf14": "09501101530003", "sscc": "(00)376123450000010039", "databar": "(01)09501101530003",
    "databar_stacked": "(01)09501101530003", "databar_limited": "(01)09501101530003",
    # zxing-cpp reads Code 39 as Full ASCII, so the HIBC flag "+A" comes back as "a".
    "hibc_39": "a123BJC5D6E71G",
}
UNSUPPORTED.add("codablock_f")  # zxing-cpp reads its rows as separate Code 128 symbols

def norm(s):
    return re.sub(r"[()\x1d\s]", "", s)

d = sys.argv[1] if len(sys.argv) > 1 else "barcode-check"
exp = json.load(open(f"{d}/expected.json"))
bad = 0
for f, want in exp.items():
    key = f[:-4]
    got = [r.text for r in zxingcpp.read_barcodes(Image.open(f"{d}/{f}"))]
    want = DECODED.get(key, want)
    w = norm(want)
    ok = any(norm(g) == w or norm(g).startswith(w) and len(norm(g)) <= len(w) + 2
             or w.startswith(norm(g)) and len(w) - len(norm(g)) <= 1
             or (key.startswith("hibc") and w.upper() in norm(g).upper())
             or (key == "codabar" and norm(g).strip("ABCD") == w.strip("ABCD"))
             or (key == "upce" and len(norm(g)) == 8)
             for g in got)
    if ok:
        status = "PASS"
    elif key in UNSUPPORTED:
        status = "SKIP"
    else:
        status = "FAIL"
        bad += 1
    print(f"{status:4} {key:26} want {want!r:45} got {got}")
sys.exit(1 if bad else 0)
