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
    let r = render_full(label, values, opt);
    (r.svg, r.errors)
}

/// Where an object ended up after anchors and conditions.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Placed {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub w_mm: f64,
    pub h_mm: f64,
    pub visible: bool,
}

pub struct Rendered {
    pub svg: String,
    pub errors: Vec<String>,
    pub boxes: Vec<Placed>,
}

/// Evaluate conditions and anchors for every object.
pub fn layout(label: &Label, values: &HashMap<String, String>) -> (Vec<Placed>, Vec<String>) {
    let now = chrono::Local::now().naive_local();
    let env = crate::expr::Env { vars: values, now };
    let mut errors = Vec::new();
    let visible: Vec<bool> = label
        .objects
        .iter()
        .map(|o| match crate::expr::condition(&o.condition, &env) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}: condition: {e}", o.id));
                true
            }
        })
        .collect();
    let idx: HashMap<&str, usize> = label.objects.iter().enumerate().map(|(i, o)| (o.id.as_str(), i)).collect();
    let mut pos: Vec<Option<(f64, f64)>> = vec![None; label.objects.len()];
    fn place(
        i: usize, label: &Label, idx: &HashMap<&str, usize>, visible: &[bool], pos: &mut Vec<Option<(f64, f64)>>,
        depth: usize, errors: &mut Vec<String>,
    ) -> (f64, f64) {
        if let Some(p) = pos[i] {
            return p;
        }
        let o = &label.objects[i];
        let mut p = (o.x_mm, o.y_mm);
        if let Some(a) = &o.anchor {
            match idx.get(a.to.as_str()) {
                Some(&t) if t != i && depth < 32 => {
                    let tp = place(t, label, idx, visible, pos, depth + 1, errors);
                    let to = &label.objects[t];
                    let shown = visible[t];
                    p.0 = match a.horizontal.as_str() {
                        "right" if shown => tp.0 + to.w_mm + o.x_mm,
                        "right" => tp.0,
                        "left" => tp.0 + o.x_mm,
                        _ => o.x_mm,
                    };
                    p.1 = match a.vertical.as_str() {
                        "below" if shown => tp.1 + to.h_mm + o.y_mm,
                        "below" => tp.1,
                        "top" => tp.1 + o.y_mm,
                        _ => o.y_mm,
                    };
                }
                Some(_) => errors.push(format!("{}: anchor loop", o.id)),
                None if a.to.is_empty() => {}
                None => errors.push(format!("{}: anchored to missing object '{}'", o.id, a.to)),
            }
        }
        pos[i] = Some(p);
        p
    }
    let mut out = Vec::new();
    for i in 0..label.objects.len() {
        let (x, y) = place(i, label, &idx, &visible, &mut pos, 0, &mut errors);
        let o = &label.objects[i];
        out.push(Placed { id: o.id.clone(), x_mm: x, y_mm: y, w_mm: o.w_mm, h_mm: o.h_mm, visible: visible[i] });
    }
    (out, errors)
}

pub fn render_full(label: &Label, values: &HashMap<String, String>, opt: &RenderOptions) -> Rendered {
    let (w, h) = (label.width_mm, label.height_mm);
    let mut s = String::new();
    let (boxes, mut errors) = layout(label, values);
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
    for (orig, b) in label.objects.iter().zip(&boxes) {
        // Hidden objects are left out of prints; the designer shows them faintly.
        if !b.visible && !opt.outline {
            continue;
        }
        let mut obj = orig.clone();
        obj.x_mm = b.x_mm;
        obj.y_mm = b.y_mm;
        let mut body = String::new();
        if let Err(e) = render_object(&obj, values, label.dpi, &mut body) {
            errors.push(format!("{}: {}", obj.id, e));
            let _ = write!(
                body,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#ffecec" stroke="#d93025" stroke-width="0.3" stroke-dasharray="1 0.6"/><text x="{}" y="{}" font-family="Helvetica" font-size="2.2" fill="#d93025">{}</text>"##,
                n(obj.x_mm), n(obj.y_mm), n(obj.w_mm), n(obj.h_mm),
                n(obj.x_mm + 0.8), n(obj.y_mm + 2.8), esc(&e)
            );
        }
        let faint = if b.visible { "" } else { r#" opacity="0.25""# };
        if obj.rotation.rem_euclid(360.0) != 0.0 {
            let (cx, cy) = (obj.x_mm + obj.w_mm / 2.0, obj.y_mm + obj.h_mm / 2.0);
            let _ = write!(s, r#"<g data-id="{}"{} transform="rotate({} {} {})">{}</g>"#, esc(&obj.id), faint, n(obj.rotation), n(cx), n(cy), body);
        } else {
            let _ = write!(s, r#"<g data-id="{}"{}>{}</g>"#, esc(&obj.id), faint, body);
        }
    }
    s.push_str("</svg>");
    Rendered { svg: s, errors, boxes }
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
    fn conditions_and_anchors() {
        let mut l = Label::default();
        let t = |id: &str, y: f64| Object { id: id.into(), x_mm: 2.0, y_mm: y, w_mm: 30.0, h_mm: 5.0, ..Default::default() };
        let mut a = t("a", 2.0);
        a.condition = "NOT EMPTY(lot)".into();
        let mut b = t("b", 1.0);
        b.anchor = Some(Anchor { to: "a".into(), horizontal: "none".into(), vertical: "below".into() });
        let mut c = t("c", 0.0);
        c.anchor = Some(Anchor { to: "a".into(), horizontal: "right".into(), vertical: "top".into() });
        l.objects = vec![b, a, c]; // order must not matter
        let mut v = HashMap::new();
        v.insert("lot".to_string(), "L42".to_string());
        let (boxes, errs) = layout(&l, &v);
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!((boxes[0].y_mm, boxes[1].visible), (2.0 + 5.0 + 1.0, true)); // b below a + 1 mm gap
        assert_eq!((boxes[2].x_mm, boxes[2].y_mm), (2.0 + 30.0 + 2.0, 2.0));
        v.insert("lot".to_string(), "".to_string());
        let (boxes, _) = layout(&l, &v);
        assert!(!boxes[1].visible);
        assert_eq!(boxes[0].y_mm, 2.0); // b moves up into a's place
        let (svg, _) = render_svg(&l, &v, &RenderOptions { outline: false });
        assert!(!svg.contains(r#"data-id="a""#));
    }

    #[test]
    fn modules_are_whole_dots() {
        let dot = 25.4 / 203.0;
        let m = dot_snap(0.33, 203);
        assert!((m / dot - (m / dot).round()).abs() < 1e-9 && m <= 0.33);
        assert_eq!(dot_snap(0.01, 203), dot);
    }
}
