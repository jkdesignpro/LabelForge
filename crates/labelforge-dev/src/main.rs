//! Development server: serves `ui/` and the render API on http://127.0.0.1:8765
//! so the designer can be tested in a browser. Not used by the desktop app.
//! Run with `cargo run -p labelforge-dev`.

use labelforge_core::api;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui");
    let addr = std::env::var("LABELFORGE_DEV_ADDR").unwrap_or_else(|_| "127.0.0.1:8765".into());
    let listener = TcpListener::bind(&addr).expect("bind");
    println!("LabelForge dev server on http://{addr}/");
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        std::thread::spawn(move || {
            let _ = handle(stream, &root);
        });
    }
}

fn handle(mut s: TcpStream, root: &Path) -> std::io::Result<()> {
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/").to_string());
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        r.read_line(&mut h)?;
        if h.trim().is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;

    let (status, ctype, out): (&str, &str, Vec<u8>) = if method == "POST" && path.starts_with("/api/") {
        let req: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        let res: Result<serde_json::Value, String> = match &path[5..] {
            "render" => serde_json::from_value(req).map_err(|e| e.to_string()).map(|r| serde_json::to_value(api::render(r)).unwrap()),
            "render_run" => serde_json::from_value(req)
                .map_err(|e| e.to_string())
                .and_then(api::render_run)
                .map(|r| serde_json::to_value(r).unwrap()),
            "example_label" => Ok(serde_json::Value::String(include_str!("../../../examples/part-tag.lfl").into())),
            _ => Err("unknown command".into()),
        };
        match res {
            Ok(v) => ("200 OK", "application/json", v.to_string().into_bytes()),
            Err(e) => ("400 Bad Request", "text/plain", e.into_bytes()),
        }
    } else {
        let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
        let rel = rel.split('?').next().unwrap_or("");
        let file = root.join(rel);
        if rel.contains("..") || !file.is_file() {
            ("404 Not Found", "text/plain", b"not found".to_vec())
        } else {
            let ct = match file.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "text/javascript",
                Some("css") => "text/css",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };
            ("200 OK", ct, std::fs::read(file)?)
        }
    };
    write!(s, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", out.len())?;
    s.write_all(&out)
}
