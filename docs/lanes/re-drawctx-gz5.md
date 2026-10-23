| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-drawctx-gz5 (GZ graphics contracts) | Codex / `re-drawctx-gz5` (RE lane, detached HEAD) | ready: GZ graphics spec with explicit gaps; 44 declarative vectors; doccheck and questions check passed; GZG1–GZG5 filed; no ABI changes or game runs | (1) implement settled contracts; (2) resolve GZG1–GZG5 before complete-owner replacement |

## Report

Added `docs/specs/gz-graphics.md` for cGZDrawContext, cGZGraphicSystem,
cGZAlphaBlt, cGZCanvasW32, and buffer/hardware/SimGL render engines. It covers
method inventories, object/interface identities, defaults, retained state,
target changes, stack scope, drawing dispatch, lifecycle, message handling,
host IO and observed failure paths. Links existing font, 3D context and span
kernel specs instead of duplicating their contracts.

Important findings: target changes reset origin/blit/suspension but retain
stacks; ClipReset does not notify the engine; engine result 1 tessellates filled
quads/polygons but produces no further triangle drawing, whereas result 2
rasterizes. Buffer Init can succeed with a null span renderer and can leave a
destination locked on rejected colour type. Unsupported QI can leave outputs
unchanged. Canvas input disabling is nested per channel.

Evidence: export queries, main-checkout decompilation and read-only executable
vtable facts; numeric conversions and selected dispatch rules verified against
instruction bytes. No REA used (no binary open); SimGL Draw/HWSetImage exported
pseudocode failed, so their unsettled device details are explicit questions.
No decompiled code or assembly listings included in the spec or commits.

Added 44 declarative conformance vectors in spec section 8. These are expected
contracts for future tests, not executed game measurements. Five indexed open
questions, GZG1–GZG5, live in `docs/specs/questions/gz-graphics.md`; index rebuilt.
They cover remaining generic geometry, device/AA rendering, graphics IO and
startup/presentation, Windows event translation, and standalone alpha kernels.
This is useful partial coverage, not a complete-owner or pixel-exact claim.

Validation: `python tools/doccheck.py check` passed;
`python tools/questions.py check` passed; `git diff --check` passed.
Cargo/clippy/ctest and game/shadow runs: n/a (spec-only lane).
No engine, plugin, capi, ABI tables or coverage-map edits. No proposed ABI rows.
Queue kind is `re` as explicitly requested by this lane's task.

Commits: `2ef733dc` initial lane record; `dd6ceaa7` initial spec/questions;
final report commit also corrects dispatch/table inventory and indexes all five
questions. Git emitted a packed-refs lock permission warning while committing
detached HEAD; commits themselves succeeded. No refs/config changed by this lane.

## Handoff

- Goal: write `docs/specs/gz-graphics.md` for GZ context, graphics system, alpha blitter, W32 canvas and render targets; no engine edits.
- Worktree is intentionally detached; commit HEAD normally, do not change refs.
- Done: spec, corrected dispatch/table facts, 44 vectors, five indexed questions; doccheck/questions/whitespace passed.
- Next: coordinator may land these docs; follow-up RE must settle GZG1–GZG5 before complete-owner coverage.
- Evidence roots: `C:/Users/tlori/FreeCity4/decomp/` and `ghidra/export/windows`; query with main `tools/re_query.py` first.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-drawctx-gz5-r1-manifest.json`.
- Evidence: context table `0x00AC16C8` (48); buffer engine `0x00AC17D0` (23); SimGL `0x00ADB500`; canvas primary `0x00ADAD88` (31).
- Questions GZG1–GZG5 filed in `docs/specs/questions/gz-graphics.md`; no existing questions answered.
- No game runs or ABI changes. Spec is explicitly partial for geometry/device rendering, graphics IO, Windows event translation and standalone alpha kernels.
