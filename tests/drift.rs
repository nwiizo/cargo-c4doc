#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-c4doc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::create_dir(root.path().join("target")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    root
}

fn snapshot(root: &Path, name: &str) {
    success(&cli(
        root,
        &["analyze", ".", "--output", &format!("target/{name}.json")],
    ));
    success(&cli(
        root,
        &["scan", ".", "--output", &format!("target/{name}-scan.json")],
    ));
}

fn read_json(root: &Path, name: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(name)).unwrap()).unwrap()
}

fn save_json(root: &Path, name: &str, value: &Value) {
    fs::write(root.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

#[test]
fn new_runtime_candidates_and_cited_source_changes_require_review() {
    let root = fixture();
    snapshot(root.path(), "before");
    success(&cli(
        root.path(),
        &[
            "init",
            "--title",
            "Example",
            "--output",
            "target/model.json",
        ],
    ));
    let source = cli(root.path(), &["cite", "src/main.rs"]);
    success(&source);
    let mut model = read_json(root.path(), "target/model.json");
    model["elements"][0]["evidence"] = json!({"status":"observed", "sources":[serde_json::from_slice::<Value>(&source.stdout).unwrap()]});
    let inferred = json!({"status":"inferred","reason":"Test fixture"});
    let components = model["elements"].as_array_mut().unwrap();
    components.push(json!({"id":"cli","kind":"container","parent":"system","name":"CLI","description":"Runs commands","technology":"Rust","evidence":inferred}));
    for (id, rationale) in [
        ("dispatch", "Routes requests"),
        ("startup", "Starts the process"),
    ] {
        components.push(json!({"id":id,"kind":"component","parent":"cli","name":id,"description":rationale,"technology":"Rust","evidence":inferred,
            "code_mapping":{"coverage":"Entry points","entries":[{"source":serde_json::from_slice::<Value>(&source.stdout).unwrap(),"rationale":rationale}]}}));
    }
    save_json(root.path(), "target/model.json", &model);

    fs::create_dir(root.path().join("src/bin")).unwrap();
    fs::create_dir(root.path().join("web")).unwrap();
    fs::create_dir(root.path().join("tests")).unwrap();
    fs::write(root.path().join("src/bin/server.rs"), "fn main() {}\n").unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();
    fs::write(root.path().join("web/app.js"), "fetch('/api/new');\n").unwrap();
    fs::write(root.path().join("compose.yaml"), "services: {}\n").unwrap();
    fs::write(
        root.path().join("tests/smoke.rs"),
        "#[test] fn works() {}\n",
    )
    .unwrap();
    snapshot(root.path(), "after");
    let args = [
        "drift",
        "target/before.json",
        "target/after.json",
        "--before-scan",
        "target/before-scan.json",
        "--after-scan",
        "target/after-scan.json",
        "--model",
        "target/model.json",
    ];
    let output = cli(root.path(), &args);
    success(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complete"], true);
    assert_eq!(report["review_required"], true);
    let changes = report["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["category"] == "cargo_target"
        && c["change"] == "added"
        && c["paths"] == json!(["src/bin/server.rs"])
        && c["requires_review"] == true));
    assert!(changes.iter().any(|c| c["category"] == "rust_source"
        && c["change"] == "added"
        && c["key"] == "src/bin/server.rs"));
    let main = changes
        .iter()
        .find(|c| c["category"] == "rust_source" && c["key"] == "src/main.rs")
        .unwrap();
    assert_eq!(main["change"], "modified");
    assert_eq!(main["cited_claims"], json!(["element:system"]));
    assert_eq!(main["mapped_components"], json!(["dispatch", "startup"]));
    assert_eq!(main["requires_review"], true);
    assert!(changes.iter().any(|c| c["category"] == "unlinked_rust"
        && c["key"] == "tests/smoke.rs"
        && c["requires_review"] == false));
    for path in ["web/app.js", "compose.yaml"] {
        assert!(changes.iter().any(|c| c["category"] == "auxiliary_file"
            && c["key"] == path
            && c["requires_review"] == true));
    }
    assert!(
        !changes
            .iter()
            .any(|c| c["category"] == "auxiliary_file" && c["key"] == "src/main.rs")
    );
    let mut gated = args.to_vec();
    gated.push("--check");
    let failed = cli(root.path(), &gated);
    assert_eq!(failed.status.code(), Some(1));
    assert_eq!(failed.stdout, output.stdout);
    assert_eq!(cli(root.path(), &args).stdout, output.stdout);
}

#[test]
fn identical_observations_and_unlinked_tests_do_not_fail_the_review_gate() {
    let root = fixture();
    snapshot(root.path(), "before");
    success(&cli(
        root.path(),
        &[
            "drift",
            "target/before.json",
            "target/before.json",
            "--check",
        ],
    ));
    fs::create_dir(root.path().join("tests")).unwrap();
    fs::write(
        root.path().join("tests/smoke.rs"),
        "#[test] fn works() {}\n",
    )
    .unwrap();
    snapshot(root.path(), "after");
    let output = cli(
        root.path(),
        &[
            "drift",
            "target/before.json",
            "target/after.json",
            "--check",
        ],
    );
    success(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["review_required"], false);
    assert_eq!(report["coverage"]["auxiliary"], "not_supplied");
    assert_eq!(report["changes"].as_array().unwrap().len(), 1);
    assert_eq!(report["changes"][0]["category"], "unlinked_rust");
}

#[test]
fn incomplete_or_incomparable_inputs_never_report_a_clean_comparison() {
    let root = fixture();
    snapshot(root.path(), "before");
    for case in ["incomplete", "diagnostics", "selection", "root"] {
        let mut after = read_json(root.path(), "target/before.json");
        match case {
            "incomplete" => after["complete"] = json!(false),
            "diagnostics" => after["diagnostics"] = json!(["unsupported macro"]),
            "selection" => after["packages"][0]["name"] = json!("different-package"),
            "root" => after["source_root"] = json!("/another/checkout"),
            _ => unreachable!(),
        }
        save_json(root.path(), "target/after.json", &after);
        let output = cli(
            root.path(),
            &["drift", "target/before.json", "target/after.json"],
        );
        assert_eq!(output.status.code(), Some(1), "{case}");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["complete"], false, "{case}");
        assert_eq!(report["review_required"], true, "{case}");
        assert!(
            !report["diagnostics"].as_array().unwrap().is_empty(),
            "{case}"
        );
    }
}

#[test]
fn incomplete_scan_is_disclosed_and_missing_scan_pairs_are_usage_errors() {
    let root = fixture();
    snapshot(root.path(), "before");
    let mut scan = read_json(root.path(), "target/before-scan.json");
    scan["truncated"] = json!(true);
    save_json(root.path(), "target/after-scan.json", &scan);
    let output = cli(
        root.path(),
        &[
            "drift",
            "target/before.json",
            "target/before.json",
            "--before-scan",
            "target/before-scan.json",
            "--after-scan",
            "target/after-scan.json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complete"], false);
    assert_eq!(report["coverage"]["auxiliary"], "incomplete");
    assert_eq!(
        cli(
            root.path(),
            &[
                "drift",
                "target/before.json",
                "target/before.json",
                "--before-scan",
                "target/before-scan.json"
            ]
        )
        .status
        .code(),
        Some(2)
    );
}

#[test]
fn drift_rejects_bad_versions_and_conflicting_file_hashes_and_protects_output() {
    let root = fixture();
    snapshot(root.path(), "before");
    for case in ["version", "conflicting_hash", "unsafe_path", "missing_root"] {
        let mut after = read_json(root.path(), "target/before.json");
        match case {
            "version" => after["schema_version"] = json!(2),
            "conflicting_hash" => {
                let mut module = after["modules"][0].clone();
                module["source"]["sha256"] = json!("0".repeat(64));
                after["modules"].as_array_mut().unwrap().push(module);
            }
            "unsafe_path" => after["modules"][0]["source"]["path"] = json!("../outside.rs"),
            "missing_root" => after["modules"] = json!([]),
            _ => unreachable!(),
        }
        save_json(root.path(), "target/after.json", &after);
        assert_eq!(
            cli(
                root.path(),
                &["drift", "target/before.json", "target/after.json"]
            )
            .status
            .code(),
            Some(1),
            "{case}"
        );
    }
    fs::write(root.path().join("target/keep.json"), "manual content\n").unwrap();
    assert_eq!(
        cli(
            root.path(),
            &[
                "drift",
                "target/before.json",
                "target/before.json",
                "--output",
                "target/keep.json"
            ]
        )
        .status
        .code(),
        Some(1)
    );
    assert_eq!(
        fs::read_to_string(root.path().join("target/keep.json")).unwrap(),
        "manual content\n"
    );
}

#[test]
fn removed_targets_moved_sources_and_feature_changes_are_reported() {
    let root = fixture();
    fs::create_dir(root.path().join("src/bin")).unwrap();
    fs::write(root.path().join("src/bin/server.rs"), "fn main() {}\n").unwrap();
    snapshot(root.path(), "before");
    fs::remove_file(root.path().join("src/bin/server.rs")).unwrap();
    fs::rename(
        root.path().join("src/main.rs"),
        root.path().join("src/runner.rs"),
    )
    .unwrap();
    fs::write(root.path().join("Cargo.toml"), "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[[bin]]\nname = \"example\"\npath = \"src/runner.rs\"\n[features]\na = []\nb = []\ndefault = [\"a\", \"b\"]\n").unwrap();
    snapshot(root.path(), "after");
    let output = cli(
        root.path(),
        &[
            "drift",
            "target/before.json",
            "target/after.json",
            "--output",
            "target/report.json",
        ],
    );
    success(&output);
    assert!(output.stdout.is_empty());
    let report = read_json(root.path(), "target/report.json");
    let changes = report["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["category"] == "cargo_target"
        && c["change"] == "removed"
        && c["key"] == "example:bin:server"));
    assert!(changes.iter().any(|c| c["category"] == "cargo_target"
        && c["change"] == "modified"
        && c["paths"] == json!(["src/main.rs", "src/runner.rs"])));
    assert!(
        changes
            .iter()
            .any(|c| c["category"] == "package_metadata" && c["change"] == "modified")
    );
    assert!(changes.iter().any(|c| c["category"] == "rust_source"
        && c["change"] == "removed"
        && c["key"] == "src/main.rs"));
    let mut reordered = read_json(root.path(), "target/after.json");
    reordered["packages"][0]["features"]["default"] = json!(["b", "a"]);
    save_json(root.path(), "target/reordered.json", &reordered);
    let output = cli(
        root.path(),
        &[
            "drift",
            "target/after.json",
            "target/reordered.json",
            "--check",
        ],
    );
    success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["changes"],
        json!([])
    );
}

#[test]
fn incompatible_or_stale_scans_cannot_hide_missing_coverage() {
    let root = fixture();
    snapshot(root.path(), "before");
    for case in ["limits", "hash", "missing_file", "duplicate", "skipped"] {
        let mut after = read_json(root.path(), "target/before-scan.json");
        let index = after["files"]
            .as_array()
            .unwrap()
            .iter()
            .position(|f| f["path"] == "src/main.rs")
            .unwrap();
        match case {
            "limits" => after["max_files"] = json!(1),
            "hash" => after["files"][index]["sha256"] = json!("0".repeat(64)),
            "missing_file" => {
                after["files"].as_array_mut().unwrap().remove(index);
            }
            "duplicate" => {
                let file = after["files"][index].clone();
                after["files"].as_array_mut().unwrap().push(file);
            }
            "skipped" => {
                after["skipped"] = json!([{"path":"web/app.js", "reason":"file too large"}])
            }
            _ => unreachable!(),
        }
        save_json(root.path(), "target/after-scan.json", &after);
        let output = cli(
            root.path(),
            &[
                "drift",
                "target/before.json",
                "target/before.json",
                "--before-scan",
                "target/before-scan.json",
                "--after-scan",
                "target/after-scan.json",
            ],
        );
        assert_eq!(output.status.code(), Some(1), "{case}");
        assert!(
            !output.stdout.is_empty() || !output.stderr.is_empty(),
            "{case}"
        );
    }
}

#[test]
fn a_source_shared_by_library_and_binary_is_reported_once() {
    let root = fixture();
    fs::write(
        root.path().join("src/main.rs"),
        "mod shared;\nfn main() {}\n",
    )
    .unwrap();
    fs::write(root.path().join("src/lib.rs"), "mod shared;\n").unwrap();
    fs::write(root.path().join("src/shared.rs"), "pub fn run() {}\n").unwrap();
    snapshot(root.path(), "before");
    fs::write(
        root.path().join("src/shared.rs"),
        "pub fn run() { println!(\"new\"); }\n",
    )
    .unwrap();
    snapshot(root.path(), "after");
    let output = cli(
        root.path(),
        &[
            "drift",
            "target/before.json",
            "target/after.json",
            "--check",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["changes"].as_array().unwrap().len(), 1);
    assert_eq!(report["changes"][0]["key"], "src/shared.rs");
}
