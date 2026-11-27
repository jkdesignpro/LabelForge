// Renders every barcode type through the LabelForge core (each type's example
// data) and writes PNGs, so tools/decode.py can check them with an independent
// decoder (zxing-cpp).
// Usage: cargo run -p labelforge-dev &   node tools/verify-barcodes.cjs out/   python3 tools/decode.py out/
const { chromium } = require('playwright');
const fs = require('node:fs');
const out = process.argv[2] || 'barcode-check';
const API = 'http://127.0.0.1:8765/api/';
fs.mkdirSync(out, { recursive: true });
const post = async (cmd, body) => (await fetch(API + cmd, { method: 'POST', body: JSON.stringify(body || {}) })).json();
(async () => {
  const syms = await post('symbologies');
  const b = await chromium.launch();
  const p = await b.newPage();
  const expected = {};
  for (const s of syms) {
    const square = ['two_d', 'hibc'].includes(s.group) && !/39|128/.test(s.key);
    const label = { schema: 1, name: 't', width_mm: 90, height_mm: square ? 60 : 40, corner_radius_mm: 0, dpi: 600, variables: [],
      objects: [{ id: 'b', x_mm: 10, y_mm: 8, w_mm: square ? 44 : 70, h_mm: square ? 44 : 24, rotation: 0, locked: false,
        kind: { type: 'barcode', symbology: s.key, data: s.example, show_text: true, qr_ecc: '', color: '#000000' } }] };
    const r = await post('render', { label, outline: false });
    if (r.errors.length) { console.error('RENDER ERROR', s.key, r.errors); process.exitCode = 1; continue; }
    await p.setContent(`<body style="margin:0;background:#fff">${r.svg.replace('width="90mm"', 'width="1350"').replace(/height="\d+mm"/, '')}</body>`);
    const f = `${s.key}.png`;
    await p.screenshot({ path: `${out}/${f}`, fullPage: true });
    expected[f] = s.example;
  }
  fs.writeFileSync(`${out}/expected.json`, JSON.stringify(expected, null, 1));
  await b.close();
})();
