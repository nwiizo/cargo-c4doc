# English and Japanese documentation

Default to English for models, generated descriptions, comparisons, and handwritten guides.
Conversation and source-document language do not automatically select the output language.
Honor an explicit Japanese request or an established document requirement without asking again.
If explicitly asked for both languages, preserve both editions.

## Output locations

Unless the user supplies a different destination, use these locations relative to the target project:

| Language | Example model | Generated output |
| --- | --- | --- |
| English | `c4.json` | `docs/c4doc` |
| Japanese | `c4.ja.json` | `docs/c4doc-ja` |

Use `docs/c4doc-ja` for requested Japanese documentation unless another destination is specified,
even when only Japanese is requested. Keep any existing English edition. Keep the Japanese model
outside the generated directory; when an English model already exists, preserve its location
and add `.ja` before `.json` for its counterpart.
For example, `docs/c4doc-model.json` becomes `docs/c4doc-model.ja.json`.

After authoring the Japanese model, render and verify it with the same output path:

```sh
cargo c4doc render c4.ja.json --root . --output docs/c4doc-ja
cargo c4doc render c4.ja.json --root . --output docs/c4doc-ja --check
```

## What to translate

Translate titles, element display names, descriptions, inferred reasons, view descriptions and notes,
dynamic-step descriptions, questions, and handwritten explanations. Preserve product names,
C4 type names, IDs, enum values, schema versions, paths, commands, API methods/routes,
technology names, line ranges, hashes, and factual claims.

Keep evidence classifications unchanged: translating `inferred` prose must not promote it to
`observed`. Do not translate source code or edit source files to match the prose. Prefer a
paraphrase with a citation when explaining material written in another language.

For two requested languages, use the separate models and output directories above.
Reuse IDs and evidence; change only presentation text.
Keep mapping source ranges, branch structure, relationship references, and scope
aligned. Translate coverage, contribution rationales and branch labels. Keep perspective
keys and values consistent with their filters in both editions.
Review both models after source changes. Do not create a second edition unless requested or
overwrite a preserved edition to generate a translation.

## Japanese terminology

Use natural Japanese explanations. Introduce English C4 terms on first use when helpful,
then keep terminology consistent:

| C4 term | Japanese explanation |
| --- | --- |
| System Context | システムと利用者・外部システムの関係 |
| Container | アプリケーションまたはデータストア。Docker コンテナとは区別する |
| Component | 一つの Container 内で責務を担うコードのまとまり |
| Code | 型・関数などの実装要素 |
| Dynamic | 条件を絞った処理のやり取り |
| Deployment | 特定の環境での配置 |
| Landscape | 複数のシステムと利用者の全体像 |

Use relationship labels with an action and object, for example "変更比較を要求する".
Preserve the reading path and each view's insight when translating. Prefer a natural reader
question and a direct answer; move supporting detail into separate notes in both editions.
Explain `observed` as support from inspected evidence, not necessarily an executed runtime
observation. Use readable display names rather than literal translations of Rust module names.

## Render and verify

Edit models and regenerate; never translate managed `.md` or `.mmd` files directly. Rendering
validates the model; use `check --json` when separate diagnostics are useful. Confirm generated
content with `render --check` for each requested edition. Render changed diagrams to inspect Japanese
fonts, wrapping, punctuation, and legibility. Confirm that boundaries, directions, conditions,
omissions, and confidence remain unchanged.

Built-in renderer headings, C4 type labels, legends, and diagnostics are currently English.
Japanese model text is supported, but there is no `--language` flag. Do not invent one or claim
complete UI localization. Retain standard labels and localize explanatory model content;
disclose this limit when fully localized output is required.
