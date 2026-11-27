//! Barcode and 2D code encoding.
//!
//! All symbologies are encoded by Zint (vendored in `vendor/zint`, BSD-3),
//! a long-established, standards-tested barcode library. A small C shim
//! (`vendor/lf_zint.c`) turns Zint's vector output into an SVG fragment.
//! Every symbology is checked by decoding rendered output with an
//! independent decoder (zxing-cpp) before release; see docs/TESTING.md.

use std::ffi::c_char;

/// How data for a symbology is interpreted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Input {
    /// Text (UTF-8).
    PlainText,
    /// GS1 element string with AIs in brackets: `(01)…(10)…`.
    Gs1Ais,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Linear,
    Retail,
    Gs1,
    TwoD,
    Stacked,
    Postal,
    Hibc,
    Other,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Sym {
    pub key: &'static str,
    pub name: &'static str,
    #[serde(skip)]
    pub zint: i32,
    #[serde(skip)]
    pub input: Input,
    pub group: Group,
    /// Example data that encodes correctly; used for new objects and tests.
    pub example: &'static str,
}

const fn s(key: &'static str, name: &'static str, zint: i32, input: Input, group: Group, example: &'static str) -> Sym {
    Sym { key, name, zint, input, group, example }
}
use Group::*;
use Input::*;

/// Every symbology LabelForge offers. Keys are stored in label files, so
/// never rename one; add new ones instead.
pub const SYMBOLOGIES: &[Sym] = &[
    // Linear
    s("code128", "Code 128", 20, PlainText, Linear, "LF-0001"),
    s("code39", "Code 39", 8, PlainText, Linear, "PART-42"),
    s("code39_ext", "Code 39 Extended (full ASCII)", 9, PlainText, Linear, "Part-42a"),
    s("code93", "Code 93", 25, PlainText, Linear, "PART42"),
    s("itf", "Interleaved 2 of 5", 3, PlainText, Linear, "12345678"),
    s("codabar", "Codabar", 18, PlainText, Linear, "A40156B"),
    s("code11", "Code 11", 1, PlainText, Linear, "123-45"),
    s("msi", "MSI Plessey", 47, PlainText, Linear, "1234567"),
    s("logmars", "LOGMARS", 50, PlainText, Linear, "LOGMARS42"),
    s("telepen", "Telepen", 32, PlainText, Linear, "Telepen"),
    s("pharma", "Pharmacode", 51, PlainText, Linear, "1234"),
    // Retail
    s("ean13", "EAN-13", 13, PlainText, Retail, "400638133393"),
    s("ean8", "EAN-8", 13, PlainText, Retail, "9638507"),
    s("upca", "UPC-A", 34, PlainText, Retail, "03600029145"),
    s("upce", "UPC-E", 37, PlainText, Retail, "0123456"),
    s("isbn", "ISBN (EAN-13)", 69, PlainText, Retail, "9781234567897"),
    // GS1
    s("gs1_128", "GS1-128", 16, Gs1Ais, Gs1, "(01)09501101530003(17)261231(10)LOT42"),
    s("itf14", "ITF-14", 89, PlainText, Gs1, "0950110153000"),
    s("ean14", "EAN-14", 72, PlainText, Gs1, "0950110153000"),
    s("sscc", "SSCC-18 (NVE-18)", 75, PlainText, Gs1, "37612345000001003"),
    s("databar", "GS1 DataBar Omnidirectional", 29, PlainText, Gs1, "0950110153000"),
    s("databar_stacked", "GS1 DataBar Stacked Omni", 80, PlainText, Gs1, "0950110153000"),
    s("databar_limited", "GS1 DataBar Limited", 30, PlainText, Gs1, "0950110153000"),
    s("databar_expanded", "GS1 DataBar Expanded", 31, Gs1Ais, Gs1, "(01)09501101530003(17)261231(10)LOT42"),
    s("databar_expanded_stacked", "GS1 DataBar Expanded Stacked", 81, Gs1Ais, Gs1, "(01)09501101530003(17)261231(10)LOT42"),
    s("gs1_datamatrix", "GS1 Data Matrix", 71, Gs1Ais, Gs1, "(01)09501101530003(17)261231(10)LOT42"),
    s("gs1_qr", "GS1 QR Code", 58, Gs1Ais, Gs1, "(01)09501101530003(17)261231(10)LOT42"),
    // 2D
    s("qr", "QR Code", 58, PlainText, TwoD, "https://example.com"),
    s("micro_qr", "Micro QR Code", 97, PlainText, TwoD, "LF0001"),
    s("rmqr", "Rectangular Micro QR (rMQR)", 145, PlainText, TwoD, "LF-0001"),
    s("datamatrix", "Data Matrix", 71, PlainText, TwoD, "LF-0001"),
    s("aztec", "Aztec Code", 92, PlainText, TwoD, "LF-0001 303SS"),
    s("pdf417", "PDF417", 55, PlainText, Stacked, "LabelForge PDF417"),
    s("pdf417_compact", "Compact PDF417", 56, PlainText, Stacked, "LabelForge"),
    s("micro_pdf417", "MicroPDF417", 84, PlainText, Stacked, "LabelForge"),
    s("maxicode", "MaxiCode", 57, PlainText, TwoD, "LabelForge MaxiCode"),
    s("dotcode", "DotCode", 115, PlainText, TwoD, "LF-0001"),
    s("hanxin", "Han Xin", 116, PlainText, TwoD, "LF-0001"),
    s("codablock_f", "Codablock-F", 74, PlainText, Stacked, "LabelForge"),
    s("code16k", "Code 16K", 23, PlainText, Stacked, "LabelForge"),
    // HIBC (health industry)
    s("hibc_128", "HIBC Code 128", 98, PlainText, Hibc, "A123BJC5D6E71"),
    s("hibc_39", "HIBC Code 39", 99, PlainText, Hibc, "A123BJC5D6E71"),
    s("hibc_datamatrix", "HIBC Data Matrix", 102, PlainText, Hibc, "A123BJC5D6E71"),
    s("hibc_qr", "HIBC QR Code", 104, PlainText, Hibc, "A123BJC5D6E71"),
    s("hibc_pdf417", "HIBC PDF417", 106, PlainText, Hibc, "A123BJC5D6E71"),
    s("hibc_aztec", "HIBC Aztec Code", 112, PlainText, Hibc, "A123BJC5D6E71"),
    // Postal
    s("postnet", "USPS POSTNET", 40, PlainText, Postal, "12345678901"),
    s("planet", "USPS PLANET", 82, PlainText, Postal, "12345678901"),
    s("usps_imail", "USPS Intelligent Mail", 85, PlainText, Postal, "01234567094987654321"),
    s("rm4scc", "Royal Mail 4-State (RM4SCC)", 70, PlainText, Postal, "W1J0TR01"),
    s("mailmark", "Royal Mail Mailmark 4-State", 121, PlainText, Postal, "11210012341234567AB19XY1A"),
    s("kix", "Dutch KIX", 90, PlainText, Postal, "2500GG30250"),
    s("auspost", "Australia Post", 63, PlainText, Postal, "96184209"),
    s("japanpost", "Japan Post", 76, PlainText, Postal, "15400233-16-4-205"),
    s("upu_s10", "UPU S10", 120, PlainText, Postal, "EE876543216CA"),
    s("dpd", "DPD", 96, PlainText, Postal, "%000393206219912345678101040"),
];

pub fn lookup(key: &str) -> Option<&'static Sym> {
    SYMBOLOGIES.iter().find(|s| s.key == key)
}

pub fn name(key: &str) -> &str {
    lookup(key).map(|s| s.name).unwrap_or(key)
}

/// Zint's vector output as an SVG fragment in Zint units. The shim sets
/// Zint's scale so that 1 unit = 1 module (X-dimension).
pub struct ZintSvg {
    pub fragment: String,
    pub width: f64,
    pub height: f64,
    pub modules: i32,
    pub rows: i32,
    /// Bar height used, in modules (adjustable for linear codes).
    pub bar_height: f64,
    /// Zint warning text, e.g. "non-compliant", empty if none.
    pub warning: String,
    /// The symbol keeps a fixed width:height ratio (2D codes).
    pub fixed_ratio: bool,
}

extern "C" {
    fn lf_zint_svg(
        symbology: i32, data: *const u8, len: i32, input_mode: i32, option_1: i32, option_2: i32, option_3: i32,
        height_x: f32, show_hrt: i32, quiet: i32, primary: *const c_char, out: *mut c_char, outcap: i32, vw: *mut f32,
        vh: *mut f32, modules: *mut i32, rows: *mut i32, hx: *mut f32, err: *mut c_char, errcap: i32,
    ) -> i32;
    fn ZBarcode_Cap(symbol_id: i32, cap_flag: u32) -> u32;
}

const UNICODE_MODE: i32 = 1;
const GS1_MODE: i32 = 2;
const GS1PARENS_MODE: i32 = 0x10;
const CAP_FIXED_RATIO: u32 = 0x0100;

pub struct Options<'a> {
    pub show_text: bool,
    pub ecc: &'a str,
    pub columns: i32,
    pub maxi_mode: i32,
    pub maxi_primary: &'a str,
    /// Linear codes: bar height in modules, 0 = Zint default.
    pub bar_height: f64,
    /// Add the standard quiet zones inside the box (always on for ITF-14,
    /// whose bearer box needs them).
    pub quiet_zone: bool,
}

pub fn encode(key: &str, data: &str, o: &Options) -> Result<ZintSvg, String> {
    let sym = lookup(key).ok_or_else(|| format!("Unknown barcode type '{key}'"))?;
    if data.is_empty() {
        return Err("Barcode data is empty".into());
    }
    let mut data = data.to_string();
    let mut zint = sym.zint;
    // EAN-13/8 and UPC take the digits without check digit; with it, Zint verifies it.
    match key {
        "ean13" | "ean8" => {
            let base = if key == "ean13" { 12 } else { 7 };
            if !data.chars().all(|c| c.is_ascii_digit()) || (data.len() != base && data.len() != base + 1) {
                return Err(format!("{} needs {} digits (or {} with check digit)", sym.name, base, base + 1));
            }
            if data.len() == base + 1 {
                zint = 14; // EANX_CHK
            }
        }
        "upca" if data.len() == 12 => zint = 35,
        "upce" if data.len() == 8 => zint = 38,
        _ => {}
    }
    let input_mode = match sym.input {
        PlainText => UNICODE_MODE,
        Gs1Ais => GS1_MODE | GS1PARENS_MODE,
    };
    let ecc = o.ecc.trim().to_ascii_uppercase();
    let (mut o1, mut o2, mut o3) = (-1, 0, 0);
    match key {
        "qr" | "gs1_qr" | "micro_qr" | "hibc_qr" | "rmqr" => {
            o1 = match ecc.as_str() { "L" => 1, "M" => 2, "Q" => 3, "H" => 4, _ => -1 };
            if key == "rmqr" && !(o1 == 2 || o1 == 4) {
                o1 = 2; // rMQR supports M and H only
            }
        }
        "aztec" | "hibc_aztec" => o1 = ecc.parse::<i32>().ok().filter(|v| (1..=4).contains(v)).unwrap_or(-1),
        "pdf417" | "pdf417_compact" | "hibc_pdf417" => {
            o1 = ecc.parse::<i32>().ok().filter(|v| (0..=8).contains(v)).unwrap_or(-1);
            o2 = o.columns.clamp(0, 30);
        }
        "micro_pdf417" => o2 = o.columns.clamp(0, 4),
        "maxicode" => {
            o1 = if (2..=6).contains(&o.maxi_mode) { o.maxi_mode } else { -1 };
        }
        "datamatrix" | "gs1_datamatrix" | "hibc_datamatrix" => o3 = 100, // DM_SQUARE: square symbols only
        _ => {}
    }
    if key == "codabar" {
        data = data.to_ascii_uppercase();
    }
    let primary = std::ffi::CString::new(o.maxi_primary).map_err(|_| "Invalid MaxiCode primary data")?;
    let mut out = vec![0u8; 4 << 20];
    let mut err = vec![0u8; 256];
    let (mut vw, mut vh, mut modules, mut rows, mut hx) = (0f32, 0f32, 0i32, 0i32, 0f32);
    let rc = unsafe {
        lf_zint_svg(
            zint, data.as_ptr(), data.len() as i32, input_mode, o1, o2, o3, o.bar_height as f32,
            o.show_text as i32, (o.quiet_zone || key == "itf14") as i32, primary.as_ptr(), out.as_mut_ptr() as *mut c_char, out.len() as i32,
            &mut vw, &mut vh, &mut modules, &mut rows, &mut hx, err.as_mut_ptr() as *mut c_char, err.len() as i32,
        )
    };
    let cstr = |b: &[u8]| String::from_utf8_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())]).into_owned();
    let msg = cstr(&err);
    if rc >= 5 {
        return Err(format!("{}: {}", sym.name, clean(&msg)));
    }
    Ok(ZintSvg {
        fragment: cstr(&out),
        width: vw as f64,
        height: vh as f64,
        modules,
        rows,
        bar_height: hx as f64,
        warning: clean(&msg),
        fixed_ratio: unsafe { ZBarcode_Cap(zint, CAP_FIXED_RATIO) } != 0,
    })
}

/// Zint messages start with "Error 123: "; keep the plain text.
fn clean(m: &str) -> String {
    match m.split_once(": ") {
        Some((head, rest)) if head.starts_with("Error") || head.starts_with("Warning") => rest.to_string(),
        _ => m.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options<'static> {
        Options { show_text: true, ecc: "", columns: 0, maxi_mode: 0, maxi_primary: "", bar_height: 0.0, quiet_zone: false }
    }

    #[test]
    fn every_example_encodes() {
        for s in SYMBOLOGIES {
            let r = encode(s.key, s.example, &opts()).unwrap_or_else(|e| panic!("{}: {e}", s.key));
            assert!(r.modules > 0 && r.width > 0.0 && r.height > 0.0, "{}", s.key);
            assert!(r.fragment.contains("<path"), "{}", s.key);
        }
    }

    #[test]
    fn keys_unique() {
        let mut k: Vec<_> = SYMBOLOGIES.iter().map(|s| s.key).collect();
        k.sort();
        k.dedup();
        assert_eq!(k.len(), SYMBOLOGIES.len());
    }

    #[test]
    fn checks_and_errors() {
        assert!(encode("ean13", "4006381333931", &opts()).is_ok());
        assert!(encode("ean13", "4006381333932", &opts()).is_err());
        assert!(encode("ean13", "abc", &opts()).is_err());
        assert!(encode("gs1_128", "(01)12345", &opts()).is_err());
        assert!(encode("nope", "1", &opts()).is_err());
        let r = encode("qr", "x", &opts()).unwrap();
        assert!(r.fixed_ratio);
        let r = encode("code128", "x", &opts()).unwrap();
        assert!(!r.fixed_ratio);
        // 1 unit = 1 module: Code 128 is 11 modules per character + 13 for stop.
        assert_eq!(r.width, r.modules as f64);
    }

    #[test]
    fn linear_height_is_adjustable() {
        let a = encode("code128", "LF-0001", &Options { bar_height: 20.0, ..opts() }).unwrap();
        let b = encode("code128", "LF-0001", &Options { bar_height: 40.0, ..opts() }).unwrap();
        assert!((b.height - a.height - 20.0).abs() < 0.5, "{} {}", a.height, b.height);
    }
}
