//! Turn rendered label SVG into PNG, 1-bit printer bitmaps and PDF.
//!
//! Uses resvg/usvg (pure Rust) with the system's fonts, so exports and
//! direct printing look the same on every platform.

use resvg::tiny_skia;
use resvg::usvg;
use std::sync::{Arc, OnceLock};

fn options() -> usvg::Options<'static> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let db = DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        // Make the generic families point at a font that exists here.
        for fam in ["Helvetica", "Arial", "Liberation Sans", "DejaVu Sans", "Segoe UI"] {
            if db.faces().any(|f| f.families.iter().any(|(n, _)| n == fam)) {
                db.set_sans_serif_family(fam);
                break;
            }
        }
        Arc::new(db)
    });
    let mut o = usvg::Options::default();
    o.fontdb = db.clone();
    o.font_family = db.family_name(&usvg::fontdb::Family::SansSerif).to_string();
    o
}

fn tree(svg: &str) -> Result<usvg::Tree, String> {
    usvg::Tree::from_str(svg, &options()).map_err(|e| format!("Cannot read label SVG: {e}"))
}

/// Render to an RGBA pixmap at `dpi` with a white background.
pub fn pixmap(svg: &str, dpi: f64) -> Result<tiny_skia::Pixmap, String> {
    let t = tree(svg)?;
    // usvg sizes are in CSS px (96/in); the SVG declares its size in mm.
    let scale = (dpi / 96.0) as f32;
    let size = t.size();
    let w = (size.width() * scale).round().max(1.0) as u32;
    let h = (size.height() * scale).round().max(1.0) as u32;
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or("Label too large to render")?;
    pm.fill(tiny_skia::Color::WHITE);
    resvg::render(&t, tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Ok(pm)
}

pub fn png(svg: &str, dpi: f64) -> Result<Vec<u8>, String> {
    pixmap(svg, dpi)?.encode_png().map_err(|e| e.to_string())
}

/// A 1-bit image for label printers. Rows are packed MSB first, padded to
/// whole bytes; a set bit means a black dot.
#[derive(Debug, Clone, PartialEq)]
pub struct Mono {
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
    pub data: Vec<u8>,
}

impl Mono {
    pub fn get(&self, x: u32, y: u32) -> bool {
        self.data[(y * self.bytes_per_row + x / 8) as usize] & (0x80 >> (x % 8)) != 0
    }
    /// True if no dot is black.
    pub fn is_blank(&self) -> bool {
        self.data.iter().all(|b| *b == 0)
    }
}

/// Render at the printer's dpi and threshold to black/white (luminance below
/// `threshold`, 0-255, becomes black). Label printers cannot print grey, so
/// a hard threshold keeps barcode edges sharp.
pub fn mono(svg: &str, dpi: f64, threshold: u8) -> Result<Mono, String> {
    let pm = pixmap(svg, dpi)?;
    Ok(threshold_pixmap(&pm, threshold))
}

pub fn threshold_pixmap(pm: &tiny_skia::Pixmap, threshold: u8) -> Mono {
    let (w, h) = (pm.width(), pm.height());
    let bpr = w.div_ceil(8);
    let mut data = vec![0u8; (bpr * h) as usize];
    for (i, px) in pm.pixels().iter().enumerate() {
        let c = px.demultiply();
        // Transparent = white (paper).
        let a = c.alpha() as u32;
        let lum = (299 * c.red() as u32 + 587 * c.green() as u32 + 114 * c.blue() as u32) / 1000;
        let lum = (lum * a + 255 * (255 - a)) / 255;
        if lum < threshold as u32 {
            let (x, y) = (i as u32 % w, i as u32 / w);
            data[(y * bpr + x / 8) as usize] |= 0x80 >> (x % 8);
        }
    }
    Mono { width: w, height: h, bytes_per_row: bpr, data }
}

/// One PDF with one page per SVG, each page sized to its SVG (vector, fonts embedded).
pub fn pdf(pages: &[String]) -> Result<Vec<u8>, String> {
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
    use std::collections::HashMap;
    if pages.is_empty() {
        return Err("Nothing to export".into());
    }
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let page_tree = alloc.bump();
    let mut pdf = Pdf::new();
    let mut kids = Vec::new();
    let mut chunks = Vec::new();
    let mut page_info = Vec::new();
    for svg in pages {
        let t = tree(svg)?;
        let (chunk, id) = svg2pdf::to_chunk(&t, svg2pdf::ConversionOptions::default()).map_err(|e| format!("PDF: {e:?}"))?;
        let mut map = HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_id = *map.get(&id).ok_or("PDF: missing object")?;
        let page_id = alloc.bump();
        let content_id = alloc.bump();
        // CSS px -> PDF points.
        let (w, h) = (t.size().width() * 0.75, t.size().height() * 0.75);
        kids.push(page_id);
        page_info.push((page_id, content_id, svg_id, w, h));
        chunks.push(chunk);
    }
    pdf.catalog(catalog).pages(page_tree);
    pdf.pages(page_tree).kids(kids.iter().copied()).count(kids.len() as i32);
    for (page_id, content_id, svg_id, w, h) in &page_info {
        let mut page = pdf.page(*page_id);
        page.media_box(Rect::new(0.0, 0.0, *w, *h));
        page.parent(page_tree);
        page.contents(*content_id);
        page.resources().x_objects().pair(Name(b"S1"), *svg_id);
        page.finish();
        let mut c = Content::new();
        c.transform([*w, 0.0, 0.0, *h, 0.0, 0.0]).x_object(Name(b"S1"));
        pdf.stream(*content_id, &c.finish());
    }
    for c in &chunks {
        pdf.extend(c);
    }
    Ok(pdf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="5mm" viewBox="0 0 10 5"><rect x="0" y="0" width="5" height="5" fill="#000"/></svg>"##;

    #[test]
    fn mono_at_203_dpi() {
        let m = mono(SVG, 203.0, 128).unwrap();
        // 10 mm at 203 dpi = 79.9 dots.
        assert_eq!(m.width, 80);
        assert_eq!(m.height, 40);
        assert!(m.get(0, 0) && m.get(38, 39));
        assert!(!m.get(41, 0) && !m.get(79, 39));
    }

    #[test]
    fn png_and_pdf() {
        let p = png(SVG, 300.0).unwrap();
        assert_eq!(&p[1..4], b"PNG");
        let d = pdf(&[SVG.to_string(), SVG.to_string()]).unwrap();
        let s = String::from_utf8_lossy(&d);
        assert!(s.starts_with("%PDF"));
        assert!(s.contains("/Count 2"));
    }
}
