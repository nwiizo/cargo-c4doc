use crate::{Error, Result, check, model::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Produce the complete document set in memory before touching the destination.
pub fn documents(
    model: &Model,
    root: &Path,
    output: &Path,
    strict: bool,
) -> Result<BTreeMap<String, String>> {
    let report = check::validate(model, root);
    if !report.passes(strict) {
        return Err(Error::Invalid(format!(
            "model check failed: {} errors, {} warnings; run cargo c4doc check --json for details",
            report.errors.len(),
            report.warnings.len()
        )));
    }
    // All map lookups below use IDs checked by validate, including scopes and endpoints.
    let elements: BTreeMap<_, _> = model.elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let relationships: BTreeMap<_, _> = model
        .relationships
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let mut views_by_scope: BTreeMap<&str, Vec<&View>> = BTreeMap::new();
    for view in &model.views {
        if let Some(scope) = view.scope.as_deref() {
            views_by_scope.entry(scope).or_default().push(view);
        }
    }
    let mut documents = BTreeMap::new();
    let mut index = format!(
        "# {}\n\n{}\n\n## Explore the architecture\n\nFollow the views in order, or jump to the question you have.\n\n",
        markdown(&model.title),
        markdown(&model.description)
    );
    for (view_index, original) in model.views.iter().enumerate() {
        let projected = original.projected(&elements, &relationships);
        let view = projected.as_ref();
        let mut scenario = Vec::new();
        scenario_events(&view.steps, &mut scenario);
        let diagram = diagram(view, &elements, &relationships, &scenario);
        let indent = " ".repeat((view_index + 1).to_string().len() + 2);
        index.push_str(&format!(
            "{}. [{}]({}.md) · {}\n\n{indent}{}\n\n",
            view_index + 1,
            markdown(&view.title),
            view.id,
            view.kind.label(),
            markdown(&view.description)
        ));
        let scope = view
            .scope
            .as_deref()
            .map(|id| elements[id].name.as_str())
            .unwrap_or("Cross-system");
        let mut doc = format!(
            "# {}\n\n[All views](README.md) · [Diagram source]({}.mmd) · [Evidence](evidence.md)\n\n{}\n\n- Kind: {}\n- Scope: {}\n",
            markdown(&view.title),
            view.id,
            markdown(&view.description),
            view.kind.label(),
            markdown(scope)
        );
        if let Some(environment) = &view.environment {
            doc.push_str(&format!("- Environment: {}\n", markdown(environment)));
        }
        doc.push_str(&format!("\n```mermaid\n{diagram}```\n"));
        if let Some(p) = &view.perspective {
            doc.push_str(&format!("\n## Perspective: {}\n", markdown(&p.key)));
            let mut groups: BTreeMap<Option<&str>, Vec<String>> = BTreeMap::new();
            for id in &view.elements {
                let element = elements[id.as_str()];
                if let Some(values) = element.perspectives.get(&p.key) {
                    for value in values {
                        groups
                            .entry(Some(value))
                            .or_default()
                            .push(markdown(&element.name));
                    }
                } else {
                    groups
                        .entry(None)
                        .or_default()
                        .push(markdown(&element.name));
                }
            }
            for (label, names) in groups {
                doc.push_str(&format!(
                    "\n- **{}** — {}\n",
                    markdown(label.unwrap_or("Unassigned")),
                    names.join(", ")
                ));
            }
            for id in &view.relationships {
                let r = relationships[id.as_str()];
                if let Some(values) = r.perspectives.get(&p.key) {
                    doc.push_str(&format!(
                        "\n- {} → {} ({}): {}\n",
                        markdown(&elements[r.from.as_str()].name),
                        markdown(&elements[r.to.as_str()].name),
                        markdown(&r.description),
                        markdown(&values.join(", "))
                    ));
                }
            }
            let alternatives: Vec<_> = model
                .views
                .iter()
                .filter(|other| {
                    other.id != view.id
                        && other.kind == view.kind
                        && other.scope == view.scope
                        && other.environment == view.environment
                        && other.perspective.is_some()
                })
                .map(|other| format!("[{}]({}.md)", markdown(&other.title), other.id))
                .collect();
            if !alternatives.is_empty() {
                doc.push_str(&format!(
                    "\nOther perspectives: {}.\n",
                    alternatives.join(" · ")
                ));
            }
        }
        if !view.notes.is_empty() {
            doc.push_str("\n## Reading notes\n");
            for note in &view.notes {
                doc.push_str(&format!("\n{}\n", markdown(note)));
            }
        }
        doc.push_str("\n## Elements\n\n| Name | Kind | Technology | Responsibility | Evidence |\n| --- | --- | --- | --- | --- |\n");
        for id in &view.elements {
            let element = elements[id.as_str()];
            let mut name = markdown(&format!("{}{}", element.name, qualifiers(element)));
            if element.code_mapping.is_some() {
                name.push_str(&format!(
                    "<br/>[Implementation](evidence.md#implementation-{})",
                    element.id
                ));
            }
            if let Some(related) = views_by_scope.get(id.as_str()) {
                for detail in related.iter().filter(|detail| detail.id != view.id) {
                    name.push_str(&format!(
                        "<br/>[{}]({}.md)",
                        markdown(&detail.title),
                        detail.id
                    ));
                }
            }
            doc.push_str(&format!(
                "| {name} | {} | {} | {} | {} |\n",
                element.kind.label(),
                markdown(element.technology.as_deref().unwrap_or("—")),
                markdown(&element.description),
                evidence_text(&element.evidence, root, output)
            ));
        }
        if !view.relationships.is_empty() {
            doc.push_str("\n## Relationships\n\n| From → To | Interaction | Technology | Evidence |\n| --- | --- | --- | --- |\n");
            for id in &view.relationships {
                let relationship = relationships[id.as_str()];
                doc.push_str(&format!(
                    "| {} → {} | {} | {} | {} |\n",
                    markdown(&elements[relationship.from.as_str()].name),
                    markdown(&elements[relationship.to.as_str()].name),
                    markdown(&relationship.description),
                    markdown(relationship.technology.as_deref().unwrap_or("—")),
                    evidence_text(&relationship.evidence, root, output)
                ));
            }
        }
        if !view.steps.is_empty() {
            doc.push_str("\n## Scenario\n\nStep order and scenario wording are authored by the agent and require review. Evidence below supports the referenced static interaction; it does not prove execution order.\n\n| Step | From → To | Interaction | Relationship evidence |\n| --- | --- | --- | --- |\n");
            let mut number = 0;
            for event in &scenario {
                let step = match event {
                    ScenarioEvent::Step(step) => step,
                    ScenarioEvent::Branch { heading, label, .. } => {
                        doc.push_str(&format!("| | | **{heading}** {} | |\n", markdown(label)));
                        continue;
                    }
                    ScenarioEvent::End => {
                        doc.push_str("| | | **End block** | |\n");
                        continue;
                    }
                };
                number += 1;
                let relationship = relationships[step.relationship.as_str()];
                let (from, to) = step.participants(relationship);
                let name = |id: &str| {
                    markdown(
                        view.lifelines
                            .get(id)
                            .map(|l| l.name.as_str())
                            .unwrap_or_else(|| elements[id].name.as_str()),
                    )
                };
                doc.push_str(&format!(
                    "| {} | {} → {} | {} | {} |\n",
                    number,
                    name(from),
                    name(to),
                    markdown(&step.description),
                    evidence_text(&relationship.evidence, root, output)
                ));
            }
        }
        if let Some(next) = model.views.get(view_index + 1) {
            doc.push_str(&format!(
                "\nContinue reading: [{}]({}.md).\n",
                markdown(&next.title),
                next.id
            ));
        }
        documents.insert(format!("{}.md", view.id), doc);
        documents.insert(format!("{}.mmd", view.id), diagram);
    }
    index.push_str("\n## Evidence and unresolved questions\n\n[Source citations and inference reasons](evidence.md). Observed means a source was cited and its path, line range, and SHA-256 were checked. It does not mean the claim's semantics were automatically proved.\n");
    if model.questions.is_empty() {
        index.push_str("\nNo open questions recorded.\n");
    }
    for question in &model.questions {
        index.push_str(&format!("\n- {}\n", markdown(question)));
    }
    index.push_str(&format!(
        "\nModel checks: {} errors, {} warnings.\n",
        report.errors.len(),
        report.warnings.len()
    ));
    for warning in &report.warnings {
        index.push_str(&format!(
            "\n- {}: {}\n",
            markdown(&warning.location),
            markdown(&warning.message)
        ));
    }
    index.push_str("\nGenerated by cargo-c4doc. Edit the input model, then regenerate.\n");
    let mut evidence = String::from(
        "# Evidence\n\n[All views](README.md)\n\nSource digests cover the complete file. After a source changes, inspect the affected claim before replacing the citation with `cargo c4doc cite`.\n\n| Claim | Status and references |\n| --- | --- |\n",
    );
    for element in &model.elements {
        evidence.push_str(&format!(
            "| Element: {} | {} |\n",
            markdown(&element.name),
            evidence_text(&element.evidence, root, output)
        ));
    }
    for relationship in &model.relationships {
        evidence.push_str(&format!(
            "| Relationship: {} | {} |\n",
            markdown(&relationship.description),
            evidence_text(&relationship.evidence, root, output)
        ));
    }
    for element in &model.elements {
        if let Some(mapping) = &element.code_mapping {
            evidence.push_str(&format!("\n<a id=\"implementation-{}\"></a>\n\n## Implementation of {}\n\n{}\n\n| Source | Contribution |\n| --- | --- |\n",
                element.id, markdown(&element.name), markdown(&mapping.coverage)));
            for entry in &mapping.entries {
                evidence.push_str(&format!(
                    "| {} | {} |\n",
                    source_link(&entry.source, root, output),
                    markdown(&entry.rationale)
                ));
            }
        }
    }
    documents.insert("README.md".into(), index);
    documents.insert("evidence.md".into(), evidence);
    Ok(documents)
}

fn qualifiers(e: &Element) -> String {
    if let Some(range) = &e.replicas {
        format!(" ×{}..{}", range.min, range.max)
    } else {
        match e.code_access {
            Some(CodeAccess::Public) => " / Public".into(),
            Some(CodeAccess::Internal) => " / Internal".into(),
            None => String::new(),
        }
    }
}

enum ScenarioEvent<'a> {
    Step(&'a Step),
    Branch {
        keyword: &'static str,
        heading: &'static str,
        label: &'a str,
    },
    End,
}

fn scenario_events<'a>(items: &'a [ScenarioItem], events: &mut Vec<ScenarioEvent<'a>>) {
    for item in items {
        match item {
            ScenarioItem::Interaction(step) => events.push(ScenarioEvent::Step(step)),
            ScenarioItem::Block(block) => {
                for (index, branch) in block.branches.iter().enumerate() {
                    let (keyword, heading) = match (block.kind, index) {
                        (ControlKind::Optional, _) => ("opt", "Optional"),
                        (ControlKind::Loop, _) => ("loop", "Loop"),
                        (ControlKind::Alternative, 0) => ("alt", "If"),
                        (ControlKind::Alternative, _) => ("else", "Else"),
                        (ControlKind::Parallel, 0) => ("par", "Parallel"),
                        (ControlKind::Parallel, _) => ("and", "And"),
                    };
                    events.push(ScenarioEvent::Branch {
                        keyword,
                        heading,
                        label: &branch.label,
                    });
                    scenario_events(&branch.steps, events);
                }
                events.push(ScenarioEvent::End);
            }
        }
    }
}

fn diagram(
    view: &View,
    elements: &BTreeMap<&str, &Element>,
    relationships: &BTreeMap<&str, &Relationship>,
    scenario: &[ScenarioEvent<'_>],
) -> String {
    let scope = view
        .scope
        .as_deref()
        .map(|id| elements[id].name.as_str())
        .unwrap_or("Cross-system");
    let mut title = format!("{}: {scope} — {}", view.kind.label(), view.title);
    if let Some(environment) = &view.environment {
        title.push_str(&format!(" ({environment})"));
    }
    if let Some(p) = &view.perspective {
        title.push_str(&format!(" — {}", p.key));
        if !p.values.is_empty() {
            title.push_str(&format!(": {}", p.values.join(", ")));
        }
    }
    let title: String = title
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    // JSON strings are valid YAML scalars; model text cannot inject frontmatter keys.
    // Frontmatter titles are plain text, not Mermaid entity-encoded node labels.
    let title_scalar = serde_json::Value::String(title.clone())
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let mut text = format!("---\ntitle: {title_scalar}\n---\n");
    let accessible_title = mermaid(&title);
    if view.kind == ViewKind::Dynamic {
        text.push_str(&format!(
            "sequenceDiagram\n    accTitle: {accessible_title}\n    autonumber\n"
        ));
        let used: BTreeSet<_> = scenario
            .iter()
            .filter_map(|event| match event {
                ScenarioEvent::Step(s) => {
                    Some(s.participants(relationships[s.relationship.as_str()]))
                }
                _ => None,
            })
            .flat_map(|(from, to)| [from, to])
            .collect();
        let mut participants = Vec::new();
        for base in &view.elements {
            let aliases: Vec<_> = view
                .lifelines
                .iter()
                .filter(|(_, l)| l.element == *base)
                .collect();
            if aliases.is_empty() || used.contains(base.as_str()) {
                participants.push((
                    base.as_str(),
                    elements[base.as_str()].name.as_str(),
                    elements[base.as_str()],
                ));
            }
            for (id, life) in aliases {
                participants.push((id.as_str(), life.name.as_str(), elements[base.as_str()]));
            }
        }
        for (id, name, element) in &participants {
            let role = if element.kind == Kind::Person {
                "actor"
            } else {
                "participant"
            };
            text.push_str(&format!(
                "    {role} n_{id} as {} [{} / {}]\n",
                mermaid(name),
                element.kind.label(),
                element.evidence.label()
            ));
        }
        if let (Some((first, ..)), Some((last, ..))) = (participants.first(), participants.last()) {
            text.push_str(&format!(
                "    Note over n_{first},n_{last}: {}\n",
                legend(view.kind).replace("; ", "<br/>")
            ));
            for note in &view.diagram_notes {
                text.push_str(&format!(
                    "    Note over n_{first},n_{last}: {}\n",
                    mermaid(note)
                ));
            }
        }
        for event in scenario {
            let step = match event {
                ScenarioEvent::Step(step) => step,
                ScenarioEvent::Branch { keyword, label, .. } => {
                    text.push_str(&format!("    {keyword} {}\n", mermaid(label)));
                    continue;
                }
                ScenarioEvent::End => {
                    text.push_str("    end\n");
                    continue;
                }
            };
            let relationship = relationships[step.relationship.as_str()];
            let (from, to) = step.participants(relationship);
            let arrow = if step.response { "-->>" } else { "->>" };
            text.push_str(&format!(
                "    n_{from}{arrow}n_{to}: {} [Relationship evidence: {}]\n",
                mermaid(&step.description),
                relationship.evidence.label()
            ));
        }
        return text;
    }
    text.push_str(&format!("flowchart TB\n    accTitle: {accessible_title}\n"));
    let selected: BTreeSet<_> = view.elements.iter().map(String::as_str).collect();
    if view.kind == ViewKind::Deployment {
        for id in &view.elements {
            let element = elements[id.as_str()];
            if element.parent.is_none() {
                deployment_element(
                    &mut text,
                    element,
                    &selected,
                    elements,
                    view.perspective.as_ref(),
                );
            }
        }
    } else {
        let boundary = matches!(
            view.kind,
            ViewKind::Container | ViewKind::Component | ViewKind::Code
        )
        .then_some(view.scope.as_deref())
        .flatten();
        let system = boundary
            .filter(|_| view.show_system_boundary)
            .and_then(|id| elements[id].parent.as_deref());
        for id in &view.elements {
            let element = elements[id.as_str()];
            if (boundary.is_none() || element.parent.as_deref() != boundary)
                && system.is_none_or(|s| element.parent.as_deref() != Some(s))
            {
                node(&mut text, element, elements, view.perspective.as_ref());
            }
        }
        if let Some(id) = system {
            text.push_str(&format!(
                "    subgraph b_{id}[\"{} [Software system]\"]\n",
                mermaid(&elements[id].name)
            ));
            for child in view
                .elements
                .iter()
                .map(|key| elements[key.as_str()])
                .filter(|e| e.parent.as_deref() == Some(id))
            {
                node(&mut text, child, elements, view.perspective.as_ref());
            }
        }
        if let Some(scope) = boundary {
            let parent = elements[scope];
            text.push_str(&format!(
                "    subgraph b_{scope}[\"{} [{}]\"]\n",
                mermaid(&parent.name),
                parent.kind.label()
            ));
            for id in &view.elements {
                let element = elements[id.as_str()];
                if element.parent.as_deref() == boundary {
                    node(&mut text, element, elements, view.perspective.as_ref());
                }
            }
            text.push_str("    end\n");
        }
        if system.is_some() {
            text.push_str("    end\n");
        }
    }
    for id in &view.relationships {
        let relationship = relationships[id.as_str()];
        let mut label = match &relationship.technology {
            Some(technology) => format!("{} [{}]", relationship.description, technology),
            None => relationship.description.clone(),
        };
        if let Some(p) = &view.perspective
            && let Some(values) = relationship.perspectives.get(&p.key)
        {
            label.push_str(&format!(" / {}: {}", p.key, values.join(", ")));
        }
        let edge = if matches!(relationship.evidence, Evidence::Inferred { .. }) {
            "-.->"
        } else {
            "-->"
        };
        text.push_str(&format!(
            "    n_{} {edge}|\"{}\"| n_{}\n",
            relationship.from,
            mermaid(&label),
            relationship.to
        ));
    }
    text.push_str("    classDef observed fill:#e8f1fa,stroke:#28577c,color:#132f46\n    classDef inferred fill:#fff4de,stroke:#8a5100,color:#573300,stroke-dasharray:5 5\n");
    // Keep the key outside model boundaries, with no architectural edges or model ID.
    for (i, note) in view.diagram_notes.iter().enumerate() {
        text.push_str(&format!(
            "    c4doc_note_{i}[\"Note: {}\"]\n",
            mermaid(note)
        ));
    }
    text.push_str(&format!(
        "    c4doc_legend[\"{}\"]\n    style c4doc_legend fill:#fff,stroke:#666,color:#222\n",
        legend(view.kind).replace("; ", "<br/>")
    ));
    text
}

fn legend(kind: ViewKind) -> &'static str {
    if kind == ViewKind::Dynamic {
        "Legend: Solid arrows: interactions; Dashed arrows: responses; Observed: cited source, not semantic proof; Inferred: assumption requiring review; Relationship evidence does not prove execution order"
    } else {
        "Legend (not a model element); Blue solid boxes: Observed; Orange dashed boxes: Inferred; Solid arrows: Observed; Dotted arrows: Inferred; Observed: cited source, not semantic proof; Inferred: assumption requiring review; Boundary labels name their C4 type; Deployment nesting shows hosting"
    }
}

fn node(
    text: &mut String,
    element: &Element,
    elements: &BTreeMap<&str, &Element>,
    perspective: Option<&Perspective>,
) {
    let mut label = format!("{} [{}]", element.name, element.kind.label());
    label.push_str(&qualifiers(element));
    if let Some(technology) = &element.technology {
        label.push_str(&format!(" / {technology}"));
    }
    if let Some(target) = &element.instance_of {
        label.push_str(&format!(" of {}", elements[target.as_str()].name));
    }
    label.push_str(&format!(" — {}", element.description));
    if let Some(p) = perspective {
        let values = element
            .perspectives
            .get(&p.key)
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "Unassigned".into());
        label.push_str(&format!(" / {}: {values}", p.key));
    }
    let class = if matches!(element.evidence, Evidence::Observed { .. }) {
        "observed"
    } else {
        "inferred"
    };
    text.push_str(&format!(
        "    n_{}[\"{}\"]:::{class}\n",
        element.id,
        mermaid(&label)
    ));
}

fn deployment_element(
    text: &mut String,
    element: &Element,
    selected: &BTreeSet<&str>,
    elements: &BTreeMap<&str, &Element>,
    perspective: Option<&Perspective>,
) {
    let children: Vec<_> = selected
        .iter()
        .map(|id| elements[id])
        .filter(|e| e.parent.as_deref() == Some(&element.id))
        .collect();
    if element.kind != Kind::DeploymentNode || children.is_empty() {
        node(text, element, elements, perspective);
        return;
    }
    text.push_str(&format!(
        "    subgraph n_{}[\"{} [Deployment node]{} / {} / {}{}\"]\n",
        element.id,
        mermaid(&element.name),
        mermaid(&qualifiers(element)),
        mermaid(element.technology.as_deref().unwrap_or("")),
        element.evidence.label(),
        perspective
            .and_then(|p| element
                .perspectives
                .get(&p.key)
                .map(|values| mermaid(&format!(" / {}: {}", p.key, values.join(", ")))))
            .unwrap_or_default()
    ));
    for child in children {
        deployment_element(text, child, selected, elements, perspective);
    }
    text.push_str("    end\n");
}

fn evidence_text(evidence: &Evidence, root: &Path, output: &Path) -> String {
    match evidence {
        Evidence::Inferred { reason } => format!("**Inferred**: {}", markdown(reason)),
        Evidence::Observed { sources } => {
            let links: Vec<_> = sources
                .iter()
                .map(|citation| source_link(citation, root, output))
                .collect();
            format!("Observed: {}", links.join("; "))
        }
    }
}

fn source_link(citation: &SourceRef, root: &Path, output: &Path) -> String {
    let url = relative_url(output, &root.join(&citation.path));
    format!(
        "[{}:{}–{}]({url}#L{}-L{})",
        markdown(&citation.path),
        citation.start_line,
        citation.end_line,
        citation.start_line,
        citation.end_line
    )
}

fn relative_url(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result = "../".repeat(from.len() - common);
    let rest = to[common..]
        .iter()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    for byte in rest.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

fn markdown(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        match ch {
            '\r' | '\n' => output.push(' '),
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '|' => output.push_str("&#124;"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' => {
                output.push('\\');
                output.push(ch);
            }
            _ => output.push(ch),
        }
    }
    output
}

fn mermaid(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        if ch.is_control() {
            output.push(' ');
        } else if ch.is_ascii_punctuation() && !matches!(ch, '-' | '_' | '.' | '/' | ':') {
            output.push_str(&format!("#{};", ch as u32));
        } else {
            output.push(ch);
        }
    }
    output
}
