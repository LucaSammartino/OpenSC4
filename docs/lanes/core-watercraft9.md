| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft9: watercraft owner closure | Codex / core-watercraft9 | in progress | Audit existing module against current spec, close settled gaps, run gates. |

## Handoff
- Goal: close cSC4Watercraft owner in Rust, no capi/plugin/ABI changes.
- Existing `engine/core/src/watercraft/` already contains an implementation and closure audit; inspect before extending.
- Done: core integration/visual basis, compressed routes, resolved Init/codec/docking/undocking, following and collision substeps implemented; new physics/collision vector tests.
- Focused checks: first snapshot 42/42; second snapshot 49/49. Final source adds following/collision vectors after that snapshot.
- Formatting: 14 explicit owned files pass; initial relative-path invocation failed (preserved qbuild evidence); absolute paths required.
- Next: run full gate and queued verify, fix failures, then complete pending rows in module README. Do not claim owner closure.
- Facts: spec WC3–WC5 remain open, all other WC questions answered. README now distinguishes specified pending work from genuinely open contracts.
- Pending major work: nonlinear float-channel commands/shared bookkeeping; full route picker; inherited animation/seek; crash scalar/sinking math and detector orchestration; geometry/lifecycle/debug fidelity and remaining named vectors.
- No game requests or gates outstanding.
