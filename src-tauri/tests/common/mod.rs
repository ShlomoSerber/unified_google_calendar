//! Shared helpers for integration tests: fixture loading and a wiremock server.
#![allow(dead_code)]

use std::path::PathBuf;

use wiremock::MockServer;

pub fn fixture(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/google")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub async fn server() -> MockServer {
    MockServer::start().await
}
