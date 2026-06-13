//! Invoke rustc on probe crates (spec §7.6, §17.3).

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustcError {
    #[error("failed to run rustc: {0}")]
    Io(#[from] std::io::Error),
    #[error("rustc not found on PATH")]
    NotFound,
    #[error("rustc failed: {summary}")]
    CheckFailed { summary: String, stderr: String },
}

pub fn is_borrow_check_failure(stderr: &str) -> bool {
    stderr.contains("borrow")
        || stderr.contains("E0382")
        || stderr.contains("E0502")
        || stderr.contains("E0505")
        || stderr.contains("E0597")
}

pub fn check_rust_source(source: &str) -> Result<(), RustcError> {
    let dir = TempDir::new()?;
    let lib_path = dir.path().join("lib.rs");
    std::fs::write(&lib_path, source)?;

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("lib")
        .arg(&lib_path)
        .current_dir(dir.path())
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let summary = stderr
                .lines()
                .find(|l| l.contains("error"))
                .unwrap_or("borrow-check failed")
                .to_string();
            Err(RustcError::CheckFailed { summary, stderr })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(RustcError::NotFound),
        Err(e) => Err(RustcError::Io(e)),
    }
}

pub fn write_probe_to(path: &Path, source: &str) -> Result<(), RustcError> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(source.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_rust_compiles() {
        let src = "pub fn ok() {}";
        if let Err(RustcError::NotFound) = check_rust_source(src) {
            return;
        }
        check_rust_source(src).expect("valid rust should compile");
    }

    #[test]
    fn use_after_move_fails_borrow_check() {
        let src = r#"
pub fn forward(msg: String) {
    let x = msg;
    println!("{}", x);
    println!("{}", msg);
}
"#;
        if let Err(RustcError::NotFound) = check_rust_source(src) {
            return;
        }
        let err = check_rust_source(src).expect_err("use after move");
        assert!(is_borrow_check_failure(&err.to_string()));
    }
}
