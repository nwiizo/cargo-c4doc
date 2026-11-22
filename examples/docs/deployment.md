# How could a local CLI instance be shown?

[All views](README.md) · [Diagram source](deployment.mmd) · [Evidence](evidence.md)

This inferred deployment example places one CLI instance on a developer workstation. It illustrates notation and does not document an observed host.

- Kind: Deployment
- Scope: cargo-c4doc
- Environment: local\_example

```mermaid
---
title: "Deployment: cargo-c4doc — How could a local CLI instance be shown? (local_example)"
---
flowchart TB
    accTitle: Deployment: cargo-c4doc — How could a local CLI instance be shown#63; #40;local_example#41;
    subgraph n_workstation["Developer workstation [Deployment node] / Operating system / Inferred"]
    n_local_cli["CLI process #91;Instance#93; of cargo-c4doc CLI — Example instance started with cargo c4doc"]:::inferred
    end
    classDef observed fill:#e8f1fa,stroke:#28577c,color:#132f46
    classDef inferred fill:#fff4de,stroke:#8a5100,color:#573300,stroke-dasharray:5 5
    c4doc_legend["Legend (not a model element)<br/>Blue solid boxes: Observed<br/>Orange dashed boxes: Inferred<br/>Solid arrows: Observed<br/>Dotted arrows: Inferred<br/>Observed: cited source, not semantic proof<br/>Inferred: assumption requiring review<br/>Boundary labels name their C4 type<br/>Deployment nesting shows hosting"]
    style c4doc_legend fill:#fff,stroke:#666,color:#222
```

## Reading notes

For authors learning Deployment notation: the outer node describes the hosting environment, and the nested instance refers back to the CLI Container. The inferred styling keeps this assumption visible.

Replace this example with inspected environment evidence before using it as an account of a real deployment.

## Elements

| Name | Kind | Technology | Responsibility | Evidence |
| --- | --- | --- | --- | --- |
| Developer workstation | Deployment node | Operating system | Example local environment hosting the CLI | **Inferred**: Illustrates local CLI execution; no specific host or production environment was investigated. |
| CLI process | Instance | — | Example instance started with cargo c4doc | **Inferred**: Represents one CLI invocation to demonstrate deployment notation. |
