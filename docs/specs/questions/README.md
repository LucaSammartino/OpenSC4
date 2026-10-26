# Spec questions: how the files work

Use this procedure when assigned question filing or answering. Engine lanes otherwise use the
[prompt's](../../../.claude/workflows/fc4-lane.js) `SPEC-GAP` comments and keep going.

## Layout

- `docs/specs/QUESTIONS.md`: generated prefix legend, counts and next free IDs.
- `docs/specs/questions/<spec>.md`: open questions; `<spec>` is the spec's filename stem.
- `docs/specs/questions/answered/<spec>.md`: archive; read only when the task names it.
- `docs/specs/questions/unsorted.md`: unassigned entries, normally absent.

## Asking a question

1. Take the next free ID from the spec's index row, or a new two-letter prefix absent from the legend. Never reuse IDs.
2. Append to `docs/specs/questions/<spec>.md` (stem of `docs/specs/<spec>.md`). A new file starts with
   `# <spec>: open questions` and a one-line comment. Entry format:

   ```
   <!-- from: <lane or report> (<date>) -->
   - **<ID> - section <section>:** <question> [current choice]
   ```

3. Regenerate/check; commit entry and index together. Engine lanes ask the coordinator to run tools.

## Answering a question

Write the rule into `docs/specs/<spec>.md`, strike the entry through (`~~...~~`), and add
`**ANSWERED <date> (<lane>):** MATCHES|DIFFERS — <rule>` with the section. Move entry and provenance to the
archive's end, preserving the ID. Regenerate/check.

## Commands

- `python tools/questions.py index`: regenerate index.
- `python tools/questions.py check`: exit 1 on stale index, duplicate open IDs or missing IDs.
- `python tools/questions.py find <ID>`: locate ID and line.

`QUESTIONS.md` is generated; on conflict take either side and regenerate it.
