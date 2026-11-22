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
    fs::write(
        root.path().join("src/main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    root
}

fn inferred() -> Value {
    json!({"status": "inferred", "reason": "Example architecture to be confirmed by the owner"})
}

fn model() -> Value {
    json!({
        "schema_version": 1,
        "title": "Example service",
        "description": "Serve an example request",
        "elements": [
            {"id":"user", "kind":"person", "name":"Reader", "description":"Reads results", "evidence":inferred()},
            {"id":"app", "kind":"system", "name":"Example service", "description":"Serves results", "evidence":inferred()},
            {"id":"api", "kind":"container", "parent":"app", "name":"API", "description":"Handles requests", "technology":"Rust", "evidence":inferred()},
            {"id":"handler", "kind":"component", "parent":"api", "name":"Handler", "description":"Builds responses", "technology":"Rust module", "evidence":inferred()},
            {"id":"main_fn", "kind":"code", "parent":"handler", "name":"main", "description":"Runs the program", "technology":"Rust function", "evidence":inferred()},
            {"id":"host", "kind":"deployment_node", "name":"Developer machine", "description":"Runs a local binary", "technology":"macOS", "environment":"development", "evidence":inferred()},
            {"id":"api_local", "kind":"instance", "parent":"host", "instance_of":"api", "name":"Local API", "description":"Local process", "environment":"development", "evidence":inferred()}
        ],
        "relationships": [
            {"id":"uses", "from":"user", "to":"app", "description":"Reads results", "evidence":inferred()},
            {"id":"requests", "from":"user", "to":"api", "description":"Requests results", "technology":"HTTPS", "evidence":inferred()}
        ],
        "views": [
            {"id":"context", "kind":"context", "scope":"app", "title":"System context", "description":"Who uses the service", "elements":["user","app"], "relationships":["uses"]},
            {"id":"containers", "kind":"container", "scope":"app", "title":"Containers", "description":"Runtime boundaries", "elements":["user","api"], "relationships":["requests"]},
            {"id":"components", "kind":"component", "scope":"api", "title":"Components", "description":"API responsibilities", "elements":["handler"]},
            {"id":"code", "kind":"code", "scope":"handler", "title":"Code", "description":"Implementation detail", "elements":["main_fn"]},
            {"id":"landscape", "kind":"landscape", "title":"Landscape", "description":"Systems and people", "elements":["user","app"], "relationships":["uses"]},
            {"id":"request_flow", "kind":"dynamic", "scope":"app", "title":"Read results", "description":"A request", "elements":["user","api"], "steps":[{"relationship":"requests", "description":"GET /results"}]},
            {"id":"deployment", "kind":"deployment", "scope":"app", "environment":"development", "title":"Development", "description":"Local process placement", "elements":["host","api_local"]}
        ],
        "questions": ["Confirm the production environment"]
    })
}

fn save_model(root: &Path, value: &Value) {
    fs::write(
        root.join("c4.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

#[test]
fn scan_respects_ignores_and_reports_bounded_text_inventory() {
    let root = fixture();
    fs::write(root.path().join(".gitignore"), "ignored.rs\n").unwrap();
    fs::write(root.path().join("ignored.rs"), "private").unwrap();
    fs::write(root.path().join(".env"), "SECRET=value").unwrap();
    fs::create_dir(root.path().join("target")).unwrap();
    fs::write(root.path().join("target/generated.rs"), "generated").unwrap();
    let output = cli(root.path(), &["scan", "."]);
    success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    let files = result["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["path"], "src/main.rs");
    assert_eq!(files[0]["lines"], 3);
    assert_eq!(files[0]["sha256"].as_str().unwrap().len(), 64);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SECRET"));
}

#[test]
fn init_requires_review_and_never_overwrites() {
    let root = fixture();
    success(&cli(
        root.path(),
        &["init", "--title", "Example", "--output", "c4.json"],
    ));
    success(&cli(root.path(), &["check", "c4.json", "--root", "."]));
    assert_eq!(
        cli(
            root.path(),
            &["check", "c4.json", "--root", ".", "--strict"]
        )
        .status
        .code(),
        Some(1)
    );
    let before = fs::read(root.path().join("c4.json")).unwrap();
    assert_eq!(
        cli(
            root.path(),
            &["init", "--title", "Other", "--output", "c4.json"]
        )
        .status
        .code(),
        Some(1)
    );
    assert_eq!(fs::read(root.path().join("c4.json")).unwrap(), before);
}

#[test]
fn all_views_render_and_check_detects_stale_or_edited_output() {
    let root = fixture();
    save_model(root.path(), &model());
    success(&cli(
        root.path(),
        &["render", "c4.json", "--root", ".", "--output", "docs"],
    ));
    for id in [
        "context",
        "containers",
        "components",
        "code",
        "landscape",
        "request_flow",
        "deployment",
    ] {
        let markdown = fs::read_to_string(root.path().join(format!("docs/{id}.md"))).unwrap();
        assert!(markdown.contains("```mermaid"));
        assert!(root.path().join(format!("docs/{id}.mmd")).exists());
    }
    let containers = fs::read_to_string(root.path().join("docs/containers.mmd")).unwrap();
    assert!(
        containers.contains("subgraph"),
        "the system boundary must be visible"
    );
    let flow = fs::read_to_string(root.path().join("docs/request_flow.mmd")).unwrap();
    assert!(flow.contains("\nsequenceDiagram\n"));
    assert!(flow.contains("GET /results"));
    success(&cli(
        root.path(),
        &[
            "render", "c4.json", "--root", ".", "--output", "docs", "--check",
        ],
    ));
    let mut updated = model();
    updated["description"] = json!("Updated architecture");
    save_model(root.path(), &updated);
    assert_eq!(
        cli(
            root.path(),
            &[
                "render", "c4.json", "--root", ".", "--output", "docs", "--check"
            ]
        )
        .status
        .code(),
        Some(1)
    );
    success(&cli(
        root.path(),
        &["render", "c4.json", "--root", ".", "--output", "docs"],
    ));
    fs::write(root.path().join("docs/context.md"), "Manual notes").unwrap();
    assert_eq!(
        cli(
            root.path(),
            &["render", "c4.json", "--root", ".", "--output", "docs"]
        )
        .status
        .code(),
        Some(1)
    );
    assert_eq!(
        fs::read_to_string(root.path().join("docs/context.md")).unwrap(),
        "Manual notes"
    );
}

#[test]
fn reading_guide_separates_detail_and_follows_the_authored_view_order() {
    let root = fixture();
    let mut value = model();
    value["views"] = json!([value["views"][1], value["views"][0]]);
    value["views"][0]["notes"] = json!([
        "Follow the request from the reader to the API.",
        "補足: <script>alert(1)</script> [link](javascript:alert(1))"
    ]);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let index = fs::read_to_string(root.path().join("docs/README.md")).unwrap();
    assert!(index.find("(containers.md)").unwrap() < index.find("(context.md)").unwrap());
    let page = fs::read_to_string(root.path().join("docs/containers.md")).unwrap();
    let diagram = fs::read_to_string(root.path().join("docs/containers.mmd")).unwrap();
    assert!(page.find("Runtime boundaries").unwrap() < page.find("```mermaid").unwrap());
    let note = value["views"][0]["notes"][0].as_str().unwrap();
    assert!(page.find(&diagram).unwrap() < page.find(note).unwrap());
    assert!(!index.contains(note));
    assert!(!diagram.contains(note));
    assert!(page.contains("補足:"));
    assert!(!page.contains("<script>"));
    assert!(!page.contains("[link](javascript:"));
    assert!(page.contains("[System context](context.md)"));
    let last_page = fs::read_to_string(root.path().join("docs/context.md")).unwrap();
    assert!(!last_page.contains("Continue reading:"));

    // A one-view model must not retain a link to the removed next view.
    value["views"].as_array_mut().unwrap().truncate(1);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let page = fs::read_to_string(root.path().join("docs/containers.md")).unwrap();
    assert!(!page.contains("(context.md)"));
}

#[test]
fn elements_link_to_their_scoped_views_in_reading_order() {
    let root = fixture();
    let mut value = model();
    value["views"][0]["title"] = json!("Context [public]");
    value["elements"][0]["name"] = value["elements"][1]["name"].clone();
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));

    let landscape = fs::read_to_string(root.path().join("docs/landscape.md")).unwrap();
    let system = landscape
        .lines()
        .find(|line| line.contains("| Software system |"))
        .unwrap();
    assert!(system.contains(r"[Context \[public\]](context.md)"));
    assert!(system.find("(context.md)").unwrap() < system.find("(containers.md)").unwrap());
    let person = landscape
        .lines()
        .find(|line| line.contains("| Person |"))
        .unwrap();
    assert!(!person.contains("(context.md)"));
    let context = fs::read_to_string(root.path().join("docs/context.md")).unwrap();
    let system = context
        .lines()
        .find(|line| line.contains("| Software system |"))
        .unwrap();
    assert!(system.contains("(containers.md)"));
    assert!(!system.contains("(context.md)"));
}

#[test]
fn component_code_mapping_validates_and_renders_its_sources() {
    let root = fixture();
    let citation = cli(root.path(), &["cite", "src/main.rs"]);
    success(&citation);
    let mapping = json!({
        "coverage": "Request handling entry points",
        "entries": [{"rationale": "Starts the request handler", "source": serde_json::from_slice::<Value>(&citation.stdout).unwrap()}]
    });
    let mut value = model();
    value["elements"][3]["code_mapping"] = mapping.clone();
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let page = fs::read_to_string(root.path().join("docs/evidence.md")).unwrap();
    assert!(page.contains("Request handling entry points"));
    assert!(page.contains("Starts the request handler"));
    assert!(page.contains("../src/main.rs#L1-L3"), "{page}");
    let mut self_citing = value.clone();
    let generated = cli(root.path(), &["cite", "docs/evidence.md"]);
    success(&generated);
    self_citing["elements"][3]["code_mapping"]["entries"][0]["source"] =
        serde_json::from_slice(&generated.stdout).unwrap();
    save_model(root.path(), &self_citing);
    let refused = cli(root.path(), &["render", "c4.json", "--output", "docs"]);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("outside the generated document directory")
    );
    assert_eq!(
        fs::read_to_string(root.path().join("docs/evidence.md")).unwrap(),
        page
    );

    for (index, invalid, message) in [
        (0, mapping.clone(), "only valid for components"),
        (3, json!({"coverage":" ", "entries":[]}), "coverage"),
        (
            3,
            json!({"coverage":"Entry points", "entries":[]}),
            "at least one",
        ),
    ] {
        let mut invalid_model = value.clone();
        invalid_model["elements"][index]["code_mapping"] = invalid;
        save_model(root.path(), &invalid_model);
        let failed = cli(root.path(), &["check", "c4.json", "--json"]);
        assert_eq!(failed.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&failed.stdout).contains(message));
    }
    save_model(root.path(), &value);
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let stale = cli(root.path(), &["check", "c4.json", "--json"]);
    assert_eq!(stale.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&stale.stdout).contains("source digest changed"));
}

#[test]
fn shared_deployment_keeps_environment_and_host_checks() {
    let root = fixture();
    let mut value = model();
    let mut peer = value["elements"][1].clone();
    peer["id"] = json!("peer");
    let mut instance = value["elements"][6].clone();
    instance["id"] = json!("peer_local");
    instance["instance_of"] = json!("peer");
    value["elements"]
        .as_array_mut()
        .unwrap()
        .extend([peer, instance]);
    let view = &mut value["views"][6];
    view.as_object_mut().unwrap().remove("scope");
    view["elements"]
        .as_array_mut()
        .unwrap()
        .push(json!("peer_local"));
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let page = fs::read_to_string(root.path().join("docs/deployment.md")).unwrap();
    assert!(page.contains("Cross-system"));
    assert!(page.contains("n_peer_local"));
    value["views"][6]["environment"] = json!("production");
    save_model(root.path(), &value);
    assert!(!cli(root.path(), &["check", "c4.json"]).status.success());
}

#[test]
fn perspective_filters_elements_and_arrows_and_groups_shared_membership() {
    let root = fixture();
    let mut value = model();
    value["elements"][0]["perspectives"] = json!({"owner":["Operations"]});
    value["elements"][1]["perspectives"] =
        json!({"owner":["Platform","Product"],"security":["Internal"]});
    value["views"][4]["perspective"] = json!({"key":"owner","values":["Platform"]});
    let mut security = value["views"][4].clone();
    security["id"] = json!("security");
    security["title"] = json!("Security zones");
    security["perspective"] = json!({"key":"security"});
    value["views"].as_array_mut().unwrap().push(security);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let page = fs::read_to_string(root.path().join("docs/landscape.md")).unwrap();
    assert!(page.contains("Platform") && page.contains("Product"));
    assert!(!page.contains("Reader") && !page.contains("n_user"));
    assert!(!page.contains("Reads results"));
    assert!(page.contains("[Security zones](security.md)"));
    let security = fs::read_to_string(root.path().join("docs/security.md")).unwrap();
    assert!(security.contains("Unassigned") && security.contains("Internal"));
    let diagram = fs::read_to_string(root.path().join("docs/landscape.mmd")).unwrap();
    assert!(diagram.contains("Platform") && !diagram.contains("n_user"));
    value["views"][4]["perspective"] = json!({"key":"security","values":["Public"]});
    save_model(root.path(), &value);
    assert!(!cli(root.path(), &["check", "c4.json"]).status.success());
}

#[test]
fn scenario_renders_nested_branches_retries_and_parallel_work() {
    let root = fixture();
    let mut value = model();
    let request = value["views"][5]["steps"][0].clone();
    let response =
        json!({"relationship":"requests","description":"Return results","response":true});
    value["views"][5]["steps"] = json!([
        {"kind":"alternative","branches":[
            {"label":"Cache miss","steps":[{"kind":"loop","branches":[{"label":"While busy","steps":[request]}]}]},
            {"label":"Cache hit","steps":[response]}]},
        {"kind":"parallel","branches":[{"label":"First reader","steps":[request]},{"label":"Second reader","steps":[request]}]},
        {"kind":"optional","branches":[{"label":"Requested","steps":[response]}]}
    ]);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let diagram = fs::read_to_string(root.path().join("docs/request_flow.mmd")).unwrap();
    for token in [
        "alt Cache miss",
        "loop While busy",
        "else Cache hit",
        "par First reader",
        "and Second reader",
        "opt Requested",
        "n_api-->>n_user",
    ] {
        assert!(diagram.contains(token), "{token}");
    }
    let page = fs::read_to_string(root.path().join("docs/request_flow.md")).unwrap();
    assert!(
        page.split("## Scenario")
            .last()
            .unwrap()
            .contains("While busy")
    );
    value["views"][5]["steps"][0]["branches"][0]["steps"][0]["branches"][0]["steps"][0]["relationship"] =
        json!("missing");
    save_model(root.path(), &value);
    assert!(!cli(root.path(), &["check", "c4.json"]).status.success());
}

#[test]
fn vague_relationship_label_reports_the_specific_arrow() {
    let root = fixture();
    let mut value = model();
    value["relationships"][0]["description"] = json!(" Uses ");
    save_model(root.path(), &value);
    let output = cli(root.path(), &["check", "c4.json", "--json"]);
    success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        result["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["location"] == "relationship:uses"
                && w["message"].as_str().unwrap().contains("exchanged"))
    );
}

#[test]
fn focal_system_needs_a_visible_interaction() {
    let root = fixture();
    let mut value = model();
    value["elements"].as_array_mut().unwrap().push(json!({"id":"peer","kind":"system","name":"Peer","description":"External service","evidence":inferred()}));
    value["views"][0]["elements"] = json!(["user", "app", "peer"]);
    value["relationships"][0]["to"] = json!("peer");
    value["views"][4]["elements"] = json!(["user", "app", "peer"]);
    save_model(root.path(), &value);
    let output = cli(root.path(), &["check", "c4.json", "--json"]);
    success(&output);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("focal system has no visible interaction")
    );
}

#[test]
fn release_views_keep_architecture_context() {
    let root = fixture();
    let mut value = model();
    value["views"][2]["show_system_boundary"] = json!(true);
    value["elements"][4]["code_access"] = json!("internal");
    value["elements"][5]["replicas"] = json!({"min":1,"max":10});
    value["elements"][6]["perspectives"] = json!({"security":["Encrypted"]});
    value["views"][6]["perspective"] = json!({"key":"security","values":["Encrypted"]});
    value["relationships"][0]["perspectives"] = json!({"security":["Authenticated"]});
    value["views"][0]["perspective"] = json!({"key":"security","values":["Authenticated"]});
    value["views"][0]["diagram_notes"] = json!(["Audit logging omitted"]);
    value["views"][5]["lifelines"] = json!({"first":{"element":"api","name":"Worker A"},"second":{"element":"api","name":"Worker B"}});
    value["views"][5]["steps"] = json!([
        {"relationship":"requests","description":"Request work","to_lifeline":"first"},
        {"relationship":"requests","description":"Return work","response":true,"from_lifeline":"second"}]);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    for (file, labels) in [
        ("components.mmd", vec!["subgraph b_app", "subgraph b_api"]),
        ("code.mmd", vec!["Internal"]),
        (
            "deployment.mmd",
            vec!["1..10", "Encrypted", "subgraph n_host"],
        ),
        (
            "context.mmd",
            vec!["Authenticated", "Audit logging omitted", "n_user", "n_app"],
        ),
        (
            "request_flow.mmd",
            vec![
                "Worker A",
                "Worker B",
                "n_user->>n_first",
                "n_second-->>n_user",
            ],
        ),
        (
            "request_flow.md",
            vec!["Reader → Worker A", "Worker B → Reader"],
        ),
    ] {
        let page = fs::read_to_string(root.path().join("docs").join(file)).unwrap();
        for label in labels {
            assert!(page.contains(label), "{file}: {label}");
        }
    }
    value["elements"][5]["replicas"]["min"] = json!(11);
    value["views"][5]["steps"][0]["from_lifeline"] = json!("first");
    save_model(root.path(), &value);
    let report = cli(root.path(), &["check", "c4.json", "--json"]);
    assert!(!report.status.success());
    let diagnostics = String::from_utf8_lossy(&report.stdout);
    assert!(diagnostics.contains("replica range") && diagnostics.contains("lifeline endpoint"));
}

#[test]
fn standalone_diagrams_explain_scope_notation_and_evidence_without_markdown() {
    let root = fixture();
    let mut value = model();
    value["views"][0]["title"] = json!("日本語: \"context\"\n---\nconfig: <script>");
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    for view in value["views"].as_array().unwrap() {
        let id = view["id"].as_str().unwrap();
        let diagram = fs::read_to_string(root.path().join(format!("docs/{id}.mmd"))).unwrap();
        assert!(
            diagram.starts_with("---\ntitle: "),
            "{id}: visible title missing"
        );
        let title_line = diagram
            .lines()
            .nth(1)
            .unwrap()
            .strip_prefix("title: ")
            .unwrap();
        let title: String = serde_json::from_str(title_line).unwrap();
        assert!(
            !title.contains('\n'),
            "title must not inject new diagram statements"
        );
        assert!(!diagram.contains("<script>"));
        assert!(diagram.contains("Legend"), "{id}: notation missing");
        if id == "landscape" {
            assert!(title.contains("Cross-system"));
        } else if id == "components" {
            assert!(title.contains("API"));
        } else if id == "code" {
            assert!(title.contains("Handler"));
        } else {
            assert!(title.contains("Example service"));
        }
        if id == "deployment" {
            assert!(title.contains("(development)"));
        }
        if id == "context" {
            assert!(title.contains("日本語: \"context\""));
        }
        if id == "request_flow" {
            assert!(diagram.contains("Relationship evidence: Inferred"));
            assert!(diagram.contains("does not prove execution order"));
            assert!(diagram.contains("Dashed arrows: responses"));
            let markdown = fs::read_to_string(root.path().join("docs/request_flow.md")).unwrap();
            assert!(!markdown.contains("dotted arrows indicate inferred"));
        } else {
            assert!(diagram.contains("Blue solid boxes: Observed"));
            assert!(diagram.contains("Dotted arrows: Inferred"));
            assert!(diagram.contains("not semantic proof"));
        }
    }
}

#[test]
fn check_rejects_mixed_levels_missing_endpoints_and_path_traversal() {
    let root = fixture();
    for (label, value) in [
        ("mixed abstraction", {
            let mut v = model();
            v["views"][0]["elements"] = json!(["user", "app", "handler"]);
            v
        }),
        ("missing endpoint", {
            let mut v = model();
            v["relationships"][0]["to"] = json!("missing");
            v
        }),
        ("unsafe view id", {
            let mut v = model();
            v["views"][0]["id"] = json!("../../outside");
            v
        }),
        ("unknown field", {
            let mut v = model();
            v["typo"] = json!(true);
            v
        }),
        ("missing explanation", {
            let mut v = model();
            v["elements"][0]["evidence"]["reason"] = json!("");
            v
        }),
    ] {
        save_model(root.path(), &value);
        let output = cli(root.path(), &["check", "c4.json", "--root", "."]);
        assert_eq!(output.status.code(), Some(1), "{label}");
    }
}

#[test]
fn source_citations_are_checked_for_line_ranges_and_changes() {
    let root = fixture();
    let citation = cli(
        root.path(),
        &[
            "cite",
            "src/main.rs",
            "--root",
            ".",
            "--start",
            "1",
            "--end",
            "3",
        ],
    );
    success(&citation);
    let source: Value = serde_json::from_slice(&citation.stdout).unwrap();
    let mut value = model();
    value["elements"][4]["evidence"] = json!({"status":"observed", "sources":[source]});
    save_model(root.path(), &value);
    success(&cli(root.path(), &["check", "c4.json", "--root", "."]));
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let output = cli(root.path(), &["check", "c4.json", "--root", ".", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!report["errors"].as_array().unwrap().is_empty());
    assert_eq!(
        cli(root.path(), &["cite", "../outside.rs", "--root", "."])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        cli(
            root.path(),
            &["cite", "src/main.rs", "--root", ".", "--start", "0"]
        )
        .status
        .code(),
        Some(2)
    );
}

#[test]
fn invalid_boundaries_never_reach_document_publication() {
    let root = fixture();
    for (label, value) in [
        ("case-insensitive README collision", {
            let mut v = model();
            v["views"][0]["id"] = json!("readme");
            v
        }),
        ("empty container boundary", {
            let mut v = model();
            v["views"][1]["elements"] = json!(["user"]);
            v["views"][1]["relationships"] = json!([]);
            v
        }),
        ("deployment cycle", {
            let mut v = model();
            v["elements"][5]["parent"] = json!("host");
            v
        }),
        ("mixed deployment environment", {
            let mut v = model();
            v["elements"][6]["environment"] = json!("production");
            v
        }),
        ("missing hosting boundary", {
            let mut v = model();
            v["views"][6]["elements"] = json!(["api_local"]);
            v
        }),
        ("dynamic endpoint absent", {
            let mut v = model();
            v["views"][5]["elements"] = json!(["api"]);
            v
        }),
        ("invalid instance", {
            let mut v = model();
            v["elements"][6]["instance_of"] = json!("handler");
            v
        }),
        ("future version", {
            let mut v = model();
            v["schema_version"] = json!(2);
            v
        }),
        ("empty observed evidence", {
            let mut v = model();
            v["elements"][0]["evidence"] = json!({"status":"observed", "sources":[]});
            v
        }),
    ] {
        save_model(root.path(), &value);
        let output = cli(
            root.path(),
            &["render", "c4.json", "--root", ".", "--output", "docs"],
        );
        assert_eq!(output.status.code(), Some(1), "{label}");
        assert!(!root.path().join("docs").exists(), "{label}");
    }
}

#[test]
fn scan_limits_and_binary_files_are_visible_to_the_agent() {
    let root = fixture();
    fs::write(root.path().join("other.rs"), "// other\n").unwrap();
    let limited = cli(root.path(), &["scan", ".", "--max-files", "1"]);
    success(&limited);
    let inventory: Value = serde_json::from_slice(&limited.stdout).unwrap();
    assert_eq!(inventory["truncated"], true);
    assert_eq!(inventory["files"].as_array().unwrap().len(), 1);
    fs::write(root.path().join("binary.rs"), [0, 255]).unwrap();
    let small = cli(root.path(), &["scan", ".", "--max-bytes", "10"]);
    success(&small);
    let inventory: Value = serde_json::from_slice(&small.stdout).unwrap();
    assert_eq!(inventory["skipped"].as_array().unwrap().len(), 2);
}

#[test]
fn strict_check_accepts_sourced_model_and_json_reports_parse_errors() {
    let root = fixture();
    let mut value = model();
    let output = cli(root.path(), &["cite", "src/main.rs"]);
    success(&output);
    let source: Value = serde_json::from_slice(&output.stdout).unwrap();
    for group in ["elements", "relationships"] {
        for item in value[group].as_array_mut().unwrap() {
            item["evidence"] = json!({"status":"observed", "sources":[source.clone()]});
        }
    }
    value["questions"] = json!([]);
    save_model(root.path(), &value);
    // This checks structural provenance only, not whether these fixture claims are true.
    success(&cli(root.path(), &["check", "c4.json", "--strict"]));
    fs::write(root.path().join("c4.json"), "{broken").unwrap();
    let output = cli(root.path(), &["check", "c4.json", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!report["errors"].as_array().unwrap().is_empty());
}

#[test]
fn regeneration_removes_obsolete_generated_views_but_refuses_extra_files() {
    let root = fixture();
    let mut value = model();
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    value["views"].as_array_mut().unwrap().remove(4);
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    assert!(!root.path().join("docs/landscape.md").exists());
    assert!(!root.path().join("docs/landscape.mmd").exists());
    fs::write(root.path().join("docs/notes.md"), "Keep me").unwrap();
    assert_eq!(
        cli(root.path(), &["render", "c4.json", "--output", "docs"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        fs::read_to_string(root.path().join("docs/notes.md")).unwrap(),
        "Keep me"
    );
    assert_eq!(
        cli(root.path(), &["render", "c4.json", "--output", "."])
            .status
            .code(),
        Some(1)
    );
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_source_or_output_boundaries() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    let outside = fixture();
    symlink(outside.path(), root.path().join("linked")).unwrap();
    symlink(
        outside.path().join("src/main.rs"),
        root.path().join("link.rs"),
    )
    .unwrap();
    assert_eq!(
        cli(root.path(), &["cite", "link.rs"]).status.code(),
        Some(1)
    );
    assert_eq!(
        cli(root.path(), &["cite", "linked/src/main.rs"])
            .status
            .code(),
        Some(1)
    );
    save_model(root.path(), &model());
    assert_eq!(
        cli(
            root.path(),
            &["render", "c4.json", "--output", "linked/docs"]
        )
        .status
        .code(),
        Some(1)
    );
    assert!(!outside.path().join("docs").exists());
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    fs::remove_file(root.path().join("docs/context.md")).unwrap();
    symlink(
        outside.path().join("src/main.rs"),
        root.path().join("docs/context.md"),
    )
    .unwrap();
    assert_eq!(
        cli(root.path(), &["render", "c4.json", "--output", "docs"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("src/main.rs")).unwrap(),
        "fn main() {\n    println!(\"hello\");\n}\n"
    );
}

#[test]
fn labels_are_escaped_and_repeated_dynamic_steps_keep_order() {
    let root = fixture();
    let mut value = model();
    value["elements"][0]["name"] = json!("Reader\"\n<script>|[x](evil)`");
    value["views"][5]["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({"relationship":"requests", "description":"200 OK", "response":true}));
    save_model(root.path(), &value);
    success(&cli(
        root.path(),
        &["render", "c4.json", "--output", "docs"],
    ));
    let markdown = fs::read_to_string(root.path().join("docs/context.md")).unwrap();
    assert!(!markdown.contains("<script>"));
    assert!(!markdown.contains("[x](evil)"));
    let flow = fs::read_to_string(root.path().join("docs/request_flow.mmd")).unwrap();
    assert!(!flow.contains("<script>"));
    assert!(flow.contains("n_user->>n_api: GET /results"));
    assert!(flow.contains("n_api-->>n_user: 200 OK"));
    assert!(flow.find("GET /results").unwrap() < flow.find("200 OK").unwrap());
}

#[test]
fn cargo_subcommand_and_direct_binary_have_identical_behavior() {
    let root = fixture();
    let direct = cli(root.path(), &["scan", "."]);
    success(&direct);
    let executable = Path::new(env!("CARGO_BIN_EXE_cargo-c4doc"));
    let search_path = std::env::var_os("PATH").unwrap();
    let paths = std::iter::once(executable.parent().unwrap().to_path_buf())
        .chain(std::env::split_paths(&search_path));
    let output = Command::new("cargo")
        .current_dir(root.path())
        .env("PATH", std::env::join_paths(paths).unwrap())
        .args(["c4doc", "scan", "."])
        .output()
        .unwrap();
    success(&output);
    assert_eq!(output.stdout, direct.stdout);
    assert_eq!(
        cli(root.path(), &["c4doc", "--help"]).status.code(),
        Some(0)
    );
}
