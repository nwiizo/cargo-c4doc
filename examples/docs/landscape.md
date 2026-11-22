# How would a broader system map begin?

[All views](README.md) · [Diagram source](landscape.mmd) · [Evidence](evidence.md)

This minimal Landscape example contains cargo-c4doc and its author without selecting one system as the scope.

- Kind: System landscape
- Scope: Cross-system

```mermaid
---
title: "System landscape: Cross-system — How would a broader system map begin?"
---
flowchart TB
    accTitle: System landscape: Cross-system — How would a broader system map begin#63;
    n_author["Developer and coding agent #91;Person#93; — Inspects code and models architecture with evidence"]:::observed
    n_tool["cargo-c4doc #91;Software system#93; — Supports Rust C4 documentation through analysis#44; change review#44; validation#44; and rendering"]:::observed
    n_author -->|"Requests analysis#44; change review#44; model validation#44; and documentation"| n_tool
    classDef observed fill:#e8f1fa,stroke:#28577c,color:#132f46
    classDef inferred fill:#fff4de,stroke:#8a5100,color:#573300,stroke-dasharray:5 5
    c4doc_legend["Legend (not a model element)<br/>Blue solid boxes: Observed<br/>Orange dashed boxes: Inferred<br/>Solid arrows: Observed<br/>Dotted arrows: Inferred<br/>Observed: cited source, not semantic proof<br/>Inferred: assumption requiring review<br/>Boundary labels name their C4 type<br/>Deployment nesting shows hosting"]
    style c4doc_legend fill:#fff,stroke:#666,color:#222
```

## Reading notes

This additional view is for authors learning the available notation. It has the same elements as Context because this example models only one system; it does not add an invented ecosystem.

In a project guide, add a Landscape view when relationships across multiple systems answer a useful question. It is not a required stop for every reader.

## Elements

| Name | Kind | Technology | Responsibility | Evidence |
| --- | --- | --- | --- | --- |
| Developer and coding agent | Person | — | Inspects code and models architecture with evidence | Observed: [src/main.rs:14–25](../../src/main.rs#L14-L25) |
| cargo-c4doc<br/>[How does code become an architecture guide?](context.md)<br/>[What do I need to run?](containers.md)<br/>[How could a local CLI instance be shown?](deployment.md) | Software system | — | Supports Rust C4 documentation through analysis, change review, validation, and rendering | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |

## Relationships

| From → To | Interaction | Technology | Evidence |
| --- | --- | --- | --- |
| Developer and coding agent → cargo-c4doc | Requests analysis, change review, model validation, and documentation | — | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |

Continue reading: [How could a local CLI instance be shown?](deployment.md).
