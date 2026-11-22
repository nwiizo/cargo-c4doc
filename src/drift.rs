//! Observation changes are review candidates, not inferred C4 boundaries or violations.
use crate::{
    Error, Result,
    model::{Kind, Model},
    rust_analysis::Analysis,
    source::{self, Inventory},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    CargoTarget,
    PackageMetadata,
    RustSource,
    UnlinkedRust,
    AuxiliaryFile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Serialize)]
pub struct Change {
    pub category: Category,
    pub change: ChangeKind,
    pub key: String,
    pub paths: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_sha256: Option<String>,
    pub cited_claims: BTreeSet<String>,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub mapped_components: BTreeSet<String>,
    pub requires_review: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuxiliaryCoverage {
    NotSupplied,
    Compared,
    Incomplete,
}

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub rust: bool,
    pub auxiliary: AuxiliaryCoverage,
    pub model_citations: bool,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub before_root: PathBuf,
    pub after_root: PathBuf,
    pub packages: Vec<String>,
    pub complete: bool,
    pub review_required: bool,
    pub coverage: Coverage,
    pub changes: Vec<Change>,
    pub diagnostics: Vec<String>,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    paths: BTreeSet<String>,
    fingerprint: String,
    sha256: Option<String>,
    requires_review: bool,
}

type Observations = BTreeMap<(Category, String), Observation>;
type Citations = BTreeMap<String, BTreeSet<String>>;

#[derive(Default)]
struct ModelPaths {
    claims: Citations,
    components: Citations,
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&source::read_bounded(
        path,
        32 * 1024 * 1024,
    )?)?)
}

fn version(version: u32) -> Result<()> {
    if version != 1 {
        return Err(Error::Invalid(format!(
            "unsupported observation/model schema_version: {version}"
        )));
    }
    Ok(())
}

fn file_observation(path: &str, sha256: &str, requires_review: bool) -> Result<Observation> {
    source::validate_relative(path)?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Invalid(format!("{path}: invalid SHA-256")));
    }
    let sha256 = sha256.to_ascii_lowercase();
    Ok(Observation {
        paths: BTreeSet::from([path.into()]),
        fingerprint: sha256.clone(),
        sha256: Some(sha256),
        requires_review,
    })
}

fn observations(analysis: &mut Analysis) -> Result<Observations> {
    version(analysis.schema_version)?;
    if analysis.packages.is_empty() {
        return Err(Error::Invalid("analysis has no selected packages".into()));
    }
    let mut observations = Observations::new();
    for package in &mut analysis.packages {
        source::validate_relative(&package.manifest)?;
        // Declaration order does not change the architecture. Retain all dependency flags.
        for dependency in &mut package.dependencies {
            dependency.features.sort();
        }
        package.dependencies.sort();
        for values in package.features.values_mut() {
            values.sort();
        }
        let entry = Observation {
            paths: BTreeSet::from([package.manifest.clone()]),
            fingerprint: serde_json::to_string(&(
                &package.edition,
                &package.dependencies,
                &package.features,
            ))?,
            sha256: None,
            requires_review: true,
        };
        if observations
            .insert((Category::PackageMetadata, package.name.clone()), entry)
            .is_some()
        {
            return Err(Error::Invalid(format!(
                "duplicate package: {}",
                package.name
            )));
        }
        for target in &mut package.targets {
            source::validate_relative(&target.source)?;
            target.kind.sort();
            let key = format!("{}:{}:{}", package.name, target.kind.join("+"), target.name);
            let entry = Observation {
                paths: BTreeSet::from([target.source.clone()]),
                fingerprint: target.source.clone(),
                sha256: None,
                requires_review: true,
            };
            if observations
                .insert((Category::CargoTarget, key.clone()), entry)
                .is_some()
            {
                return Err(Error::Invalid(format!("duplicate Cargo target: {key}")));
            }
        }
    }
    for module in &analysis.modules {
        let source = &module.source;
        let entry = file_observation(&source.path, &source.sha256, true)?;
        let key = (Category::RustSource, source.path.clone());
        if let Some(previous) = observations.insert(key, entry)
            && previous.sha256.as_deref() != Some(&source.sha256.to_ascii_lowercase())
        {
            return Err(Error::Invalid(format!(
                "{}: conflicting module source hashes",
                source.path
            )));
        }
    }
    if analysis.complete && analysis.diagnostics.is_empty() {
        for target in analysis.packages.iter().flat_map(|p| &p.targets) {
            if !observations.contains_key(&(Category::RustSource, target.source.clone())) {
                return Err(Error::Invalid(format!(
                    "{}: complete analysis is missing a target's root module",
                    target.source
                )));
            }
        }
    }
    for path in &analysis.unlinked_rust_files {
        source::validate_relative(path)?;
        if observations.contains_key(&(Category::RustSource, path.clone())) {
            return Err(Error::Invalid(format!(
                "{path}: both reachable and unlinked Rust source"
            )));
        }
        observations.insert(
            (Category::UnlinkedRust, path.clone()),
            Observation {
                paths: BTreeSet::from([path.clone()]),
                fingerprint: String::new(),
                sha256: None,
                requires_review: false,
            },
        );
    }
    Ok(observations)
}

fn scan_observations(inventory: &Inventory, rust: &Observations) -> Result<Observations> {
    version(inventory.schema_version)?;
    let mut observations = Observations::new();
    let mut seen = BTreeSet::new();
    for file in &inventory.files {
        if !seen.insert(&file.path) {
            return Err(Error::Invalid(format!(
                "{}: duplicate inventory path",
                file.path
            )));
        }
        let entry = file_observation(
            &file.path,
            &file.sha256,
            file.language != "documentation" && file.language != "rust",
        )?;
        if let Some(source) = rust.get(&(Category::RustSource, file.path.clone())) {
            if source.sha256 != entry.sha256 {
                return Err(Error::Invalid(format!(
                    "{}: scan and analyze source hashes disagree",
                    file.path
                )));
            }
            continue;
        }
        let category = if file.language == "rust" {
            Category::UnlinkedRust
        } else {
            Category::AuxiliaryFile
        };
        observations.insert((category, file.path.clone()), entry);
    }
    for (category, path) in rust.keys() {
        if *category == Category::RustSource && !seen.contains(path) {
            return Err(Error::Invalid(format!(
                "{path}: reachable Rust source missing from scan; use analyze's source_root"
            )));
        }
    }
    Ok(observations)
}

fn model_paths(model: Option<&Path>) -> Result<ModelPaths> {
    let mut index = ModelPaths::default();
    let Some(path) = model else {
        return Ok(index);
    };
    let model: Model = read_json(path)?;
    version(model.schema_version)?;
    let claims = model
        .elements
        .iter()
        .map(|e| (format!("element:{}", e.id), &e.evidence))
        .chain(
            model
                .relationships
                .iter()
                .map(|r| (format!("relationship:{}", r.id), &r.evidence)),
        );
    for (id, evidence) in claims {
        for source in evidence.sources() {
            source::validate_relative(&source.path)?;
            index
                .claims
                .entry(source.path.clone())
                .or_default()
                .insert(id.clone());
        }
    }
    for element in &model.elements {
        if element.code_mapping.is_some() && element.kind != Kind::Component {
            return Err(Error::Invalid(format!(
                "element:{}: code_mapping is only valid for components",
                element.id
            )));
        }
        for entry in element.mapped_entries() {
            source::validate_relative(&entry.source.path)?;
            index
                .components
                .entry(entry.source.path.clone())
                .or_default()
                .insert(element.id.clone());
        }
    }
    Ok(index)
}

fn changes(before: &Observations, after: &Observations, index: &ModelPaths) -> Vec<Change> {
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
    keys.into_iter()
        .filter_map(|(category, key)| {
            let old = before.get(&(*category, key.clone()));
            let new = after.get(&(*category, key.clone()));
            if old == new {
                return None;
            }
            let change = match (old, new) {
                (None, Some(_)) => ChangeKind::Added,
                (Some(_), None) => ChangeKind::Removed,
                _ => ChangeKind::Modified,
            };
            let paths: BTreeSet<_> = old
                .into_iter()
                .chain(new)
                .flat_map(|o| o.paths.iter().cloned())
                .collect();
            let matching = |entries: &Citations| {
                paths
                    .iter()
                    .filter_map(|p| entries.get(p))
                    .flatten()
                    .cloned()
                    .collect::<BTreeSet<String>>()
            };
            let cited_claims = matching(&index.claims);
            let mapped_components = matching(&index.components);
            Some(Change {
                category: *category,
                change,
                key: key.clone(),
                paths,
                before_sha256: old.and_then(|o| o.sha256.clone()),
                after_sha256: new.and_then(|o| o.sha256.clone()),
                cited_claims,
                mapped_components,
                requires_review: old.into_iter().chain(new).any(|o| o.requires_review),
            })
        })
        .collect()
}

/// Compare saved observations without executing the repository or touching model/output files.
pub fn compare_files(
    before: &Path,
    after: &Path,
    model: Option<&Path>,
    scans: Option<(&Path, &Path)>,
) -> Result<Report> {
    let mut before: Analysis = read_json(before)?;
    let mut after: Analysis = read_json(after)?;
    let mut old = observations(&mut before)?;
    let mut new = observations(&mut after)?;
    let index = model_paths(model)?;
    let scope = |analysis: &Analysis| -> BTreeSet<_> {
        analysis
            .packages
            .iter()
            .map(|p| (p.name.clone(), p.manifest.clone()))
            .collect()
    };
    let mut report = Report {
        schema_version: 1,
        before_root: before.source_root.clone(),
        after_root: after.source_root.clone(),
        packages: scope(&after).into_iter().map(|(name, _)| name).collect(),
        complete: true,
        review_required: false,
        coverage: Coverage {
            rust: false,
            auxiliary: AuxiliaryCoverage::NotSupplied,
            model_citations: model.is_some(),
        },
        changes: vec![],
        diagnostics: vec![],
        limitations: vec![
            "Changes are review candidates, not proven new features, C4 boundaries, architecture violations, or complete model coverage.",
            "Model annotations match whole paths in claim citations and component code mappings. Run check separately to validate source ranges and digests.",
            "Rust comparison covers targets, recorded package metadata and whole-file hashes of reachable modules. It does not resolve routes, calls, macros, cfg variants, or dependency versions absent from analyze output.",
            "Unlinked Rust and documentation changes are informational, not review-gate failures. Unlinked does not mean unused or deliberately excluded; inspect these entries for relevant build/generated code.",
            "Auxiliary coverage requires paired scans from analyze's source_root with equal limits. Scan v1 does not record its root; matching Rust hashes cannot prove non-Rust input completeness or equal ignore rules.",
            "Browser, configuration, deployment, and other auxiliary files are compared by path/hash only. Hidden, ignored, unsupported, and pruned files remain outside inventory coverage.",
        ],
    };
    for (label, analysis) in [("before", &before), ("after", &after)] {
        if !analysis.complete || !analysis.diagnostics.is_empty() {
            report.diagnostics.push(format!(
                "{label}: Rust analysis incomplete; recollect before comparison"
            ));
            report
                .diagnostics
                .extend(analysis.diagnostics.iter().map(|d| format!("{label}: {d}")));
        }
    }
    if before.source_root != after.source_root {
        report.diagnostics.push(
            "source_root differs; compare snapshots collected from the same checkout path".into(),
        );
    }
    if scope(&before) != scope(&after) {
        report.diagnostics.push(format!("package selection differs (before: {:?}; after: {:?}); review package additions/removals and recollect matching scopes", scope(&before), scope(&after)));
    }
    report.coverage.rust = report.diagnostics.is_empty();
    if let Some((before_scan, after_scan)) = scans {
        let a: Inventory = read_json(before_scan)?;
        let b: Inventory = read_json(after_scan)?;
        version(a.schema_version)?;
        version(b.schema_version)?;
        report.coverage.auxiliary = AuxiliaryCoverage::Incomplete;
        if a.truncated || b.truncated || !a.skipped.is_empty() || !b.skipped.is_empty() {
            report.diagnostics.push(format!("auxiliary scan incomplete: before truncated={}, skipped={:?}; after truncated={}, skipped={:?}", a.truncated, a.skipped, b.truncated, b.skipped));
        } else if (a.max_files, a.max_bytes) != (b.max_files, b.max_bytes) {
            report
                .diagnostics
                .push("auxiliary scan limits differ; recollect with matching limits".into());
        } else if report.coverage.rust {
            old.extend(scan_observations(&a, &old)?);
            new.extend(scan_observations(&b, &new)?);
            report.coverage.auxiliary = AuxiliaryCoverage::Compared;
        }
    }
    if report.coverage.rust {
        report.changes = changes(&old, &new, &index);
    }
    report.complete = report.diagnostics.is_empty();
    report.review_required = !report.complete || report.changes.iter().any(|c| c.requires_review);
    Ok(report)
}
