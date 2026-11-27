//! JSON request/response types shared by the desktop app and the dev server,
//! so the user interface talks to both in exactly the same way.

use crate::{render, resolve_values, Label};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct RenderRequest {
    pub label: Label,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    /// Position in the print run (0-based), drives serial counters.
    #[serde(default)]
    pub index: i64,
    /// Draw the label outline (designer) or not (printing).
    #[serde(default)]
    pub outline: bool,
}

#[derive(Serialize)]
pub struct RenderResponse {
    pub svg: String,
    pub values: HashMap<String, String>,
    /// Problems to show the user (bad barcode data, bad pattern...).
    pub errors: Vec<String>,
}

pub fn render(req: RenderRequest) -> RenderResponse {
    let now = chrono::Local::now().naive_local();
    let mut errors = Vec::new();
    let values = match resolve_values(&req.label, &req.inputs, req.index, now) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            req.inputs.clone()
        }
    };
    let (svg, errs) = render::render_svg(&req.label, &values, &render::RenderOptions { outline: req.outline });
    errors.extend(errs);
    RenderResponse { svg, values, errors }
}

/// Render a whole print run: `quantity` labels starting at `index`.
#[derive(Deserialize)]
pub struct RunRequest {
    pub label: Label,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    #[serde(default)]
    pub index: i64,
    pub quantity: u32,
}

pub fn render_run(req: RunRequest) -> Result<Vec<RenderResponse>, String> {
    if req.quantity == 0 || req.quantity > 10_000 {
        return Err("Quantity must be between 1 and 10000".into());
    }
    let mut out = Vec::with_capacity(req.quantity as usize);
    for i in 0..req.quantity as i64 {
        let r = render(RenderRequest { label: req.label.clone(), inputs: req.inputs.clone(), index: req.index + i, outline: false });
        if !r.errors.is_empty() {
            return Err(format!("Label {}: {}", i + 1, r.errors.join("; ")));
        }
        out.push(r);
    }
    Ok(out)
}
