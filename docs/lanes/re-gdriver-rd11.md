| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdriver-rd11: DX7 RD6–RD11 closure | Codex / `re-gdriver-rd11` | in progress; RD8–RD11 answered; doccheck PASS | Resume RD6/RD7 from Handoff; finish report and queue only after closure. |

## Report

Answered RD8–RD11 with instruction-checked behavior and thirteen independent prose vectors (DX7-19–31).
The answers distinguish stage construction from dirty-only reset, static borrowed adapter text, mode-vector
cleanup from sticky errors, final-submission HRESULT from last failure, missing white diffuse colour from
untouched UV bytes, and generated versus submitted incomplete-quad indices. Context setup accepts the first
depth descriptor and has no alternate-device retry. Driver destruction AddRefs its texture manager [quirk].
Projection finite conformance requires extended intermediates; ambient FPU traps/status/NaN payload behavior
is not a portable-core obligation. RD6/RD7 remain open; this is not a ready lane.

Validation: `python tools/questions.py index` regenerated the index; `python tools/doccheck.py check` PASS.
No cargo/clippy/CTest or game runs apply to this documentation-only milestone. ABI unchanged; no rows proposed.
Only local export/decomp/PE evidence used; no REA calls and no engine/plugin/capi reads or edits.
Git printed a packed-refs lock permission error during commits, but each detached commit completed; no refs
or configuration were changed. No merge-queue file is written while continuation is required.

## Handoff

- Goal: answer RD6–RD11 in renderer-driver.md; spec text only, no engine reads/edits.
- Done: read rules, lane format, open questions and core-gdriver5 report.
- Owner closure table is in engine/, which this task forbids reading; use the report's explicit RD6–RD11 exclusions and the questions instead.
- Done: RD10/RD11 rules added: white missing colour; untouched missing UV; incomplete quad overrun; final draw HRESULT; ambient x87 CW and finite discriminator.
- Done: `0d5ac1e1d` establishes RD10/RD11 rules. RD8/RD9 prose added: first depth descriptor, selector quirks, setup errors; zero material/clear defaults, stage constructor versus dirty-only reset, list helper and destructor AddRef quirk.
- Done: `16da6b791` RD8/RD9 milestone; corrected static GetDriverInfo pointer and destructor details after instruction checks. Archived RD8–RD11 and added DX7-19–31.
- Done: `ffd29d331` archived RD8–RD11 and added vectors; index regeneration and doccheck PASS. Final checkpoint also commits generated QUESTIONS.md and reconciles stale RD8 boundaries.
- Next: finish RD6/RD7, then final report/queue. RD6/RD7 still open; no ready claim.
- RD6 selector pseudo is main decomp/nSGLDX7.c lines 173–604; 0088F8D0, source bits helpers 0088BCD0/0088BD20. Converter families main decomp/_unnamed/region_00880000.c ~6400–8770 (0088Dxxx–0088F770); arithmetic still needs asm checks and full matrix.
- RD7 pseudo for lighting/material: same unnamed file lines 1808–1975, 00883F30–00884200; light range constant 00AC4658 double words E0000000 47EFFFFF (=FLT_MAX), then FSQRT and single store 00883F2A. Remaining buffer/region operations 00885330–00885790, 00886750–00886Dxx.
- RD7 raw auxiliary tables: owner+0=00AC47EC (13 words read; first 11 public plus destruction entries), +4=00AC47DC (QI/ref/ref/CopyColorBuffer), +8=00AC47AC (12 slots: QI/ref/ref, 883F30/883FA0/884000/886CA0/886B90/noop/noop/884200/8840D0).
- RE helper build-runs/inspect-rd.py reads PE constants; all edits via apply_patch. Guard counts context, not just this response's visible tokens; no queue until six answers complete.
- Worktree reserved with detached HEAD; commit normally without changing refs.
- Main RE source: C:/Users/tlori/FreeCity4; manifest: build-runs/run4-re-gdriver-rd11-r1-manifest.json there.
- No game runs, ABI rows or implementation changes planned.
