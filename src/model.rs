use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub schema_version: u32,
    pub title: String,
    pub description: String,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    pub views: Vec<View>,
    #[serde(default)]
    pub questions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Person,
    System,
    Container,
    Component,
    Code,
    DeploymentNode,
    InfrastructureNode,
    Instance,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::System => "Software system",
            Self::Container => "Container",
            Self::Component => "Component",
            Self::Code => "Code",
            Self::DeploymentNode => "Deployment node",
            Self::InfrastructureNode => "Infrastructure node",
            Self::Instance => "Instance",
        }
    }

    pub fn is_deployment(self) -> bool {
        matches!(
            self,
            Self::DeploymentNode | Self::InfrastructureNode | Self::Instance
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Element {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technology: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    pub evidence: Evidence,
    /// Implementation locations selected by the author for this component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_mapping: Option<CodeMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_access: Option<CodeAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<ReplicaRange>,
    /// Named viewpoints and their values, such as owner or security classification.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub perspectives: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeMapping {
    /// Which part of the component's implementation was inspected.
    pub coverage: String,
    pub entries: Vec<CodeMappingEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodeAccess {
    Public,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplicaRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeMappingEntry {
    pub source: SourceRef,
    /// How this location contributes to the component's responsibility.
    pub rationale: String,
}

impl Element {
    pub fn mapped_entries(&self) -> &[CodeMappingEntry] {
        self.code_mapping.as_ref().map_or(&[], |m| &m.entries)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Evidence {
    Observed { sources: Vec<SourceRef> },
    Inferred { reason: String },
}

impl Evidence {
    pub fn sources(&self) -> &[SourceRef] {
        match self {
            Self::Observed { sources } => sources,
            Self::Inferred { .. } => &[],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Observed { .. } => "Observed",
            Self::Inferred { .. } => "Inferred",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    /// UTF-8 repository-relative path, using forward slashes. No symlinks.
    pub path: String,
    /// Inclusive, one-based line numbers.
    pub start_line: u64,
    pub end_line: u64,
    /// SHA-256 of the complete file bytes, produced by `cargo c4doc cite`.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    pub id: String,
    pub from: String,
    pub to: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technology: Option<String>,
    pub evidence: Evidence,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub perspectives: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Context,
    Container,
    Component,
    Code,
    Landscape,
    Dynamic,
    Deployment,
}

impl ViewKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Context => "System context",
            Self::Container => "Container",
            Self::Component => "Component",
            Self::Code => "Code",
            Self::Landscape => "System landscape",
            Self::Dynamic => "Dynamic",
            Self::Deployment => "Deployment",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub kind: ViewKind,
    pub title: String,
    pub description: String,
    /// Supporting paragraphs shown on the view page, after the diagram.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagram_notes: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub show_system_boundary: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lifelines: BTreeMap<String, Lifeline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    pub elements: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<Perspective>,
    #[serde(default)]
    pub relationships: Vec<String>,
    #[serde(default)]
    pub steps: Vec<ScenarioItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Perspective {
    pub key: String,
    /// Empty selects the whole view. Otherwise include elements with any listed value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lifeline {
    pub element: String,
    pub name: String,
}

impl View {
    pub(crate) fn projected(
        &self,
        elements: &BTreeMap<&str, &Element>,
        relationships: &BTreeMap<&str, &Relationship>,
    ) -> Cow<'_, Self> {
        let Some(p) = self.perspective.as_ref().filter(|p| !p.values.is_empty()) else {
            return Cow::Borrowed(self);
        };
        let mut filtered = self.clone();
        let matching: BTreeSet<_> = self
            .elements
            .iter()
            .map(String::as_str)
            .filter(|id| elements.get(id).is_some_and(|e| p.matches(&e.perspectives)))
            .collect();
        let mut visible = matching.clone();
        filtered.relationships.retain(|id| {
            relationships.get(id.as_str()).is_some_and(|r| {
                if p.matches(&r.perspectives) {
                    visible.extend([r.from.as_str(), r.to.as_str()]);
                    true
                } else {
                    matching.contains(r.from.as_str()) && matching.contains(r.to.as_str())
                }
            })
        });
        if self.kind == ViewKind::Deployment {
            for id in visible.clone() {
                let mut parent = elements.get(id).and_then(|e| e.parent.as_deref());
                while let Some(id) = parent {
                    if !visible.insert(id) {
                        break;
                    }
                    parent = elements.get(id).and_then(|e| e.parent.as_deref());
                }
            }
        }
        filtered.elements.retain(|id| visible.contains(id.as_str()));
        Cow::Owned(filtered)
    }
}

impl Perspective {
    pub fn matches(&self, tags: &BTreeMap<String, Vec<String>>) -> bool {
        self.values.is_empty()
            || tags
                .get(&self.key)
                .is_some_and(|labels| labels.iter().any(|v| self.values.contains(v)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ScenarioItem {
    Interaction(Step),
    Block(ScenarioBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioBlock {
    pub kind: ControlKind,
    pub branches: Vec<ScenarioBranch>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    Optional,
    Loop,
    Alternative,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioBranch {
    pub label: String,
    pub steps: Vec<ScenarioItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// References a static relationship; reuse it for repeated interactions.
    pub relationship: String,
    pub description: String,
    /// A response on the existing relationship, in the reverse direction.
    #[serde(default)]
    pub response: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_lifeline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_lifeline: Option<String>,
}

impl Step {
    pub fn logical_endpoints<'a>(&self, relationship: &'a Relationship) -> (&'a str, &'a str) {
        if self.response {
            (&relationship.to, &relationship.from)
        } else {
            (&relationship.from, &relationship.to)
        }
    }
    pub fn participants<'a>(&'a self, relationship: &'a Relationship) -> (&'a str, &'a str) {
        let (from, to) = self.logical_endpoints(relationship);
        (
            self.from_lifeline.as_deref().unwrap_or(from),
            self.to_lifeline.as_deref().unwrap_or(to),
        )
    }
}

pub fn starter(title: String) -> Model {
    Model {
        schema_version: 1,
        title: title.clone(),
        description: "Draft: describe the system's purpose after inspecting its code and documentation.".into(),
        elements: vec![Element {
            id: "system".into(), kind: Kind::System, name: title,
            description: "Draft system boundary; confirm responsibilities and ownership.".into(),
            technology: None, parent: None, instance_of: None, environment: None, code_mapping: None, code_access: None, replicas: None, perspectives: BTreeMap::new(),
            evidence: Evidence::Inferred { reason: "Initialized from the requested title; no repository research has been performed.".into() },
        }],
        relationships: vec![],
        views: vec![View {
            id: "context".into(), kind: ViewKind::Context, title: "System context".into(),
            description: "Identify people and external systems, then add supported relationships.".into(),
            notes: vec![], diagram_notes: vec![], lifelines: BTreeMap::new(), show_system_boundary: false, perspective: None,
            scope: Some("system".into()), environment: None,
            elements: vec!["system".into()], relationships: vec![], steps: vec![],
        }],
        questions: vec!["Who uses the system, what value does it provide, and which external systems does it interact with?".into()],
    }
}
