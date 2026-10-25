# Spec questions: how the files work

Use this procedure when assigned question filing or answering. Engine lanes otherwise use the
[prompt's](../../../.claude/workflows/fc4-lane.js) `SPEC-GAP` comments and keep going.

## Layout

| Path | What it holds | Who reads it |
|---|---|---|
| `docs/specs/QUESTIONS.md` | Generated prefix legend, counts and next free IDs. | Assigned question work. |
| `docs/specs/questions/<spec>.md` | Open questions about `docs/specs/<spec>.md`. | The lane working on that spec. |
| `docs/specs/questions/answered/<spec>.md` | Struck-through entries with `ANSWERED` text. | Only when the task names it. |
| `docs/specs/questions/unsorted.md` | Entries that could not be assigned to a spec. Normally absent. | Whoever files it away. |

`<spec>` is the file stem of the spec in `docs/specs/` (`zones` for `zones.md`).

## Asking a question

1. Take the next free ID from the spec's index row, or a new two-letter prefix absent from the legend. Never reuse IDs.
2. Append to `docs/specs/questions/<spec>.md` (stem of `docs/specs/<spec>.md`). A new file starts with
   `# <spec>: open questions` and a one-line comment. Entry format:

   ```
   <!-- from: <lane or report> (<date>) -->
   - **<ID> - section <section>:** <question> [current choice]
   ```

3. Regenerate and check with the commands below; commit the entry and index together. Engine lanes assigned this
   work ask the coordinator to run tools (the clean-room wall still applies).

## Answering a question

1. Write the answer into the spec itself (`docs/specs/<spec>.md`).
2. Strike through (`~~...~~`) and add `**ANSWERED <date> (<lane>):** MATCHES|DIFFERS — <rule>` with the spec section.
3. Move the whole entry, with its provenance line, from the open file to the END of
   `docs/specs/questions/answered/<spec>.md`. Do not delete it: the ID stays findable and numbering stays stable.
4. Regenerate and check the index.

## Commands

- `python tools/questions.py index` regenerates `docs/specs/QUESTIONS.md`. It is deterministic and idempotent.
- `python tools/questions.py check` exits 1 if the index is stale, an ID is duplicated within a spec's open file, or an
  entry has no ID.
- `python tools/questions.py find <ID>` prints where an ID lives (open file or archive, and the line).

`QUESTIONS.md` is generated; on conflict take either side and regenerate it.
