# LabelForge

Free, open-source label designer and printing software for machine shops and
general use. Design labels with text, barcodes, QR and Data Matrix codes,
shapes and images, fill them from variables and serial patterns, and print to
any printer. Native desktop app for **macOS** (Apple Silicon and Intel),
**Windows** and **Linux**.

LabelForge aims for feature parity with commercial label software such as
NiceLabel 10. That software is a feature reference only: LabelForge has its own
code, design and file format. See [docs/PLAN.md](docs/PLAN.md) for the full
feature list and roadmap.

Licence: [GNU AGPL v3 or later](LICENSE).

## What works in 0.1

- Label setup: width, height, corner radius, printer dpi.
- Objects: text (multi-line, font, size, bold, italic, alignment, colour, inverse),
  rectangle, ellipse, line, embedded image.
- Barcodes: Code 128 (automatic set B/C), Code 39, Code 93, EAN-13, EAN-8,
  Interleaved 2 of 5, QR Code (L/M/Q/H), Data Matrix. Check digits are added
  automatically for EAN. Bars are snapped to whole printer dots.
- Designer: click to select, drag to move, corner grip to resize, arrow keys to nudge
  (Shift for 5 mm), 0.5 mm snap, zoom/fit, undo/redo, duplicate, delete, front/back,
  lock, rotation 0/90/180/270, object list.
- Variables, used as `{name}` in any text or barcode:
  fixed value, asked when printing, serial/pattern counter, date/time.
- Serial patterns with prefix and suffix, e.g. `LF-{YY}{MM}-{SEQ:0000}-A`
  → `LF-2611-0042-A`. Number, base-36 and letter counters, start and step,
  date/time tokens, GS1 mod 10, Luhn and Code 39 mod 43 check digits.
- Print: answer prompts, choose quantity and first serial position, preview the
  first and last label, then print through the system print dialog
  (any printer with a driver; also "Save as PDF").
- Save/open `.lfl` label files (readable JSON), export SVG.

Not yet: direct printer languages (Godex EZPL, ZPL), CSV/Excel/database import,
Odoo, symbol library. See the [roadmap](docs/PLAN.md#releases).

## Install

Download the installer for your system from
[Releases](https://github.com/jkdesignpro/labelforge/releases), or build it yourself.

## Build from source

Needs [Rust](https://rustup.rs) (1.90 or newer) and [Node.js](https://nodejs.org) 20+
(only to run the Tauri build tool).

```sh
git clone https://github.com/jkdesignpro/labelforge
cd labelforge
npm install
npm run build                 # app for this computer
npm run build:mac-universal   # macOS: one app for Apple Silicon + Intel
                              # (first: rustup target add aarch64-apple-darwin x86_64-apple-darwin)
```

The app is written to `target/release/bundle/` (`.app`/`.dmg` on macOS,
`.msi`/`.exe` on Windows, `.deb`/`.AppImage`/`.rpm` on Linux).

Linux needs the WebKitGTK development packages first, e.g. on Ubuntu:
`sudo apt install libwebkit2gtk-4.1-dev librsvg2-dev patchelf`.

Run in development mode with `npm run dev`.

## Usage

1. Start LabelForge and click **Example** to load a machine-shop part tag, or
   **New** for a blank label. Set the label size on the left.
2. Add objects from the **Add** list. Drag them on the label; use the corner grip to
   resize. Edit everything in **Properties** on the right.
3. Add **Variables** and use them in text or barcode data as `{name}`.
   A serial variable with pattern `{part}-{SEQ:0000}` and start 1 gives
   `10098101-0001`, `10098101-0002`, ... when `part` is `10098101`.
4. Click **Print…**, fill in the questions, set the quantity, check the first and
   last label previews, and print. In the system dialog pick your label printer
   and set the paper size to your label size.

### Example: part tag

[examples/part-tag.lfl](examples/part-tag.lfl) is a 100 × 50 mm tag with a part
number and material asked at print time, today's date, a Code 128 serial
`LF-YYMM-0001`, and a QR code holding `part;serial`.

### Keyboard

| Keys | Action |
|---|---|
| Ctrl/⌘ S, Ctrl/⌘ Shift S | Save, Save as |
| Ctrl/⌘ O | Open |
| Ctrl/⌘ P | Print |
| Ctrl/⌘ Z, Ctrl/⌘ Shift Z | Undo, redo |
| Ctrl/⌘ D | Duplicate |
| Delete | Delete selected |
| Arrows, Shift+arrows | Nudge 0.5 mm, 5 mm |

## Project layout

| Path | What |
|---|---|
| `crates/labelforge-core` | Rust engine: label model, barcodes, patterns, SVG rendering. No UI. |
| `src-tauri` | Desktop app (Tauri 2): file dialogs, printing, wiring to the core. |
| `ui` | Designer interface (plain HTML/CSS/JavaScript, no build step). |
| `crates/labelforge-dev` | Small local server to run the UI in a browser for testing. Not shipped. |
| `tools` | Barcode verification scripts. |
| `examples` | Example labels. |
| `docs` | Plan/roadmap and testing notes. |

## Testing

```sh
cargo test -p labelforge-core
```

Barcode output is also checked by decoding rendered images with an independent
decoder; see [docs/TESTING.md](docs/TESTING.md).

## Changes

Every change is logged in [CHANGELOG.md](CHANGELOG.md) and released under a new
version number.
