| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft9: watercraft owner closure | Codex / core-watercraft9 | in progress | Audit existing module against current spec, close settled gaps, run gates. |

## Handoff
- Goal: close cSC4Watercraft owner in Rust, no capi/plugin/ABI changes.
- Existing `engine/core/src/watercraft/` already contains an implementation and closure audit; inspect before extending.
- Done: `ef60c30f6` implements integration/visual basis, compressed routes, resolved Init/codec/docking/undocking, following and collision substeps; new physics/collision vector tests.
- Focused checks: first snapshot 42/42; second snapshot 49/49. Final source adds following/collision vectors after that snapshot.
- Formatting: 14 explicit owned files pass; initial relative-path invocation failed; absolute paths required.
- Full gate on ef60c30f6: workspace 6999 passed, 0 failed/ignored; clippy 0 diagnostics. Logs: build-runs/qbuild-test.log and qbuild-clippy.log.
- Outstanding native build/CTest: qbuild-20261024-084907-build-ctest, launched by `qbuild.py all`; same operation session 68044 must be collected, never duplicated for progress.
- Next: collect native terminal result, then complete pending README rows. Final completion still requires explicit qbuild verify on the completed owner. Do not claim owner closure.
- Facts: spec WC3–WC5 remain open, all other WC questions answered. README now distinguishes specified pending work from genuinely open contracts.
- Pending major work: nonlinear float-channel commands/shared bookkeeping; full route picker; inherited animation/seek; crash scalar/sinking math and detector orchestration; geometry/lifecycle/debug fidelity and remaining named vectors.
- No game requests. No merge-queue entry: this is incomplete work for continuation.
