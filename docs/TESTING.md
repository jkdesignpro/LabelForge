# Testing

## Core unit tests

```sh
cargo test -p labelforge-core
```

Covers the serial pattern engine (counters, dates, check digits against
published examples), Code 128 character-set choice, EAN check digits, QR and
Data Matrix structure, SVG rendering of every object type, the `.lfl` round
trip, dot snapping and the example label.

## Barcode decode check

Every symbology is rendered through the real core and decoded with
[zxing-cpp](https://github.com/zxing-cpp/zxing-cpp), an independent decoder,
so a wrong encoder table is caught (this is how the Codabar library bug was found).

```sh
cargo run -p labelforge-dev &          # serves the UI and API on 127.0.0.1:8765
npm i -D playwright && npx playwright install chromium   # once
node tools/verify-barcodes.cjs barcode-check
pip install zxing-cpp pillow           # once
python3 tools/decode.py barcode-check  # prints PASS/FAIL per barcode
```

Result on 2026-11-27 (0.1.0): 10 of 10 PASS (Code 128 x3, Code 39, Code 93,
EAN-13, EAN-8, ITF, QR, Data Matrix).

## Designer in a browser

`cargo run -p labelforge-dev` then open <http://127.0.0.1:8765/>. The browser
version uses file download/upload instead of native dialogs, otherwise it is
the same interface as the desktop app.

## Printing on real printers

Before each release, print the example part tag on each printer we support and
scan the barcodes with a phone or scanner. Record the result here.

| Date | Version | Printer | Path | Result |
|---|---|---|---|---|
| 2026-11-27 | 0.1.0 | (none, Linux desktop app launch check under Xvfb) | n/a | App starts, designer renders through the Rust core |
| | | Godex 600 dpi | System driver | not yet tested |
| | | Rebo SMS-R1 | System driver | not yet tested |
