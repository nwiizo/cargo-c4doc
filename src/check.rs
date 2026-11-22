use crate::{model::*, source};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub location: String,
    pub message: String,
}

impl Report {
    pub fn passes(&self, strict: bool) -> bool {
        self.errors.is_empty() && (!strict || self.warnings.is_empty())
    }

    fn require(&mut self, condition: bool, location: &str, message: impl Into<String>) {
        if !condition {
            self.errors.push(Diagnostic {
                location: location.into(),
                message: message.into(),
            });
        }
    }

    fn warn(&mut self, location: &str, message: impl Into<String>) {
        self.warnings.push(Diagnostic {
            location: location.into(),
            message: message.into(),
        });
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn nonempty(text: &str) -> bool {
    !text.trim().is_empty()
}

type Elements<'a> = BTreeMap<&'a str, &'a Element>;
type SourceCache = BTreeMap<String, std::result::Result<(String, u64), String>>;

pub fn validate(model: &Model, root: &Path) -> Report {
    let mut report = Report::default();
    report.require(root.is_dir(), "root", "source root must be a directory");
    report.require(
        model.schema_version == 1,
        "schema_version",
        "only schema_version 1 is supported",
    );
    report.require(nonempty(&model.title), "title", "title is required");
    report.require(
        nonempty(&model.description),
        "description",
        "description is required",
    );
    report.require(
        !model.elements.is_empty(),
        "elements",
        "at least one element is required",
    );
    report.require(
        !model.views.is_empty(),
        "views",
        "at least one view is required",
    );
    let elements: Elements<'_> = model.elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let relationships: BTreeMap<_, _> = model
        .relationships
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    report.require(
        elements.len() == model.elements.len(),
        "elements",
        "duplicate element ID",
    );
    report.require(
        relationships.len() == model.relationships.len(),
        "relationships",
        "duplicate relationship ID",
    );
    let mut sources = SourceCache::new();
    for element in &model.elements {
        let loc = format!("element:{}", element.id);
        report.require(
            valid_id(&element.id),
            &loc,
            "ID must match [a-z][a-z0-9_]{0,63}",
        );
        report.require(
            nonempty(&element.name) && nonempty(&element.description),
            &loc,
            "name and responsibility description are required",
        );
        validate_element(element, &elements, &loc, &mut report);
        for (key, values) in &element.perspectives {
            report.require(
                nonempty(key) && !values.is_empty() && unique_labels(values),
                &loc,
                "perspectives require a name and distinct nonblank values",
            );
        }
        validate_evidence(&element.evidence, root, &loc, &mut sources, &mut report);
        if let Some(mapping) = &element.code_mapping {
            let loc = format!("{loc}.code_mapping");
            report.require(
                element.kind == Kind::Component,
                &loc,
                "code_mapping is only valid for components",
            );
            report.require(
                nonempty(&mapping.coverage),
                &loc,
                "describe the mapping coverage",
            );
            report.require(
                !mapping.entries.is_empty(),
                &loc,
                "map at least one implementation location",
            );
            for (i, entry) in mapping.entries.iter().enumerate() {
                let loc = format!("{loc}.entries[{i}]");
                report.require(
                    nonempty(&entry.rationale),
                    &loc,
                    "describe how this source contributes to the component",
                );
                validate_citation(&entry.source, root, &loc, &mut sources, &mut report);
            }
        }
    }
    for relationship in &model.relationships {
        let loc = format!("relationship:{}", relationship.id);
        for (key, values) in &relationship.perspectives {
            report.require(
                nonempty(key) && !values.is_empty() && unique_labels(values),
                &loc,
                "perspectives require a name and distinct nonblank values",
            );
        }
        report.require(
            valid_id(&relationship.id),
            &loc,
            "ID must match [a-z][a-z0-9_]{0,63}",
        );
        report.require(
            nonempty(&relationship.description),
            &loc,
            "describe the interaction, not just 'uses'",
        );
        if matches!(
            relationship
                .description
                .trim()
                .trim_end_matches(['.', '。'])
                .trim_end()
                .to_ascii_lowercase()
                .as_str(),
            "uses" | "depends on" | "communicates with" | "利用する" | "使用する" | "依存する"
        ) {
            report.warn(
                &loc,
                format!(
                    "Describe what is requested or exchanged from {} to {}.",
                    relationship.from, relationship.to
                ),
            );
        }
        report.require(
            relationship.technology.as_ref().is_none_or(|t| nonempty(t)),
            &loc,
            "technology must not be blank",
        );
        let from = elements.get(relationship.from.as_str());
        let to = elements.get(relationship.to.as_str());
        report.require(
            from.is_some() && to.is_some(),
            &loc,
            "both endpoints must exist",
        );
        if let (Some(from), Some(to)) = (from, to) {
            report.require(from.kind.is_deployment() == to.kind.is_deployment(), &loc, "logical elements and deployment instances cannot be endpoints of the same relationship");
            if from.kind.is_deployment() {
                report.require(
                    from.environment == to.environment,
                    &loc,
                    "deployment relationship crosses environments",
                );
                report.require(
                    from.kind != Kind::DeploymentNode && to.kind != Kind::DeploymentNode,
                    &loc,
                    "connect instances or infrastructure nodes, not hosting boundaries",
                );
            }
            if from.kind == Kind::Container
                && to.kind == Kind::Container
                && relationship.technology.is_none()
            {
                report.warn(&loc, "container interaction has no protocol or technology");
            }
        }
        validate_evidence(
            &relationship.evidence,
            root,
            &loc,
            &mut sources,
            &mut report,
        );
    }
    let mut view_ids = BTreeSet::new();
    let mut used = BTreeSet::new();
    for view in &model.views {
        let loc = format!("view:{}", view.id);
        report.require(
            valid_id(&view.id) && !matches!(view.id.as_str(), "evidence" | "readme"),
            &loc,
            "view ID must match [a-z][a-z0-9_]{0,63} and not be 'evidence' or 'readme'",
        );
        report.require(view_ids.insert(&view.id), &loc, "duplicate view ID");
        report.require(
            nonempty(&view.title) && nonempty(&view.description),
            &loc,
            "title and purpose description are required",
        );
        validate_view(view, &elements, &relationships, &loc, &mut report);
        let projected = view.projected(&elements, &relationships);
        if view.kind == ViewKind::Context
            && projected.elements.len() > 1
            && !projected
                .relationships
                .iter()
                .filter_map(|id| relationships.get(id.as_str()))
                .any(|r| {
                    view.scope
                        .as_ref()
                        .is_some_and(|s| s == &r.from || s == &r.to)
                })
        {
            report.warn(&loc, "focal system has no visible interaction");
        }
        used.extend(
            projected
                .elements
                .iter()
                .filter_map(|id| elements.get(id.as_str()))
                .map(|e| e.id.as_str()),
        );
        if !matches!(
            view.kind,
            ViewKind::Dynamic | ViewKind::Deployment | ViewKind::Context
        ) && projected.elements.len() > 1
            && projected.relationships.is_empty()
        {
            report.warn(&loc, "view has multiple elements but no relationships");
        }
        if let Some(scope) = &view.scope {
            used.insert(scope);
        }
    }
    for element in &model.elements {
        // Ancestor boundaries of visible elements are also represented by the model.
        let is_ancestor = used
            .iter()
            .any(|id| has_ancestor(id, &element.id, &elements));
        if !used.contains(element.id.as_str()) && !is_ancestor {
            report.warn(
                &format!("element:{}", element.id),
                "element is not represented by any view",
            );
        }
    }
    for (index, question) in model.questions.iter().enumerate() {
        report.require(
            nonempty(question),
            &format!("questions[{index}]"),
            "question must not be blank",
        );
        report.warn(&format!("questions[{index}]"), question);
    }
    report
}

fn validate_element(element: &Element, elements: &Elements<'_>, loc: &str, report: &mut Report) {
    let parent = element
        .parent
        .as_deref()
        .and_then(|id| elements.get(id))
        .copied();
    report.require(
        element.parent.is_none() || parent.is_some(),
        loc,
        "parent must exist",
    );
    let valid_parent = match element.kind {
        Kind::Person | Kind::System => element.parent.is_none(),
        Kind::Container => parent.is_some_and(|p| p.kind == Kind::System),
        Kind::Component => parent.is_some_and(|p| p.kind == Kind::Container),
        Kind::Code => parent.is_some_and(|p| p.kind == Kind::Component),
        Kind::DeploymentNode => {
            element.parent.is_none() || parent.is_some_and(|p| p.kind == Kind::DeploymentNode)
        }
        Kind::InfrastructureNode | Kind::Instance => {
            parent.is_some_and(|p| p.kind == Kind::DeploymentNode)
        }
    };
    report.require(valid_parent, loc, "invalid parent kind for this C4 element");
    report.require(
        !has_ancestor(&element.id, &element.id, elements),
        loc,
        "parent hierarchy contains a cycle",
    );
    let mut ancestor = parent;
    let mut depth = 0;
    while let Some(current) = ancestor {
        depth += 1;
        if depth > 32 {
            break;
        }
        ancestor = current
            .parent
            .as_deref()
            .and_then(|id| elements.get(id))
            .copied();
    }
    report.require(depth <= 32, loc, "parent hierarchy exceeds 32 levels");
    if element.kind.is_deployment() {
        report.require(
            element.environment.as_deref().is_some_and(nonempty),
            loc,
            "deployment elements require an environment",
        );
        if let Some(parent) = parent {
            report.require(
                element.environment == parent.environment,
                loc,
                "parent and child must belong to the same environment",
            );
        }
    } else {
        report.require(
            element.environment.is_none(),
            loc,
            "logical C4 elements cannot belong to a deployment environment",
        );
    }
    report.require(
        element.code_access.is_none() || element.kind == Kind::Code,
        loc,
        "code_access applies only to code elements",
    );
    if let Some(range) = &element.replicas {
        report.require(
            matches!(element.kind, Kind::DeploymentNode | Kind::Instance) && range.min <= range.max,
            loc,
            "replica range requires a deployment node or instance and min <= max",
        );
    }
    if element.kind == Kind::Instance {
        let target = element
            .instance_of
            .as_deref()
            .and_then(|id| elements.get(id));
        report.require(
            target.is_some_and(|t| matches!(t.kind, Kind::System | Kind::Container)),
            loc,
            "instance_of must reference a system or container",
        );
    } else {
        report.require(
            element.instance_of.is_none(),
            loc,
            "instance_of is only valid for instances",
        );
    }
    if matches!(
        element.kind,
        Kind::Container
            | Kind::Component
            | Kind::Code
            | Kind::DeploymentNode
            | Kind::InfrastructureNode
    ) {
        report.require(
            element.technology.as_deref().is_some_and(nonempty),
            loc,
            "technology is required at this level",
        );
    } else {
        report.require(
            element.technology.as_deref().is_none_or(nonempty),
            loc,
            "technology must not be blank",
        );
    }
}

fn has_ancestor(id: &str, ancestor: &str, elements: &Elements<'_>) -> bool {
    let mut current = elements.get(id).and_then(|e| e.parent.as_deref());
    let mut visited = BTreeSet::new();
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        if !visited.insert(id) {
            break;
        }
        current = elements.get(id).and_then(|e| e.parent.as_deref());
    }
    false
}

fn validate_evidence(
    evidence: &Evidence,
    root: &Path,
    loc: &str,
    cache: &mut SourceCache,
    report: &mut Report,
) {
    match evidence {
        Evidence::Inferred { reason } => {
            report.require(nonempty(reason), loc, "inferred claims require a reason");
            report.warn(loc, format!("inferred: {reason}"));
        }
        Evidence::Observed { sources } => {
            report.require(
                !sources.is_empty(),
                loc,
                "observed claims require at least one source citation",
            );
            for citation in sources {
                validate_citation(citation, root, loc, cache, report);
            }
        }
    }
}

fn validate_citation(
    citation: &SourceRef,
    root: &Path,
    loc: &str,
    cache: &mut SourceCache,
    report: &mut Report,
) {
    let result = cache.entry(citation.path.clone()).or_insert_with(|| {
        source::source_bytes(root, &citation.path)
            .and_then(|bytes| Ok((source::digest(&bytes), source::line_count(&bytes)?)))
            .map_err(|e| e.to_string())
    });
    match result {
        Ok((hash, lines)) => {
            report.require(
                citation.start_line > 0
                    && citation.end_line >= citation.start_line
                    && citation.end_line <= *lines,
                loc,
                format!(
                    "{}: invalid line range {}..{}; file has {lines} lines",
                    citation.path, citation.start_line, citation.end_line
                ),
            );
            report.require(
                citation.sha256 == *hash,
                loc,
                format!(
                    "{}: source digest changed; review the claim before refreshing its citation",
                    citation.path
                ),
            );
        }
        Err(error) => report.require(false, loc, error.clone()),
    }
}

fn visible_at(
    kind: ViewKind,
    element: &Element,
    scope: Option<&Element>,
    elements: &Elements<'_>,
) -> bool {
    match kind {
        ViewKind::Context | ViewKind::Landscape => {
            matches!(element.kind, Kind::Person | Kind::System)
        }
        ViewKind::Container => {
            element.kind == Kind::Person
                || (element.kind == Kind::System && scope.is_none_or(|s| element.id != s.id))
                || (element.kind == Kind::Container
                    && element.parent.as_deref() == scope.map(|s| s.id.as_str()))
        }
        ViewKind::Component => {
            element.kind == Kind::Person
                || (element.kind == Kind::System
                    && scope.is_none_or(|s| !has_ancestor(&s.id, &element.id, elements)))
                || (element.kind == Kind::Container && scope.is_none_or(|s| s.id != element.id))
                || (element.kind == Kind::Component
                    && element.parent.as_deref() == scope.map(|s| s.id.as_str()))
        }
        ViewKind::Code => {
            element.kind == Kind::Code && element.parent.as_deref() == scope.map(|s| s.id.as_str())
        }
        ViewKind::Deployment => element.kind.is_deployment(),
        ViewKind::Dynamic => {
            let level = match scope.map(|s| s.kind) {
                Some(Kind::System) => ViewKind::Container,
                Some(Kind::Container) => ViewKind::Component,
                Some(Kind::Component) => ViewKind::Code,
                _ => ViewKind::Context,
            };
            visible_at(level, element, scope, elements)
        }
    }
}

fn unique_labels(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().all(|v| nonempty(v) && seen.insert(v.trim()))
}

fn validate_view(
    view: &View,
    elements: &Elements<'_>,
    relationships: &BTreeMap<&str, &Relationship>,
    loc: &str,
    report: &mut Report,
) {
    let scope = view
        .scope
        .as_deref()
        .and_then(|id| elements.get(id))
        .copied();
    let valid_scope = match view.kind {
        ViewKind::Context | ViewKind::Container => scope.is_some_and(|s| s.kind == Kind::System),
        ViewKind::Deployment => {
            view.scope.is_none() || scope.is_some_and(|e| e.kind == Kind::System)
        }
        ViewKind::Component => scope.is_some_and(|s| s.kind == Kind::Container),
        ViewKind::Code => scope.is_some_and(|s| s.kind == Kind::Component),
        ViewKind::Landscape => view.scope.is_none(),
        ViewKind::Dynamic => {
            view.scope.is_none()
                || scope.is_some_and(|s| {
                    matches!(s.kind, Kind::System | Kind::Container | Kind::Component)
                })
        }
    };
    report.require(
        valid_scope,
        loc,
        "scope is missing or has an invalid kind for this view",
    );
    let selected: BTreeSet<_> = view.elements.iter().map(String::as_str).collect();
    report.require(!selected.is_empty(), loc, "view must contain elements");
    report.require(
        !view.show_system_boundary || view.kind == ViewKind::Component,
        loc,
        "show_system_boundary requires a component view",
    );
    report.require(
        view.diagram_notes.iter().all(|n| nonempty(n)),
        loc,
        "diagram notes must not be blank",
    );
    report.require(
        view.lifelines.is_empty() || view.kind == ViewKind::Dynamic,
        loc,
        "lifelines require a dynamic view",
    );
    for (id, life) in &view.lifelines {
        report.require(
            valid_id(id)
                && !elements.contains_key(id.as_str())
                && selected.contains(life.element.as_str())
                && nonempty(&life.name),
            loc,
            "lifeline needs a unique ID, a name and a selected element",
        );
    }
    if let Some(p) = &view.perspective {
        report.require(
            view.kind != ViewKind::Dynamic,
            loc,
            "perspectives require a static or deployment view",
        );
        report.require(
            nonempty(&p.key) && unique_labels(&p.values),
            loc,
            "perspective needs a key and distinct nonblank filter values",
        );
        let labels: BTreeSet<_> = selected
            .iter()
            .filter_map(|id| elements.get(id))
            .filter_map(|e| e.perspectives.get(&p.key))
            .chain(
                view.relationships
                    .iter()
                    .filter_map(|id| relationships.get(id.as_str()))
                    .filter_map(|r| r.perspectives.get(&p.key)),
            )
            .flatten()
            .collect();
        report.require(
            !labels.is_empty() && p.values.iter().all(|v| labels.contains(v)),
            loc,
            "perspective key or filter value has no matching element in the view",
        );
        let projected = view.projected(elements, relationships);
        if view.kind == ViewKind::Context {
            report.require(
                scope.is_some_and(|e| projected.elements.contains(&e.id)),
                loc,
                "perspective must retain the context's focal system",
            );
        } else if matches!(
            view.kind,
            ViewKind::Container | ViewKind::Component | ViewKind::Code
        ) {
            report.require(
                projected
                    .elements
                    .iter()
                    .filter_map(|id| elements.get(id.as_str()))
                    .any(|e| e.parent == view.scope),
                loc,
                "perspective must retain an element inside its scope",
            );
        }
    }
    report.require(
        selected.len() == view.elements.len(),
        loc,
        "duplicate view element",
    );
    if matches!(
        view.kind,
        ViewKind::Container | ViewKind::Component | ViewKind::Code
    ) {
        report.require(
            selected
                .iter()
                .filter_map(|id| elements.get(id))
                .any(|e| e.parent.as_deref() == view.scope.as_deref()),
            loc,
            "view must show at least one element inside its scoped boundary",
        );
    }
    if view.kind == ViewKind::Context {
        report.require(
            scope.is_some_and(|s| selected.contains(s.id.as_str())),
            loc,
            "context must include its scoped system",
        );
    }
    for id in &selected {
        let Some(element) = elements.get(id) else {
            report.require(false, loc, format!("unknown element: {id}"));
            continue;
        };
        report.require(
            visible_at(view.kind, element, scope, elements),
            loc,
            format!("{id} mixes abstraction levels or lies outside the view scope"),
        );
        if view.kind == ViewKind::Deployment {
            if element.kind == Kind::DeploymentNode {
                report.require(
                    selected
                        .iter()
                        .filter_map(|id| elements.get(id))
                        .any(|e| e.parent.as_deref() == Some(&element.id)),
                    loc,
                    format!("hosting node {id} has no visible children"),
                );
            }
            report.require(
                element.environment == view.environment,
                loc,
                format!("{id} is in a different environment"),
            );
            if let Some(parent) = &element.parent {
                report.require(
                    selected.contains(parent.as_str()),
                    loc,
                    format!("hosting node {parent} must appear in this deployment view"),
                );
            }
        } else {
            report.require(
                !selected
                    .iter()
                    .any(|other| has_ancestor(id, other, elements)),
                loc,
                format!("{id} and one of its ancestors are both shown as elements"),
            );
        }
    }
    if view.kind == ViewKind::Deployment {
        report.require(
            view.environment.as_deref().is_some_and(nonempty),
            loc,
            "deployment view requires an environment",
        );
        let has_instance = selected
            .iter()
            .filter_map(|id| elements.get(id))
            .filter_map(|e| e.instance_of.as_deref())
            .any(|id| scope.is_none_or(|s| id == s.id || has_ancestor(id, &s.id, elements)));
        report.require(
            has_instance,
            loc,
            "deployment must include a software instance, belonging to the focal system when scoped",
        );
    } else {
        report.require(
            view.environment.is_none(),
            loc,
            "environment is only valid for deployment views",
        );
    }
    let mut relation_ids = BTreeSet::new();
    for id in &view.relationships {
        report.require(relation_ids.insert(id), loc, "duplicate view relationship");
    }
    let check_relationship = |id: &String, report: &mut Report| match relationships.get(id.as_str())
    {
        Some(r) => report.require(
            selected.contains(r.from.as_str()) && selected.contains(r.to.as_str()),
            loc,
            format!("relationship {id} has an endpoint outside this view"),
        ),
        None => report.require(false, loc, format!("unknown relationship: {id}")),
    };
    for id in &view.relationships {
        check_relationship(id, report);
    }
    if view.kind == ViewKind::Dynamic {
        report.require(
            !view.steps.is_empty() && view.relationships.is_empty(),
            loc,
            "dynamic views require ordered steps and no relationships list",
        );
        validate_steps(&view.steps, loc, report, &|step, report| {
            check_relationship(&step.relationship, report);
            if let Some(r) = relationships.get(step.relationship.as_str()) {
                let (from, to) = step.logical_endpoints(r);
                for (alias, expected) in [(&step.from_lifeline, from), (&step.to_lifeline, to)] {
                    report.require(
                        alias.as_ref().is_none_or(|id| {
                            view.lifelines
                                .get(id)
                                .is_some_and(|l| l.element == expected)
                        }),
                        loc,
                        "lifeline endpoint must represent the corresponding relationship endpoint",
                    );
                }
            }
        });
    } else {
        report.require(
            view.steps.is_empty(),
            loc,
            "steps are only valid for dynamic views",
        );
    }
}

fn validate_steps(
    items: &[ScenarioItem],
    loc: &str,
    report: &mut Report,
    check_relationship: &impl Fn(&Step, &mut Report),
) {
    for (index, item) in items.iter().enumerate() {
        let loc = format!("{loc}.steps[{index}]");
        match item {
            ScenarioItem::Interaction(step) => {
                check_relationship(step, report);
                report.require(
                    nonempty(&step.description),
                    &loc,
                    "dynamic step needs an interaction description",
                );
            }
            ScenarioItem::Block(block) => {
                let count = block.branches.len();
                report.require(
                    match block.kind {
                        ControlKind::Optional | ControlKind::Loop => count == 1,
                        ControlKind::Alternative | ControlKind::Parallel => count >= 2,
                    },
                    &loc,
                    "optional and loop need one branch; alternative and parallel need at least two",
                );
                for (index, branch) in block.branches.iter().enumerate() {
                    let loc = format!("{loc}.branches[{index}]");
                    report.require(
                        nonempty(&branch.label) && !branch.steps.is_empty(),
                        &loc,
                        "scenario branch needs a label and steps",
                    );
                    validate_steps(&branch.steps, &loc, report, check_relationship);
                }
            }
        }
    }
}
