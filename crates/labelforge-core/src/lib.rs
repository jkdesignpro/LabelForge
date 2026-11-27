//! LabelForge core: label model, barcode encoding, serial patterns and
//! rendering. Free software under the GNU AGPL v3 or later.

pub mod api;
pub mod barcode;
pub mod counters;
pub mod expr;
pub mod model;
pub mod pattern;
pub mod raster;
pub mod render;

pub use model::*;

use chrono::NaiveDateTime;
use std::collections::HashMap;

/// Inputs for resolving one label of a print run.
#[derive(Debug, Clone, Default)]
pub struct Run<'a> {
    /// 0-based position of this label in the run; serial counters give
    /// `start + index * step`.
    pub index: i64,
    /// Values typed at print time (prompts) or from a data row. They override
    /// fixed values and prompt defaults.
    pub inputs: Option<&'a HashMap<String, String>>,
    /// First reserved value of each global counter for this run
    /// (see [`counters`]); missing counters show as 0 in previews.
    pub counters: Option<&'a HashMap<String, i64>>,
    /// Apply prompt input rules (on when printing, off in the designer).
    pub check_rules: bool,
}

/// Work out every variable's value for one label in a print run.
///
/// Order: data fields, fixed values and prompts first, then serials, global
/// counters and dates, then formulas in the order they are listed (so a
/// formula can use an earlier formula).
pub fn resolve_values(label: &Label, inputs: &HashMap<String, String>, index: i64, now: NaiveDateTime) -> Result<HashMap<String, String>, String> {
    resolve(label, &Run { index, inputs: Some(inputs), counters: None, check_rules: true }, now)
}

pub fn resolve(label: &Label, run: &Run, now: NaiveDateTime) -> Result<HashMap<String, String>, String> {
    let empty = HashMap::new();
    let inputs = run.inputs.unwrap_or(&empty);
    let mut values: HashMap<String, String> = inputs.clone();
    for v in &label.variables {
        match &v.source {
            VarSource::Fixed { value } => {
                values.entry(v.name.clone()).or_insert_with(|| value.clone());
            }
            VarSource::Prompt { default, rules, .. } => {
                let given = values.get(&v.name).cloned().unwrap_or_else(|| default.clone());
                let checked = if run.check_rules { rules.check(&v.name, &given)? } else { given };
                values.insert(v.name.clone(), checked);
            }
            VarSource::Field { default, .. } => {
                values.entry(v.name.clone()).or_insert_with(|| default.clone());
            }
            _ => {}
        }
    }
    let index = run.index;
    for v in &label.variables {
        let r = match &v.source {
            VarSource::Serial { pattern, start, step } => {
                pattern::expand(pattern, &pattern::Context { seq: start + index * step, now, vars: &values })
            }
            VarSource::Global { counter, pattern, step } => {
                let base = run.counters.and_then(|c| c.get(counter)).copied().unwrap_or(0);
                pattern::expand(pattern, &pattern::Context { seq: base + index * step, now, vars: &values })
            }
            VarSource::Date { pattern, offset_days, offset_months } => {
                let mut t = now + chrono::Duration::days(*offset_days);
                if *offset_months != 0 {
                    let m = chrono::Months::new(offset_months.unsigned_abs() as u32);
                    t = if *offset_months > 0 { t.checked_add_months(m) } else { t.checked_sub_months(m) }.ok_or("Date out of range")?;
                }
                pattern::expand(pattern, &pattern::Context { seq: index, now: t, vars: &values })
            }
            _ => continue,
        };
        let val = r.map_err(|e| format!("Variable '{}': {}", v.name, e))?;
        values.insert(v.name.clone(), val);
    }
    for v in &label.variables {
        if let VarSource::Formula { expr } = &v.source {
            let r = expr::eval(expr, &expr::Env { vars: &values, now }).map_err(|e| format!("Formula '{}': {}", v.name, e))?;
            values.insert(v.name.clone(), r.text());
        }
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
        l.variables.push(Variable { name: "part".into(), source: VarSource::Prompt { prompt: "Part no.".into(), default: "000".into(), rules: Default::default() } });
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

#[cfg(test)]
mod v02_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn formulas_dates_rules_globals() {
        let mut l = Label::default();
        let rules = InputRules { required: true, allowed: "digits".into(), max_len: 8, ..Default::default() };
        l.variables = vec![
            Variable { name: "part".into(), source: VarSource::Prompt { prompt: "Part".into(), default: String::new(), rules } },
            Variable { name: "expiry".into(), source: VarSource::Date { pattern: "{DD}/{MM}/{YYYY}".into(), offset_days: 30, offset_months: 0 } },
            Variable { name: "sn".into(), source: VarSource::Global { counter: "shop".into(), pattern: "S{SEQ:00000}".into(), step: 1 } },
            Variable { name: "code".into(), source: VarSource::Formula { expr: r#"LEFT(part, 4) & "/" & sn"#.into() } },
        ];
        let now = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap().and_hms_opt(9, 0, 0).unwrap();
        let mut inputs = HashMap::new();
        let mut counters = HashMap::new();
        counters.insert("shop".to_string(), 500);
        let run = |inputs: &HashMap<String, String>, idx: i64| {
            resolve(&l, &Run { index: idx, inputs: Some(inputs), counters: Some(&counters), check_rules: true }, now)
        };
        assert!(run(&inputs, 0).unwrap_err().contains("required"));
        inputs.insert("part".into(), "1009x".into());
        assert!(run(&inputs, 0).unwrap_err().contains("digits"));
        inputs.insert("part".into(), "10098101".into());
        let v = run(&inputs, 2).unwrap();
        assert_eq!(v["expiry"], "27/12/2026");
        assert_eq!(v["sn"], "S00502");
        assert_eq!(v["code"], "1009/S00502");
    }
}
