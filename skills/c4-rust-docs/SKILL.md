---
name: c4-rust-docs
description: Create, update, or review C4 architecture guides for Rust projects with cargo-c4doc.
---

# Rust C4 documentation

Produce a guide that helps readers understand a Rust system and find relevant implementation
entry points. `cargo-c4doc` collects observations, validates models, and renders documents;
the coding agent determines responsibilities, boundaries, and what the evidence supports.

## Scope and defaults

The user's requested deliverable, scope, and established choices take precedence over this
skill's defaults. A documentation request includes editing its model, regenerating the requested
output, and resolving issues introduced by those edits. A review-only request ends with findings.
Documenting the target does not require refactoring its implementation.

Default to English in `docs/c4doc`, even in a Japanese conversation. Explicitly requested Japanese
documentation uses `docs/c4doc-ja`, including Japanese-only output. Honor an explicit destination
or established document-language requirement. Use [Language guidance](references/languages.md)
for translation and multilingual maintenance.

## Select the work and references needed

- **New model or architectural changes:** use `cargo c4doc analyze <target> --output <new-analysis.json>`;
  `--package <name>` limits the package selection. Use its `source_root` for later `--root`
  arguments and inspect `complete` and `diagnostics`. Follow relevant callers, implementations,
  assets, and configuration; `scan` helps inventory non-Rust inputs. Use
  [Rust modeling](references/rust-modeling.md) for boundary decisions and
  [Model format](references/model.md) when authoring structural fields.
- **Existing prose or presentation:** edit the model's display fields and regenerate affected
  output. Reuse reviewed evidence while it remains current; a wording change alone does not
  require fresh whole-project analysis or a target Rust test run. For a new guide or a substantial
  rewrite, use [Reader experience](references/reader-experience.md) to connect reader questions,
  concrete insights, and a useful reading path.
- **Source drift, including uncited files:** use [Drift review](references/drift.md) for comparing
  saved observations, checking scope, and deciding which views need updates. Without a comparable
  baseline, inspect the available source diff and state the coverage limit.
- **Comparison with Litho or another generator:** use
  [Reviewing generated documentation](references/reviewing-generated-docs.md) for comparable
  inputs, post-generation edits, and evidence-based findings.

These are task-dependent entry points, not a sequence to run for every request.
Load only references needed for the current work.

## Find the CLI

Use `cargo c4doc --help` if available. A copied skill does not bundle an executable or
require a neighboring Cargo project. If the CLI is missing, install from an available
cargo-c4doc source checkout with `cargo install --locked --path <checkout>`, or from
crates.io with `cargo install cargo-c4doc --version 0.1.0 --locked`.
Rust 1.100 or newer and a C compiler/linker are required. Ensure Cargo's binary
directory is on PATH. Do not change the target project's dependencies to make the CLI available.

For a skill linked from a cargo-c4doc checkout, resolve its physical directory and verify
that `../../Cargo.toml` belongs to `cargo-c4doc` before using that fallback. A local build
with `cargo build --release --locked --manifest-path <path>` also works; invoke the binary
from Cargo's target directory by absolute path. Respect local command wrappers such as
RTK. The maintained Rust CLI owns analysis, validation, and generation; do not replace it
with disposable scripts. If unavailable, continue useful source/model work and identify
which execution checks could not be completed.

## Modeling and evidence

- Choose views for the reader's task. Context and Container are useful starting points for a
  whole-system guide; a focused update or library explanation may need a different selection.
  Crates, files, and OS process counts do not establish C4 boundaries. Reuse IDs across views.
- Cite inspected source with an `analyze` observation or `cargo c4doc cite <relative-path>
  --root <source_root> --start <line> --end <line>`. Hashes cover whole files: reassess affected
  claims before refreshing citations. Keep assumptions `inferred` and unresolved issues in `questions`.
- `complete: true` covers supported syntax, and successful validation checks structure and
  citation integrity. Neither proves semantic support, runtime ordering, or architecture coverage.

## Complete the requested output

Use `cargo c4doc render <model> --root <source_root> --output <directory>` to validate and
generate. Use `check <model> --root <source_root> --json` for diagnostics or a model-only review;
`--strict` is appropriate when the scope should contain no inferences or unresolved questions.

Keep models and handwritten documents outside the managed directory. If publication refuses
manual edits, preserve them and render to a new directory for comparison; do not bypass protection.
Use the render command with `--check` to confirm the requested output matches the model.

Finish a generation request with the requested guide rendered, validation issues resolved or
explained, and the changed content reviewed. Inspect changed links and diagrams, using an available
Mermaid renderer when layout is affected. Report the entry document, model, checks performed,
and material limits. Review findings are not a stopping point when their fixes are already in scope.
