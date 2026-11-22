# Reviewing architecture changes with drift

`cargo c4doc drift` compares two saved `analyze` results and reports changes that may
require updates to a C4 model. It detects new Cargo targets and reachable Rust files
even when the model has no citations for them. Optional `scan` results extend the
comparison to browser assets, configuration, deployment files, and other recognized text files.

The command reads JSON only. It does not run Cargo, execute repository code, rewrite
the model, or acknowledge changes automatically. Existing `analyze`, `scan`, and model
JSON formats remain version 1 and are accepted without migration.

## Capture and compare

Collect observations before editing code. Keep them outside scanned inputs, such as
under `target/` or in a separate artifact directory. Use unused filenames: output
creation refuses overwrites. Preserve review baselines as long as needed; Cargo cleanup
can remove files under `target/`.

```sh
# From the target Rust project, before a code change
cargo c4doc analyze . --output target/c4-before.json
cargo c4doc scan . --output target/c4-before-scan.json

# After the code change, using the same package selection and scan settings
cargo c4doc analyze . --output target/c4-after.json
cargo c4doc scan . --output target/c4-after-scan.json

cargo c4doc drift target/c4-before.json target/c4-after.json \
  --before-scan target/c4-before-scan.json \
  --after-scan target/c4-after-scan.json \
  --model docs/c4doc-model.json \
  --output target/c4-drift.json
```

Run both scans from the returned `analyze.source_root`. This is especially important
when `--package` selects a workspace member. Prefix commands with `rtk proxy` in
RTK-managed environments. `--model` adds claim citations and component mappings to
matching paths. Omit both scan options for Rust-only comparison.

Read review candidates with an existing JSON tool:

```sh
jq '.changes[] | select(.requires_review)' target/c4-drift.json
```

Read the changed code, determine whether responsibilities or boundaries changed, then
update the model and regenerate documents. Run `check` and `render --check` separately.
Only select a new baseline after reviewing the changes. Adding a citation alone does
not clear an outstanding change between the same observation pair.

## Report format

The report uses `schema_version: 1`. Changes are sorted by category and key; paths and
claim references are sorted and deduplicated. Consumers should use field names and
explicit category/change values, not positions or prose in diagnostics.

| Field | Meaning |
| --- | --- |
| `before_root`, `after_root`, `packages` | Recorded source roots and current selected package names |
| `complete` | The supplied observations were comparable; not full architecture coverage |
| `review_required` | At least one review candidate exists, or the comparison is incomplete |
| `coverage.rust` | Compatible, complete Rust observations were compared |
| `coverage.auxiliary` | `not_supplied`, `compared`, or `incomplete` |
| `coverage.model_citations` | A model was supplied for path-based citation annotation |
| `changes` | Added, removed, and modified observations |
| `diagnostics`, `limitations` | Why a comparison is incomplete and what it cannot establish |

Each change includes `category`, `change` (`added`, `removed`, or `modified`), `key`,
`paths`, `cited_claims`, and `requires_review`. File changes also carry the available
`before_sha256` and `after_sha256`. A moved target contains both old and new source
paths; renamed target IDs or files appear as removal/addition pairs.

When a changed path appears in a component's `code_mapping`, the change also
includes `mapped_components`, a sorted list of distinct component IDs. The field
is omitted when empty. Look up the IDs in the model to see the reviewed range
and rationale. A match covers the whole file and leaves the review gate unchanged.

For example, adding `src/bin/server.rs` can produce this entry:

```json
{
  "category": "cargo_target",
  "change": "added",
  "key": "example:bin:server",
  "paths": ["src/bin/server.rs"],
  "cited_claims": [],
  "requires_review": true
}
```

This is an executable candidate to inspect, not an automatically classified Container.
The new file also appears as a `rust_source` change. Citation references use
`element:<id>` and `relationship:<id>` and do not depend on the document's language.

| Category | Compared data | Review gate |
| --- | --- | --- |
| `cargo_target` | Package/kind/name identity and source path | Requires review |
| `package_metadata` | Edition, recorded dependencies and flags, feature definitions, manifest path | Requires review |
| `rust_source` | Whole-file hashes of reachable production module files; shared files counted once | Requires review, including cited files |
| `unlinked_rust` | Files outside production traversal; hashes when scans are supplied | Informational |
| `auxiliary_file` | Optional inventory paths and hashes | Requires review except files classified as documentation |

Unlinked Rust files may be tests, examples, build inputs, or conditional/generated code;
they are not necessarily unused or intentionally excluded. Inspect their entries when
relevant. Inline test edits inside a production file still change its whole-file hash.
Dependency declaration order and feature-list order are normalized, but dependency
versions absent from `analyze` output cannot be compared. A paired scan can surface
the manifest's content change without interpreting it.

## Exit behavior and invalid inputs

Add `--check` when the caller wants a review gate:

```sh
cargo c4doc drift target/c4-before.json target/c4-after.json \
  --model docs/c4doc-model.json --check
```

| Result | Exit status |
| --- | --- |
| Complete comparison, without `--check` | 0, including when changes are reported |
| Complete comparison with `--check`, no review candidates | 0 |
| `--check` finds review candidates | 1, with the JSON report still emitted |
| Incomplete observations, different source roots or package selections, incomplete scans, or unequal scan limits | 1, with diagnostics in the JSON report |
| Malformed JSON, unsupported versions, conflicting hashes, scan/analyze mismatch, unsafe paths, or I/O failure | 1, with an error on stderr |
| Invalid arguments, including only one scan option | 2 |

Input reads are bounded to 32 MiB per JSON file. Comparisons require the same recorded
checkout path and package-name/manifest selection. A package addition/removal is surfaced
as a scope mismatch; review it and recollect matching selections. The command currently
does not remap roots across separate worktrees.

## Limits that affect interpretation

`drift` finds observation changes; it does not identify the business meaning of a new
feature, resolve HTTP routes, or prove the model covers a changed function. `--model`
matches claim and mapping paths and deliberately accepts stale citation hashes for review
annotation. Run `cargo c4doc check` to validate the model and current sources.

Scan v1 does not record its root or ignore configuration. The command verifies scan
limits and matches reachable Rust paths/hashes, but the caller must keep scan roots and
ignore rules consistent. Browser/configuration/deployment changes are file comparisons,
not semantic analysis. Ignored, hidden, unsupported, and pruned files remain outside
coverage. Generated JSON and configuration may be review candidates if included in a scan;
store comparison artifacts outside the inventory and choose intentional exclusions explicitly.

## Verification cases

Integration tests add a binary, modify a cited file, add browser/deployment files and
unlinked tests, remove/move targets, reorder features, and share a file across bin/lib.
They also exercise malformed/incomplete inputs, incompatible scans, deterministic
output, exit behavior, and existing-file protection.

In the development checkout, comparison against observations captured before this
feature found the new `src/drift.rs`, four modified production files, and the informational
`tests/drift.rs`. The review gate returned 1 even with the previous model supplied.
A fresh cargo-coupling analysis compared with its earlier stored analysis returned no
Rust changes and exit 0; auxiliary scans were not supplied for that check. These are
workflow checks, not a claim that every architecture change can be detected.
