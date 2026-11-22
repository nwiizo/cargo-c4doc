#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn analyze(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-c4doc"))
        .args(["analyze", path.to_str().unwrap()])
        .output()
        .unwrap()
}

#[test]
fn cargo_targets_modules_interfaces_and_calls_have_source_locations() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("src/nested")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        "mod api;\nmod nested { pub mod service; }\nfn main() { api::serve(); }\n",
    )
    .unwrap();
    fs::write(root.path().join("src/api.rs"), "use std::path::Path;\npub struct Server;\npub trait Handler { fn handle(&self); }\npub async fn serve() {}\nimpl Server { pub fn start(&self) { helper(); } }\nfn helper() {}\n#[cfg(test)]\nmod tests { fn ignored() {} }\n").unwrap();
    fs::write(
        root.path().join("src/nested/service.rs"),
        "pub(crate) type Id = u64;\n",
    )
    .unwrap();
    fs::write(root.path().join("src/orphan.rs"), "pub fn unused() {}\n").unwrap();
    let output = analyze(root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["packages"][0]["name"], "example");
    assert_eq!(report["packages"][0]["targets"][0]["kind"][0], "bin");
    let items = report["items"].as_array().unwrap();
    let serve = items.iter().find(|item| item["name"] == "serve").unwrap();
    assert_eq!(serve["module"], "crate::api");
    assert_eq!(serve["source"]["path"], "src/api.rs");
    assert_eq!(serve["source"]["start_line"], 4);
    assert_eq!(serve["visibility"], "pub");
    assert!(
        serve["signature"]
            .as_str()
            .unwrap()
            .contains("async fn serve")
    );
    let method = items.iter().find(|item| item["name"] == "start").unwrap();
    assert!(method["owner"].as_str().unwrap().contains("Server"));
    assert!(
        method["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["expression"] == "helper")
    );
    assert!(
        items
            .iter()
            .any(|item| item["name"] == "Id" && item["module"] == "crate::nested::service")
    );
    assert!(items.iter().any(
        |item| item["name"] == "handle" && item["owner"].as_str().unwrap().contains("Handler")
    ));
    assert!(
        !items
            .iter()
            .any(|item| item["name"] == "unused" || item["name"] == "ignored")
    );
    assert!(
        report["unlinked_rust_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file == "src/orphan.rs")
    );
    assert!(
        report["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|import| import["expression"] == "use std::path::Path;")
    );
}

#[test]
fn missing_modules_and_parse_errors_are_reported_as_incomplete_analysis() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        "mod absent;\npub fn broken( {\n",
    )
    .unwrap();
    let output = analyze(root.path());
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complete"], false);
    assert!(!report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn explicit_module_paths_and_feature_guards_remain_visible() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("src/lib.rs"), "#[path = \"implementation.rs\"]\npub mod api;\n#[cfg(feature = \"http\")]\npub fn http() {}\n#[cfg(not(test))]\npub fn production() {}\n").unwrap();
    fs::write(
        root.path().join("src/implementation.rs"),
        "pub fn run() {}\n",
    )
    .unwrap();
    let output = analyze(root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let items = report["items"].as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|item| item["name"] == "run" && item["module"] == "crate::api")
    );
    assert!(items.iter().any(|item| item["name"] == "production"));
    let http = items.iter().find(|item| item["name"] == "http").unwrap();
    assert!(
        http["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|attribute| attribute.as_str().unwrap().contains("feature"))
    );
}

#[test]
fn conditional_imports_keep_their_guard_and_call_summaries_omit_argument_bodies() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("src/lib.rs"), "#[cfg(feature = \"http\")]\nuse std::io;\npub fn run() { let values = [1]; values.iter().map(|v| { let argument_body = v + 1; argument_body }).collect::<Vec<_>>(); }\n").unwrap();
    let output = analyze(root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["imports"][0]["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|attribute| attribute.as_str().unwrap().contains("http"))
    );
    let calls = report["items"][0]["calls"].as_array().unwrap();
    assert!(
        calls
            .iter()
            .any(|call| call["expression"].as_str().unwrap().contains("collect"))
    );
    assert!(!calls.iter().any(|call| {
        call["expression"]
            .as_str()
            .unwrap()
            .contains("argument_body")
    }));
}
