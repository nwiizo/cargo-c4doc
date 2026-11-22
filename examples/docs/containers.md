# What do I need to run?

[All views](README.md) · [Diagram source](containers.mmd) · [Evidence](evidence.md)

One Rust executable provides collection, comparison, validation, and rendering. Authors pass commands and models to the CLI and read its JSON or document output.

- Kind: Container
- Scope: cargo-c4doc

```mermaid
---
title: "Container: cargo-c4doc — What do I need to run?"
---
flowchart TB
    accTitle: Container: cargo-c4doc — What do I need to run#63;
    n_author["Developer and coding agent #91;Person#93; — Inspects code and models architecture with evidence"]:::observed
    subgraph b_tool["cargo-c4doc [Software system]"]
    n_cli["cargo-c4doc CLI #91;Container#93; / Rust / clap — Runs subcommands and emits JSON#44; documents#44; and diagnostics"]:::observed
    end
    n_author -->|"Supplies arguments and models#44; then reads results #91;Command line / JSON / Markdown#93;"| n_cli
    classDef observed fill:#e8f1fa,stroke:#28577c,color:#132f46
    classDef inferred fill:#fff4de,stroke:#8a5100,color:#573300,stroke-dasharray:5 5
    c4doc_legend["Legend (not a model element)<br/>Blue solid boxes: Observed<br/>Orange dashed boxes: Inferred<br/>Solid arrows: Observed<br/>Dotted arrows: Inferred<br/>Observed: cited source, not semantic proof<br/>Inferred: assumption requiring review<br/>Boundary labels name their C4 type<br/>Deployment nesting shows hosting"]
    style c4doc_legend fill:#fff,stroke:#666,color:#222
```

## Reading notes

The boundary contains one application. Rust libraries linked into it support its work without creating additional C4 Containers.

This view helps authors understand the tool's runtime shape. The coding agent is represented as an author; its hosting environment and model provider are outside this scope.

## Elements

| Name | Kind | Technology | Responsibility | Evidence |
| --- | --- | --- | --- | --- |
| Developer and coding agent | Person | — | Inspects code and models architecture with evidence | Observed: [src/main.rs:14–25](../../src/main.rs#L14-L25) |
| cargo-c4doc CLI<br/>[Where does the model get checked?](components.md)<br/>[What happens when I render a model?](render_flow.md) | Container | Rust / clap | Runs subcommands and emits JSON, documents, and diagnostics | Observed: [src/main.rs:135–299](../../src/main.rs#L135-L299) |

## Relationships

| From → To | Interaction | Technology | Evidence |
| --- | --- | --- | --- |
| Developer and coding agent → cargo-c4doc CLI | Supplies arguments and models, then reads results | Command line / JSON / Markdown | Observed: [src/main.rs:135–299](../../src/main.rs#L135-L299) |

Continue reading: [Where does the model get checked?](components.md).
