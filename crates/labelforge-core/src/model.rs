//! The label document model. Saved to disk as JSON (`.lfl` files).
//!
//! All positions and sizes are in millimetres. Conversion to printer dots
//! happens only at print time, so one label prints the same at any dpi.

use serde::{Deserialize, Serialize};

/// Current `.lfl` schema version. Bump when the format changes and add a
/// migration in [`Label::from_json`].
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Label {
    pub schema: u32,
    pub name: String,
    pub width_mm: f64,
    pub height_mm: f64,
    /// Corner radius of the physical label, for the on-screen outline only.
    pub corner_radius_mm: f64,
    /// Printer resolution the label is designed for (used for previews).
    pub dpi: u32,
    pub objects: Vec<Object>,
    pub variables: Vec<Variable>,
}

impl Default for Label {
    fn default() -> Self {
        Label {
            schema: SCHEMA_VERSION,
            name: "Untitled".into(),
            width_mm: 100.0,
            height_mm: 50.0,
            corner_radius_mm: 2.0,
            dpi: 600,
            objects: Vec::new(),
            variables: Vec::new(),
        }
    }
}

impl Label {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("label serialises")
    }

    pub fn from_json(s: &str) -> Result<Label, String> {
        let label: Label = serde_json::from_str(s).map_err(|e| format!("Not a valid LabelForge file: {e}"))?;
        if label.schema > SCHEMA_VERSION {
            return Err(format!(
                "This label was made with a newer LabelForge (format {}). Please update.",
                label.schema
            ));
        }
        Ok(label)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Object {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub w_mm: f64,
    pub h_mm: f64,
    /// Rotation in degrees, clockwise, about the object's centre.
    pub rotation: f64,
    pub locked: bool,
    pub kind: Kind,
}

impl Default for Object {
    fn default() -> Self {
        Object {
            id: String::new(),
            x_mm: 5.0,
            y_mm: 5.0,
            w_mm: 30.0,
            h_mm: 10.0,
            rotation: 0.0,
            locked: false,
            kind: Kind::Text(Text::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    Text(Text),
    Rect(Shape),
    Ellipse(Shape),
    /// A line from the object's top-left corner to its bottom-right corner.
    Line(Shape),
    Barcode(Barcode),
    /// An embedded image, stored as a `data:` URI so the label file is self-contained.
    Image { data_uri: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Text {
    /// The text. `{name}` is replaced by the value of variable `name`.
    pub text: String,
    pub font: String,
    pub size_pt: f64,
    pub bold: bool,
    pub italic: bool,
    pub align: Align,
    pub color: String,
    /// White text on a filled box.
    pub inverse: bool,
}

impl Default for Text {
    fn default() -> Self {
        Text {
            text: "Text".into(),
            font: "Helvetica".into(),
            size_pt: 12.0,
            bold: false,
            italic: false,
            align: Align::Left,
            color: "#000000".into(),
            inverse: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Shape {
    pub stroke_mm: f64,
    pub stroke: String,
    /// Fill colour, or empty for no fill.
    pub fill: String,
    /// Corner radius (rectangles only).
    pub radius_mm: f64,
}

impl Default for Shape {
    fn default() -> Self {
        Shape { stroke_mm: 0.3, stroke: "#000000".into(), fill: String::new(), radius_mm: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Barcode {
    /// Symbology key, e.g. `code128`, `qr`, `gs1_datamatrix`. The full list
    /// with names and example data is [`crate::barcode::SYMBOLOGIES`].
    pub symbology: String,
    /// Data to encode. `{name}` is replaced by the value of variable `name`.
    /// GS1 types take Application Identifiers in brackets, e.g.
    /// `(01)09501101530003(17)261231(10)LOT42`.
    pub data: String,
    /// Print the human-readable text (1D codes).
    pub show_text: bool,
    /// Kept for files from 0.1; the text size now follows the barcode standard.
    pub text_size_pt: f64,
    /// Error correction: QR/Micro QR `L` `M` `Q` `H`; Aztec `1`-`4`;
    /// PDF417 security level `0`-`8`. Empty = automatic.
    pub qr_ecc: String,
    /// PDF417/MicroPDF417 data columns, 0 = automatic.
    pub columns: i32,
    /// MaxiCode mode 2-6 (0 = automatic, mode 4).
    pub maxi_mode: i32,
    /// MaxiCode modes 2/3: postcode, country and service class, e.g. `152382802840001`.
    pub maxi_primary: String,
    /// Include the standard quiet zones inside the object box.
    pub quiet_zone: bool,
    pub color: String,
}

impl Default for Barcode {
    fn default() -> Self {
        Barcode {
            symbology: "code128".into(),
            data: "LF-0001".into(),
            show_text: true,
            text_size_pt: 8.0,
            qr_ecc: "M".into(),
            columns: 0,
            maxi_mode: 0,
            maxi_primary: String::new(),
            quiet_zone: false,
            color: "#000000".into(),
        }
    }
}

/// A named value used in text and barcode data as `{name}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    pub name: String,
    #[serde(flatten)]
    pub source: VarSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum VarSource {
    /// A fixed value.
    Fixed { value: String },
    /// Asked for when printing.
    Prompt {
        #[serde(default)]
        prompt: String,
        #[serde(default)]
        default: String,
    },
    /// Generated from a pattern; see [`crate::pattern`].
    Serial {
        pattern: String,
        #[serde(default = "one")]
        start: i64,
        #[serde(default = "one")]
        step: i64,
    },
    /// Current date/time from a pattern such as `{DD}/{MM}/{YYYY}`.
    Date { pattern: String },
}

fn one() -> i64 {
    1
}
