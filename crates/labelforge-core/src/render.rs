//! Render a label to SVG (millimetre user units).
//!
//! The SVG is used for the on-screen designer, print preview, printing
//! through the system driver, and PDF/PNG export.

use crate::barcode;
use crate::model::*;
use std::collections::HashMap;
use std::fmt::Write;

const PT_TO_MM: f64 = 25.4 / 72.0;

pub struct RenderOptions {
    /// Draw the label outline (designer/preview). Off when printing.
    pub outline: bool,
}

/// Replace `{name}` with variable values. `{{`/`}}` give literal braces.
/// Unknown names are left as `{name}` so the designer shows what is missing.
pub fn fill(template: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find(['{', '}']) {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..1]);
            rest = &tail[2..];
        } else if tail.starts_with('{') {
            match tail.find('}') {
                Some(end) => {
                    let key = &tail[1..end];
                    match values.get(key) {
                        Some(v) => out.push_str(v),
                        None => out.push_str(&tail[..=end]),
                    }
                    rest = &tail[end + 1..];
                }
                None => {
                    out.push_str(tail);
                    rest = "";
                }
            }
        } else {
            out.push('}');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn n(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// Render the whole label. Returns the SVG and a list of per-object errors
/// (e.g. data a barcode cannot encode), which are also drawn in red.
pub fn render_svg(label: &Label, values: &HashMap<String, String>, opt: &RenderOptions) -> (String, Vec<String>) {
    let (w, h) = (label.width_mm, label.height_mm);
    let mut s = String::new();
    let mut errors = Vec::new();
    let _ = write!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}">"#,
        w = n(w),
        h = n(h)
    );
    if opt.outline {
        let _ = write!(
            s,
            r##"<rect x="0" y="0" width="{}" height="{}" rx="{r}" ry="{r}" fill="#ffffff" stroke="#9aa0a6" stroke-width="0.2"/>"##,
            n(w),
            n(h),
            r = n(label.corner_radius_mm)
        );
    }
    for obj in &label.objects {
        let mut body = String::new();
        if let Err(e) = render_object(obj, values, label.dpi, &mut body) {
            errors.push(format!("{}: {}", obj.id, e));
            let _ = write!(
                body,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#ffecec" stroke="#d93025" stroke-width="0.3" stroke-dasharray="1 0.6"/><text x="{}" y="{}" font-family="Helvetica" font-size="2.2" fill="#d93025">{}</text>"##,
                n(obj.x_mm), n(obj.y_mm), n(obj.w_mm), n(obj.h_mm),
                n(obj.x_mm + 0.8), n(obj.y_mm + 2.8), esc(&e)
            );
        }
        if obj.rotation.rem_euclid(360.0) != 0.0 {
            let (cx, cy) = (obj.x_mm + obj.w_mm / 2.0, obj.y_mm + obj.h_mm / 2.0);
            let _ = write!(s, r#"<g data-id="{}" transform="rotate({} {} {})">{}</g>"#, esc(&obj.id), n(obj.rotation), n(cx), n(cy), body);
        } else {
            let _ = write!(s, r#"<g data-id="{}">{}</g>"#, esc(&obj.id), body);
        }
    }
    s.push_str("</svg>");
    (s, errors)
}

fn render_object(o: &Object, values: &HashMap<String, String>, dpi: u32, s: &mut String) -> Result<(), String> {
    let (x, y, w, h) = (o.x_mm, o.y_mm, o.w_mm, o.h_mm);
    match &o.kind {
        Kind::Text(t) => {
            let size = t.size_pt * PT_TO_MM;
            let text = fill(&t.text, values);
            if t.inverse {
                let _ = write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#, n(x), n(y), n(w), n(h), esc(&t.color));
            }
            let (anchor, tx) = match t.align {
                Align::Left => ("start", x),
                Align::Center => ("middle", x + w / 2.0),
                Align::Right => ("end", x + w),
            };
            let color = if t.inverse { "#ffffff".to_string() } else { t.color.clone() };
            let _ = write!(
                s,
                r#"<text font-family="{}, Helvetica, Arial, sans-serif" font-size="{}" font-weight="{}" font-style="{}" fill="{}" text-anchor="{}" xml:space="preserve">"#,
                esc(&t.font), n(size),
                if t.bold { "bold" } else { "normal" },
                if t.italic { "italic" } else { "normal" },
                esc(&color), anchor
            );
            // One <tspan> per line; first baseline at roughly the cap height.
            for (i, line) in text.split('\n').enumerate() {
                let by = y + size * 0.8 + i as f64 * size * 1.2;
                let _ = write!(s, r#"<tspan x="{}" y="{}">{}</tspan>"#, n(tx), n(by), esc(line));
            }
            s.push_str("</text>");
        }
        Kind::Rect(sh) => {
            let _ = write!(
                s,
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{r}" ry="{r}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                n(x), n(y), n(w), n(h),
                fill_attr(&sh.fill), esc(&sh.stroke), n(sh.stroke_mm),
                r = n(sh.radius_mm)
            );
        }
        Kind::Ellipse(sh) => {
            let _ = write!(
                s,
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                n(x + w / 2.0), n(y + h / 2.0), n(w / 2.0), n(h / 2.0),
                fill_attr(&sh.fill), esc(&sh.stroke), n(sh.stroke_mm)
            );
        }
        Kind::Line(sh) => {
            let _ = write!(
                s,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" stroke-linecap="square"/>"#,
                n(x), n(y), n(x + w), n(y + h), esc(&sh.stroke), n(sh.stroke_mm)
            );
        }
        Kind::Image { data_uri } => {
            if !data_uri.starts_with("data:image/") {
                return Err("Image must be an embedded PNG, JPEG or SVG".into());
            }
            let _ = write!(
                s,
                r#"<image x="{}" y="{}" width="{}" height="{}" href="{}" preserveAspectRatio="xMidYMid meet"/>"#,
                n(x), n(y), n(w), n(h), esc(data_uri)
            );
        }
        Kind::Barcode(b) => {
            let data = fill(&b.data, values);
            let opts = |bar_height: f64| barcode::Options {
                show_text: b.show_text,
                ecc: &b.qr_ecc,
                columns: b.columns,
                maxi_mode: b.maxi_mode,
                maxi_primary: &b.maxi_primary,
                bar_height,
                quiet_zone: b.quiet_zone,
            };
            let first = barcode::encode(&b.symbology, &data, &opts(0.0))?;
            let unit = 1.0; // Zint units per module (set by the shim)
            let z = if first.fixed_ratio {
                first
            } else {
                // Fit the width, then re-encode with the bar height that fills the box.
                let module_mm = dot_snap(w / (first.width / unit), dpi);
                let extra = first.height / unit - first.bar_height; // text, guards, row gaps
                let rows = first.rows.max(1) as f64;
                let want = (h / module_mm - extra).max(rows);
                barcode::encode(&b.symbology, &data, &opts(want))?
            };
            let width_mod = z.width / unit;
            let height_mod = z.height / unit;
            // Whole printer dots per module keeps every bar the same width when
            // printed, which matters for scanning. The symbol is centred in its box.
            let module_mm = if z.fixed_ratio {
                dot_snap((w / width_mod).min(h / height_mod), dpi)
            } else {
                dot_snap(w / width_mod, dpi)
            };
            let k = module_mm / unit;
            let ox = x + (w - z.width * k) / 2.0;
            let oy = y + (h - z.height * k) / 2.0;
            let _ = write!(
                s,
                r#"<g color="{}" transform="translate({} {}) scale({})">{}</g>"#,
                esc(&b.color), n(ox), n(oy), format!("{:.6}", k), z.fragment
            );
        }
    }
    Ok(())
}

/// Round a module size down to a whole number of printer dots (at least one).
fn dot_snap(mm: f64, dpi: u32) -> f64 {
    if dpi == 0 {
        return mm;
    }
    let dot = 25.4 / dpi as f64;
    (mm / dot).floor().max(1.0) * dot
}

fn fill_attr(f: &str) -> String {
    if f.is_empty() { "none".into() } else { esc(f) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_variables() {
        let mut v = HashMap::new();
        v.insert("a".to_string(), "1".to_string());
        assert_eq!(fill("x{a}y{b}{{z}}", &v), "x1y{b}{z}");
    }

    #[test]
    fn renders_all_kinds() {
        let mut l = Label::default();
        for (i, kind) in [
            Kind::Text(Text { text: "Part {part}\n<&>".into(), ..Default::default() }),
            Kind::Rect(Shape::default()),
            Kind::Ellipse(Shape::default()),
            Kind::Line(Shape::default()),
            Kind::Barcode(Barcode::default()),
            Kind::Barcode(Barcode { symbology: "qr".into(), ..Default::default() }),
            Kind::Barcode(Barcode { symbology: "ean13".into(), data: "abc".into(), ..Default::default() }),
        ]
        .into_iter()
        .enumerate()
        {
            l.objects.push(Object { id: format!("o{i}"), kind, ..Default::default() });
        }
        let mut v = HashMap::new();
        v.insert("part".into(), "10098101".into());
        let (svg, errs) = render_svg(&l, &v, &RenderOptions { outline: true });
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains("Part 10098101"));
        assert!(svg.contains("&lt;&amp;&gt;"));
        assert_eq!(errs.len(), 1, "EAN-13 with letters must report an error: {errs:?}");
        assert!(errs[0].starts_with("o6"));
    }

    #[test]
    fn modules_are_whole_dots() {
        let dot = 25.4 / 203.0;
        let m = dot_snap(0.33, 203);
        assert!((m / dot - (m / dot).round()).abs() < 1e-9 && m <= 0.33);
        assert_eq!(dot_snap(0.01, 203), dot);
    }
}
