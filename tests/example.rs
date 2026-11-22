#![allow(clippy::unwrap_used)]

use std::process::Command;

#[test]
fn bundled_documents_match_the_model_and_current_source() {
    let result = Command::new(env!("CARGO_BIN_EXE_cargo-c4doc"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "render",
            "examples/c4doc.json",
            "--root",
            ".",
            "--output",
            "examples/docs",
            "--check",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
