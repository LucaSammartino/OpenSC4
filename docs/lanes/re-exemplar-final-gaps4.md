| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-exemplar-final-gaps4: EX52/EX53 native contracts | `re-exemplar-final-gaps4` (RE specs) | blocked on Git metadata permissions; docs uncommitted at base `801cbe48`; questions check and diff check PASS; no ABI/game changes | 1. Restore worktree Git write access and commit the five listed docs. 2. Investigate remaining EX52 e name-source data. |

## Report

Spec work is complete for EX52 a/b/c/d/f, EX52 e syntax/lookup, and EX53. EX52 e source/table contents
remain explicitly open. All evidence is static: local export instructions with bytes, bounded decompile
reads and original executable data words. No REA tools were callable in this transport; no REA request,
runtime run, workflow, subagent, engine read/edit, tool edit, ABI edit, merge or push was performed.

### Measured contracts

- EX52 a/b MATCHES: B allocate/free `0x0090CF54`/`0x0090CF63`, including vector growth; word 3 is an
  end-of-storage pointer, elements are 8 bytes. Spec gives a two-entry destination/free vector.
- EX52 c MATCHES: slot 28 thiscall `(bytes, byteCount)`; 8-byte binary / 10-byte text signature; tail
  dispatch to record slot 51 and writable packed-file target verified.
- EX52 d DIFFERS from filed foreign-property limitation: property QI `0xE52184AB`, slot 6 `(keyOut)`;
  returned key pointer to record slot 49, queried reference released. Variant-held resource alone does
  not select kind 1. Spec gives spy-interface discrimination.
- EX52 e syntax DIFFERS from filed unsupported text path: header, parent/count/property lines, dynamic
  INI field-5 readable names, supported type formatting and malformed unsupported-type lines specified.
  Fresh scalar count is **0**, signed small integers widen before hex formatting, 64-bit array `0x`
  occurs only on the final element. Entry raw-write errors are ignored; signature error is tested.
- EX52 f DIFFERS in EAX: both wrappers leave the new property's Release result; two list references
  explain native 2, one base-holder entry explains 1. No literal 2 requirement or public return-type
  declaration is inferred. Source value is copied, not retained; null slot-14 variant has no result claim.
- EX53 DIFFERS: Char/RZChar → one raw byte; scalar Unicode ≤255 → that byte, otherwise `?`.
  Output is assigned with explicit length even when byte 0; getter returns false. Spec includes all
  requested `41`/`E9`/`00E9`/`FFE9` cases plus NUL and the `0100` boundary.

### Questions and validation

Proved entries are in `docs/specs/questions/answered/data-formats.md`; only EX52 e source/content remains
in `docs/specs/questions/data-formats.md`. `python tools/questions.py index` regenerated
`docs/specs/QUESTIONS.md`; `python tools/questions.py check` and `git diff --check` passed.
Cargo/clippy/ctest and game validation: n/a, docs only. No ABI rows proposed. No new native-run verdicts;
the prior `gef4-r3-union2032` native-2/replacement-1 observation is explained statically, not rerun.

### Delivery blocker and exact next step

Both milestone and final staging/commit attempts failed with `Unable to create
C:/Users/tlori/FreeCity4/.git/worktrees/re-exemplar-final-gaps4/index.lock: Permission denied`.
The tip remains **801cbe48**, with no lane commits; the proof was written before the first milestone
commit attempt. No merge-queue entry was written because that tip does not contain this work.
After restoring Git metadata write access, stage this lane file plus `docs/specs/data-formats.md`,
`docs/specs/questions/data-formats.md`, `docs/specs/questions/answered/data-formats.md` and
`docs/specs/QUESTIONS.md`; commit using `build-runs/msg-re-exemplar-final-gaps4-2.txt`, then update this
report with the commit and enqueue a docs-only, ABI-false entry per `docs/merge-train.md`.

To continue EX52 e, find a population path for global INI string `0x00B47A80` (empty constructor
`0x00A74680`, consumer `0x0067FE80`); a fixed table was not found, so names were not guessed.
Evidence helpers/logs stay untracked under `build-runs/`: `native-read.py` and
`EX52-write-entry-asm.txt`; none should be committed. Bash q.py wrapper failed with
`Bash/Service/CreateInstance/E_ACCESSDENIED`; bounded PowerShell worked.
Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-exemplar-final-gaps4-r1-manifest.json`;
latest measured context before the final report: 90944 / 150000 (within budget).


### Coordinator delivery recovery (2026-10-22)
Worker stopped after sandbox-denied Git metadata writes; guard result is preserved. Coordinator reviewed the bounded docs diff and committed these exact five files outside the worker sandbox. Static proofs and remaining questions above are unchanged. This is docs-only delivery, not a new lane round or runtime validation. Queue records the committed tip.
