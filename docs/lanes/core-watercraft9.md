| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft9: watercraft owner closure | Codex / core-watercraft9 | in progress | Audit existing module against current spec, close settled gaps, run gates. |

## Handoff
- ROUND 2: float-channel commands, nonlinear/inverse speed curves, shared 13-channel bookkeeping, skier transforms, heading/rudder and speed tween implemented.
- Focused first snapshot: 57 passed. Later WC6/WC9 vectors still need checking. q.py failed Bash CreateFileMapping; direct cargo works.
- New WC9: literal §6.2.2 steering conflicts with WC8-08. Literal prose implemented with marked vector; no WC8-08 claim.
- NEXT ROUND 2: route picker, inherited seek/animation, crash scalar/sinking and detector; then absolute-path fmt, qbuild all + verify, READY queue full actual HEAD, module engine/core/src/watercraft.
- Detached HEAD intentional: commit normally, never switch/update refs. Final READY queue required even if owner remains open. No game requests.
- Goal: close cSC4Watercraft owner in Rust, no capi/plugin/ABI changes.
- Existing `engine/core/src/watercraft/` already contains an implementation and closure audit; inspect before extending.
- Done: `ef60c30f6` implements integration/visual basis, compressed routes, resolved Init/codec/docking/undocking, following and collision substeps; new physics/collision vector tests.
- Focused checks: first snapshot 42/42; second snapshot 49/49. Final source adds following/collision vectors after that snapshot.
- Formatting: 14 explicit owned files pass; initial relative-path invocation failed; absolute paths required.
- Full gate on ef60c30f6: workspace 6999 passed, 0 failed/ignored; clippy 0 diagnostics. Logs: build-runs/qbuild-test.log and qbuild-clippy.log.
- Native terminal result collected: qbuild-20261024-084907-build-ctest PASS, 253/253 build edges, CTest 79/79; `qbuild all` PASS. Logs: build-runs/fc4verify-build.log and fc4verify-ctest.log. No outstanding operation.
- Next: replace legacy boolean commands in commands.rs with §§6.2.1–6.2.3 float channels/nonlinear curves/bookkeeping, encode WC8/WC6 vectors, then finish other pending README rows. Final completion still requires explicit qbuild verify on the completed owner. Do not claim owner closure.
- Facts: spec WC3–WC5 remain open, all other WC questions answered. README now distinguishes specified pending work from genuinely open contracts.
- Pending major work: nonlinear float-channel commands/shared bookkeeping; full route picker; inherited animation/seek; crash scalar/sinking math and detector orchestration; geometry/lifecycle/debug fidelity and remaining named vectors.
- Formatter helper: build-runs/format-watercraft.py formats explicit owned files through apply_patch (strips rustfmt stdout filename header); qbuild fmt needs absolute file paths.
- In-game shadow: no adapter/wiring this round; future focused adapter can compare runtime/visual transforms, route cells, state/collision decisions. Full owner comparison needs pending work and WC3–WC5 answers.
- No game requests. No merge-queue entry: this is incomplete work for continuation.
