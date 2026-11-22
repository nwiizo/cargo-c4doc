# Reviewing existing generated documentation

Use this guidance when comparing against Litho or another generator. Review explanatory value
and C4 structure separately. Identify post-generation edits before attributing results to a tool.
Preserve reference artifacts; write the comparison in the requested output language.

## Establish comparable conditions

- Record the source revision and uncommitted changes, tool version, model, additional documents,
  and exclusions. Reuse run records when available instead of repeating an LLM run solely for comparison.
- Separate definition counts, unique Rust files, all-language inventories, and document counts.
  Check duplicate files reached from bin/lib targets and differences in tests, benchmarks, and assets.
- Do not compare syntax-analysis time with agent research and writing time. Internal completion
  percentages and estimated costs are not accuracy measures or actual billing.

## Review claims against evidence

| Dimension | Check |
| --- | --- |
| Purpose, audience, scope | Question answered, omitted modes, decisions inherited from existing documents |
| Context | People and systems; avoid mixing file resources and execution environments as peers |
| Container | Applications/data stores, system boundary, technologies, communication protocols |
| Component | Responsibilities and entry points; avoid mixing functions, DTOs, state, and assets |
| Relationships | Direction and purpose; distinguish calls, communication, and data flow |
| Dynamic and deployment | Scenario/environment; distinguish code, configuration, and operational evidence |
| Consistency and updates | Shared IDs, names, boundaries, citations, manual-edit protection, source drift |
| Readable output | Titles, view types, legends, text size, crossings; assess Markdown and images separately |

Validate source-line links separately from heading links; see [Model format](model.md) for local
`#L...` limitations. Excluding fragments from a link checker does not replace evidence validation.

Ordinary Mermaid flowcharts can express C4 abstractions. `C4Context` syntax does not establish
architectural correctness. Do not add diagrams simply to fill four levels or seven view types.

For each difference, record the passage, code/C4 evidence, consequence for readers, and response.
Separate current architecture from improvement proposals. Inspect supplied C4 publications and
identify the actual version, including local changes.

## Lessons from cargo-coupling

CLI and Axum were modes of one binary; the browser UI ran separately. RustEmbed assets coexisted
with unpkg.com imports, so embedding did not establish offline operation. `main` and `web::design`
used shared `assess` logic. `GET /api/design` selected cached results or comparison assessment
according to its query. Reconfirm these patterns in each target before generalizing.

Passing `check --strict` establishes structure and citation file/range/hash consistency, not semantic
support, conditional ordering, or coverage of new dependencies. Rust `complete: true` has similar
limits. Distinguish gaps resolved by reading code from organizational, operational, or planning
information that people must supply.
