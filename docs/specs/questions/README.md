# Spec questions: how the files work

Questions about a spec used to live in one 680 KB file in filing order. They now live one file per spec, so a lane
reads only the questions for the spec it is working on. This directory is spec-side material and is safe for engine
lanes and RE lanes alike.

## Layout

| Path | What it holds | Who reads it |
|---|---|---|
| `docs/specs/QUESTIONS.md` | Generated index: rules, prefix legend, a table of open/answered counts and the next free ID per prefix. | Everyone, first. |
| `docs/specs/questions/<spec>.md` | Open questions about `docs/specs/<spec>.md`. | The lane working on that spec. |
| `docs/specs/questions/answered/<spec>.md` | Archive of answered entries, kept struck through with their `ANSWERED` text. | Nobody by default; open it only to look up an old answer by ID. |
| `docs/specs/questions/unsorted.md` | Entries that could not be assigned to a spec. Normally absent. | Whoever files it away. |

`<spec>` is the file stem of the spec in `docs/specs/` (`zones` for `zones.md`).

## Asking a question

1. Open `docs/specs/QUESTIONS.md` and find your spec's row. Its "Next free ID" column gives the next number for each
   prefix. If the spec has no row yet, pick a new two-letter prefix that is not in the legend.
2. Append the entry at the END of `docs/specs/questions/<spec>.md`. If the file does not exist, create it with the
   two header lines of any other question file (`# <spec>: open questions` and a one-line comment).
3. Put a provenance line directly above the entry: `<!-- from: <lane or report> (<date>) -->`.
4. Write the entry as a bold ID followed by the question, with the current choice in brackets:

   ```
   <!-- from: the zones core (2026-10-13) -->
   - **ZQ17 - section 8.3 slot 10:** does the walk start at the NW corner? [yes]
   ```

   The ID keeps the spec's subsystem prefix and continues its numbering. Never reuse an ID. A spec may have more
   than one prefix.
5. Run `python tools/questions.py index`, then `python tools/questions.py check`, and commit the question file and
   the regenerated `QUESTIONS.md` together.

Because lanes only append at the end of their own spec's file, two lanes asking about different specs never touch
the same file, and the merge train can union two appends to the same file.

## Answering a question

1. Write the answer into the spec itself (`docs/specs/<spec>.md`).
2. Strike the entry through (`~~...~~`) and add an `**ANSWERED <date>**` line saying where the answer went.
3. Move the whole entry, with its provenance line, from the open file to the END of
   `docs/specs/questions/answered/<spec>.md`. Do not delete it: the ID stays findable and numbering stays stable.
4. Run `python tools/questions.py index` and `python tools/questions.py check`.

## Commands

- `python tools/questions.py index` regenerates `docs/specs/QUESTIONS.md`. It is deterministic and idempotent.
- `python tools/questions.py check` exits 1 if the index is stale, an ID is duplicated within a spec's open file, or an
  entry has no ID.
- `python tools/questions.py find <ID>` prints where an ID lives (open file or archive, and the line).

`QUESTIONS.md` is generated: never edit it by hand. On a merge conflict in it, take either side and run
`python tools/questions.py index`.
