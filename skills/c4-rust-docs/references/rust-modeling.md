# Turning Rust observations into C4 decisions

| Observation | Useful evidence | What it does not establish |
| --- | --- | --- |
| Cargo workspace/package | Organization and declared dependencies | Product, ownership, or system boundaries |
| bin target / main | Executable candidates, arguments, I/O | Deployment reality or a long-running service |
| lib / rlib / proc-macro | Linked functionality or code generation | An independent application Container |
| mod / use / pub | Organization and written visibility | Business responsibility or externally reachable API |
| trait / impl | Interfaces and implementation candidates | Runtime implementation selection |
| Call expression | Where to investigate an interaction | Resolved dispatch, order, or frequency |
| Docker / Kubernetes / Terraform | Environment-specific placement candidates | An applied or operational deployment |

A C4 Container is an application or data store. Do not map crates, Docker containers,
modules, or dependencies to C4 Containers one-to-one. A simple Rust CLI usually has one
application Container with internal Components for analysis, transformation, and I/O.
Runtime isolation is a logical boundary, not a count of OS processes: separately runnable
applications may share a host process, and one application may have many deployed instances.
Inspect interfaces, independent execution, data ownership, and deployment intent together.
Confirm external services through calls and configuration; a `reqwest` dependency alone
does not prove one exists. Tokio, serde, and tree-sitter are usually implementation technologies.

For a CLI with an HTTP server mode, inspect whether these are modes of one application or
separately running applications. Do not split a bin mechanically. Conversely, JavaScript running
in a browser and calling an HTTP API is a separate Container candidate. Embedding assets does not
merge execution environments. Distinguish the browser product from the application's own browser UI.

Follow `include_str!`, `RustEmbed`, and static routes into HTML, JavaScript, and configuration.
Inspect `script src`, `fetch`, WebSockets, CDNs, and subprocess invocations such as Cargo and Git.
Successful Rust analysis does not cover those connections. Source files, standard output,
and temporary worktrees can be explained in an I/O table without inventing external software
systems. If modeled as data stores, explain their contents, ownership boundary, and lifecycle.

For a library-only crate, check whether its host application is within scope. Do not invent
a server without host requirements. If a hypothetical host is necessary, mark it `inferred`
and explain the assumption; focus on useful Component and Code views.

## Ownership, data stores, and groupings

System boundaries express a useful software offering and responsibility for it. Repository,
workspace, team, product domain, and bounded-context names are clues, not automatic boundaries.
An API and its database schema can be two Containers in one System; separately owned services
may instead be Systems. Feature teams can work across multiple Systems without redefining each
feature as another System. Explain the chosen viewpoint and treat other teams' implementation
details as opaque unless the task calls for a shared view.

For persistence, model the application-owned schema, bucket, or meaningful set of files.
The database server or cluster belongs in a Deployment view when placement matters. Distinguish
sharing a server from sharing a schema. For messaging, inspect queue/topic names, event types,
publishers, consumers, retention, and ownership. Model useful queues/topics as data-store
Containers, or describe a simple channel on a relationship when omission improves the view.
One generic broker box often hides producer/consumer coupling. Do not invent queue ownership.

Architectural layers, bounded contexts, shared libraries, and team groupings do not automatically
add levels to System → Container → Component → Code. A Component groups functionality behind
an interface; it may span several modules inside one Container. Use a supporting mapping table
for implementation responsibilities spanning modules, or record a component's `code_mapping`
to include its rationale and citations in drift review. Use element `perspectives` for
cross-cutting ownership or security groupings. Preserve the existing C4 parents.

For each difficult boundary, record: the decision, responsibility and interface, inspected source
locations, plausible alternative, and what evidence would change the decision. Keep this prose
outside generated directories. Do not force a large decision log for obvious boundaries.

## similarity-rs and cargo-coupling

Use these diagnostics when duplication or coupling could clarify a boundary under investigation.
When needed, check installed `--help` for supported options. Typical commands are:

```sh
similarity-rs <rust-source> --skip-test --threshold 0.90 --min-lines 10
cargo coupling <crate> --exclude-tests --hotspots=10
```

Inspect Git history and repository instructions. Use `--no-git` only when history is unavailable
and the target permits it; report missing volatility evidence. Never use it for cargo-coupling
itself, which requires history-based evaluation. Check file counts and module names. If analysis
expands to the workspace, limit conclusions to the requested scope. If a tool is unavailable,
report the omitted diagnostic and continue source-based investigation.

- Similarity identifies candidates for shared responsibility. Similar-looking transformations
  can have different input formats and failure behavior; read them before grouping.
- Coupling locates potential change propagation. Scores alone cannot justify boundaries;
  inspect public interfaces and callers.
- Separate existing architecture from proposals. Do not draw a proposed shared library or
  service as an existing Component or Container.

The RustParser in `mizchi/similarity` extracts functions, types, and line locations with
tree-sitter. cargo-c4doc uses that parsing foundation for reachability, Cargo metadata,
and citations; structural similarity calculation remains the job of similarity-rs.

## Review each view

- Context: users, purposes, and interactions with other software systems.
- Container: application/data-store boundaries and communication; distinguish bin, lib, and browser code.
- Component: responsibilities and interfaces inside one Container, not a directory listing.
  Do not place DTOs such as `ProjectMetrics`, state such as `AppState`, or static assets alongside
  responsibility-based Components. Use a Code view or supporting table when those details matter.
- Code: useful types, functions, and traits; disclose macro and dispatch uncertainty.
- Dynamic: scenarios tied to static relationship IDs. State arguments, queries, and success conditions.
  Distinguish cached responses from recomputation, in-process workers from services, and source-based
  ordering from runtime traces. Choose scenarios that clarify an important behavior; exhaustive
  sequence diagrams are unnecessary. Use optional, loop, alternative, or parallel blocks
  for conditions needed to understand a scenario. Cite the underlying interactions.
- Deployment: one environment's nodes and instances. Record configured scale with a replica
  range and give individually identified replicas distinct instance IDs. A node
  hosts software, while an infrastructure node represents a service such as DNS or a load balancer.
  Link instances to existing Systems/Containers, distinguish desired configuration from observed
  deployment, and keep development and production placements in separate views.
- Landscape: useful context across systems, not a list of crates.

Read each arrow as a sentence: source, label, destination. Dependency, data-flow, and event-flow
directions are all useful when consistent with their labels and the view's stated purpose.
Include meaningful technology/protocol details at the level where readers need them. Avoid
two static arrows merely to show an ordinary request and response; distinct interactions can
justify separate arrows. Blue/orange and solid/dashed evidence styling are this tool's notation,
not a universal C4 standard. In Dynamic views, dashed arrows mean responses; evidence is textual.

## Completeness and maintenance

For a whole-system guide, Context and Container are useful starting points. Select views for
the actual reader's question; a focused update or library explanation need not add both.
Code views and full module graphs are not mandatory C4 deliverables. Explicitly omit unrelated
responsibilities and cross-cutting concerns; use several focused views over shared IDs when a
diagram becomes dense.

Keep three checks distinct: citations can become stale, generated files can diverge from the
model, and architecture can change without touching any cited file. `check` and `render --check`
cover the first two. `drift` compares saved observations to surface new targets, changed metadata,
and source files; paired scans extend it to auxiliary files. It cannot determine whether a
feature's meaning is modeled. Review targets, routes, dependencies, assets, and deployment changes
against the model's declared scope. Do not equate `complete: true` or `--strict` with complete
architecture coverage or proven semantics.

The CLI enforces a fixed logical hierarchy. Omit the focal System for a Deployment view
of shared infrastructure within one environment. Report unsupported abstractions as tool
limitations and preserve the actual architectural boundaries.

References: [C4 diagrams](https://c4model.com/diagrams),
[similarity-rs](https://github.com/mizchi/similarity/tree/main/crates/similarity-rs).
