# C4 alignment and tool direction

Reviewed on 2026-09-14 against the supplied `the-c4-model` checkout and the current
cargo-c4doc implementation. The tool should help an agent maintain a small, explicit
architecture model whose views answer readers' questions. Syntax extraction, model
validation, and repeatable publication support that job; they cannot choose the right
architecture abstractions on their own.

This review deepens an earlier cargo-coupling/Litho comparison from a separate development checkout.
It uses the original English chapters, especially the discussion of model/code gaps,
ownership, messaging, and modeling at scale. Chapter references below identify the research
inputs recorded under Research provenance; those files and the comparison report are not
bundled or required to use this tool. No book text or figures are bundled.

## What the book changes about the design

### Boundaries require architectural decisions

Chapter 2 defines a Container through an application or data store and its logical
isolation. Its example of separately isolated applications sharing one JVM shows why
counting OS processes is insufficient. Conversely, multiple deployed copies of an
application remain instances of one logical Container. A Rust `bin`, Docker container,
workspace member, or linked library is therefore evidence to investigate, not a direct
C4 classification. See Chapter 2, Container (`content/en/ch02.md`, lines 83–174).

For cargo-coupling, CLI and `--web` are modes of the same native application: the entry
point selects the server path and calls the linked implementation. Its browser UI
executes JavaScript and contacts the HTTP API, so it is another Container. This decision
rests on entry points, calls, and execution environments together. It should be revisited
if the project adds an independently runnable server, another host application, or a
materially different deployment model. The comparison's Container view and citations
recorded this evidence in the separate development checkout.

System boundaries also require a viewpoint. Chapter 11 models services owned within one
software offering differently from services managed as independent systems. It also
warns against treating every feature team or capability as a new System. Ownership is
useful evidence but does not establish a one-team/one-System rule. The CLI cannot derive
these decisions from Cargo package names. See Chapter 11, Microservices and teams (`content/en/ch11.md`, lines 177–342).

### Data ownership is different from hosting

The word database can mean a schema, a server, or a dataset; those meanings produce
different diagrams. An API application and its MySQL schema should not become Components
inside an invented Container whose technology is "Rust and MySQL". They are separate
Container candidates. The database server belongs in deployment topology. Sharing a
server and sharing a schema are different design decisions. See Chapter 11, Abstraction versus organization (`content/en/ch11.md`, lines 88–173).

The same reasoning applies to messaging. A single broker hub hides which producers and
consumers are coupled. Model relevant queues/topics as data stores with ownership and
event-specific relationships, or explain a simple channel on an arrow when that better
serves the reader. Deployment nodes can then show which broker or cluster hosts them.
The book allows both explicit and simplified views; the tool should not force every
queue onto every diagram. See Chapter 11, Message-driven architectures (`content/en/ch11.md`, lines 292–327).

For cargo-coupling, source files, configuration, reports, and temporary worktrees are
explained in an I/O table. They are relevant resources, but their existence alone does
not require another service or data-store Container. Cargo and Git are shown as external
software tools because actual subprocess integrations were inspected.

### Components are responsibilities, while groupings organize them

Chapter 5 scopes a Component view to one Container. Chapter 11 explains why layers,
bounded contexts, and library boundaries can group existing abstractions instead of
adding another abstraction level. A Component can span several Rust modules behind an
interface; a `pub struct` can simply be a data structure used by those Components.
See Chapter 5 (`content/en/ch05.md`) and
Chapter 11, Groupings (`content/en/ch11.md`, lines 143–173).

In cargo-coupling, `ProjectMetrics` and `AppState` describe data and retained state.
The focused Component view instead describes analysis, evaluation, history, CLI, and
HTTP responsibilities, with a supporting module/function mapping. It intentionally
does not inventory every module or claim exhaustive coverage. Similarity and coupling
metrics help choose code to inspect; neither metric decides a Component boundary.

### The model should survive changes to its pictures

Chapter 12 distinguishes drawing shapes from defining elements and relationships once
and selecting different views over them. This supports cargo-c4doc's existing shared
JSON model, stable IDs, and separation between validation and rendering. A rename
propagates through a language edition without copying diagram labels by hand.
See Chapter 12, Diagramming and modeling (`content/en/ch12.md`, lines 71–89).

Scaling means selecting useful subgraphs, explicitly omitting irrelevant cross-cutting
concerns, and sometimes adding ownership or security perspectives. It does not mean
always rendering a larger graph. Context and Container views are the starting point;
Component, Code, and Dynamic views need a reader's question. See
Chapter 12, Scaling (`content/en/ch12.md`, lines 91–181) and
Chapter 12, Next steps (`content/en/ch12.md`, lines 231–260).

English and Japanese currently use separate model files. Stable IDs make structural
comparison possible, but translation synchronization is still an agent responsibility;
one translated edition is not automatically regenerated from the other.

### Evidence freshness is only one kind of drift

Chapter 11's model/code gap is more substantial than an old hash. A complete graph of
code definitions may still fail to communicate architecture, while naming and module
conventions alone may fail to recover conceptual Components. See
Chapter 11, Model/code gap (`content/en/ch11.md`, lines 14–75).

The current workflow distinguishes three cases:

| Change | Current detection | Required response |
| --- | --- | --- |
| A cited file changes | `check` rejects its stale SHA-256 | Read the changed source and reassess the claim before updating citations |
| Generated files differ from the model | `render --check` reports drift | Review the model/output difference; preserve manual edits |
| A new target, route, queue, asset, or deployment is absent from the model | `drift` compares saved targets, metadata, and file hashes; optional scans include auxiliary files | Review candidates against the declared scope; file changes do not establish feature semantics or coverage |

An unchanged model can therefore pass citation and rendering checks while omitting a new feature.
The subsequently implemented [drift command](drift.md) surfaces changes outside cited files
when they appear in the compared observations. It still cannot prove feature coverage.
`analyze.complete` concerns supported Rust traversal, and `--strict` concerns errors
and warnings in the supplied model. Neither means architectural completeness or
automatic semantic proof.

### A diagram needs context when shared on its own

Chapter 10 recommends readable titles identifying type and scope, meaningful element
and relationship labels, consistent notation, and a key explaining visual differences.
Its colors are examples rather than an obligatory C4 palette. Arrow direction can
describe dependencies or message/data flow, provided that labels make the meaning clear.
See Chapter 10, Titles and notation (`content/en/ch10.md`, lines 16–50) and
Chapter 10, Relationships and keys (`content/en/ch10.md`, lines 219–304).

The initial renderer supplied this context in Markdown but omitted a visible title and
key from standalone Mermaid images. It also reused static evidence styling prose for
Dynamic views, where dashed arrows actually indicate responses. This review led to
the implementation changes below, rather than adding more model fields.

## Changes implemented from this review

- Mermaid output now includes a visible title containing view type and scope, plus
  environment for Deployment views. The accessibility title is retained. Frontmatter
  is quoted and diagram text is escaped. Mermaid documents
  [YAML frontmatter support from 10.5](https://mermaid.js.org/config/configuration.html).
- Static diagrams include a neutral legend outside architecture boundaries. It has no
  model ID or architectural relationships. Dynamic diagrams include a note explaining
  solid interactions, dashed responses, and the limits of relationship evidence.
  Arrow labels explicitly say `Relationship evidence: Observed/Inferred`.
- Markdown legends now match each diagram's notation. The JSON model, source checks,
  model schema version, and protected publication workflow remain compatible.
- The shared [skill](../skills/c4-rust-docs/SKILL.md) and
  [Rust modeling guidance](../skills/c4-rust-docs/references/rust-modeling.md) now cover
  logical isolation, ownership, schemas, queues/topics, groupings, explicit omissions,
  unsupported C4 cases, and the three kinds of drift.

The bundled seven-view example and both four-view cargo-coupling editions use the updated
renderer. Model citations were reassessed before refresh. A regression test first failed
on the missing visible title, then covered all view kinds, Japanese/escaped titles,
scope/environment, and the distinction between response notation and evidence.

Standalone diagrams still rely on Markdown for detailed scenario preconditions,
omissions, and source citations. Dynamic participants currently show name, C4 type,
and evidence classification; their full technologies and responsibilities are in the
element table. Manual layout review remains necessary for long labels or dense views.

## Deliberate restrictions and remaining work

This table records the initial review. The November follow-up below records the changes
to deployment scope, perspectives, scenarios, and code mappings.

| Area | Current behavior | Book-informed decision |
| --- | --- | --- |
| Diagram notation | Mermaid flowcharts and sequences, including flowcharts for Code views | Adequate while type, scope, relationships, and notation remain clear; no need to adopt experimental Mermaid C4 syntax merely for its name |
| View scope | Fixed scopes and a constrained hierarchy | A helpful default; unsupported advanced abstractions should be reported rather than misclassified |
| Deployment | Requires a focal System and one environment; can include external System instances | The book also permits shared-infrastructure views without one focal System; support that only when a real case requires it |
| Groupings and perspectives | No structured group, ownership, or security overlay fields | Use supporting tables now; future groupings must not change the C4 parent hierarchy |
| Dynamic scenarios | Ordered references to static relationships, optional response reversal | Useful for selected scenarios; conditions are prose, not executable guards or trace validation |
| Model/code mapping | Source citations attached to claims | Sufficient for provenance, insufficient for coverage or automatic architecture conformance |
| Localization | Agent-authored explanatory text; renderer-owned text remains English | Keep English and requested Japanese in separate paths; verify structure and evidence remain aligned |

Chapter 7 supports selected scenario diagrams and optional richer sequence notation;
it does not demand exhaustive traces. Chapter 8 distinguishes nodes that host software
from infrastructure services and permits shared-system deployment views when useful.
See Chapter 7 (`content/en/ch07.md`) and
Chapter 8 (`content/en/ch08.md`, lines 16–61).

The first follow-up priority is now implemented as [observation drift](drift.md): an additive
JSON report that detects changed/new Cargo targets, metadata, and reachable Rust sources;
optional paired inventories cover auxiliary files. Informational unlinked Rust remains
distinct, and citation matches do not suppress review candidates. Root/package-scope mismatch
and incomplete observations cannot report a clean comparison. It is a review aid, not a
complete architecture coverage checker.

The initial review proposed three implementation priorities:

| Priority | User problem | Smallest useful change and acceptance criteria |
| --- | --- | --- |
| Next when a cross-module case needs it | Citations do not explain why files form one Component | Explicit, optional code-to-element mappings with rationale and declared coverage. Validate target IDs and source locations; retain multiple roles when justified; do not infer ownership from directories |
| Later, driven by a concrete system | Layers, service groupings, or ownership tables become unwieldy | Add orthogonal grouping/metadata without new C4 parents. Group removal must leave the underlying architecture graph unchanged; conflicting ownership must remain visible |
| Later, driven by a scenario | Conditions or parallel work are lost in prose | Add reviewed scenario metadata or structured control regions. Preserve existing ordered-step models and distinguish authored sequence evidence from runtime observations |

Schema extensions need explicit examples and compatibility decisions before implementation.
None of these priorities requires embedding an LLM provider, copying Litho's wiki pipeline,
reimplementing similarity-rs, or adding a graph database. The coding agent remains the
semantic author; the Rust CLI should concentrate on reproducible observations, checks,
and publication.

## Follow-up review, 2026-11-20

All seven view kinds have a bundled example. A reader could already follow the author's
chosen order, but starting at the landscape page and choosing a particular system required
searching the index. Chapter 9 describes navigating from the landscape to a selected
system's diagrams (`content/en/ch09.md`, line 69). The generated Markdown now lists each
matching scoped view under the element's name, in model order. For example, follow
cargo-c4doc from the [landscape](../examples/docs/landscape.md) to its Container view,
then follow the CLI to its Component view. These links use existing IDs and also work
when several scenarios or environments describe one element.

These were the gaps at the start of the follow-up:

| Reader task | Current limit | What would demonstrate the capability |
| --- | --- | --- |
| Review a component after a Rust module changes (Chapters 6 and 11) | Citations locate evidence and drift lists changed files. Neither records which parts of a module implement a component's interface or whether the mapping is partial. | Start with one component spanning two modules. Record why both contribute and let a reviewer find the affected component from either module. Keep the mapping explicit when one module serves multiple components. |
| See two peer systems on shared infrastructure (Chapter 8, line 19) | One environment can contain instances from several systems, but validation requires one focal system and an instance belonging to it. | Describe both systems on one host without naming either as the focus. Existing single-system views should still reject a missing focal instance. |
| Find a team's responsibilities across systems (Chapters 9, 11 and 12) | A view can select a subset of elements. Ownership, layers, and security perspectives can be described in prose but cannot be selected as model data. | Show the same IDs grouped by team in one view and by system in another, preserving C4 parents and evidence. |
| Follow a retry or conditional response (Chapter 7, line 44) | An ordered step can reuse or reverse a relationship. The conditions for executing it appear in prose or a separate scenario. | Represent the optional or repeated steps unambiguously if real scenarios warrant structured conditions. |
| Review a relationship label (Chapter 10 and the Appendix) | The validator requires a nonblank description. A label such as “uses” can still pass without telling the reader what crosses the boundary. | Have the reviewer read each arrow with both endpoints and identify what is requested or exchanged. Whether that sentence matches the intended interaction needs domain knowledge. |

Two CLI checks reproduced the original limits. In a temporary copy of the example, adding a
second system instance to the workstation passed validation. Removing only the deployment
view's scope then produced two errors about its focal system. In another copy, changing a
relationship description to “uses” produced no diagnostic for that relationship.

### Implemented capabilities

| Task | CLI behavior |
| --- | --- |
| Review a changed module | A component can declare the scope of its code mapping, cite several implementation ranges, and explain each contribution. The evidence page shows the mapping. A changed path can match several components in the drift report. |
| Describe shared hosts | A Deployment view can omit its focal system. The validator still checks environment, host ancestry, and software instances. |
| Explore ownership and security | A static logical view can select a named perspective and filter by one or more values. The renderer groups the selected elements, labels nodes, and includes edges whose endpoints remain visible. Other perspective pages at that scope are linked. |
| Describe conditional behavior | Nested optional, alternative, loop, and parallel blocks render in the Mermaid sequence diagram and its Markdown scenario table. Branches reference the same static relationships as ordinary steps. |
| Review vague labels | Bare labels such as “uses” or “depends on” trigger a warning identifying the relationship and its endpoints. Domain review is still needed for whether a more specific label is accurate. |

See the [model format](../skills/c4-rust-docs/references/model.md) for examples and
the [drift report](drift.md) for path-based matching and exit behavior.

## Research provenance

Verification of the initial renderer implementation and documents (before the subsequent
drift feature; see [its verification cases](drift.md#verification-cases)):

- `cargo fmt --check`, Clippy with all targets/features and warnings denied, and
  `cargo test --all --all-targets` passed (18 integration tests). The release binary
  was rebuilt and used for final generation checks.
- All seven bundled views and both four-view cargo-coupling editions passed
  `render --check`; cargo-coupling additionally passed strict validation. All 15
  Mermaid sources rendered with local Chrome. Representative English/Japanese
  layouts and the corrected punctuation in the Deployment title were inspected.
- Offline file-link and heading-fragment checks passed for both repositories'
  documentation. GitHub-style source-line fragments were excluded from the heading
  check; model validation checked citation ranges and hashes separately.
- similarity-rs reported the same two pairs before and after the change: Markdown
  versus Mermaid escaping, and labels for two different enums. They serve distinct
  formats/types, so no abstraction change followed. cargo-coupling inspected eight
  files/modules and found no significant hotspots. This new tool repository has no
  commit history, so its coupling check used `--no-git` and has no volatility evidence;
  the separate cargo-coupling case study used real Git history.

The source checkout HEAD was `5540b6d0cee0658757bcd7e587981dc98010a1cb`.
The inspected working tree includes uncommitted English chapter additions (10–12) and
Japanese translations, so the revision alone does not identify all research inputs.
These SHA-256 values identify the English files used for this deeper review:

| File | SHA-256 |
| --- | --- |
| `content/en/ch02.md` | `6eee46ff21f696cff8facd7109a4a3ae2fed19a345b348f9eebb45267860c0d5` |
| `content/en/ch05.md` | `a68dda46c8aaf63d281574a0e159f8d359442e95cb66e82bb673bec445f4efea` |
| `content/en/ch07.md` | `37766f2e5c19fdeae116af1af75b8b932c87cad291b16923a2d1ecc13590accd` |
| `content/en/ch08.md` | `57f90dc3dfebad6dd88aa9dcad9077db88f7f517ebc38c2297dbc59ec7dff1a7` |
| `content/en/ch10.md` | `09d5469aef34be182a10790de2f45eb10468618e6bd80d9f99e8a3f625b7e20f` |
| `content/en/ch11.md` | `d06bad361a30f2fa2fba17671820c83ab2c844187fffb3f1fc96dcbe21262c11` |
| `content/en/ch12.md` | `efcd7aeb7ce2ecb892ff54321d0d9e674daf5fe0b9d69dcd7b47d3d61295e6ee` |

Book files, reference-tool source code, and existing Litho output were inspected without modification.
The cargo-coupling comparison report records its target revision, inputs, post-generation
edits, source coverage, and the limits of comparing the two workflows.
