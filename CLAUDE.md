# cargo-c4doc

A Rust CLI for creating C4 documentation with coding agents. Separate Cargo/tree-sitter
observations, agent modeling decisions, model validation, and document generation.

- `src/rust_analysis.rs`: traverse production Cargo targets; observations are not resolved call graphs.
- `src/drift.rs`: compare saved observations; citation matches never suppress review candidates.
- `src/model.rs` / `src/check.rs`: versioned JSON, C4 hierarchy, and evidence validation.
- `src/source.rs`: bounded reads, citations, and auxiliary inventory.
- `src/render.rs` / `src/output.rs`: Markdown/Mermaid rendering and manual-edit protection.
- `skills/c4-rust-docs/`: agent workflow maintained with the CLI.

Write documentation in English by default. Support requested Japanese through the skill's
language guidance. Preserve technical identifiers and evidence across translations.

Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --all --all-targets`. Check example citations and generated diffs as well.
Do not use unwrap/expect/unsafe in production code. Preserve structured-output compatibility.
Do not promote library dependencies to independent C4 Containers automatically.
Preserve failure handling that protects manual edits. Commit or publish only when requested.
