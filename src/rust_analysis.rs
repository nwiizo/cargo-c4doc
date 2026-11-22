use crate::{Error, Result, io_error, model::SourceRef, source};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};
use tree_sitter::{Node, Parser};

#[derive(Debug, Deserialize, Serialize)]
pub struct Analysis {
    pub(crate) schema_version: u32,
    pub source_root: PathBuf,
    pub packages: Vec<Package>,
    pub modules: Vec<Module>,
    pub items: Vec<Item>,
    pub imports: Vec<Import>,
    pub unlinked_rust_files: Vec<String>,
    pub diagnostics: Vec<String>,
    pub complete: bool,
    limitations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    edition: String,
    targets: Vec<CargoTarget>,
    dependencies: Vec<Dependency>,
    features: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub(crate) name: String,
    pub(crate) manifest: String,
    pub(crate) edition: String,
    pub(crate) targets: Vec<Target>,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) features: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Target {
    pub(crate) name: String,
    pub(crate) kind: Vec<String>,
    pub(crate) source: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    name: String,
    rename: Option<String>,
    kind: Option<String>,
    optional: bool,
    uses_default_features: bool,
    pub(crate) features: Vec<String>,
    target: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Module {
    package: String,
    target: String,
    module: String,
    pub(crate) source: SourceRef,
    attributes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Item {
    package: String,
    target: String,
    module: String,
    name: String,
    kind: String,
    owner: Option<String>,
    visibility: String,
    signature: String,
    attributes: Vec<String>,
    source: SourceRef,
    calls: Vec<Call>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Call {
    expression: String,
    line: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Import {
    package: String,
    target: String,
    module: String,
    expression: String,
    attributes: Vec<String>,
    source: SourceRef,
}

pub fn analyze(path: &Path, package: Option<&str>) -> Result<Analysis> {
    let path = dunce::canonicalize(path).map_err(|e| io_error(path, e))?;
    let manifest = if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path
    };
    let parent = manifest
        .parent()
        .ok_or_else(|| Error::Invalid("manifest needs a parent".into()))?;
    // Cargo owns target discovery, workspace membership, inherited/renamed dependencies,
    // and target-specific declarations. This never runs builds or procedural macros.
    let output = Command::new("cargo")
        .current_dir(parent)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
            "--locked",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .map_err(|e| io_error("cargo metadata", e))?;
    if !output.status.success() {
        return Err(Error::Invalid(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)?;
    let source_root = dunce::canonicalize(&metadata.workspace_root)
        .map_err(|e| io_error(&metadata.workspace_root, e))?;
    let root_is_package = metadata
        .packages
        .iter()
        .any(|p| p.manifest_path == manifest);
    let packages: Vec<_> = metadata
        .packages
        .into_iter()
        .filter(|p| {
            metadata.workspace_members.contains(&p.id)
                && package
                    .map(|name| p.name == name)
                    .unwrap_or(!root_is_package || p.manifest_path == manifest)
        })
        .collect();
    if packages.is_empty() {
        return Err(Error::Invalid("no matching workspace package".into()));
    }
    let mut analysis = Analysis {
        schema_version: 1,
        source_root,
        packages: vec![],
        modules: vec![],
        items: vec![],
        imports: vec![],
        unlinked_rust_files: vec![],
        diagnostics: vec![],
        complete: true,
        limitations: vec![
            "Static Rust syntax observations, not compiler-resolved symbols, runtime call graphs, or C4 boundaries.",
            "Cargo metadata --no-deps --offline --locked supplies production targets and declared dependencies; transitive dependency resolution is omitted.",
            "#[cfg(test)] modules and #[test]/#[tokio::test]/#[async_std::test] items are skipped; other cfg predicates are retained without evaluation.",
            "Public means the written visibility, not reachability through re-exports or private parent modules.",
            "Call expressions preserve syntax; aliases, method dispatch, trait resolution, function pointers, and macro expansion need agent review.",
            "Unlinked files may be tests, examples, build inputs, or conditionally/generated modules; do not assume they are dead code.",
        ].into_iter().map(String::from).collect(),
    };
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut visited = BTreeSet::new();
    for package in packages {
        let package_dir = package
            .manifest_path
            .parent()
            .ok_or_else(|| Error::Invalid("package manifest has no parent".into()))?;
        let mut targets = Vec::new();
        for target in package.targets {
            if target
                .kind
                .iter()
                .any(|kind| matches!(kind.as_str(), "test" | "bench" | "example" | "custom-build"))
            {
                continue;
            }
            let relative = relative(&analysis.source_root, &target.src_path)?;
            let target_id = format!("{}:{}", target.kind.join("+"), target.name);
            let mut walker = RustWalker {
                parser: &mut parser,
                analysis: &mut analysis,
                visited: &mut visited,
                active: BTreeSet::new(),
                package: &package.name,
                target: &target_id,
            };
            walker.file(&relative, "crate", &[], 0);
            targets.push(Target {
                name: target.name,
                kind: target.kind,
                source: relative,
            });
        }
        let inventory = source::scan(package_dir, 5000, source::MAX_SOURCE_BYTES)?;
        if inventory.truncated || !inventory.skipped.is_empty() {
            analysis.diagnostics.push(format!(
                "{}: file inventory incomplete (truncated: {}, skipped: {})",
                package.name,
                inventory.truncated,
                inventory.skipped.len()
            ));
        }
        for file in inventory.files {
            if file.path.ends_with(".rs") {
                let relative = relative(&analysis.source_root, &package_dir.join(&file.path))?;
                if !visited.contains(&relative) {
                    analysis.unlinked_rust_files.push(relative);
                }
            }
        }
        analysis.packages.push(Package {
            name: package.name,
            manifest: relative(&analysis.source_root, &package.manifest_path)?,
            edition: package.edition,
            targets,
            dependencies: package.dependencies,
            features: package.features,
        });
    }
    // A workspace package can also appear under a scanned ancestor package directory.
    analysis
        .unlinked_rust_files
        .retain(|path| !visited.contains(path));
    analysis.unlinked_rust_files.sort();
    analysis.unlinked_rust_files.dedup();
    analysis.packages.sort_by(|a, b| a.name.cmp(&b.name));
    analysis.complete = analysis.diagnostics.is_empty();
    Ok(analysis)
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        Error::Invalid(format!(
            "{} is outside source root {}",
            path.display(),
            root.display()
        ))
    })?;
    let relative = relative
        .to_str()
        .ok_or_else(|| Error::Invalid("non-UTF-8 Rust source path".into()))?
        .replace('\\', "/");
    source::validate_relative(&relative)?;
    Ok(relative)
}

struct RustWalker<'a> {
    parser: &'a mut Parser,
    analysis: &'a mut Analysis,
    visited: &'a mut BTreeSet<String>,
    active: BTreeSet<String>,
    package: &'a str,
    target: &'a str,
}

struct Context<'a> {
    file: &'a str,
    text: &'a str,
    digest: &'a str,
    module: &'a str,
    module_dir: &'a Path,
    file_scope: bool,
    owner: Option<&'a str>,
    attributes: &'a [String],
    depth: usize,
}

impl RustWalker<'_> {
    fn file(&mut self, file: &str, module: &str, attributes: &[String], depth: usize) {
        if depth > 32 || self.active.contains(file) {
            self.analysis
                .diagnostics
                .push(format!("{file}: recursive or excessively deep module tree"));
            return;
        }
        let bytes = match source::source_bytes(&self.analysis.source_root, file) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.analysis.diagnostics.push(error.to_string());
                return;
            }
        };
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(error) => {
                self.analysis.diagnostics.push(format!("{file}: {error}"));
                return;
            }
        };
        let Some(tree) = self.parser.parse(text, None) else {
            self.analysis
                .diagnostics
                .push(format!("{file}: Rust parser returned no syntax tree"));
            return;
        };
        if tree.root_node().has_error() {
            self.analysis.diagnostics.push(format!(
                "{file}: Rust syntax errors; recovered items are incomplete"
            ));
        }
        let hash = source::digest(&bytes);
        let path = Path::new(file);
        let parent = path.parent().unwrap_or(Path::new(""));
        let module_dir = if depth == 0 || path.file_name().is_some_and(|name| name == "mod.rs") {
            parent.to_owned()
        } else {
            parent.join(path.file_stem().unwrap_or_default())
        };
        self.analysis.modules.push(Module {
            package: self.package.into(),
            target: self.target.into(),
            module: module.into(),
            source: SourceRef {
                path: file.into(),
                start_line: 1,
                end_line: text.lines().count().max(1) as u64,
                sha256: hash.clone(),
            },
            attributes: attributes.to_vec(),
        });
        self.visited.insert(file.into());
        self.active.insert(file.into());
        self.declarations(
            tree.root_node(),
            &Context {
                file,
                text,
                digest: &hash,
                module,
                module_dir: &module_dir,
                file_scope: true,
                owner: None,
                attributes,
                depth,
            },
        );
        self.active.remove(file);
    }

    fn declarations(&mut self, node: Node<'_>, context: &Context<'_>) {
        if context.depth > 32 {
            self.analysis.diagnostics.push(format!(
                "{}: inline module tree exceeds 32 levels",
                context.file
            ));
            return;
        }
        let mut attributes = Vec::new();
        for child in node.named_children(&mut node.walk()) {
            if child.kind() == "attribute_item" {
                attributes.push(node_text(child, context.text).into());
                continue;
            }
            if child.is_extra() {
                continue;
            }
            let own_attributes = std::mem::take(&mut attributes);
            if is_test(&own_attributes) {
                continue;
            }
            let mut inherited = context.attributes.to_vec();
            inherited.extend(own_attributes.iter().cloned());
            let context = Context {
                attributes: &inherited,
                ..*context
            };
            match child.kind() {
                "mod_item" => self.module(child, &context, &own_attributes),
                "use_declaration" | "extern_crate_declaration" => {
                    self.analysis.imports.push(Import {
                        package: self.package.into(),
                        target: self.target.into(),
                        module: context.module.into(),
                        expression: node_text(child, context.text).into(),
                        attributes: context.attributes.to_vec(),
                        source: citation(child, &context),
                    })
                }
                "impl_item" | "trait_item" => {
                    if child.kind() == "trait_item" {
                        self.item(child, &context);
                    }
                    if let Some(body) = child.child_by_field_name("body") {
                        let owner = context.text[child.start_byte()..body.start_byte()]
                            .trim()
                            .to_string();
                        self.declarations(
                            body,
                            &Context {
                                owner: Some(&owner),
                                ..context
                            },
                        );
                    }
                }
                "function_item"
                | "function_signature_item"
                | "struct_item"
                | "enum_item"
                | "type_item"
                | "const_item"
                | "static_item"
                | "union_item" => self.item(child, &context),
                "macro_invocation" => self.analysis.diagnostics.push(format!(
                    "{}:{}: item macro is not expanded",
                    context.file,
                    child.start_position().row + 1
                )),
                _ => {}
            }
        }
    }

    fn module(&mut self, node: Node<'_>, context: &Context<'_>, attributes: &[String]) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name, context.text);
        let module = format!("{}::{name}", context.module);
        let explicit = attributes.iter().find_map(|attribute| {
            let compact: String = attribute.chars().filter(|c| !c.is_whitespace()).collect();
            compact
                .starts_with("#[path=")
                .then(|| attribute.split('"').nth(1))
                .flatten()
        });
        let explicit_base = if context.file_scope {
            Path::new(context.file).parent().unwrap_or(Path::new(""))
        } else {
            context.module_dir
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.analysis.modules.push(Module {
                package: self.package.into(),
                target: self.target.into(),
                module: module.clone(),
                source: citation(node, context),
                attributes: context.attributes.to_vec(),
            });
            let directory = explicit
                .map(|path| explicit_base.join(path))
                .unwrap_or_else(|| context.module_dir.join(name));
            self.declarations(
                body,
                &Context {
                    module: &module,
                    module_dir: &directory,
                    file_scope: false,
                    depth: context.depth + 1,
                    ..*context
                },
            );
            return;
        }
        if attributes
            .iter()
            .any(|attribute| attribute.contains("cfg_attr") && attribute.contains("path"))
        {
            self.analysis.diagnostics.push(format!(
                "{}: conditional #[path] for {module} requires review",
                context.file
            ));
            return;
        }
        let candidates = if let Some(path) = explicit {
            // At file scope, #[path] is relative to the declaring file, not foo/ for foo.rs.
            vec![explicit_base.join(path)]
        } else {
            vec![
                context.module_dir.join(format!("{name}.rs")),
                context.module_dir.join(name).join("mod.rs"),
            ]
        };
        let existing: Vec<_> = candidates
            .iter()
            .filter(|path| self.analysis.source_root.join(path).exists())
            .collect();
        if existing.len() != 1 {
            self.analysis.diagnostics.push(format!(
                "{}: module {module} has {} matching source files (expected one)",
                context.file,
                existing.len()
            ));
            return;
        }
        let Some(path) = existing[0].to_str() else {
            self.analysis
                .diagnostics
                .push("non-UTF-8 module path".into());
            return;
        };
        let path = path.replace(std::path::MAIN_SEPARATOR, "/");
        self.file(&path, &module, context.attributes, context.depth + 1);
    }

    fn item(&mut self, node: Node<'_>, context: &Context<'_>) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let visibility = node
            .named_children(&mut node.walk())
            .find(|child| child.kind() == "visibility_modifier")
            .map(|n| node_text(n, context.text))
            .unwrap_or(
                if context.owner.is_some_and(|owner| owner.contains("trait ")) {
                    "trait member"
                } else {
                    "private"
                },
            );
        let body = node.child_by_field_name("body");
        let signature_end = body
            .map(|body| body.start_byte())
            .unwrap_or(node.end_byte());
        let signature = context.text[node.start_byte()..signature_end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let mut calls = Vec::new();
        if matches!(node.kind(), "function_item" | "function_signature_item") {
            let mut stack = vec![node];
            while let Some(current) = stack.pop() {
                if current.kind() == "call_expression"
                    && let Some(function) = current.child_by_field_name("function")
                {
                    calls.push(Call {
                        expression: callee(function, context.text, 0),
                        line: (current.start_position().row + 1) as u64,
                    });
                }
                stack.extend(current.named_children(&mut current.walk()));
            }
            calls.sort_by_key(|call| call.line);
        }
        self.analysis.items.push(Item {
            package: self.package.into(),
            target: self.target.into(),
            module: context.module.into(),
            name: node_text(name, context.text).into(),
            kind: node.kind().trim_end_matches("_item").into(),
            owner: context.owner.map(str::to_owned),
            visibility: visibility.into(),
            signature,
            attributes: context.attributes.to_vec(),
            source: citation(node, context),
            calls,
        });
    }
}

fn node_text<'a>(node: Node<'_>, text: &'a str) -> &'a str {
    &text[node.byte_range()]
}

// Retain the callee while omitting argument/closure bodies from receiver chains.
fn callee(node: Node<'_>, text: &str, depth: usize) -> String {
    if depth > 8 {
        return "…".into();
    }
    match node.kind() {
        "field_expression" => {
            let receiver = node
                .child_by_field_name("value")
                .map(|n| callee(n, text, depth + 1))
                .unwrap_or_else(|| "_".into());
            let field = node
                .child_by_field_name("field")
                .map(|n| node_text(n, text))
                .unwrap_or("_");
            format!("{receiver}.{field}")
        }
        "call_expression" => node
            .child_by_field_name("function")
            .map(|n| format!("{}(...)", callee(n, text, depth + 1)))
            .unwrap_or_else(|| "_".into()),
        "generic_function" => node
            .child_by_field_name("function")
            .map(|n| callee(n, text, depth + 1))
            .unwrap_or_else(|| "_".into()),
        "identifier"
        | "scoped_identifier"
        | "scoped_type_identifier"
        | "type_identifier"
        | "self" => node_text(node, text).chars().take(200).collect(),
        _ => "_".into(),
    }
}

fn citation(node: Node<'_>, context: &Context<'_>) -> SourceRef {
    SourceRef {
        path: context.file.into(),
        start_line: (node.start_position().row + 1) as u64,
        end_line: (node.end_position().row + usize::from(node.end_position().column > 0))
            .max(node.start_position().row + 1) as u64,
        sha256: context.digest.into(),
    }
}

fn is_test(attributes: &[String]) -> bool {
    attributes.iter().any(|attribute| {
        let compact: String = attribute.chars().filter(|c| !c.is_whitespace()).collect();
        matches!(
            compact.as_str(),
            "#[test]" | "#[tokio::test]" | "#[async_std::test]" | "#[cfg(test)]"
        ) || compact.starts_with("#[tokio::test(")
            || compact.starts_with("#[async_std::test(")
    })
}
