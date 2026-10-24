| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdriver-rd11: DX7 RD6–RD11 specs | Codex / `re-gdriver-rd11` | ready; doccheck PASS; RD8–RD11 answered, RD6/RD7 narrowed | Land spec batch; remaining converter arithmetic and CopyColorBuffer are explicit open questions. |

## Report

This ready specification batch covers RD6–RD11 in `docs/specs/renderer-driver.md`. RD8–RD11 are fully
answered; RD6/RD7 settled portions are archived, with precisely narrowed residual entries retained as the
round-2 task permits. Readiness means the documented rules are reviewable and ready to land, not that the
two remaining proof boundaries have been solved.

Round 1 (`0d5ac1e1d`, `16da6b791`, `ffd29d331`, checkpoint `1cb2bf7a1`) specified white missing diffuse
colour, untouched missing UV bytes, incomplete-quad index overrun/count differences, final-submission
HRESULT, x87 projection intermediates, stage construction versus dirty-only reset, static borrowed driver
text, mode-vector cleanup, destructor texture-manager AddRef, device-selector rules and first-depth-format
acceptance with no alternate-device retry. Thirteen independent vectors are DX7-19–31.

Round 2 (`5e1c273d0`, `2cc7c6db4`) adds eighteen prose conformance vectors DX7-32–49, for 31 in this lane:

- §9.6 specifies complete ordinary descriptor ranking and stable ties. The bit-count target includes alpha;
  excess precision outranks shortage and alpha ranks can supersede RGB preferences.
- All 120 source/request converter-availability cells are determined by the chosen descriptor key, rather
  than by request number. A fixture tests every cell. DXT5 has no ordinary software decoder in this table;
  an allocatable fallback surface need not support a DXT5 upload.
- Direct compressed copying uses the locked linear byte size, without block rounding. Source-0 identity
  copying and source-5/source-9 expansion to keys 1/2 have exact instruction-checked pixel examples.
- §§12.1, 12.4 attribute the three auxiliary tables, specify low-byte enables, immediate light/material
  submissions, property/null-pointer behavior, ambient bit conversion and exact sqrt(FLT_MAX) range
  (`5F7FFFFF` under nearest/even). These tables return no independent vertex-buffer object.
- §12.5 specifies region handles, first-free reuse, ownership, creation retry, save/restore rectangles,
  inclusive-bound predicate quirk, missing-surface gates and asymmetric loss recovery. Partial save can
  return false even after its retry successfully copies, because prior contents were discarded.

Questions: original RD6/RD7 entries and their settled answers are archived in
`docs/specs/questions/answered/renderer-driver.md`; residual RD6 is only the other non-copy converter
arithmetic and DXT1/DXT3 software-decode details. Residual RD7 is only CopyColorBuffer (`00883950`)
lock-probe cache, destination/refcount ownership, clipping/orientation, conversion and failure behavior.
Their decompiler stack/pointer aliases are insufficient evidence; no unverified arithmetic was promoted to
a rule. RD1/RD3/RD4 were not in this round and retain their existing narrow entries.

Validation: `python tools/questions.py index` regenerated `docs/specs/QUESTIONS.md`;
`python tools/doccheck.py check` PASS; `git diff 1cb2bf7a1 HEAD --check` PASS. A first staging command named
the wrong generated-index path and staged nothing; the corrected explicit-path staging and commit succeeded.
Cargo/clippy/CTest and game/shadow runs are n/a for this prose-only batch. No ABI exports or proposed rows.
No engine reads/edits. Only the main checkout's local export/decomp/PE was queried; no REA calls or errors.
Private evidence helpers remain ignored under this worktree's `build-runs/`.

The reserved worktree remains detached as instructed. Git printed a packed-refs lock permission diagnostic
while each detached commit completed; shared refs/configuration were not modified. The final ignored queue
entry is `build-runs/merge-queue/re-gdriver-rd11.json`, kind `re` as requested, using actual full HEAD.
Next: land this batch; assign the exact residual RD6/RD7 entries for further instruction checks separately.
