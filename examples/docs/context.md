# How does code become an architecture guide?

[All views](README.md) · [Diagram source](context.mmd) · [Evidence](evidence.md)

The author uses cargo-c4doc to collect source observations, check a model, and generate documentation. The coding agent decides what those observations mean for the architecture.

- Kind: System context
- Scope: cargo-c4doc

```mermaid
---
title: "System context: cargo-c4doc — How does code become an architecture guide?"
---
flowchart TB
    accTitle: System context: cargo-c4doc — How does code become an architecture guide#63;
    n_author["Developer and coding agent #91;Person#93; — Inspects code and models architecture with evidence"]:::observed
    n_tool["cargo-c4doc #91;Software system#93; — Supports Rust C4 documentation through analysis#44; change review#44; validation#44; and rendering"]:::observed
    n_author -->|"Requests analysis#44; change review#44; model validation#44; and documentation"| n_tool
    classDef observed fill:#e8f1fa,stroke:#28577c,color:#132f46
    classDef inferred fill:#fff4de,stroke:#8a5100,color:#573300,stroke-dasharray:5 5
    c4doc_legend["Legend (not a model element)<br/>Blue solid boxes: Observed<br/>Orange dashed boxes: Inferred<br/>Solid arrows: Observed<br/>Dotted arrows: Inferred<br/>Observed: cited source, not semantic proof<br/>Inferred: assumption requiring review<br/>Boundary labels name their C4 type<br/>Deployment nesting shows hosting"]
    style c4doc_legend fill:#fff,stroke:#666,color:#222
```

## Reading notes

Start at the author and follow the request to the tool. The same CLI supports collecting evidence, checking a model, reviewing source changes, and publishing a guide.

This view introduces the workflow to documentation authors. Source-file I/O is outside its scope; the responsibility and interaction views explain the implementation.

## Elements

| Name | Kind | Technology | Responsibility | Evidence |
| --- | --- | --- | --- | --- |
| Developer and coding agent | Person | — | Inspects code and models architecture with evidence | Observed: [src/main.rs:14–25](../../src/main.rs#L14-L25) |
| cargo-c4doc<br/>[What do I need to run?](containers.md)<br/>[How could a local CLI instance be shown?](deployment.md) | Software system | — | Supports Rust C4 documentation through analysis, change review, validation, and rendering | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |

## Relationships

| From → To | Interaction | Technology | Evidence |
| --- | --- | --- | --- |
| Developer and coding agent → cargo-c4doc | Requests analysis, change review, model validation, and documentation | — | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |

Continue reading: [What do I need to run?](containers.md).
