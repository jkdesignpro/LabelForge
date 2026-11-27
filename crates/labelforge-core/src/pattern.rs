//! Serial and data patterns.
//!
//! A pattern is plain text with tokens in braces. Everything outside braces
//! is copied as-is, so prefixes and suffixes are just text:
//!
//! | Token | Meaning | Example (seq 42, 27 Nov 2026 14:05) |
//! |---|---|---|
//! | `{SEQ}` | counter, no padding | `42` |
//! | `{SEQ:0000}` | counter, zero padded to 4 | `0042` |
//! | `{SEQ36:4}` | counter in base 36 (0-9, A-Z), padded | `0016` |
//! | `{SEQAZ:3}` | counter in letters only (A-Z), padded | `ABQ` |
//! | `{YYYY}` `{YY}` `{MM}` `{DD}` | date | `2026` `26` `11` `27` |
//! | `{HH}` `{NN}` `{SS}` | time (24 h) | `14` `05` `00` |
//! | `{JJJ}` | day of year | `331` |
//! | `{WW}` | ISO week | `48` |
//! | `{CHK:MOD10}` | GS1 mod 10 check digit of everything before it | |
//! | `{CHK:LUHN}` | Luhn check digit of the digits before it | |
//! | `{CHK:MOD43}` | Code 39 mod 43 check character | |
//! | `{name}` | value of another variable | |
//! | `{{` `}}` | literal brace | |
//!
//! Example: `LF-{YY}{MM}-{SEQ:0000}-A` gives `LF-2611-0042-A`.

use chrono::{Datelike, NaiveDateTime, Timelike};
use std::collections::HashMap;

pub struct Context<'a> {
    pub seq: i64,
    pub now: NaiveDateTime,
    pub vars: &'a HashMap<String, String>,
}

/// Expand a pattern. Unknown tokens are an error so typos are caught early.
pub fn expand(pattern: &str, ctx: &Context) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut tok = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(ch) => tok.push(ch),
                        None => return Err(format!("Missing '}}' after '{{{tok}'")),
                    }
                }
                let v = token(&tok, &out, ctx)?;
                out.push_str(&v);
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

fn token(tok: &str, before: &str, ctx: &Context) -> Result<String, String> {
    let (name, arg) = match tok.split_once(':') {
        Some((n, a)) => (n.trim(), Some(a.trim())),
        None => (tok.trim(), None),
    };
    let n = &ctx.now;
    Ok(match name {
        "SEQ" => {
            let width = arg.map(|a| a.len()).unwrap_or(0);
            if ctx.seq < 0 {
                format!("-{:0width$}", -ctx.seq)
            } else {
                format!("{:0width$}", ctx.seq)
            }
        }
        "SEQ36" => pad(&radix(ctx.seq, "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ")?, arg, '0')?,
        "SEQAZ" => pad(&radix(ctx.seq, "ABCDEFGHIJKLMNOPQRSTUVWXYZ")?, arg, 'A')?,
        "YYYY" => format!("{:04}", n.year()),
        "YY" => format!("{:02}", n.year() % 100),
        "MM" => format!("{:02}", n.month()),
        "DD" => format!("{:02}", n.day()),
        "HH" => format!("{:02}", n.hour()),
        "NN" => format!("{:02}", n.minute()),
        "SS" => format!("{:02}", n.second()),
        "JJJ" => format!("{:03}", n.ordinal()),
        "WW" => format!("{:02}", n.iso_week().week()),
        "CHK" => match arg.map(|a| a.to_ascii_uppercase()).as_deref() {
            Some("MOD10") => mod10(before)?.to_string(),
            Some("LUHN") => luhn(before)?.to_string(),
            Some("MOD43") => mod43(before)?.to_string(),
            other => return Err(format!("Unknown check digit type {:?} (use MOD10, LUHN or MOD43)", other.unwrap_or(""))),
        },
        _ => match ctx.vars.get(name) {
            Some(v) => v.clone(),
            None => return Err(format!("Unknown token or variable {{{tok}}}")),
        },
    })
}

fn radix(mut v: i64, digits: &str) -> Result<String, String> {
    if v < 0 {
        return Err("Letter/base-36 counters cannot be negative".into());
    }
    let d: Vec<char> = digits.chars().collect();
    let b = d.len() as i64;
    let mut s = Vec::new();
    loop {
        s.push(d[(v % b) as usize]);
        v /= b;
        if v == 0 {
            break;
        }
    }
    Ok(s.iter().rev().collect())
}

fn pad(s: &str, arg: Option<&str>, fill: char) -> Result<String, String> {
    let width = match arg {
        None => 0,
        Some(a) => a.parse::<usize>().map_err(|_| format!("Width must be a number, got '{a}'"))?,
    };
    let mut out: String = std::iter::repeat(fill).take(width.saturating_sub(s.chars().count())).collect();
    out.push_str(s);
    Ok(out)
}

fn digits_of(s: &str) -> Result<Vec<u32>, String> {
    let d: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.is_empty() {
        return Err("Check digit needs digits before it".into());
    }
    Ok(d)
}

/// GS1 mod 10 (EAN/UPC/SSCC): weights 3,1,3,1... from the right.
pub fn mod10(s: &str) -> Result<u32, String> {
    let d = digits_of(s)?;
    let sum: u32 = d.iter().rev().enumerate().map(|(i, v)| if i % 2 == 0 { v * 3 } else { *v }).sum();
    Ok((10 - sum % 10) % 10)
}

/// Luhn (mod 10, doubling every second digit from the right).
pub fn luhn(s: &str) -> Result<u32, String> {
    let d = digits_of(s)?;
    let sum: u32 = d
        .iter()
        .rev()
        .enumerate()
        .map(|(i, v)| if i % 2 == 0 { let x = v * 2; if x > 9 { x - 9 } else { x } } else { *v })
        .sum();
    Ok((10 - sum % 10) % 10)
}

const C39: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

/// Code 39 mod 43 check character.
pub fn mod43(s: &str) -> Result<char, String> {
    let mut sum = 0usize;
    for c in s.chars() {
        let up = c.to_ascii_uppercase();
        sum += C39.find(up).ok_or_else(|| format!("'{c}' is not allowed in Code 39"))?;
    }
    Ok(C39.as_bytes()[sum % 43] as char)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn ctx(seq: i64, vars: &HashMap<String, String>) -> Context<'_> {
        let now = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap().and_hms_opt(14, 5, 0).unwrap();
        Context { seq, now, vars }
    }

    #[test]
    fn prefix_counter_suffix() {
        let v = HashMap::new();
        assert_eq!(expand("LF-{YY}{MM}-{SEQ:0000}-A", &ctx(42, &v)).unwrap(), "LF-2611-0042-A");
        assert_eq!(expand("{SEQ}", &ctx(7, &v)).unwrap(), "7");
        assert_eq!(expand("{YYYY}{JJJ} W{WW} {HH}:{NN}", &ctx(1, &v)).unwrap(), "2026331 W48 14:05");
    }

    #[test]
    fn alpha_counters() {
        let v = HashMap::new();
        assert_eq!(expand("{SEQ36:4}", &ctx(42, &v)).unwrap(), "0016");
        assert_eq!(expand("{SEQ36}", &ctx(35, &v)).unwrap(), "Z");
        assert_eq!(expand("{SEQAZ:3}", &ctx(0, &v)).unwrap(), "AAA");
        assert_eq!(expand("{SEQAZ:3}", &ctx(27, &v)).unwrap(), "ABB");
    }

    #[test]
    fn check_digits() {
        // EAN-13 4006381333931: check digit 1 (well-known example).
        assert_eq!(mod10("400638133393").unwrap(), 1);
        // Luhn example 7992739871 -> 3.
        assert_eq!(luhn("7992739871").unwrap(), 3);
        // Code 39 "CODE39" -> W.
        assert_eq!(mod43("CODE39").unwrap(), 'W');
        let v = HashMap::new();
        assert_eq!(expand("400638133393{CHK:MOD10}", &ctx(1, &v)).unwrap(), "4006381333931");
    }

    #[test]
    fn variables_and_escapes() {
        let mut v = HashMap::new();
        v.insert("part".to_string(), "10098101".to_string());
        assert_eq!(expand("{part}/{SEQ:000} {{x}}", &ctx(3, &v)).unwrap(), "10098101/003 {x}");
        assert!(expand("{nope}", &ctx(3, &v)).is_err());
        assert!(expand("{SEQ", &ctx(3, &v)).is_err());
    }
}
