# cargo c4doc model format

Use `cargo c4doc schema` for the complete schema. Models are UTF-8 JSON with `schema_version: 1`.
Unknown fields are rejected. Top-level fields are `title`, `description`, `elements`, `relationships`,
`views`, and `questions`; relationships and questions may be omitted.

For a new model, `cargo c4doc init --title <name> --output <new-model.json>` provides a draft
to replace with inspected responsibilities and evidence. Reuse an existing model for updates.

## Elements and evidence

```json
{
  "id": "api",
  "kind": "container",
  "parent": "order_system",
  "name": "Order API",
  "description": "Accepts orders and returns their status",
  "technology": "Rust / Axum",
  "evidence": {
    "status": "inferred",
    "reason": "Routes were inspected, but deployment boundaries still need confirmation"
  }
}
```

IDs match `[a-z][a-z0-9_]{0,63}`. Elements require `name`, a responsibility-oriented `description`,
and `evidence`. `technology` is required for container, component, code, deployment_node,
and infrastructure_node.

| kind | parent | Additional fields |
| --- | --- | --- |
| person / system | None | User or software system |
| container | system | Application or data store |
| component | container | Responsibility inside the Container |
| code | component | Type, function, trait, etc. |
| deployment_node | deployment_node or none | environment required |
| infrastructure_node | deployment_node | environment required |
| instance | deployment_node | instance_of references a system/container; environment required |

Observed claims use `{"status":"observed","sources":[<SourceRef>]}`. SourceRef contains
`path`, `start_line`, `end_line`, and `sha256`; obtain it with `cite` or `analyze`.
Inferred claims require a reason. Never cite uninspected sources to pass validation.

Sources are UTF-8 files beneath `--root`, using relative paths without `..`, absolute paths,
or symlinks. For external evidence, record an authorized summary, URL, and verified observations
in a handwritten project document and cite that file. Do not copy entire external publications.

### Map code to a component

A component can have a `code_mapping`. Write its `coverage` as a sentence specifying
the reviewed part of the implementation. Each entry has a `source` (the complete
`SourceRef` from `cite`) and a `rationale` explaining that location's contribution.
For example, one entry might cite dispatch of retryable work and another the module
implementing the retry policy. The same file can contribute to several components.
Mappings appear on the evidence page and link from the Elements table.
When a mapped path changes, `drift --model` lists matching component IDs under
`mapped_components`. Inspect the changed lines to determine the actual impact.
The source release includes an example mapping on the validation component in `examples/c4doc.json`.

Generated fragments such as `#L10-L20` target GitHub's source viewer. Local Markdown/HTML viewers
may not support them. Check file existence with a link checker and ranges/hashes with `check`;
validate heading links separately. Do not add anchors to source code to pass a link check.

## Relationships

Require `id`, `from`, `to`, `description`, and `evidence`. `technology` is optional, but describe
protocols or formats for Container-level interactions. Explain what is requested or exchanged,
not merely "depends on." Define relationships at appropriate levels and select IDs for each view.
Parent-child relationships do not automatically imply communication.
Generic labels such as `uses` and `depends on` produce a warning. Name the action
and what is exchanged, then read the arrow from its source to its destination.

## Views

Each view has `id`, `kind`, `title`, `description`, and `elements: [ID...]`.
Select static relationships with `relationships: [ID...]`. View IDs `evidence` and `readme` are reserved.

The order of `views` is the reading order for the index and next-page links. Use a short
`description` as the answer or takeaway shown in both the index and the view. Optional
`notes: ["First supporting paragraph", "Another paragraph"]` appear after the view's diagram,
before its reference tables. They do not appear in the index or standalone Mermaid output.
Titles, descriptions, and notes are plain text: Markdown/HTML is escaped and each string's
line breaks become spaces. Use separate notes for separate paragraphs.

Existing version 1 models without `notes` still load; absent or empty notes produce no notes
section. This addition keeps `schema_version: 1`, but older binaries reject `notes` as an unknown
field. Upgrade the CLI before using it; to retain an older binary, keep the supporting prose
in `description` or a companion document instead. Editorial prose requires source review;
`check` does not establish that a description or note is true.

| kind | scope | Allowed elements |
| --- | --- | --- |
| context | system | The scope itself, people, other systems |
| container | system | Direct Containers, people, other systems |
| component | container | Direct Components, people, other Containers, external systems |
| code | component | Direct code elements |
| landscape | None | People and systems |
| dynamic | None, system, container, or component | The level immediately inside scope and applicable external participants |
| deployment | system or omitted | Nodes and software instances in one environment |

For Container, Component, and Code views, scope is rendered as a boundary; do not also list it
as an element. Do not mix a selected ancestor and its child as peers. Include both endpoints of
every selected relationship. Deployment requires `environment`, instance parents, and node ancestors.
Omit the deployment scope to show peer systems sharing a host. Include at least one
software instance. With a focal system, at least one instance must belong to it.

### Ownership and security perspectives

Assign named values to elements, for example
`"perspectives": {"owner":["Payments","Operations"],"security":["Internal"]}`.
Set `"perspective":{"key":"owner","values":["Payments"]}` on a static logical view
to keep elements with any selected value and relationships between the remaining
endpoints. A Context view must retain its focal system, and a scoped detail view
must retain an element inside its boundary. Omit `values` to include all selected
elements, including those with no assignment. Multiple memberships appear in full.
Groupings appear below the diagram and as text on the nodes. To offer another
perspective, add a view with its own ID and the same kind and scope, selecting
`"perspective":{"key":"security"}`. Each page links to the other perspectives.
The underlying C4 parent hierarchy stays the same.

Deployment views also support perspectives. Tag a deployed instance or a relationship
with the same `perspectives` map used for logical elements. A matching relationship
retains both endpoints, and a selected deployment element retains its hosting ancestors.
For example, tag a production store with `{"security":["Encrypted"]}` and select the
`security` perspective on the production view.

### Boundaries, scale, interfaces, and diagram notes

- Context views with multiple visible elements warn if no displayed relationship touches the focal system.
- In a Component view, set `"show_system_boundary":true` to draw the parent system
  around the scoped container and other visible containers belonging to that system.
- A deployment node or instance accepts `"replicas":{"min":1,"max":10}`. The
  inclusive range describes configured scale. Use separate instance IDs when identity matters.
- A Code element accepts `"code_access":"public"` or `"internal"` relative to its
  component interface. Check written visibility from `analyze`, parent modules, and
  re-exports before choosing. Omit the field when the exposure is unknown.
- Add `"diagram_notes":["Audit logging omitted"]` to include a note in the standalone
  Mermaid diagram and its embedded copy. Use `notes` for supplementary Markdown prose.

### Scenarios

A Dynamic view can distinguish individual lifelines for one logical element.
Declare, for example, `"lifelines":{"worker_a":{"element":"worker","name":"Worker A"},"worker_b":{"element":"worker","name":"Worker B"}}`.
The logical element must occur in `elements`. On a step, `from_lifeline` and
`to_lifeline` select the sender and receiver in the displayed direction, including
when `response:true` reverses the relationship. Omitted endpoints use their logical
element. A lifeline ID must be distinct from all model element IDs.

Dynamic views use ordered `steps`, not `relationships`. A relationship can repeat:

```json
{
  "id": "place_order",
  "kind": "dynamic",
  "scope": "order_system",
  "title": "Place an order",
  "description": "Submit an order and receive its identifier",
  "elements": ["customer", "api"],
  "steps": [
    {"relationship": "customer_calls_api", "description": "POST /orders"},
    {"relationship": "customer_calls_api", "description": "Return the order identifier", "response": true}
  ]
}
```

`response: true` reverses the relationship for a response. Evidence is shared from the static
relationship; it does not verify scenario ordering or wording as a runtime observation.
Inspect relevant code, tests, or traces. State conditions that define the scenario in the
description and record inspection limits in notes.

A step can also be a control block. Every branch needs a label and at least one
step. `optional` and `loop` take one branch whose label states the condition.
`alternative` and `parallel` take at least two branches. Blocks can nest. Leaf
steps still reference a relationship with both endpoints in this view. For example:

```json
{
  "kind": "alternative",
  "branches": [
    {"label":"In cache","steps":[{"relationship":"lookup","description":"Return cached result","response":true}]},
    {"label":"Cache miss","steps":[{"kind":"loop","branches":[
      {"label":"While pages remain","steps":[{"relationship":"fetch_page","description":"Read the next page"}]}
    ]}]}
  ]
}
```

Version 1 JSON models with flat steps remain valid. Upgrade the CLI before using
mappings, perspectives, structured steps, or deployment views without a scope.
For Rust callers that construct model structs directly, initialize the optional
`Element::code_mapping` and `View::perspective` fields to `None` and
`Element::perspectives` to its empty default. Wrap existing `Step` values in
`ScenarioItem::Interaction` when constructing `View::steps`.
