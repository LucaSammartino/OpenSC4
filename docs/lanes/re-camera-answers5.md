| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-camera-answers5: camera owner CA7–CA10 | Codex; `re-camera-answers5` (reserved detached worktree) | ready; CA7–CA10 answered; doccheck passed | Merge docs; engine lane can reconcile control-owner residuals using §§6.2–6.5. |

## Report

Extended [camera.md](../specs/camera.md) §§2.1, 2.3–2.4, 2.6–2.7 and 7 to finish the requested native control-owner contracts. Added independent analytic conformance fixtures in §§6.2–6.5; these are prose vectors for implementers, not executed engine tests or captured game outputs.

- **CA7:** derived equation, plane/pixel points and previous points start at positive zero. The plane-point cache is the **eye's**, not the target's. Conversion uses live attached-camera ViewXform basis/scale at transform-relative +0x34, not control-owned unit scale or pixel/metre factors; no camera-less numeric conversion guarantee.
- **CA8:** all four commands take no arguments and return booleans. Left/right use signed remainder after 32-bit +1/+3. Zoom commands step from virtual zoom with strict double 1.5 threshold; boundary rejection and delegated SetZoom rejection differ, including magnification reset and false-after-mutation cases.
- **CA9:** every generic kind accepted with a camera; kind 1 rebuilds bounds silently, kind 2 sends ZR and clamps afterward, kind 3 skips projection and sends target changes, kind 0/4 sends init/refresh, unknown kinds are silent. View-utilities guard covers bounds generation too. Rotation and zoom both have kind-2 clamp effects. ReturnRenderSystem directly passes 5.
- **CA10:** QI accepts 1/C9C628EC/E4FDA3D4, writes the same object before AddRef, leaves rejected output untouched. AddRef wraps u32. Unsigned Release destroys at counts 0/1 through slot 6 flags=1. Deleting flags bit 0 alone frees the object; vector storage is always released and the camera is never detached by this destructor.

Evidence: queried the Windows export first, then targeted class-decompilation reads; checked numeric/width/ordering claims against instruction bytes. Independently read PE constants: B4E268 loader-zero-fill=0, ABA6B8 double=1.5, B0DC3C float=1, A81054 float=0, plus all seven ABAD18 vtable words. Local read-only evidence helper: `build-runs/camera-pe-evidence.py`. No REA call or fallback needed.

Questions: CA7–CA10 moved with provenance and **ANSWERED 2026-10-23 (re-camera-answers5): DIFFERS** dispositions to [answered/camera.md](../specs/questions/answered/camera.md). Regenerated QUESTIONS.md: camera open 6→2, answered 4→8; CA1/CA3 remain untouched. No new questions filed.

Gate: `python tools/questions.py check` PASS; `python tools/doccheck.py check` PASS; `git diff --check` PASS after correcting one trailing blank line. Cargo/clippy/ctest n/a for this documentation-only RE lane. No game runs, shadow measurement or replacement claim. No engine/plugin/capi/ABI-table/coverage-map edits, no ABI export change, no proposed ABI rows.

Commits: `ec66d698` contracts; `e9cec27b` independent conformance fixtures and archived answers. Git commits succeeded on detached HEAD despite a nonfatal shared packed-refs lock permission diagnostic; shared refs were not changed. The supervisor publishes the exact queued commit to the reserved branch.

## Handoff

- Ready to merge; no remaining work for the assigned CA7–CA10 questions.
- Next: engine lane reconcile owner closure using the published rules and independent fixtures; native integration/shadow/replacement remain separate work.
- CA1 screen interpretation and CA3 unidentified perspective setter remain open outside this task.
- No game requests outstanding; no ABI-row proposal file.
