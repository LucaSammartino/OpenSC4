| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr10: physical file-line observation contract | Codex, `re-scres-scr10` (reserved branch, detached HEAD) | ready: SCR10 answered; doccheck PASS | Land with native6; retain answered SCR10 when merging its unpublished open entry. |

## Report

Section 11 of [sc-resources](../specs/sc-resources.md) closes SCR10 with the
ReadLine, path-constructor, text-open and close signatures; output ownership;
per-invocation path association; nested file identities; physical line numbers
versus inclusive parser totals; newline/EOF/read-failure behavior; and safe
observation requirements. Twelve normative vectors SCR-V81–92 exercise these
rules, including deterministic evidence of the wide-mode count quirk.

Instruction checks settle complete text-file ECX, one writable S reference,
AL bool, ret 4; open uses `(1,2,7)`. CRLF and LFCR collapse, repeated identical
newlines remain distinct, and a later chunk failure returns a successful
prefix. BOM mode 2 scans words but passes a byte count as a word count to
append, potentially exposing uninitialized or beyond-buffer data. The spec
describes that quirk and refuses invented deterministic outputs for such data.

Evidence used: main-checkout `tools/re_query.py` export queries and focused
decompilation reads, with numeric widths/counts/stack cleanup verified against
instructions. REA was not called; the task states no binary is open there.
No binary or decompiled output is included in the committed documentation.

SCR10's source is the unmerged native6 worktree question file, absent from this
base. It was copied into the answered archive, with the rule in section 11 and
an answered notice in the open file. `tools/questions.py index` regenerated
the index. No new questions remain. The native6 report has no Owner closure
table or `open:` rows; its Open scope section names precisely this blocker.

Validation: `python tools/doccheck.py check` PASS. These are instruction-derived
spec vectors, with no new game observations or shadow-equality claim. Cargo,
clippy and CTest are n/a for this documentation-only RE lane. No engine,
plugin, capi, coverage map, ABI tables or exports changed; no ABI rows proposed.
No game request is outstanding. Git emitted a packed-refs lock permission
diagnostic during the first commit, but the detached-HEAD commit succeeded and
the worktree was clean afterward; no shared ref/config changes were attempted.

## Handoff

- Done: `37fa2d26d` specifies section 11, SCR-V81–92 and the answered SCR10 entry.
- Final report commit normalizes the archive entry for index counting and records validation.
- Next: coordinator lands with native6, resolving any union merge that resurrects open SCR10 in favor of the answered entry.
- Implementer can add physical input capture/replay using section 11; macro/custom-command state and unsafe wide-output parity are separate scopes.
- No game runs, ABI proposals, outstanding operations or unresolved questions.
