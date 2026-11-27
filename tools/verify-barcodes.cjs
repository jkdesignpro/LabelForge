// Renders every barcode type through the LabelForge core and writes PNGs, so
// tools/decode.py can check them with an independent decoder (zxing-cpp).
// Usage: cargo run -p labelforge-dev &   node tools/verify-barcodes.cjs out/   python3 tools/decode.py out/
const { chromium } = require('playwright');
const fs = require('node:fs');
const out = process.argv[2] || 'barcode-check';
fs.mkdirSync(out, { recursive: true });
const CASES = [
  ['code128', 'LF-2611-0042'], ['code128', '10098101'], ['code128', 'AB12345cd'], ['code39', 'PART-42'],
  ['code93', 'PART42'], ['ean13', '400638133393'], ['ean8', '9638507'], ['itf', '12345678'],
  ['qr', '10098101;LF-2611-0001'], ['datamatrix', 'LF-0001 303SS'],
];
(async () => {
const b = await chromium.launch();
const p = await b.newPage();
const expected = {};
for (const [i, [sym, data]] of CASES.entries()) {
  const is2d = sym === 'qr' || sym === 'datamatrix';
  const label = { schema: 1, name: 't', width_mm: 80, height_mm: is2d ? 40 : 30, corner_radius_mm: 0, dpi: 600, variables: [],
    objects: [{ id: 'b', x_mm: 8, y_mm: 4, w_mm: is2d ? 30 : 64, h_mm: is2d ? 30 : 20, rotation: 0, locked: false,
      kind: { type: 'barcode', symbology: sym, data, show_text: true, text_size_pt: 8, qr_ecc: 'M', color: '#000000' } }] };
  const r = await (await fetch('http://127.0.0.1:8765/api/render', { method: 'POST', body: JSON.stringify({ label, outline: false }) })).json();
  if (r.errors.length) { console.error(sym, r.errors); process.exitCode = 1; continue; }
  await p.setContent(`<body style="margin:0;background:#fff">${r.svg.replace('width="80mm"', 'width="1200"').replace(/height="\d+mm"/, '')}</body>`);
  const f = `${i}_${sym}.png`;
  await p.screenshot({ path: `${out}/${f}`, fullPage: true });
  expected[f] = data;
}
fs.writeFileSync(`${out}/expected.json`, JSON.stringify(expected, null, 1));
await b.close();
})();
