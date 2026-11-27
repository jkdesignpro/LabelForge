//! LabelForge core: label model, barcode encoding, serial patterns and
//! rendering. Free software under the GNU AGPL v3 or later.

pub mod api;
pub mod barcode;
pub mod model;
pub mod pattern;
pub mod render;

pub use model::*;

use chrono::NaiveDateTime;
use std::collections::HashMap;

/// Work out every variable's value for one label in a print run.
///
/// * `index` is the 0-based position of this label in the run; serial
///   counters give `start + index * step`.
/// * `inputs` holds values typed at print time (prompts) or from a data row;
///   they override fixed values and prompt defaults.
pub fn resolve_values(
    label: &Label,
    inputs: &HashMap<String, String>,
    index: i64,
    now: NaiveDateTime,
) -> Result<HashMap<String, String>, String> {
    let mut values: HashMap<String, String> = inputs.clone();
    // Fixed and prompt values first so serial/date patterns can use them.
    for v in &label.variables {
        match &v.source {
            VarSource::Fixed { value } => {
                values.entry(v.name.clone()).or_insert_with(|| value.clone());
            }
            VarSource::Prompt { default, .. } => {
                values.entry(v.name.clone()).or_insert_with(|| default.clone());
            }
            _ => {}
        }
    }
    for v in &label.variables {
        let r = match &v.source {
            VarSource::Serial { pattern, start, step } => {
                let ctx = pattern::Context { seq: start + index * step, now, vars: &values };
                pattern::expand(pattern, &ctx)
            }
            VarSource::Date { pattern } => {
                let ctx = pattern::Context { seq: index, now, vars: &values };
                pattern::expand(pattern, &ctx)
            }
            _ => continue,
        };
        let val = r.map_err(|e| format!("Variable '{}': {}", v.name, e))?;
        values.insert(v.name.clone(), val);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn serial_run_and_roundtrip() {
        let mut l = Label::default();
        l.variables.push(Variable { name: "part".into(), source: VarSource::Prompt { prompt: "Part no.".into(), default: "000".into() } });
        l.variables.push(Variable { name: "sn".into(), source: VarSource::Serial { pattern: "{part}-{YY}-{SEQ:0000}".into(), start: 100, step: 5 } });
        let now = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap().and_hms_opt(9, 0, 0).unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("part".to_string(), "10098101".to_string());
        assert_eq!(resolve_values(&l, &inputs, 0, now).unwrap()["sn"], "10098101-26-0100");
        assert_eq!(resolve_values(&l, &inputs, 2, now).unwrap()["sn"], "10098101-26-0110");
        assert_eq!(resolve_values(&l, &HashMap::new(), 0, now).unwrap()["sn"], "000-26-0100");

        let json = l.to_json();
        assert_eq!(Label::from_json(&json).unwrap(), l);
        assert!(Label::from_json("{\"schema\": 99}").is_err());
    }

    #[test]
    fn example_label_loads() {
        let s = include_str!("../../../examples/part-tag.lfl");
        let l = Label::from_json(s).unwrap();
        assert!(!l.objects.is_empty());
        let now = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap().and_hms_opt(9, 0, 0).unwrap();
        let v = resolve_values(&l, &HashMap::new(), 0, now).unwrap();
        let (_, errs) = render::render_svg(&l, &v, &render::RenderOptions { outline: false });
        assert!(errs.is_empty(), "{errs:?}");
    }
}
