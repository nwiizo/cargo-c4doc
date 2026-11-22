# Help readers discover how the system works

Use this guidance when creating or rewriting the reader-facing guide. Keep the research
record and comparison methodology in supporting documents outside the generated directory.

## Give the reader a reason to open the next page

Start the model description with a recognizable task and the behavior the guide will explain.
Find a concrete detail in the implementation that rewards investigation: two interfaces sharing
one assessment, a request crossing a runtime boundary, or a freshness check before comparison.
Explain its consequence for someone using or changing the system. Do not invent motivations,
performance benefits, incidents, or guarantees to make the story interesting.

The index should work as a reading path, not a catalog of C4 terminology. Arrange `views` in
the order that helps this audience, usually purpose → execution boundaries → responsibilities
→ one interaction. A small project may only need the first two. Optional examples of other
view types must explain why they are included.

## Write each view in layers

- `title`: a question the reader has, or a concrete action. For example, "What do the CLI and
  Web UI share?" The renderer already supplies the C4 kind and scope; avoid repeating them.
- `description`: a short answer that also makes sense in the index. Prefer one or two sentences.
  Keep a condition here when omitting it would misrepresent the answer, such as a success-only
  sequence or an inferred deployment.
- `notes`: optional paragraphs after the diagram. Point to named boxes or arrows and explain
  what to notice and why it matters. Give a relevant source entry point or next investigation
  when supported. Retain the audience, scope, conditions, and deliberate omissions here.
- Elements, relationships, and steps: concise responsibilities and action labels that fit in
  the diagram. The generated reference tables carry technologies and source citations.

For example, a view of cargo-coupling's existing assessment implementation can use this
editorial fragment alongside its unchanged IDs, scope, elements, and relationships:

```json
{
  "title": "What do the CLI and Web UI share?",
  "description": "Both entry points call the same design assessment. Start there when changing how assessment results are assembled.",
  "notes": [
    "Find Design assessment, then follow the incoming calls from the CLI and the HTTP adapter. Both reach src/design/assessment.rs.",
    "This view is for developers changing assessment behavior. It selects those calls and omits ordinary reporting and history listing."
  ]
}
```

Adapt the pattern to the inspected project; this is not a canned claim for every Rust CLI.
All prose fields are plain text. The renderer escapes Markdown and HTML; use separate `notes`
entries for paragraphs. Put richer tutorials, runnable examples, or comparison tables in a
handwritten companion document outside the managed output directory.

## Review the reading experience

Read the index and one complete path as a newcomer. Check that the opening explains a useful
behavior, the page titles help choose where to go, and each diagram has an explained point of
interest. The reader should leave knowing either how a task works or where a related change
starts in the source. Avoid adding a fixed set of filler sections to every page.

Keep prose aligned with the selected diagram and its citations. A syntactic call is not an
executed trace; a shared function does not prove identical output under different settings.
Do not bury a condition that changes the behavior or promote a hypothesis into a fact for a
cleaner narrative. Check images for crowded labels, visible scope, and readable legends.

For Japanese, preserve the same task, insight, conditions, and evidence. Use natural questions
such as "解析はどこで動く？" and direct explanations rather than translating English syntax
word for word. Follow [language guidance](languages.md) for output paths and stable fields.
