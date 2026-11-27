//! Global counters shared between labels, print runs and computers.
//!
//! Each counter is a small text file `<name>.counter` in a counter folder,
//! holding the next free number. Put the folder on a network share and every
//! PC printing from it draws from the same sequence, so serials never repeat.
//! A lock file guards each update; a lock older than 30 s is treated as left
//! behind by a crashed program and removed.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

fn valid(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) || name.starts_with('.') {
        return Err(format!("Counter name '{name}' may only use letters, digits, - _ ."));
    }
    Ok(())
}

fn file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.counter"))
}

struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn lock(dir: &Path, name: &str) -> Result<Lock, String> {
    let p = dir.join(format!("{name}.lock"));
    let start = Instant::now();
    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(&p) {
            Ok(mut f) => {
                let _ = write!(f, "{}", std::process::id());
                return Ok(Lock(p));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let stale = fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .map(|t| SystemTime::now().duration_since(t).unwrap_or_default() > Duration::from_secs(30))
                    .unwrap_or(false);
                if stale {
                    let _ = fs::remove_file(&p);
                    continue;
                }
                if start.elapsed() > Duration::from_secs(10) {
                    return Err(format!("Counter '{name}' is busy (locked by another print). Try again."));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Cannot lock counter '{name}' in {}: {e}", dir.display())),
        }
    }
}

fn read(dir: &Path, name: &str) -> Result<i64, String> {
    match fs::read_to_string(file(dir, name)) {
        Ok(s) => s.lines().next().unwrap_or("").trim().parse().map_err(|_| format!("Counter file for '{name}' is damaged")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(1),
        Err(e) => Err(format!("Cannot read counter '{name}': {e}")),
    }
}

fn write_value(dir: &Path, name: &str, v: i64) -> Result<(), String> {
    // Write to a temp file then rename, so a crash never leaves half a number.
    let tmp = dir.join(format!("{name}.counter.tmp"));
    fs::write(&tmp, format!("{v}\n")).map_err(|e| format!("Cannot write counter '{name}': {e}"))?;
    fs::rename(&tmp, file(dir, name)).map_err(|e| format!("Cannot write counter '{name}': {e}"))
}

/// The next number that will be handed out (1 for a new counter).
pub fn peek(dir: &Path, name: &str) -> Result<i64, String> {
    valid(name)?;
    read(dir, name)
}

/// Reserve `amount` numbers and return the first one.
pub fn reserve(dir: &Path, name: &str, amount: i64) -> Result<i64, String> {
    valid(name)?;
    if amount < 1 {
        return Err("Reserve at least one number".into());
    }
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create counter folder {}: {e}", dir.display()))?;
    let _l = lock(dir, name)?;
    let start = read(dir, name)?;
    write_value(dir, name, start + amount)?;
    Ok(start)
}

/// Set the next number (e.g. to start a new year at 1).
pub fn set(dir: &Path, name: &str, next: i64) -> Result<(), String> {
    valid(name)?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let _l = lock(dir, name)?;
    write_value(dir, name, next)
}

/// All counters in a folder with their next numbers.
pub fn list(dir: &Path) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(name) = n.strip_suffix(".counter") {
                if let Ok(v) = read(dir, name) {
                    out.push((name.to_string(), v));
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_is_unique_across_threads() {
        let dir = std::env::temp_dir().join(format!("lf-counters-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(peek(&dir, "serial").unwrap(), 1);
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let d = dir.clone();
                std::thread::spawn(move || (0..10).map(|_| reserve(&d, "serial", 3).unwrap()).collect::<Vec<_>>())
            })
            .collect();
        let mut all: Vec<i64> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
        all.sort();
        let expect: Vec<i64> = (0..80).map(|i| 1 + i * 3).collect();
        assert_eq!(all, expect);
        assert_eq!(peek(&dir, "serial").unwrap(), 241);
        set(&dir, "serial", 1000).unwrap();
        assert_eq!(reserve(&dir, "serial", 1).unwrap(), 1000);
        assert_eq!(list(&dir), vec![("serial".to_string(), 1001)]);
        assert!(reserve(&dir, "../x", 1).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
