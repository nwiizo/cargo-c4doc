# Reviewing changes with saved observations

Before code changes, retain the `analyze` JSON used to review the model. Collect fresh
analysis after changes with the same root and package selection. Do not replace the
baseline before reviewing the difference. Store snapshots outside scanned inputs; use
an artifact directory for durable baselines or `target/` for disposable local checks.

```sh
cargo c4doc drift before-analysis.json after-analysis.json --model c4.json
```

For browser, configuration, deployment, and other recognized text files, supply paired
`--before-scan before-scan.json --after-scan after-scan.json` arguments. Run both scans
from `analyze.source_root`, with the same limits and ignore rules. Analysis completion
only covers supported Rust syntax; inventory comparison is still file-level inspection.

Review `.changes[]` entries with `requires_review: true`. Newly reachable Rust files
and Cargo targets can expose missing model scope; they do not establish C4 boundaries.
Modified files remain candidates even if `cited_claims` lists model elements/relationships.
`mapped_components` lists component IDs whose implementation mappings include a changed
path. Read each entry's rationale and check the actual diff against its source range.
`--model` accepts stale hashes for this whole-path lookup. Use `check` for source validation.
Read affected source and update relevant views and translations. Validate the model with `check`
for a model review or through `render` when generating updated documents.

Unlinked Rust files and documentation are informational for the gate. Inspect them when
the task involves tests, examples, generators, or build behavior; do not interpret unlinked
as dead code. Production-file hashes also change when only inline tests change.

The command emits JSON. Normally it returns 0 for a complete comparison even when changes
exist. `--check` returns 1 when review is required. Incomplete analysis, changed root/package
selection, or incomplete/incompatible scans must not be presented as a clean result.
Malformed inputs produce an error rather than a complete report. Preserve output files;
`--output` refuses overwrites.

After deciding whether each change affects the architecture, document meaningful omissions,
update model citations, regenerate/check documents, and choose a new reviewed baseline.
There is no automatic acknowledgment or suppression file. The command does not resolve
route semantics or detect changes outside the observation scope. Scan v1 lacks root/ignore
metadata, so matched Rust hashes cannot prove complete auxiliary coverage.
