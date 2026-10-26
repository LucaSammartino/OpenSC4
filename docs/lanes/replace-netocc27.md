| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-netocc27: pole geometry/effects groundwork (engine) | Codex / `replace-netocc27`, detached HEAD; commits `f25ae1dfc`, `d617b1e47`; ABI exports unchanged | continue: round time guard; corrected gate running; native poles retained; no replacement credit | 1. Collect existing gate. 2. Resolve NO44 and implement tool/model/draw glue. 3. Measure (a)-(c). |

- Core: cell walk, diagonal floor rounding and wire quads; four new conformance tests passed.
- Plugin/C ABI: powerpolezap creation/retry, placement/movement, running checks, stop modes and owned-reference teardown; new lifecycle test passed.
- (a) House list + netocc to 2032: NOT RUN; live pole factory remains native.
- (b) Placement/extension/demolition/fire script and equal power month lines: NOT RUN; native tool boundary unresolved.
- (c) Netocc shadow 72/72: NOT RUN in this lane.
- First full gate (`f25ae1dfc`): cargo 7853/7853, clippy 0 diagnostics; native build failed 306/308 edges (missing FC4_ACQUIRED argument), CTest skipped.
- First failure: `build-runs/qbuild-first-failure.txt`; original `fc4verify-build.log` may be overwritten by the corrected run.
- Corrected source `d617b1e47`: macro argument fixed; second qbuild all running in exec session 66474 at guard stop. Do not launch a duplicate.
- Current gate logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`; collect the eventual native/CTest result.
- Earlier focused x64 debug compilation was cancelled without a test verdict; separate from both full gates.
- Question NO44: `docs/specs/questions/network-occupants.md` (native helper ABI, container ownership and concrete-access audit).
- Missing: native-tool bridge, cell-vector output, model-instance QI/model host and rendering submissions. Pipe classes not attempted.
- Netocc is NOT ready for the replace list. No game request or ready enqueue submitted.
- Next: collect session 66474, resolve NO44, finish the pole glue, then measure acceptance (a)-(c).
