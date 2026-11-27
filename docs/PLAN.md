# LabelForge plan and roadmap

LabelForge is a free, open-source (AGPLv3) label designer and print manager for
machine shops and general use. It aims for feature parity with commercial label
packages such as NiceLabel 10, which is used as a **feature reference only**.
LabelForge has its own code, user interface, file format and artwork.

## Goals

1. Design any label: text, barcodes, 2D codes, shapes, images and symbols.
2. Fill labels from data: typed prompts, CSV/Excel, databases and Odoo.
3. Generate serials and codes from patterns (prefix, counter, suffix, dates, check digits).
4. Print to many printers, local (USB) and remote (network), on Mac, Windows and Linux.
5. Native desktop apps, no browser or localhost server needed.

## Platform and technology

| Part | Choice | Why |
|---|---|---|
| App shell | [Tauri 2](https://tauri.app) | Native app bundles for macOS (Apple Silicon and Intel), Windows and Linux from one code base. Small downloads (a few MB), uses the system web view. |
| Core engine | Rust (`labelforge-core` crate) | Label model, layout maths, barcode encoding, pattern engine, printer drivers. Testable without the UI. |
| Designer UI | TypeScript + HTML canvas/SVG | Drag-and-drop editing, rulers, snapping, zoom. |
| Label file | `.lfl` (JSON, versioned schema) | Human-readable, diffable, easy to back up. |

All measurements are stored in **millimetres** (0.01 mm resolution) and converted
to dots only when printing, so one label prints correctly at 203, 300 or 600 dpi.

## Printing

Two paths, so almost any printer works:

1. **System driver (raster)**: the label is rendered to a bitmap at the printer's
   dpi and sent through the operating system print system (CUPS on macOS/Linux,
   the Windows spooler). Works with every printer that has a driver, including
   colour printers.
2. **Direct printer language**: LabelForge writes the printer's own command
   language and sends it raw, over USB (via the OS queue in raw mode) or the
   network (TCP port 9100). Faster, sharper text and barcodes, and lets the
   printer itself count serials. Planned languages:
   - ZPL II (Zebra and many emulations)
   - EZPL (Godex)
   - TSPL (TSC and many low-cost printers)
   - DPL (Datamax/Honeywell), later
   - SBPL (SATO), later

Jacob's printers right now:

| Printer | Plan |
|---|---|
| Godex, 600 dpi | Direct EZPL over USB and network, plus driver path. Exact model to confirm. |
| Rebo Systems SMS-R1 (colour print and cut) | Driver path for colour printing first. Contour cutting needs research into how the SMS-R1 receives cut paths (likely its own software or a cut-line spot colour in PDF). |

## Feature list (parity target)

Status: ✅ in v0.1, 🔜 planned, 💭 later.

### Designer
- ✅ Label size, corner radius, dpi. 🔜 Page or roll, gaps, columns, margins, orientation.
- ✅ Objects: text, barcode, 2D code, line, rectangle, ellipse, image. 🔜 Paragraph (auto-fit), symbol.
- ✅ Select, move, resize, rotate 0/90/180/270, lock, front/back. 🔜 Free rotation, align, distribute, group, layers.
- ✅ Snapping, zoom/fit, undo/redo, duplicate. 🔜 Rulers, visible grid, copy/paste, multi-select.
- 🔜 Fonts: system fonts, printer-resident fonts (direct mode), bold/italic, alignment, inverse.
- 💭 Rich text, curved text, templates gallery.

### Symbol libraries
- 🔜 Built-in library drawn for LabelForge: hazard and safety (ISO 7010 style), PPE, recycling,
  packaging (fragile, this way up, keep dry), electrical, machine-shop symbols (material, heat treat,
  thread, tolerance, surface finish). All own artwork in SVG.
- 🔜 User libraries: import SVG/PNG folders.

### Barcodes and 2D codes
- ✅ 1D: Code 128 (auto B/C), Code 39, Code 93, EAN-13/8, Interleaved 2 of 5. 🔜 GS1-128, UPC-A/E, ITF-14, Codabar (own verified encoder).
- ✅ 2D: QR, Data Matrix. 🔜 GS1 Data Matrix, PDF417, Aztec.
- ✅ Human-readable text, EAN check digits, modules snapped to whole printer dots. 🔜 Explicit module width, quiet zone check.

### Variable data
- ✅ Variables: prompts, fixed, counters, date/time. 🔜 Computed (formulas).
- ✅ Serial patterns: e.g. `LF-{YYYY}{MM}-{SEQ:0000}-A` with prefix, suffix, step, start, reset
  rules (daily/monthly/yearly), alphanumeric counters, check digits (mod 10, mod 43, Luhn).
- 🔜 Counter memory so numbering continues across print runs; optional printer-side counting.
- 🔜 Data sources: CSV, Excel (.xlsx), ODS, plain text; then SQLite, PostgreSQL, MySQL, SQL Server, ODBC.
- 🔜 Record selection and filtering, quantity per record, copies.

### Odoo integration
- 🔜 Connect with URL, database, user and API key (Odoo external JSON-RPC API).
- 🔜 Pull products, lots/serial numbers, stock locations, manufacturing orders, deliveries.
- 🔜 Map Odoo fields to label variables; print from a search or selection.
- 💭 Write back printed serials/lots to Odoo.

### Printing
- 🔜 Printer setup: driver or direct, language, dpi, darkness, speed, media type, offsets.
- ✅ Print preview (first/last) with real data, quantity, start position, via system driver. 🔜 Copies per label, batch from data.
- 🔜 Print history / audit log.
- 💭 Print queue and remote print server for the shop floor.

## Releases

| Version | Content |
|---|---|
| 0.1 ✅ | App on 3 platforms, label setup, text/shapes/image, 8 barcode types, variables and serial patterns, save/open `.lfl`, print through system driver, SVG export. |
| 0.2 | Remaining barcodes and 2D codes, symbol library v1, rulers, align/distribute, multi-select, PNG/PDF export. |
| 0.3 | CSV/Excel import, print per data row, counter memory between runs, formulas. |
| 0.4 | Direct printing: EZPL (Godex) and ZPL, network and USB. |
| 0.5 | Odoo integration. |
| 0.6 | Databases, print history, TSPL, Rebo SMS-R1 cut path research outcome. |
| 1.0 | NiceLabel 10 feature parity review, documentation, signed installers. |

Every change goes in [CHANGELOG.md](../CHANGELOG.md) and ships with a version number.
