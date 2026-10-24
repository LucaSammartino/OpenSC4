| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pedestrians-pn4 (PN1 one-shot initialization and play count) | `Codex` / `re-pedestrians-pn4` (RE lane, detached HEAD) | ready; prerequisite merge f60a2d43a, spec milestone de727ae3a; PN1 answered with 22 vectors; doccheck passed, questions check passed, diff whitespace check passed; ABI unchanged; game runs n/a | (1) Implement the one-shot play-count consumer from pedestrians §3 and §8; (2) close its remaining owner row. PN2 and PN3 remain open. |

## Report

Completed PN1 in `docs/specs/pedestrians.md` §2.1, §3 and §8. Merged `core-pedestrians12` first
at f60a2d43a, retaining its newer implementation-owner closure table. This lane's own changes are
specifications, the question archive/index and this report. No engine source or owner README was
read or manually edited, and no plugin, capi, ABI table or coverage-map change was made.

### Behavior settled

- Inherited automaton slot 158 is `0x00544620`: promote the two binary32 tuning bounds to double
  arguments, consume a range draw and store binary32 at `+0x200` before base Init when not initialized.
  Shipped equal bounds store −0.5 and still draw. Failed Init does not roll this operation back.
- One-shot successful Init sets each non-null model's speed to bits `0x3C23D70A`, then replaces the
  unsigned byte count at `+0x270` using tuning or exemplar bounds. Repeat successful Init redraws it;
  zero, 255, reversed bounds, and failed base Init are specified without an unresolved choice.
- The consumer is the one-shot's animation tick `0x00544E80`, primary slot 72 (`0x00A9AC70`), rather
  than a callback or accessor. It forwards the second argument (milliseconds) to opacity slot 83 and
  each model's Advance. Only the first non-null model's sampled final frame charges a play.
- Count saturates at zero; a qualifying zero requests `Remove(false)` and returns false while later
  models still advance. Held final frames charge on each call, even at zero delta. Empty/all-null lists
  do not consume or remove. This body has no pause/stationary gate or ordinary pedestrian state tick.
- Count read/write slots 177/178 are plain byte accessors. Corrected the spec's earlier statement that
  the one-shot only overrides Init. PN1 has no remaining behavior question or game-run dependency.

### Evidence and validation

Queried `tools/re_query.py` first, then the main checkout's decomp and export. The one-shot Init
decomp failed its structure reconstruction; the instruction bytes establish its complete behavior.
Tick pseudocode is at `decomp/_unnamed/region_00540000.c` lines 752–798; instructions establish its
argument positions and Boolean result, which that pseudocode does not reliably recover. No REA
query was attempted because this session has no binary open. Numeric widths/order were checked
against instructions, including `0x00544620`, `0x00544DA0`, `0x00544E80`, `0x00913A06` and `0x00913A46`.

Added 10 initialization and 12 tick/accessor conformance vectors (PN1-I1–I10, PN1-T1–T12).
Moved PN1 to `docs/specs/questions/answered/pedestrians.md`, marked **ANSWERED 2026-10-24
(re-pedestrians-pn4): DIFFERS**, and regenerated the questions index. PN2/PN3 remain open.
No new question or proposed ABI row was needed.

Foreground validation logs under this worktree's `build-runs/`:

- `pn1-doccheck.log`: `doccheck check: ok` (exit 0).
- `pn1-questions-check.log`: `questions: ok` (exit 0).
- `pn1-diff-check.log`: no whitespace errors (exit 0).
- Cargo, clippy, ctest and game/shadow runs: n/a for this RE specification change.

Git merge/commit returned exit 0 and advanced detached HEAD despite a `packed-refs.lock` permission
diagnostic. The supervisor publishes the actual committed HEAD to the reserved lane branch.

## Handoff

- Goal complete: PN1 fully specified and archived; 22 conformance vectors; no unresolved PN1 subcase.
- Commits: prerequisite merge f60a2d43a; initial record ec738e0d7; spec and question/index milestone de727ae3a.
- Next: land the queued actual HEAD, implement pedestrians §3 using §8 vectors, then update the implementation-owner closure row.
- Existing PN2 and PN3 are outside this task and remain open; do not reopen PN1.
- No game runs outstanding; ABI unchanged; no ABI proposal file.
- Merge queue: build-runs/merge-queue/re-pedestrians-pn4.json (kind re, ready, full actual HEAD).
- Validation logs and read-only evidence paths are recorded above; main checkout C:/Users/tlori/FreeCity4.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pedestrians-pn4-r1-manifest.json.
