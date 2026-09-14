//! Development-only helpers for the pixel-fidelity loop (docs/04 section 3, steps 8 to 10).
//! `dev_dump` writes a `dumpRegion` JSON produced inside the webview to
//! `$UGC_MEASURE_DIR/<name>-app.json`. Compiled to a stub in release builds.

use crate::error::AppError;

#[cfg(debug_assertions)]
pub fn write_dump(name: &str, json: &str) -> Result<String, AppError> {
    let dir = std::env::var("UGC_MEASURE_DIR")
        .map_err(|_| AppError::invalid("UGC_MEASURE_DIR is not set"))?;
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return Err(AppError::invalid("Bad dump name"));
    }
    let path = std::path::Path::new(&dir).join(format!("{name}-app.json"));
    std::fs::write(&path, json)?;
    Ok(path.display().to_string())
}

#[cfg(not(debug_assertions))]
pub fn write_dump(_name: &str, _json: &str) -> Result<String, AppError> {
    Err(AppError::invalid("Not available in release builds"))
}
