//! JSON request/response types shared by the desktop app and the dev server,
//! so the user interface talks to both in exactly the same way.

use crate::{counters, render, resolve, Label, Run, VarSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Clone)]
pub struct RenderRequest {
    pub label: Label,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    /// Position in the print run (0-based), drives serial counters.
    #[serde(default)]
    pub index: i64,
    /// Draw the label outline (designer) or not (printing). Designer renders
    /// also skip prompt input rules and show hidden objects faintly.
    #[serde(default)]
    pub outline: bool,
    /// First value of each global counter for this run.
    #[serde(default)]
    pub counters: HashMap<String, i64>,
}

#[derive(Serialize)]
pub struct RenderResponse {
    pub svg: String,
    pub values: HashMap<String, String>,
    /// Problems to show the user (bad barcode data, bad pattern...).
    pub errors: Vec<String>,
    /// Final position and visibility of each object (after anchors/conditions).
    pub boxes: Vec<render::Placed>,
}

pub fn render(req: RenderRequest) -> RenderResponse {
    let now = chrono::Local::now().naive_local();
    let mut errors = Vec::new();
    let run = Run { index: req.index, inputs: Some(&req.inputs), counters: Some(&req.counters), check_rules: !req.outline };
    let values = match resolve(&req.label, &run, now) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            req.inputs.clone()
        }
    };
    let r = render::render_full(&req.label, &values, &render::RenderOptions { outline: req.outline });
    errors.extend(r.errors);
    RenderResponse { svg: r.svg, values, errors, boxes: r.boxes }
}

/// Render a whole print run: `quantity` labels starting at `index`.
#[derive(Deserialize, Clone)]
pub struct RunRequest {
    pub label: Label,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    #[serde(default)]
    pub index: i64,
    pub quantity: u32,
    #[serde(default)]
    pub counters: HashMap<String, i64>,
}

pub fn render_run(req: RunRequest) -> Result<Vec<RenderResponse>, String> {
    if req.quantity == 0 || req.quantity > 100_000 {
        return Err("Quantity must be between 1 and 100000".into());
    }
    let mut out = Vec::with_capacity(req.quantity as usize);
    for i in 0..req.quantity as i64 {
        let r = render(RenderRequest {
            label: req.label.clone(),
            inputs: req.inputs.clone(),
            index: req.index + i,
            outline: false,
            counters: req.counters.clone(),
        });
        if !r.errors.is_empty() {
            return Err(format!("Label {}: {}", i + 1, r.errors.join("; ")));
        }
        out.push(r);
    }
    Ok(out)
}

/// Global counters used by a label, with the amount one label consumes.
pub fn label_counters(label: &Label) -> HashMap<String, i64> {
    let mut m: HashMap<String, i64> = HashMap::new();
    for v in &label.variables {
        if let VarSource::Global { counter, step, .. } = &v.source {
            let e = m.entry(counter.clone()).or_insert(1);
            *e = (*e).max(step.abs().max(1));
        }
    }
    m
}

/// Current next values (for previews; nothing is reserved).
pub fn peek_counters(dir: &Path, label: &Label) -> Result<HashMap<String, i64>, String> {
    label_counters(label).into_keys().map(|c| counters::peek(dir, &c).map(|v| (c, v))).collect()
}

/// Reserve the numbers a run of `quantity` labels needs; returns first values.
pub fn reserve_counters(dir: &Path, label: &Label, quantity: u32) -> Result<HashMap<String, i64>, String> {
    label_counters(label)
        .into_iter()
        .map(|(c, step)| counters::reserve(dir, &c, step * quantity as i64).map(|v| (c, v)))
        .collect()
}

/// Barcode types for the UI: key, name, group and example data.
pub fn symbologies() -> serde_json::Value {
    serde_json::to_value(crate::barcode::SYMBOLOGIES).unwrap()
}

/// PNG of one label at `dpi` (no outline).
#[derive(Deserialize)]
pub struct PngRequest {
    #[serde(flatten)]
    pub render: RenderRequest,
    #[serde(default = "default_dpi")]
    pub dpi: f64,
}

fn default_dpi() -> f64 {
    300.0
}

pub fn png(req: PngRequest) -> Result<Vec<u8>, String> {
    let mut r = req.render;
    r.outline = false;
    let out = render(r);
    if !out.errors.is_empty() {
        return Err(out.errors.join("; "));
    }
    crate::raster::png(&out.svg, req.dpi)
}

/// PDF with one page per label in the run.
pub fn pdf(req: RunRequest) -> Result<Vec<u8>, String> {
    let pages: Vec<String> = render_run(req)?.into_iter().map(|p| p.svg).collect();
    crate::raster::pdf(&pages)
}
