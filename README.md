# cargo-c4doc

A Rust CLI and coding-agent skill that turn a Rust project into an architecture guide you can follow back to the code.

Start with what someone wants to do. Follow the request across applications, find the responsibilities behind it, and open the supporting source. The generated guide connects short explanations, C4 diagrams, and checked citations.

The CLI collects Cargo targets and Rust syntax. A coding agent inspects the code, identifies responsibilities and runtime boundaries, and writes a JSON model. The CLI validates that model and its citations, then generates Markdown and Mermaid.

Read the [tool's own architecture guide](examples/docs/README.md) to see the output, or follow
[a model through validation and publication](examples/docs/render_flow.md) for one concrete workflow.

## Installation

Requires **Rust 1.100 or newer**, Cargo, and a C compiler/linker for the tree-sitter
dependencies. On macOS, install the Xcode Command Line Tools; on Linux, install your
distribution's C build tools; on Windows, use the Rust MSVC toolchain with Visual
Studio C++ Build Tools. Cargo must remain available when analyzing Rust projects.

From a checkout or extracted source archive, install the CLI onto Cargo's binary path:

```sh
cargo install --path . --locked
cargo c4doc --help
```

Ensure Cargo's binary directory is on `PATH` (normally `~/.cargo/bin`, or
`%USERPROFILE%\.cargo\bin` on Windows). `cargo c4doc` and `cargo-c4doc` invoke the same CLI.

To install from the GitHub source:

```sh
git clone https://github.com/nwiizo/cargo-c4doc.git
cd cargo-c4doc
cargo install --path . --locked
```

To install the release from crates.io:

```sh
cargo install cargo-c4doc --version 0.1.0 --locked
```

### Install the agent skill

Copy the entire [`skills/c4-rust-docs`](skills/c4-rust-docs/SKILL.md) directory into
your agent's skills directory as `c4-rust-docs`, including `references/` and `agents/`.
Alternatively, link that directory using an absolute path and keep the checkout in place.
Reload your agent's skills, then invoke `$c4-rust-docs`. Installing the CLI with Cargo
does not install the skill; obtain it from the matching source release. A copied skill
uses the CLI on `PATH` and does not require the author's directory layout.

## Getting started

Try the bundled model from the source root before documenting another project:

```sh
cargo c4doc check examples/c4doc.json --root . --json
cargo c4doc render examples/c4doc.json --root . --output examples/docs --check
```

Then ask your coding agent:

```text
Use $c4-rust-docs to document /path/to/rust-project.
Explain what the project lets a user do, then trace its main workflow.
Give each view a clear question, a short answer, and a useful detail to notice in the diagram.
Start with Context and Container; add other views when they help explain the workflow.
```

Documentation defaults to English in `docs/c4doc`. Explicitly requested Japanese documentation goes to `docs/c4doc-ja`, including Japanese-only requests, while preserving existing English output. An explicit destination overrides these skill defaults. The skill preserves IDs, commands, citations, and evidence classifications while translating explanatory text. Built-in renderer headings, legends, and diagnostics remain English; there is no `--language` flag. See [language guidance](skills/c4-rust-docs/references/languages.md).

When `cargo-c4doc` is on PATH, invoke it as `cargo c4doc`. Otherwise use the built binary's absolute path. In RTK-managed environments, prefix commands with `rtk proxy`.

```sh
# Run in the target Rust project
cargo c4doc analyze . --output rust-analysis.json
cargo c4doc scan . --output source-inventory.json
cargo c4doc init --title "Order service" --output c4.json
cargo c4doc schema > c4.schema.json

# The agent reads source code and edits c4.json
cargo c4doc cite src/main.rs --start 1 --end 20

# Validate and generate
cargo c4doc check c4.json --root . --json
cargo c4doc render c4.json --root . --output docs/c4doc

# Check source citations and generated content for drift
cargo c4doc render c4.json --root . --output docs/c4doc --check
```

`init` creates a draft, not a completed architecture. Copy inspected `items[].source` observations into `evidence.sources` when they support a claim. Verify interactions by reading callers, imports, and implementations. Commands that create JSON output refuse to overwrite existing files.

For a Japanese model authored as `c4.ja.json`, generate the separate edition with:

```sh
cargo c4doc render c4.ja.json --root . --output docs/c4doc-ja
cargo c4doc render c4.ja.json --root . --output docs/c4doc-ja --check
```

## Rust analysis

`analyze PATH` uses `cargo metadata --format-version 1 --no-deps --offline --locked` and tree-sitter-rust. It does not build the target or execute build.rs, tests, or procedural macros. Cargo follows the target project's toolchain configuration.

- Packages, production targets, and declared dependencies, including aliases, optional flags, features, and platform conditions.
- Ordinary modules, inline modules, and supported `#[path]` declarations reachable from bin/lib roots.
- Functions, methods, structs, enums, traits, aliases, written visibility, signatures, attributes, and citations.
- Imports, extern crates, and syntactic call expressions. Long method chains omit arguments.
- Unlinked Rust files and diagnostics for parse errors, unexpanded item macros, and other unsupported constructs.

A package manifest selects that package; a virtual workspace root selects its packages. Use `--package NAME` to select one. **Use the returned `source_root` for `check` and `render --root`**, including when analyzing one workspace member.

`complete` means traversal of supported syntax succeeded, not that type resolution or architectural understanding is complete. Incomplete analysis emits JSON and exits with status 1. Test modules and test attributes are skipped; other cfg conditions are retained without evaluation. Macro expansion, dynamic dispatch, function pointers, and conditional module paths require further investigation. Reads are limited to 2 MiB per file and traversal to 32 module levels. Unlinked files may be tests, examples, or generation inputs.

`scan` is an auxiliary inventory. It respects ignore files and excludes hidden files, symlinks, and directories such as target, node_modules, and vendor. Defaults are 5,000 files and 2 MiB per file. It emits line counts and SHA-256 hashes, not contents. Review `truncated`, `skipped`, and `limitations`. It is not a secret scanner.

Rust analysis does not cover browser-side code or CDN connections. Follow static asset embedding and HTTP routes into HTML, JavaScript, and configuration when modeling a web-enabled Rust application.

## Observation drift

Use `drift` to find changes that may require architecture review, including files that
the model does not cite:

```sh
cargo c4doc drift before-analysis.json after-analysis.json --model c4.json --check
```

It compares saved `analyze` results for the same root/package selection. Paired
`--before-scan` and `--after-scan` inputs add browser, configuration, deployment, and
other file changes. JSON output distinguishes production changes, informational unlinked
Rust files, and citation references. `--check` returns 1 for review candidates; incomplete
comparisons always return 1. See [the capture workflow, report format, and limitations](docs/drift.md).

## Models and views

`schema` generates JSON Schema from Rust types. Models use `schema_version: 1`; unknown fields, enum values, and versions are rejected. See the [model format](skills/c4-rust-docs/references/model.md) and [self-documentation model](examples/c4doc.json).

The model's `views` array sets the reading order. The index presents a short answer for each
view, and each page links to the next. In a view's Elements table, follow the links under
a name to explore other views scoped to that element. Use a reader's question for `title`, its answer for
`description`, and optional `notes` paragraphs for diagram guidance, source entry points,
conditions, and omissions. Supporting notes stay on the view page, keeping the index short.
Existing models without notes still work; older binaries must be upgraded to read the new
field. See [reader experience guidance](skills/c4-rust-docs/references/reader-experience.md).

| View | Scope | Output |
| --- | --- | --- |
| Context | One system, users, and external systems | Mermaid flowchart |
| Container | Applications/data stores within one system | Mermaid flowchart |
| Component | Responsibilities within one Container | Mermaid flowchart |
| Code | Types/functions within one Component | Mermaid flowchart |
| Landscape | Systems and users across a selected landscape | Mermaid flowchart |
| Dynamic | Ordered interactions referencing static relationships | Mermaid sequenceDiagram |
| Deployment | Nodes and instances in one environment | Mermaid flowchart |

Use a component's `code_mapping` to record which source ranges implement its responsibility
and why. The Elements table links to these mappings. If a mapped file changes,
`drift --model` names the components to review. Static logical views can group and filter
elements by owner, security classification, or another named perspective. Dynamic views can
show optional steps, retry loops, alternatives, and parallel branches. A Deployment view
can omit its focal system to show several systems on shared infrastructure. See the
[model fields and examples](skills/c4-rust-docs/references/model.md).

C4 abstractions use ordinary Mermaid shapes. Code views also use flowcharts. Each Markdown view contains a title, purpose, scope, legend, explanations, and citations. Standalone `.mmd` output includes a visible title with view type, scope, and deployment environment when applicable, an accessibility title, and a legend. Dynamic legends distinguish response arrows from evidence classifications and explain that relationship evidence does not establish execution order. Use a Mermaid renderer supporting YAML frontmatter (Mermaid 10.5+) for images; retain the Markdown for scenario conditions, omissions, and citations.

The [C4 alignment review](docs/c4-alignment.md) explains the book's boundary rules, their application to Rust and cargo-coupling, the tool's deliberate restrictions, and prioritized improvements with acceptance criteria.

Output includes `README.md`, `evidence.md`, each view's `.md`/`.mmd`, and a management manifest. Regeneration removes obsolete managed views. **Updates refuse unmanaged directories, manually edited generated files, and added handwritten files.** Keep handwritten documents outside the output directory. Files are prepared before directory replacement, with rollback for ordinary write failures. Concurrent writers and power-loss transactions are outside the supported scope.

## Evidence and validation

`observed` requires a relative source path, one-based inclusive line range, and SHA-256 of the entire file. `inferred` requires a reason. Assumptions and unresolved questions remain visible; `--strict` treats warnings as failures.

Bare relationship labels such as “uses” or “depends on” produce a warning naming both
endpoints. Describe the request, event, or data exchanged. `--strict` also applies to
these warnings in existing models.

`check` validates IDs, hierarchy, abstraction levels, scope, endpoints, deployment environments, and citation paths/ranges/hashes. **The agent must determine whether the cited text supports the claim.** Observed roles or responsibilities may be supported by project declarations rather than executed behavior. Dynamic ordering is not automatically verified against runtime traces.

A change anywhere in a cited file changes its hash. Reassess the affected claim before refreshing citations. Blind hash replacement defeats this check. Generated `#L...` fragments use GitHub conventions; local viewers may not support line navigation. Verify file existence separately from ranges and hashes.

| Exit status | Meaning |
| --- | --- |
| 0 | Completed or validated successfully |
| 1 | Invalid input/model, incomplete analysis/comparison, stale output, drift review gate, or I/O failure |
| 2 | Invalid CLI arguments |

## similarity-rs and cargo-coupling

Use similarity-rs to investigate repeated responsibilities and cargo-coupling to investigate change propagation. Neither metric determines a C4 boundary by itself. Confirm tool versions, scope, public APIs, callers, and runtime isolation.

```sh
similarity-rs src --skip-test --threshold 0.90 --min-lines 10
cargo coupling . --exclude-tests --hotspots=10
```

Check installed `--help` first. Use `--no-git` only when history is unavailable and the target permits it. cargo-coupling itself requires history-based evaluation. Do not start refactoring merely because a documentation task found similar code.

## Examples and verification

```sh
cargo run --locked -- render examples/c4doc.json --root . --output examples/docs
cargo run --locked -- render examples/c4doc.json --root . --output examples/docs --check
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

The bundled [example documentation](examples/docs/README.md) demonstrates all seven supported views. Its inferred local deployment illustrates notation, not an observed production environment. Real projects should use only views that answer useful questions. Recheck example citations after implementation changes.

CI runs the tests on Linux, macOS, and Windows, checks Rust 1.100 compatibility, and
checks formatting, Clippy, and the packaged source. See [Releasing](docs/releasing.md)
for the publication steps and verification of an installed release.

During development, `analyze --package similarity-rs` was run on local mizchi/similarity revision `a9c9f0837e7ba8529771408cd02baf232bf6bac5` (similarity-rs 0.4.2). It returned two bin/lib targets, seven target-specific module observations, 55 definitions, 29 imports, and no diagnostics. Shared RustParser files appear under both targets, so these are not unique file/symbol counts. The library does not justify another Container in the CLI runtime view. This local version is distinct from GitHub's current main.

cargo-coupling 0.4.0 at `5b019b2642f7dd7275d742d308b898b9047d2e6c` was also analyzed: 54 unique Rust files, 681 definitions, 313 imports, and no diagnostics. Its CLI/server and browser application require separate runtime modeling despite asset embedding. Its model, four views, and comparison with existing Litho output were recorded in a separate development checkout and are not bundled. The [C4 alignment review](docs/c4-alignment.md) records the resulting design decisions and research provenance.

## Sources of inspiration

- Simon Brown's *The C4 Model*, using the supplied local repository, including draft translations. Abstraction, view scope, notation, and modeling informed the design. Book text and images are not bundled.
- [C4 diagrams](https://c4model.com/diagrams): scope and view selection.
- [deepwiki-rs](https://github.com/sopaco/deepwiki-rs) and [smart-docs](https://github.com/sopaco/deepwiki-rs/blob/main/skills/smart-docs/SKILL.md): staged collection, research, and agent-authored documentation.
- [mizchi/similarity RustParser](https://github.com/mizchi/similarity/blob/main/crates/similarity-rs/src/rust_parser.rs): extracting functions, types, and source locations with tree-sitter. Similarity algorithms are not reimplemented.
- [Rust modules](https://doc.rust-lang.org/reference/items/modules.html) and [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html): structure and Cargo observations.

Independently implemented. MIT License.
