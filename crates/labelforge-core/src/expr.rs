//! A small, safe formula language for computed variables and conditions.
//!
//! ```text
//! LEFT(part, 4) & "-" & PAD(SEQ, 5, "0")
//! IF(material = "303", "STAINLESS", UPPER(material))
//! DATE("{DD}/{MM}/{YYYY}", 30)          today + 30 days
//! qty > 10 AND NOT EMPTY(lot)
//! ```
//!
//! Values are text or numbers; text that looks like a number works as one.
//! `&` joins text, `+ - * / %` are arithmetic, comparisons are
//! `= == != <> < <= > >=`, logic is `AND OR NOT`. Variables are written as
//! bare names or `{name}`. No loops, no file or network access.

use chrono::{Datelike, Months, NaiveDateTime};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Str(String),
    Num(f64),
}

impl Val {
    pub fn text(&self) -> String {
        match self {
            Val::Str(s) => s.clone(),
            Val::Num(n) => fmt_num(*n),
        }
    }
    fn num(&self) -> Result<f64, String> {
        match self {
            Val::Num(n) => Ok(*n),
            Val::Str(s) => s.trim().parse::<f64>().map_err(|_| format!("'{s}' is not a number")),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Val::Num(n) => *n != 0.0,
            Val::Str(s) => {
                let t = s.trim().to_ascii_lowercase();
                !(t.is_empty() || t == "0" || t == "false" || t == "no")
            }
        }
    }
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{:.10}", n);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Var(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            let s: String = c[st..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| format!("Bad number '{s}'"))?));
        } else if ch == '"' || ch == '\'' {
            let q = ch;
            i += 1;
            let mut s = String::new();
            loop {
                match c.get(i) {
                    None => return Err("Text is missing its closing quote".into()),
                    Some(&x) if x == q => {
                        if c.get(i + 1) == Some(&q) {
                            s.push(q);
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    }
                    Some(&x) => {
                        s.push(x);
                        i += 1;
                    }
                }
            }
            out.push(Tok::Str(s));
        } else if ch == '{' {
            let st = i + 1;
            while i < c.len() && c[i] != '}' {
                i += 1;
            }
            if i >= c.len() {
                return Err("Missing '}'".into());
            }
            out.push(Tok::Var(c[st..i].iter().collect()));
            i += 1;
        } else if ch.is_alphabetic() || ch == '_' {
            let st = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let w: String = c[st..i].iter().collect();
            match w.to_ascii_uppercase().as_str() {
                "AND" => out.push(Tok::Op("AND")),
                "OR" => out.push(Tok::Op("OR")),
                "NOT" => out.push(Tok::Op("NOT")),
                _ => out.push(Tok::Ident(w)),
            }
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let op2 = ["==", "!=", "<>", "<=", ">="].iter().find(|o| **o == two);
            if let Some(o) = op2 {
                out.push(Tok::Op(o));
                i += 2;
                continue;
            }
            let op1 = ["=", "<", ">", "+", "-", "*", "/", "%", "&", "!"].iter().find(|o| o.starts_with(ch));
            match (ch, op1) {
                ('(', _) => out.push(Tok::LParen),
                (')', _) => out.push(Tok::RParen),
                (',', _) | (';', _) => out.push(Tok::Comma),
                (_, Some(o)) => out.push(Tok::Op(if *o == "!" { "NOT" } else { o })),
                _ => return Err(format!("Unexpected '{ch}'")),
            }
            i += 1;
        }
    }
    Ok(out)
}

pub struct Env<'a> {
    pub vars: &'a HashMap<String, String>,
    pub now: NaiveDateTime,
}

struct P<'a, 'b> {
    t: Vec<Tok>,
    i: usize,
    env: &'b Env<'a>,
}

pub fn eval(src: &str, env: &Env) -> Result<Val, String> {
    if src.trim().is_empty() {
        return Ok(Val::Str(String::new()));
    }
    let mut p = P { t: lex(src)?, i: 0, env };
    let v = p.or()?;
    if p.i < p.t.len() {
        return Err(format!("Unexpected {:?}", p.t[p.i]));
    }
    Ok(v)
}

/// Evaluate a condition. Empty means "always".
pub fn condition(src: &str, env: &Env) -> Result<bool, String> {
    if src.trim().is_empty() {
        return Ok(true);
    }
    Ok(eval(src, env)?.truthy())
}

impl P<'_, '_> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn eat_op(&mut self, ops: &[&str]) -> Option<&'static str> {
        if let Some(Tok::Op(o)) = self.peek() {
            if ops.contains(o) {
                let o = *o;
                self.i += 1;
                return Some(o);
            }
        }
        None
    }
    fn or(&mut self) -> Result<Val, String> {
        let mut l = self.and()?;
        while self.eat_op(&["OR"]).is_some() {
            let r = self.and()?;
            l = Val::Num((l.truthy() || r.truthy()) as i32 as f64);
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Val, String> {
        let mut l = self.cmp()?;
        while self.eat_op(&["AND"]).is_some() {
            let r = self.cmp()?;
            l = Val::Num((l.truthy() && r.truthy()) as i32 as f64);
        }
        Ok(l)
    }
    fn cmp(&mut self) -> Result<Val, String> {
        let l = self.add()?;
        if let Some(op) = self.eat_op(&["=", "==", "!=", "<>", "<", "<=", ">", ">="]) {
            let r = self.add()?;
            let ord = match (l.num(), r.num()) {
                (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
                _ => l.text().to_lowercase().cmp(&r.text().to_lowercase()),
            };
            use std::cmp::Ordering::*;
            let b = match op {
                "=" | "==" => ord == Equal,
                "!=" | "<>" => ord != Equal,
                "<" => ord == Less,
                "<=" => ord != Greater,
                ">" => ord == Greater,
                _ => ord != Less,
            };
            return Ok(Val::Num(b as i32 as f64));
        }
        Ok(l)
    }
    fn add(&mut self) -> Result<Val, String> {
        let mut l = self.mul()?;
        while let Some(op) = self.eat_op(&["+", "-", "&"]) {
            let r = self.mul()?;
            l = match op {
                "&" => Val::Str(l.text() + &r.text()),
                "+" => Val::Num(l.num()? + r.num()?),
                _ => Val::Num(l.num()? - r.num()?),
            };
        }
        Ok(l)
    }
    fn mul(&mut self) -> Result<Val, String> {
        let mut l = self.unary()?;
        while let Some(op) = self.eat_op(&["*", "/", "%"]) {
            let r = self.unary()?;
            let (a, b) = (l.num()?, r.num()?);
            l = Val::Num(match op {
                "*" => a * b,
                _ if b == 0.0 => return Err("Division by zero".into()),
                "/" => a / b,
                _ => a % b,
            });
        }
        Ok(l)
    }
    fn unary(&mut self) -> Result<Val, String> {
        if self.eat_op(&["-"]).is_some() {
            return Ok(Val::Num(-self.unary()?.num()?));
        }
        if self.eat_op(&["NOT"]).is_some() {
            return Ok(Val::Num((!self.unary()?.truthy()) as i32 as f64));
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Val, String> {
        let t = self.peek().cloned().ok_or("Formula ends too early")?;
        self.i += 1;
        match t {
            Tok::Num(n) => Ok(Val::Num(n)),
            Tok::Str(s) => Ok(Val::Str(s)),
            Tok::Var(v) => self.var(&v),
            Tok::LParen => {
                let v = self.or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err("Missing ')'".into());
                }
                self.i += 1;
                Ok(v)
            }
            Tok::Ident(name) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.or()?);
                            match self.peek() {
                                Some(Tok::Comma) => self.i += 1,
                                Some(Tok::RParen) => break,
                                _ => return Err(format!("Expected ',' or ')' in {name}()")),
                            }
                        }
                    }
                    self.i += 1;
                    call(&name, &args, self.env)
                } else {
                    match name.to_ascii_uppercase().as_str() {
                        "TRUE" => Ok(Val::Num(1.0)),
                        "FALSE" => Ok(Val::Num(0.0)),
                        _ => self.var(&name),
                    }
                }
            }
            other => Err(format!("Unexpected {other:?}")),
        }
    }
    fn var(&self, name: &str) -> Result<Val, String> {
        self.env.vars.get(name).map(|v| Val::Str(v.clone())).ok_or_else(|| format!("Unknown variable '{name}'"))
    }
}

fn call(name: &str, a: &[Val], env: &Env) -> Result<Val, String> {
    let n = name.to_ascii_uppercase();
    let need = |k: usize| -> Result<(), String> {
        if a.len() < k {
            Err(format!("{n}() needs {k} value(s)"))
        } else {
            Ok(())
        }
    };
    let s = |i: usize| a.get(i).map(|v| v.text()).unwrap_or_default();
    let int = |i: usize, d: i64| -> Result<i64, String> { a.get(i).map(|v| v.num().map(|x| x as i64)).unwrap_or(Ok(d)) };
    let chars = |t: &str| t.chars().collect::<Vec<_>>();
    Ok(match n.as_str() {
        "LEFT" => {
            need(2)?;
            Val::Str(chars(&s(0)).into_iter().take(int(1, 0)?.max(0) as usize).collect())
        }
        "RIGHT" => {
            need(2)?;
            let c = chars(&s(0));
            let k = (int(1, 0)?.max(0) as usize).min(c.len());
            Val::Str(c[c.len() - k..].iter().collect())
        }
        "MID" => {
            need(2)?;
            let c = chars(&s(0));
            let st = (int(1, 1)?.max(1) as usize - 1).min(c.len());
            let len = int(2, c.len() as i64)?.max(0) as usize;
            Val::Str(c[st..].iter().take(len).collect())
        }
        "LEN" => Val::Num(chars(&s(0)).len() as f64),
        "UPPER" => Val::Str(s(0).to_uppercase()),
        "LOWER" => Val::Str(s(0).to_lowercase()),
        "TRIM" => Val::Str(s(0).trim().to_string()),
        "PAD" | "PADLEFT" | "PADRIGHT" => {
            need(2)?;
            let t = s(0);
            let w = int(1, 0)?.max(0) as usize;
            let f = s(2).chars().next().unwrap_or(if n == "PADRIGHT" { ' ' } else { '0' });
            let fill: String = std::iter::repeat(f).take(w.saturating_sub(t.chars().count())).collect();
            Val::Str(if n == "PADRIGHT" { t + &fill } else { fill + &t })
        }
        "REPLACE" => {
            need(3)?;
            Val::Str(s(0).replace(&s(1), &s(2)))
        }
        "CONTAINS" => Val::Num(s(0).to_lowercase().contains(&s(1).to_lowercase()) as i32 as f64),
        "STARTSWITH" => Val::Num(s(0).to_lowercase().starts_with(&s(1).to_lowercase()) as i32 as f64),
        "ENDSWITH" => Val::Num(s(0).to_lowercase().ends_with(&s(1).to_lowercase()) as i32 as f64),
        "EMPTY" => Val::Num(s(0).trim().is_empty() as i32 as f64),
        "IF" => {
            need(2)?;
            if a[0].truthy() {
                a[1].clone()
            } else {
                a.get(2).cloned().unwrap_or(Val::Str(String::new()))
            }
        }
        "COALESCE" => a.iter().find(|v| !v.text().trim().is_empty()).cloned().unwrap_or(Val::Str(String::new())),
        "ROUND" => {
            let d = int(1, 0)?;
            let m = 10f64.powi(d as i32);
            Val::Num((a.first().ok_or("ROUND() needs a value")?.num()? * m).round() / m)
        }
        "FLOOR" => Val::Num(a.first().ok_or("FLOOR() needs a value")?.num()?.floor()),
        "CEIL" => Val::Num(a.first().ok_or("CEIL() needs a value")?.num()?.ceil()),
        "ABS" => Val::Num(a.first().ok_or("ABS() needs a value")?.num()?.abs()),
        "MIN" | "MAX" => {
            need(1)?;
            let mut it = a.iter().map(|v| v.num());
            let mut m = it.next().unwrap()?;
            for v in it {
                let v = v?;
                m = if n == "MIN" { m.min(v) } else { m.max(v) };
            }
            Val::Num(m)
        }
        "FIXED" => {
            // FIXED(number, decimals): text with exactly that many decimals.
            need(1)?;
            Val::Str(format!("{:.*}", int(1, 2)?.clamp(0, 10) as usize, a[0].num()?))
        }
        "DATE" | "NOW" => {
            // DATE(pattern, days, months), NOW(pattern, minutes)
            let pat = if a.is_empty() { "{DD}/{MM}/{YYYY}".to_string() } else { s(0) };
            let mut t = env.now;
            if n == "DATE" {
                t += chrono::Duration::days(int(1, 0)?);
                let months = int(2, 0)?;
                t = if months >= 0 {
                    t.checked_add_months(Months::new(months as u32))
                } else {
                    t.checked_sub_months(Months::new((-months) as u32))
                }
                .ok_or("Date out of range")?;
            } else {
                t += chrono::Duration::minutes(int(1, 0)?);
            }
            let ctx = crate::pattern::Context { seq: 0, now: t, vars: env.vars };
            Val::Str(crate::pattern::expand(&pat, &ctx)?)
        }
        "YEAR" => Val::Num(env.now.year() as f64),
        "CHECKDIGIT" => {
            need(1)?;
            let t = s(0);
            let kind = if a.len() > 1 { s(1).to_ascii_uppercase() } else { "MOD10".into() };
            Val::Str(match kind.as_str() {
                "MOD10" => crate::pattern::mod10(&t)?.to_string(),
                "LUHN" => crate::pattern::luhn(&t)?.to_string(),
                "MOD43" => crate::pattern::mod43(&t)?.to_string(),
                k => return Err(format!("Unknown check digit type {k}")),
            })
        }
        _ => return Err(format!("Unknown function {name}()")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn run(src: &str) -> Result<String, String> {
        let mut v = HashMap::new();
        v.insert("part".to_string(), "10098101".to_string());
        v.insert("material".to_string(), "303".to_string());
        v.insert("qty".to_string(), "12".to_string());
        v.insert("lot".to_string(), "".to_string());
        let now = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap().and_hms_opt(14, 5, 0).unwrap();
        eval(src, &Env { vars: &v, now }).map(|x| x.text())
    }

    #[test]
    fn text_functions() {
        assert_eq!(run(r#"LEFT(part, 4) & "-" & PAD(qty, 5, "0")"#).unwrap(), "1009-00012");
        assert_eq!(run("RIGHT(part,3)").unwrap(), "101");
        assert_eq!(run("MID(part, 3, 2)").unwrap(), "09");
        assert_eq!(run(r#"UPPER("a") & LOWER("B") & LEN("abc")"#).unwrap(), "Ab3");
        assert_eq!(run(r#"REPLACE("a-b-c", "-", "/")"#).unwrap(), "a/b/c");
        assert_eq!(run(r#"PADRIGHT("ab", 4, ".")"#).unwrap(), "ab..");
        assert_eq!(run(r#"COALESCE(lot, "NO LOT")"#).unwrap(), "NO LOT");
    }

    #[test]
    fn maths_and_logic() {
        assert_eq!(run("qty * 2 + 1").unwrap(), "25");
        assert_eq!(run("qty / 5").unwrap(), "2.4");
        assert_eq!(run("FIXED(qty / 7, 2)").unwrap(), "1.71");
        assert_eq!(run("ROUND(2.345, 2)").unwrap(), "2.35");
        assert_eq!(run(r#"IF(material = "303", "STAINLESS", "OTHER")"#).unwrap(), "STAINLESS");
        assert_eq!(run(r#"qty > 10 AND NOT EMPTY(lot)"#).unwrap(), "0");
        assert_eq!(run(r#"qty > 10 OR EMPTY(lot)"#).unwrap(), "1");
        assert_eq!(run("{qty} <> 12").unwrap(), "0");
        assert!(run("1/0").is_err());
        assert!(run("nope + 1").is_err());
        assert!(run("LEFT(").is_err());
    }

    #[test]
    fn dates() {
        assert_eq!(run(r#"DATE("{DD}/{MM}/{YYYY}", 30)"#).unwrap(), "27/12/2026");
        assert_eq!(run(r#"DATE("{YYYY}-{MM}-{DD}", 0, 6)"#).unwrap(), "2027-05-27");
        assert_eq!(run(r#"DATE("{YY}{MM}{DD}", -27)"#).unwrap(), "261031");
        assert_eq!(run(r#"NOW("{HH}:{NN}", 90)"#).unwrap(), "15:35");
        assert_eq!(run(r#"CHECKDIGIT("400638133393")"#).unwrap(), "1");
    }
}
