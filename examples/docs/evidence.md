# Evidence

[All views](README.md)

Source digests cover the complete file. After a source changes, inspect the affected claim before replacing the citation with `cargo c4doc cite`.

| Claim | Status and references |
| --- | --- |
| Element: Developer and coding agent | Observed: [src/main.rs:14–25](../../src/main.rs#L14-L25) |
| Element: cargo-c4doc | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |
| Element: cargo-c4doc CLI | Observed: [src/main.rs:135–299](../../src/main.rs#L135-L299) |
| Element: Command dispatch | Observed: [src/main.rs:135–281](../../src/main.rs#L135-L281) |
| Element: Rust structural analysis | Observed: [src/rust\_analysis.rs:118–262](../../src/rust_analysis.rs#L118-L262) |
| Element: Observation comparison | Observed: [src/drift.rs:353–437](../../src/drift.rs#L353-L437) |
| Element: C4 model validation | Observed: [src/check.rs:58–297](../../src/check.rs#L58-L297) |
| Element: Document rendering | Observed: [src/render.rs:8–276](../../src/render.rs#L8-L276) |
| Element: Output publication | Observed: [src/output.rs:64–138](../../src/output.rs#L64-L138) |
| Element: analyze | Observed: [src/rust\_analysis.rs:118–262](../../src/rust_analysis.rs#L118-L262) |
| Element: RustWalker | Observed: [src/rust\_analysis.rs:280–287](../../src/rust_analysis.rs#L280-L287); [src/rust\_analysis.rs:302–371](../../src/rust_analysis.rs#L302-L371) |
| Element: Developer workstation | **Inferred**: Illustrates local CLI execution; no specific host or production environment was investigated. |
| Element: CLI process | **Inferred**: Represents one CLI invocation to demonstrate deployment notation. |
| Relationship: Requests analysis, change review, model validation, and documentation | Observed: [src/main.rs:26–114](../../src/main.rs#L26-L114) |
| Relationship: Supplies arguments and models, then reads results | Observed: [src/main.rs:135–299](../../src/main.rs#L135-L299) |
| Relationship: Requests the selected Rust project's structure | Observed: [src/main.rs:137–145](../../src/main.rs#L137-L145) |
| Relationship: Compares saved observations and optionally applies the review gate | Observed: [src/main.rs:167–184](../../src/main.rs#L167-L184) |
| Relationship: Requests model diagnostics | Observed: [src/main.rs:192–233](../../src/main.rs#L192-L233) |
| Relationship: Builds documents from a validated model | Observed: [src/main.rs:234–278](../../src/main.rs#L234-L278) |
| Relationship: Validates the model and citations before rendering | Observed: [src/render.rs:8–21](../../src/render.rs#L8-L21) |
| Relationship: Saves documents or compares them with existing output | Observed: [src/main.rs:269–277](../../src/main.rs#L269-L277) |
| Relationship: Creates target traversal state and invokes file | Observed: [src/rust\_analysis.rs:118–262](../../src/rust_analysis.rs#L118-L262) |

<a id="implementation-validation"></a>

## Implementation of C4 model validation

Source checks used for observed claims and component implementation mappings.

| Source | Contribution |
| --- | --- |
| [src/check.rs:453–488](../../src/check.rs#L453-L488) | Checks each cited range and digest against the inspected file. |
| [src/source.rs:56–81](../../src/source.rs#L56-L81) | Opens the repository-relative source and counts the lines used for range checks. |
