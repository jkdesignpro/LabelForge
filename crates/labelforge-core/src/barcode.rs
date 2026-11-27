//! Barcode and 2D code encoding into module patterns.
//!
//! Encoders come from well-tested open-source crates (`barcoders`, `qrcode`,
//! `datamatrix`); this module only adapts their output to one shape.
//! Every symbology here is checked by decoding rendered output with an
//! independent decoder (zxing-cpp) before release; see docs/TESTING.md.
//! Codabar is left out until we have a verified encoder: the `barcoders`
//! 2.0 Codabar tables swap the B and C start/stop characters.

use crate::model::Symbology;

/// Encoded symbol, without quiet zone.
pub enum Encoded {
    /// 1D: one entry per module, `true` = bar.
    Linear { modules: Vec<bool>, text: String },
    /// 2D: square or rectangular matrix, row-major, `true` = dark.
    Matrix { width: usize, height: usize, cells: Vec<bool> },
}

pub fn encode(sym: Symbology, data: &str, qr_ecc: &str) -> Result<Encoded, String> {
    use barcoders::sym::*;
    let err = |e: barcoders::error::Error| format!("{} cannot encode '{}': {}", name(sym), data, e);
    let lin = |m: Vec<u8>, text: &str| Encoded::Linear { modules: m.into_iter().map(|b| b == 1).collect(), text: text.to_string() };
    if data.is_empty() {
        return Err("Barcode data is empty".into());
    }
    Ok(match sym {
        Symbology::Code128 => {
            lin(code128::Code128::new(code128_auto(data)?).map_err(err)?.encode(), data)
        }
        Symbology::Code39 => lin(code39::Code39::new(data.to_ascii_uppercase()).map_err(err)?.encode(), &data.to_ascii_uppercase()),
        Symbology::Code93 => lin(code93::Code93::new(data.to_ascii_uppercase()).map_err(err)?.encode(), &data.to_ascii_uppercase()),
        Symbology::Ean13 => {
            let digits = only_digits(data, &[12, 13])?;
            let base = &digits[..12];
            let check = crate::pattern::mod10(base)?;
            if digits.len() == 13 && digits[12..] != check.to_string() {
                return Err(format!("EAN-13 check digit should be {check}"));
            }
            lin(ean13::EAN13::new(base).map_err(err)?.encode(), &format!("{base}{check}"))
        }
        Symbology::Ean8 => {
            let digits = only_digits(data, &[7, 8])?;
            let base = &digits[..7];
            let check = crate::pattern::mod10(base)?;
            if digits.len() == 8 && digits[7..] != check.to_string() {
                return Err(format!("EAN-8 check digit should be {check}"));
            }
            lin(ean8::EAN8::new(base).map_err(err)?.encode(), &format!("{base}{check}"))
        }
        Symbology::Itf => {
            let digits = only_digits(data, &[])?;
            if digits.len() % 2 != 0 {
                return Err("Interleaved 2 of 5 needs an even number of digits".into());
            }
            lin(tf::TF::interleaved(&digits).map_err(err)?.encode(), &digits)
        }
        Symbology::Qr => {
            use qrcode::{EcLevel, QrCode};
            let lvl = match qr_ecc.to_ascii_uppercase().as_str() {
                "L" => EcLevel::L,
                "Q" => EcLevel::Q,
                "H" => EcLevel::H,
                _ => EcLevel::M,
            };
            let q = QrCode::with_error_correction_level(data.as_bytes(), lvl).map_err(|e| format!("QR: {e}"))?;
            let w = q.width();
            let cells = q.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect();
            Encoded::Matrix { width: w, height: w, cells }
        }
        Symbology::DataMatrix => {
            let dm = datamatrix::DataMatrix::encode_str(data, datamatrix::SymbolList::default())
                .or_else(|_| datamatrix::DataMatrix::encode(data.as_bytes(), datamatrix::SymbolList::default()))
                .map_err(|e| format!("Data Matrix: {e:?}"))?;
            let bm = dm.bitmap();
            let (w, h) = (bm.width(), bm.height());
            let mut cells = vec![false; w * h];
            for (x, y) in bm.pixels() {
                cells[y * w + x] = true;
            }
            Encoded::Matrix { width: w, height: h, cells }
        }
    })
}

pub fn name(sym: Symbology) -> &'static str {
    match sym {
        Symbology::Code128 => "Code 128",
        Symbology::Code39 => "Code 39",
        Symbology::Code93 => "Code 93",
        Symbology::Ean13 => "EAN-13",
        Symbology::Ean8 => "EAN-8",
        Symbology::Itf => "Interleaved 2 of 5",
        Symbology::Qr => "QR Code",
        Symbology::DataMatrix => "Data Matrix",
    }
}

fn only_digits(data: &str, lens: &[usize]) -> Result<String, String> {
    if !data.chars().all(|c| c.is_ascii_digit()) {
        return Err("Only digits are allowed".into());
    }
    if !lens.is_empty() && !lens.contains(&data.len()) {
        return Err(format!("Needs {} digits", lens.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" or ")));
    }
    Ok(data.to_string())
}

/// Pick Code 128 character sets automatically: set C for runs of 4+ digits
/// (two digits per symbol, shortest barcode), set B for everything else.
fn code128_auto(data: &str) -> Result<String, String> {
    const A: char = '\u{00C0}';
    const B: char = '\u{0181}';
    const C: char = '\u{0106}';
    if let Some(bad) = data.chars().find(|c| !(' '..='~').contains(c)) {
        return Err(format!("Code 128 cannot encode '{bad}' (printable ASCII only for now)"));
    }
    let _ = A;
    let chars: Vec<char> = data.chars().collect();
    let mut out = String::new();
    let mut set = ' ';
    let mut i = 0;
    while i < chars.len() {
        let run = chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
        // Use C for an even-length digit run of 4+ (or the whole data if it is 2+ digits).
        let whole = i == 0 && run == chars.len() && run >= 2;
        if (run >= 4 || whole) && run >= 2 {
            let even = run - run % 2;
            if set != C {
                out.push(C);
                set = C;
            }
            out.extend(&chars[i..i + even]);
            i += even;
            continue;
        }
        if set != B {
            out.push(B);
            set = B;
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear(sym: Symbology, d: &str) -> (Vec<bool>, String) {
        match encode(sym, d, "M").unwrap() {
            Encoded::Linear { modules, text } => (modules, text),
            _ => panic!(),
        }
    }

    #[test]
    fn code128_sets() {
        assert_eq!(code128_auto("123456").unwrap(), "\u{0106}123456");
        assert_eq!(code128_auto("LF-0001").unwrap(), "\u{0181}LF-\u{0106}0001");
        assert_eq!(code128_auto("A1").unwrap(), "\u{0181}A1");
        assert_eq!(code128_auto("12345").unwrap(), "\u{0106}1234\u{0181}5");
        // Code 128 ends with the stop pattern 2331112 = 11 + 2 bar modules.
        let (m, _) = linear(Symbology::Code128, "LF-0001");
        assert!(m[m.len() - 2..].iter().all(|b| *b));
        assert!(m[0]);
    }

    #[test]
    fn ean13_check_digit() {
        let (m, text) = linear(Symbology::Ean13, "400638133393");
        assert_eq!(text, "4006381333931");
        assert_eq!(m.len(), 95); // EAN-13 is always 95 modules
        assert!(encode(Symbology::Ean13, "4006381333932", "M").is_err());
    }

    #[test]
    fn matrices() {
        match encode(Symbology::Qr, "https://example.com", "M").unwrap() {
            Encoded::Matrix { width, height, cells } => {
                assert_eq!(width, height);
                assert_eq!(cells.len(), width * width);
                assert!(cells[0]); // finder pattern corner is dark
            }
            _ => panic!(),
        }
        match encode(Symbology::DataMatrix, "LF-0001", "M").unwrap() {
            Encoded::Matrix { width, height, cells } => {
                assert_eq!(cells.len(), width * height);
                // Solid L finder: left column all dark.
                assert!((0..height).all(|y| cells[y * width]));
            }
            _ => panic!(),
        }
    }
}
