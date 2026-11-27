// LabelForge designer. Free software under the GNU AGPL v3 or later.
// Talks to the Rust core through `call()`: Tauri commands in the desktop app,
// or the dev server's /api in a browser.

const TAURI = window.__TAURI__;
const call = async (cmd, args = {}) => {
  if (TAURI) return TAURI.core.invoke(cmd, args);
  const body = cmd === 'render' || cmd === 'render_run' ? args.req : args;
  const r = await fetch('/api/' + cmd, { method: 'POST', body: JSON.stringify(body) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
};

const $ = (s) => document.querySelector(s);
const MM_PX = 96 / 25.4; // CSS pixels per millimetre at 100%

// Filled from the core at start-up: [{key, name, group, example}].
let SYMBOLOGIES = [];
const SYM_GROUPS = { linear: 'Linear', retail: 'Retail (EAN/UPC)', gs1: 'GS1', two_d: '2D', stacked: 'Stacked', hibc: 'HIBC (health)', postal: 'Postal', other: 'Other' };
const symName = (key) => (SYMBOLOGIES.find((s) => s.key === key) || { name: key }).name;
const symInfo = (key) => SYMBOLOGIES.find((s) => s.key === key) || { key, name: key, group: 'other', example: '' };
// Common GS1 Application Identifiers for the data helper.
const GS1_AIS = [
  ['00', 'SSCC (18 digits)'], ['01', 'GTIN (14 digits)'], ['02', 'GTIN of contents'], ['10', 'Batch / lot'],
  ['11', 'Production date YYMMDD'], ['13', 'Packaging date YYMMDD'], ['15', 'Best before YYMMDD'],
  ['17', 'Expiry date YYMMDD'], ['20', 'Variant'], ['21', 'Serial number'], ['22', 'Consumer product variant'],
  ['30', 'Count of items'], ['37', 'Count of trade items'], ['240', 'Additional product ID'],
  ['241', 'Customer part number'], ['250', 'Secondary serial'], ['3100', 'Net weight kg (0 dp)'],
  ['3102', 'Net weight kg (2 dp)'], ['3300', 'Gross weight kg'], ['400', 'Customer order number'],
  ['410', 'Ship to GLN'], ['420', 'Ship to postcode'], ['7003', 'Expiry date and time'], ['8004', 'GIAI asset ID'],
];


const blankLabel = () => ({
  schema: 1, name: 'Untitled', width_mm: 100, height_mm: 50, corner_radius_mm: 2, dpi: 600, objects: [], variables: [],
});

const state = {
  label: blankLabel(),
  path: null,
  sel: null, // selected object id
  dirty: false,
  undo: [], redo: [],
  zoom: 4,
  fit: true,
  errors: [],
};

// ---------- history ----------
const snapshot = () => JSON.stringify(state.label);
function commit(before) {
  if (before !== snapshot()) {
    state.undo.push(before);
    if (state.undo.length > 200) state.undo.shift();
    state.redo = [];
    state.dirty = true;
  }
}
function mutate(fn) {
  const before = snapshot();
  fn(state.label);
  commit(before);
  refresh();
}
function undo() {
  if (!state.undo.length) return;
  state.redo.push(snapshot());
  state.label = JSON.parse(state.undo.pop());
  if (!obj(state.sel)) state.sel = null;
  refresh();
}
function redo() {
  if (!state.redo.length) return;
  state.undo.push(snapshot());
  state.label = JSON.parse(state.redo.pop());
  refresh();
}

// ---------- helpers ----------
const obj = (id) => state.label.objects.find((o) => o.id === id);
const snap = (v) => ($('#snap').checked ? Math.round(v * 2) / 2 : Math.round(v * 100) / 100);
function newId(prefix) {
  let i = 1;
  while (obj(prefix + i)) i++;
  return prefix + i;
}
function status(msg, isErr = false) {
  const s = $('#status');
  s.textContent = msg;
  s.className = 'status' + (isErr ? ' err' : '');
}
function esc(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
}
function describe(o) {
  const k = o.kind;
  switch (k.type) {
    case 'text': return 'Text: ' + k.text.split('\n')[0];
    case 'barcode': return symName(k.symbology) + ': ' + k.data;
    case 'image': return 'Image';
    default: return k.type[0].toUpperCase() + k.type.slice(1);
  }
}

// ---------- rendering ----------
let renderSeq = 0;
async function renderCanvas() {
  const seq = ++renderSeq;
  try {
    const res = await call('render', { req: { label: state.label, inputs: {}, index: 0, outline: true } });
    if (seq !== renderSeq) return; // a newer render is on its way
    const host = $('#svghost');
    host.innerHTML = res.svg;
    const svg = host.querySelector('svg');
    const w = state.label.width_mm * MM_PX * state.zoom;
    const h = state.label.height_mm * MM_PX * state.zoom;
    svg.setAttribute('width', w);
    svg.setAttribute('height', h);
    $('#paper').style.width = w + 'px';
    $('#paper').style.height = h + 'px';
    state.errors = res.errors;
    if (res.errors.length) status(res.errors.join('  |  '), true);
    else status(`${state.label.name}  ·  ${state.label.width_mm} × ${state.label.height_mm} mm  ·  ${state.label.objects.length} objects${state.dirty ? '  ·  unsaved' : ''}${state.path ? '  ·  ' + state.path : ''}`);
  } catch (e) {
    status('Render failed: ' + e, true);
  }
}

function renderOverlay() {
  const ov = $('#overlay');
  const z = MM_PX * state.zoom;
  ov.innerHTML = '';
  for (const o of state.label.objects) {
    const d = document.createElement('div');
    d.className = 'hit' + (o.id === state.sel ? ' on' : '') + (o.locked ? ' locked' : '');
    d.dataset.id = o.id;
    // Lines can have zero width or height; keep a clickable band.
    const w = Math.max(o.w_mm * z, 6), h = Math.max(o.h_mm * z, 6);
    d.style.left = o.x_mm * z - (w - o.w_mm * z) / 2 + 'px';
    d.style.top = o.y_mm * z - (h - o.h_mm * z) / 2 + 'px';
    d.style.width = w + 'px';
    d.style.height = h + 'px';
    if (o.rotation) d.style.transform = `rotate(${o.rotation}deg)`;
    if (o.id === state.sel && !o.locked) {
      const g = document.createElement('div');
      g.className = 'grip';
      d.appendChild(g);
    }
    ov.appendChild(d);
  }
}

function renderObjList() {
  const ol = $('#objlist');
  ol.innerHTML = '';
  // Top-most object first.
  [...state.label.objects].reverse().forEach((o) => {
    const li = document.createElement('li');
    li.textContent = (o.locked ? '🔒 ' : '') + describe(o);
    li.title = o.id;
    if (o.id === state.sel) li.className = 'on';
    li.onclick = () => select(o.id);
    ol.appendChild(li);
  });
}

function renderLabelForm() {
  for (const el of document.querySelectorAll('[data-l]')) {
    if (document.activeElement !== el) el.value = state.label[el.dataset.l];
  }
}

function field(label, key, value, type = 'text', extra = '') {
  if (type === 'checkbox') return `<label class="check">${label} <input type="checkbox" data-k="${key}" ${value ? 'checked' : ''}></label>`;
  if (type === 'textarea') return `<label>${label} <textarea data-k="${key}" ${extra}>${esc(value)}</textarea></label>`;
  if (type === 'color') return `<label>${label} <input type="color" data-k="${key}" value="${esc(value || '#000000')}"></label>`;
  return `<label>${label} <input type="${type}" data-k="${key}" value="${esc(value)}" ${extra}></label>`;
}
function select_(label, key, value, opts) {
  return `<label>${label} <select data-k="${key}">${opts.map(([v, t]) => `<option value="${v}" ${v === value ? 'selected' : ''}>${t}</option>`).join('')}</select></label>`;
}

function renderProps() {
  const p = $('#props');
  const o = obj(state.sel);
  if (!o) {
    p.innerHTML = '<p class="hint">Select an object, or add one from the left.</p>';
    return;
  }
  if (p.contains(document.activeElement) && p.dataset.id === o.id) {
    // Do not rebuild while the user is typing; just update geometry fields.
    for (const k of ['x_mm', 'y_mm', 'w_mm', 'h_mm']) {
      const el = p.querySelector(`[data-k="${k}"]`);
      if (el && el !== document.activeElement) el.value = o[k];
    }
    return;
  }
  p.dataset.id = o.id;
  const k = o.kind;
  const num = 'step="0.1"';
  let h = '';
  h += field('X mm', 'x_mm', o.x_mm, 'number', num) + field('Y mm', 'y_mm', o.y_mm, 'number', num);
  h += field('Width mm', 'w_mm', o.w_mm, 'number', num) + field('Height mm', 'h_mm', o.h_mm, 'number', num);
  h += select_('Rotation', 'rotation', String(o.rotation || 0), [['0', '0°'], ['90', '90°'], ['180', '180°'], ['270', '270°']]);
  h += field('Locked', 'locked', o.locked, 'checkbox');
  if (k.type === 'text') {
    h += field('Text', 'kind.text', k.text, 'textarea');
    h += field('Font', 'kind.font', k.font, 'text', 'list="fonts"');
    h += field('Size pt', 'kind.size_pt', k.size_pt, 'number', 'step="0.5" min="2"');
    h += field('Bold', 'kind.bold', k.bold, 'checkbox') + field('Italic', 'kind.italic', k.italic, 'checkbox');
    h += select_('Align', 'kind.align', k.align || 'left', [['left', 'Left'], ['center', 'Centre'], ['right', 'Right']]);
    h += field('Colour', 'kind.color', k.color, 'color');
    h += field('Inverse', 'kind.inverse', k.inverse, 'checkbox');
    h += '<datalist id="fonts"><option>Helvetica</option><option>Arial</option><option>Arial Narrow</option><option>DIN Alternate</option><option>Courier New</option><option>Times New Roman</option><option>Verdana</option></datalist>';
  } else if (k.type === 'barcode') {
    const info = symInfo(k.symbology);
    h += `<label>Type <select data-k="kind.symbology">${Object.entries(SYM_GROUPS).map(([g, gname]) => {
      const list = SYMBOLOGIES.filter((x) => x.group === g);
      return list.length ? `<optgroup label="${gname}">${list.map((x) => `<option value="${x.key}" ${x.key === k.symbology ? 'selected' : ''}>${esc(x.name)}</option>`).join('')}</optgroup>` : '';
    }).join('')}</select></label>`;
    h += field('Data', 'kind.data', k.data, 'textarea', 'rows="2"');
    h += `<p class="hint">Example: <code>${esc(info.example)}</code> <button class="small" id="useexample">Use</button></p>`;
    if (info.key.startsWith('gs1_') || info.key.startsWith('databar_expanded')) {
      h += `<label>Add AI <select id="gs1ai"><option value="">Pick an Application Identifier…</option>${GS1_AIS.map(([a, t]) => `<option value="${a}">(${a}) ${t}</option>`).join('')}</select></label>`;
    }
    const ecc = {
      qr: [['', 'Auto'], ['L', 'L (7%)'], ['M', 'M (15%)'], ['Q', 'Q (25%)'], ['H', 'H (30%)']],
      aztec: [['', 'Auto'], ['1', '1 (10%)'], ['2', '2 (23%)'], ['3', '3 (36%)'], ['4', '4 (50%)']],
      pdf: [['', 'Auto'], ...[0, 1, 2, 3, 4, 5, 6, 7, 8].map((v) => [String(v), 'Level ' + v])],
    };
    const eccKind = /qr|rmqr/.test(k.symbology) ? 'qr' : /aztec/.test(k.symbology) ? 'aztec' : /^(pdf417|pdf417_compact|hibc_pdf417)$/.test(k.symbology) ? 'pdf' : null;
    if (eccKind) h += select_('Error corr.', 'kind.qr_ecc', k.qr_ecc || '', ecc[eccKind]);
    if (/pdf417/.test(k.symbology)) h += field('Columns', 'kind.columns', k.columns || 0, 'number', 'min="0" max="30" step="1"');
    if (k.symbology === 'maxicode') {
      h += select_('Mode', 'kind.maxi_mode', String(k.maxi_mode || 0), [['0', 'Auto (4)'], ['2', '2 (US postcode)'], ['3', '3 (intl postcode)'], ['4', '4 (standard)'], ['5', '5 (full ECC)'], ['6', '6 (reader prog.)']]);
      if (k.maxi_mode === 2 || k.maxi_mode === 3) h += field('Primary', 'kind.maxi_primary', k.maxi_primary || '');
    }
    if (!['two_d'].includes(info.group) && !/datamatrix|_qr|aztec|maxicode/.test(k.symbology)) h += field('Show text', 'kind.show_text', k.show_text, 'checkbox');
    h += field('Quiet zone', 'kind.quiet_zone', !!k.quiet_zone, 'checkbox');
    h += field('Colour', 'kind.color', k.color, 'color');
  } else if (k.type === 'rect' || k.type === 'ellipse' || k.type === 'line') {
    h += field('Line mm', 'kind.stroke_mm', k.stroke_mm, 'number', 'step="0.05" min="0"');
    h += field('Line colour', 'kind.stroke', k.stroke, 'color');
    if (k.type !== 'line') {
      h += field('Filled', 'fillon', !!k.fill, 'checkbox');
      if (k.fill) h += field('Fill colour', 'kind.fill', k.fill, 'color');
    }
    if (k.type === 'rect') h += field('Corner mm', 'kind.radius_mm', k.radius_mm, 'number', 'step="0.1" min="0"');
  } else if (k.type === 'image') {
    h += '<button id="replaceimg">Replace image…</button>';
  }
  p.innerHTML = h;
  const ue = $('#useexample');
  if (ue) ue.onclick = () => { mutate(() => { o.kind.data = symInfo(o.kind.symbology).example; }); p.dataset.id = ''; renderProps(); };
  const ai = $('#gs1ai');
  if (ai) ai.onchange = () => { if (ai.value) { mutate(() => { o.kind.data += `(${ai.value})`; }); p.dataset.id = ''; renderProps(); } };
  const ri = $('#replaceimg');
  if (ri) ri.onclick = () => { imageTarget = o.id; $('#imagefile').click(); };
}

function onPropInput(e) {
  const el = e.target;
  const key = el.dataset.k;
  if (!key) return;
  const o = obj(state.sel);
  if (!o) return;
  let v = el.type === 'checkbox' ? el.checked : el.value;
  if (el.type === 'number' || key === 'rotation' || key === 'kind.maxi_mode') {
    v = parseFloat(v);
    if (!isFinite(v)) return;
  }
  mutate(() => {
    if (key === 'fillon') o.kind.fill = v ? '#000000' : '';
    else if (key.startsWith('kind.')) o.kind[key.slice(5)] = v;
    else o[key] = v;
  });
  // Structure-changing fields rebuild the panel.
  if (['kind.symbology', 'kind.maxi_mode', 'fillon', 'locked'].includes(key)) { $('#props').dataset.id = ''; document.activeElement.blur(); renderProps(); }
}

// ---------- variables ----------
function renderVars() {
  const box = $('#vars');
  if (box.contains(document.activeElement)) return;
  box.innerHTML = '';
  state.label.variables.forEach((v, i) => {
    const d = document.createElement('div');
    d.className = 'var form';
    let h = `<div class="row"><input data-v="name" value="${esc(v.name)}" title="Variable name"><select data-v="source">
      ${[['fixed', 'Fixed'], ['prompt', 'Ask when printing'], ['serial', 'Serial / pattern'], ['date', 'Date / time']]
        .map(([s, t]) => `<option value="${s}" ${s === v.source ? 'selected' : ''}>${t}</option>`).join('')}
      </select><button class="small" data-vdel>✕</button></div>`;
    if (v.source === 'fixed') h += `<label>Value <input data-v="value" value="${esc(v.value ?? '')}"></label>`;
    if (v.source === 'prompt') h += `<label>Question <input data-v="prompt" value="${esc(v.prompt ?? '')}"></label><label>Default <input data-v="default" value="${esc(v.default ?? '')}"></label>`;
    if (v.source === 'serial') h += `<label>Pattern <input data-v="pattern" value="${esc(v.pattern ?? '')}"></label><label>Start <input type="number" data-v="start" value="${v.start ?? 1}"></label><label>Step <input type="number" data-v="step" value="${v.step ?? 1}"></label>`;
    if (v.source === 'date') h += `<label>Pattern <input data-v="pattern" value="${esc(v.pattern ?? '')}"></label>`;
    d.innerHTML = h;
    d.addEventListener('change', (e) => {
      const k = e.target.dataset.v;
      if (!k) return;
      mutate((l) => {
        const vv = l.variables[i];
        let val = e.target.value;
        if (k === 'start' || k === 'step') val = parseInt(val, 10) || 0;
        if (k === 'source') {
          l.variables[i] = { name: vv.name, source: val, ...defaultsFor(val) };
        } else vv[k] = val;
      });
      if (k === 'source') { document.activeElement.blur(); renderVars(); }
    });
    d.querySelector('[data-vdel]').onclick = () => { mutate((l) => l.variables.splice(i, 1)); renderVars(); };
    box.appendChild(d);
  });
}
function defaultsFor(src) {
  return {
    fixed: { value: '' },
    prompt: { prompt: '', default: '' },
    serial: { pattern: '{SEQ:0000}', start: 1, step: 1 },
    date: { pattern: '{DD}/{MM}/{YYYY}' },
  }[src];
}

// ---------- refresh ----------
function fitZoom() {
  if (!state.fit) return;
  const st = $('#stage');
  const z = Math.min((st.clientWidth - 80) / (state.label.width_mm * MM_PX), (st.clientHeight - 80) / (state.label.height_mm * MM_PX));
  state.zoom = Math.max(0.5, Math.min(12, z));
}
window.addEventListener('resize', () => { if (state.fit) refresh(); });

function refresh() {
  fitZoom();
  renderOverlay();
  renderObjList();
  renderProps();
  renderLabelForm();
  renderVars();
  renderCanvas();
}

function select(id) {
  state.sel = id;
  $('#props').dataset.id = '';
  refresh();
}

// ---------- adding objects ----------
const barcodeKind = (symbology, data) => ({ type: 'barcode', symbology, data, show_text: true, text_size_pt: 8, qr_ecc: '', columns: 0, maxi_mode: 0, maxi_primary: '', quiet_zone: false, color: '#000000' });
let imageTarget = null;
function add(type) {
  const L = state.label;
  const o = { id: '', x_mm: 5, y_mm: 5, w_mm: 30, h_mm: 10, rotation: 0, locked: false };
  const sh = { stroke_mm: 0.3, stroke: '#000000', fill: '', radius_mm: 0 };
  switch (type) {
    case 'text': o.id = newId('text'); o.kind = { type: 'text', text: 'Text', font: 'Helvetica', size_pt: 12, bold: false, italic: false, align: 'left', color: '#000000', inverse: false }; o.w_mm = 40; o.h_mm = 6; break;
    case 'barcode': o.id = newId('barcode'); o.kind = barcodeKind('code128', '12345678'); o.w_mm = 45; o.h_mm = 15; break;
    case 'qr': o.id = newId('qr'); o.kind = barcodeKind('qr', 'https://example.com'); o.w_mm = o.h_mm = 20; break;
    case 'datamatrix': o.id = newId('dm'); o.kind = barcodeKind('datamatrix', 'LF-0001'); o.w_mm = o.h_mm = 12; break;
    case 'rect': o.id = newId('rect'); o.kind = { type: 'rect', ...sh }; o.h_mm = 15; break;
    case 'ellipse': o.id = newId('ellipse'); o.kind = { type: 'ellipse', ...sh }; o.w_mm = o.h_mm = 15; break;
    case 'line': o.id = newId('line'); o.kind = { type: 'line', ...sh }; o.h_mm = 0; break;
    case 'image': imageTarget = null; $('#imagefile').click(); return;
  }
  o.x_mm = Math.max(0, Math.min(o.x_mm, L.width_mm - o.w_mm));
  mutate((l) => l.objects.push(o));
  select(o.id);
}

$('#imagefile').addEventListener('change', (e) => {
  const f = e.target.files[0];
  e.target.value = '';
  if (!f) return;
  const rd = new FileReader();
  rd.onload = () => {
    const uri = rd.result;
    const img = new Image();
    img.onload = () => {
      if (imageTarget && obj(imageTarget)) {
        mutate(() => { obj(imageTarget).kind.data_uri = uri; });
        return;
      }
      const w = 25, h = Math.round((25 * img.naturalHeight) / (img.naturalWidth || 1) * 10) / 10 || 25;
      const o = { id: newId('image'), x_mm: 5, y_mm: 5, w_mm: w, h_mm: h, rotation: 0, locked: false, kind: { type: 'image', data_uri: uri } };
      mutate((l) => l.objects.push(o));
      select(o.id);
    };
    img.src = uri;
  };
  rd.readAsDataURL(f);
});

// ---------- mouse: select, move, resize ----------
let drag = null;
$('#overlay').addEventListener('pointerdown', (e) => {
  const hit = e.target.closest('.hit');
  if (!hit) { select(null); return; }
  const o = obj(hit.dataset.id);
  if (state.sel !== o.id) select(o.id);
  if (o.locked) return;
  drag = {
    id: o.id, before: snapshot(), resize: e.target.classList.contains('grip'),
    sx: e.clientX, sy: e.clientY, ox: o.x_mm, oy: o.y_mm, ow: o.w_mm, oh: o.h_mm,
  };
  e.target.setPointerCapture(e.pointerId);
  e.preventDefault();
});
window.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const o = obj(drag.id);
  const z = MM_PX * state.zoom;
  const dx = (e.clientX - drag.sx) / z, dy = (e.clientY - drag.sy) / z;
  if (drag.resize) {
    o.w_mm = Math.max(o.kind.type === 'line' ? 0 : 1, snap(drag.ow + dx));
    o.h_mm = Math.max(o.kind.type === 'line' ? 0 : 1, snap(drag.oh + dy));
  } else {
    o.x_mm = snap(drag.ox + dx);
    o.y_mm = snap(drag.oy + dy);
  }
  renderOverlay();
  renderProps();
  renderCanvas();
});
window.addEventListener('pointerup', () => {
  if (!drag) return;
  commit(drag.before);
  drag = null;
  refresh();
});

// ---------- file actions ----------
async function confirmDiscard() {
  if (!state.dirty) return true;
  const msg = 'Discard unsaved changes?';
  if (TAURI?.dialog?.ask) return TAURI.dialog.ask(msg, { title: 'LabelForge', kind: 'warning' });
  return window.confirm(msg);
}
function load(label, path) {
  state.label = label;
  state.path = path;
  state.sel = null;
  state.undo = []; state.redo = [];
  state.dirty = false;
  refresh();
}
async function openFile() {
  if (!(await confirmDiscard())) return;
  if (TAURI) {
    try {
      const r = await call('open_label');
      if (r) load(r[1], r[0]);
    } catch (e) { status(String(e), true); }
  } else $('#openfile').click();
}
$('#openfile').addEventListener('change', async (e) => {
  const f = e.target.files[0];
  e.target.value = '';
  if (!f) return;
  try {
    const l = JSON.parse(await f.text());
    if (!l.schema || l.schema > 1) throw new Error('Not a LabelForge file, or made by a newer version');
    load(l, f.name);
  } catch (err) { status('Open failed: ' + err.message, true); }
});
async function saveFile(as) {
  if (TAURI) {
    try {
      const p = await call('save_label', { label: state.label, path: as ? null : state.path });
      if (p) { state.path = p; state.dirty = false; refresh(); status('Saved ' + p); }
    } catch (e) { status('Save failed: ' + e, true); }
  } else {
    download(state.label.name + '.lfl', JSON.stringify(state.label, null, 2), 'application/json');
    state.dirty = false;
  }
}
function download(name, text, type) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
async function exportSvg() {
  const res = await call('render', { req: { label: state.label, inputs: {}, index: 0, outline: false } });
  if (TAURI) {
    try { const p = await call('export_file', { name: state.label.name + '.svg', ext: 'svg', contents: res.svg }); if (p) status('Exported ' + p); }
    catch (e) { status(String(e), true); }
  } else download(state.label.name + '.svg', res.svg, 'image/svg+xml');
}

// ---------- printing ----------
function printInputs() {
  const inputs = {};
  for (const el of document.querySelectorAll('#promptfields input')) inputs[el.dataset.p] = el.value;
  return inputs;
}
async function updatePrintPreview() {
  const qty = Math.max(1, parseInt($('#pqty').value, 10) || 1);
  const start = Math.max(0, parseInt($('#pstart').value, 10) || 0);
  const inputs = printInputs();
  $('#perr').textContent = '';
  try {
    const first = await call('render', { req: { label: state.label, inputs, index: start, outline: true } });
    const last = await call('render', { req: { label: state.label, inputs, index: start + qty - 1, outline: true } });
    $('#pfirst').innerHTML = first.svg;
    $('#plast').innerHTML = last.svg;
    const errs = [...first.errors, ...last.errors];
    $('#perr').textContent = errs.join('; ');
    $('#pgo').disabled = errs.length > 0;
  } catch (e) { $('#perr').textContent = String(e); }
}
function openPrint() {
  const prompts = state.label.variables.filter((v) => v.source === 'prompt');
  $('#promptfields').innerHTML = prompts.map((v) => `<label>${esc(v.prompt || v.name)} <input data-p="${esc(v.name)}" value="${esc(v.default || '')}"></label>`).join('');
  $('#printdlg').showModal();
  updatePrintPreview();
}
$('#printdlg').addEventListener('input', updatePrintPreview);
$('#printdlg').addEventListener('close', async () => {
  if ($('#printdlg').returnValue !== 'default') return;
  const qty = Math.max(1, parseInt($('#pqty').value, 10) || 1);
  const start = Math.max(0, parseInt($('#pstart').value, 10) || 0);
  try {
    const pages = await call('render_run', { req: { label: state.label, inputs: printInputs(), index: start, quantity: qty } });
    const L = state.label;
    let style = document.getElementById('pagestyle');
    if (!style) { style = document.createElement('style'); style.id = 'pagestyle'; document.head.appendChild(style); }
    style.textContent = `@page { size: ${L.width_mm}mm ${L.height_mm}mm; margin: 0; } #printroot .page { width: ${L.width_mm}mm; height: ${L.height_mm}mm; }`;
    $('#printroot').innerHTML = pages.map((p) => `<div class="page">${p.svg}</div>`).join('');
    status(`Sending ${qty} label${qty > 1 ? 's' : ''} to the print dialog…`);
    if (TAURI) await call('print_page'); else window.print();
  } catch (e) { status('Print failed: ' + e, true); }
});

// ---------- wiring ----------
const actions = {
  new: async () => { if (await confirmDiscard()) load(blankLabel(), null); },
  open: openFile,
  save: () => saveFile(false),
  saveas: () => saveFile(true),
  example: async () => { if (await confirmDiscard()) load(JSON.parse(await call('example_label')), null); },
  undo, redo,
  dup: () => {
    const o = obj(state.sel);
    if (!o) return;
    const c = JSON.parse(JSON.stringify(o));
    c.id = newId(o.id.replace(/\d+$/, ''));
    c.x_mm += 2; c.y_mm += 2; c.locked = false;
    mutate((l) => l.objects.push(c));
    select(c.id);
  },
  del: () => {
    if (!state.sel || obj(state.sel)?.locked) return;
    mutate((l) => { l.objects = l.objects.filter((o) => o.id !== state.sel); });
    select(null);
  },
  front: () => reorder(1), back: () => reorder(-1),
  export: exportSvg,
  print: openPrint,
};
function reorder(dir) {
  const o = obj(state.sel);
  if (!o) return;
  mutate((l) => {
    l.objects = l.objects.filter((x) => x !== o);
    if (dir > 0) l.objects.push(o); else l.objects.unshift(o);
  });
}

document.querySelectorAll('[data-act]').forEach((b) => (b.onclick = () => actions[b.dataset.act]()));
document.querySelectorAll('[data-add]').forEach((b) => (b.onclick = () => add(b.dataset.add)));
$('#props').addEventListener('change', onPropInput);
$('#props').addEventListener('input', (e) => { if (e.target.type !== 'checkbox' && e.target.tagName !== 'SELECT') onPropInput(e); });
$('#labelform').addEventListener('change', (e) => {
  const k = e.target.dataset.l;
  let v = e.target.value;
  if (k !== 'name') { v = parseFloat(v); if (!isFinite(v) || v < 0) return; }
  mutate((l) => { l[k] = v; });
});
$('#zoom').addEventListener('change', (e) => { state.fit = e.target.value === 'fit'; if (!state.fit) state.zoom = parseFloat(e.target.value); refresh(); });
$('#addvar').onclick = () => {
  let i = 1;
  while (state.label.variables.some((v) => v.name === 'var' + i)) i++;
  mutate((l) => l.variables.push({ name: 'var' + i, source: 'serial', ...defaultsFor('serial') }));
};

window.addEventListener('keydown', (e) => {
  const typing = ['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement.tagName);
  const mod = e.ctrlKey || e.metaKey;
  if (mod && e.key.toLowerCase() === 's') { e.preventDefault(); saveFile(e.shiftKey); return; }
  if (mod && e.key.toLowerCase() === 'o') { e.preventDefault(); openFile(); return; }
  if (mod && e.key.toLowerCase() === 'p') { e.preventDefault(); openPrint(); return; }
  if (typing) return;
  if (mod && e.key.toLowerCase() === 'z') { e.preventDefault(); e.shiftKey ? redo() : undo(); return; }
  if (mod && e.key.toLowerCase() === 'y') { e.preventDefault(); redo(); return; }
  if (mod && e.key.toLowerCase() === 'd') { e.preventDefault(); actions.dup(); return; }
  if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); actions.del(); return; }
  const o = obj(state.sel);
  const step = e.shiftKey ? 5 : 0.5;
  const moves = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
  if (o && !o.locked && moves[e.key]) {
    e.preventDefault();
    mutate(() => { o.x_mm = Math.round((o.x_mm + moves[e.key][0]) * 100) / 100; o.y_mm = Math.round((o.y_mm + moves[e.key][1]) * 100) / 100; });
  }
});
window.addEventListener('beforeunload', (e) => { if (state.dirty) { e.preventDefault(); e.returnValue = ''; } });

call('symbologies').then((list) => { SYMBOLOGIES = list; refresh(); }).catch((e) => { status('Could not load barcode types: ' + e, true); refresh(); });
