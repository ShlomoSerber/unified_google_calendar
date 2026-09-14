//! Search Console verification file. See docs/05-sincronizacion.md section 3.1 and
//! docs/09-setup-usuario.md section C. Served only when the user placed
//! `google<token>.html` under `~/.config/unified-google-calendar/verify/`.

use std::path::{Path, PathBuf};

/// The file to serve for `GET /google<token>.html`, if it exists and the name is safe.
pub fn resolve(verify_dir: &Path, request_path: &str) -> Option<PathBuf> {
    let name = request_path.strip_prefix('/')?;
    let valid = name.starts_with("google")
        && name.ends_with(".html")
        && name.len() > "google.html".len()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.');
    if !valid {
        return None;
    }
    let path = verify_dir.join(name);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_serves_well_formed_names_that_exist() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("googleabc123.html"),
            "google-site-verification: googleabc123.html",
        )
        .unwrap();
        assert!(resolve(dir.path(), "/googleabc123.html").is_some());
        assert!(resolve(dir.path(), "/googlemissing.html").is_none());
        assert!(resolve(dir.path(), "/google.html").is_none());
        assert!(resolve(dir.path(), "/../etc/passwd").is_none());
        assert!(resolve(dir.path(), "/google../x.html").is_none());
        assert!(resolve(dir.path(), "/other.html").is_none());
    }
}
