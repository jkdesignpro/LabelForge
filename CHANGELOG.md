# Changelog

All notable changes to LabelForge. Versions follow [Semantic Versioning](https://semver.org).

## [0.1.0] - 2026-11-27

First version.

### Added
- Desktop app (Tauri 2) for macOS (universal), Windows and Linux.
- Label setup: size, corner radius, printer dpi.
- Objects: text, rectangle, ellipse, line, image; barcodes Code 128, Code 39,
  Code 93, EAN-13, EAN-8, Interleaved 2 of 5, QR Code, Data Matrix.
- Designer: select, drag, resize, nudge, snap, zoom/fit, undo/redo, duplicate,
  delete, front/back, lock, rotation.
- Variables: fixed, asked when printing, serial/pattern, date/time.
- Serial pattern engine with prefix/suffix text, number/base-36/letter counters,
  date/time tokens and GS1 mod 10, Luhn and mod 43 check digits.
- Barcode modules snapped to whole printer dots for reliable scanning.
- Print dialog with prompts, quantity, start position and first/last preview;
  printing through the system print dialog.
- `.lfl` label file format (JSON, schema 1), SVG export.
- Example part tag, core unit tests, barcode decode check, CI and release workflows.

### Known limits
- Codabar is not offered yet: the encoder library we use has the B and C
  start/stop characters swapped. It returns once we have a verified encoder.
- Printing goes through the system driver only; direct Godex/Zebra printing is planned for 0.4.
