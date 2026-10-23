| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-close5: camera control owner closure | Codex; detached core-camera-close5 worktree | wip: full gate passed; owner closure blocked by CA7–CA10 | Obtain missing contracts, finish §7 bodies and pre-init derived state, then review owner closure. |

## Result

The cSC4CameraControl owner is **not closed**. The camera specification gives no
behavioral contract for several bodies named in its §7 inventory. Those gaps
are filed explicitly; no wrapper arguments, kind/message mappings or accepted
COM IIDs were guessed. Queue state is `wip`, with no whole-owner coverage claim.

Implementation/audit commit: `0b0264c0`. Gate-progress handoff: `bbb19a24`.
Changes extend the existing `engine/core/src/camera` module:

- Add foreign `CameraViewUtilities` for explicit city-corner inputs and camera-position notification. Tweak recomputes the camera before notification, holds no camera lock during the callback, and rebuilds empty bounds only under the optional-view guard in §2.6.
- Add the literal zero-initialized shared AddRef ledger; control-specific QI/Release/deleting flags remain open.
- Add §6.1 discriminators that prove N completed edge projections before exhaustion, first-edge tie selection followed by the history stop, and the below-threshold target-clamp no-op. Rename the old stationary U-polygon test: it never demonstrated exhaustion.
- Add the module README's **Owner closure** table, one row per applicable spec section, plus a complete §6 case-to-test map. Existing implementation remains reused; no second camera module, ABI changes, plugin wiring or coverage-map edits.

## Validation

One full `qbuild.py --wt <WT> all` operation on final engine source, collected to
its terminal result: **qbuild PASS**. Rust: **5891 passed, 0 failed, 0 ignored**
summed over all 27 result lines. Clippy: **exit 0, diagnostics 0** with warnings
denied. CMake: **217/217 build edges PASS**. Full CTest: **68/68 PASS**, zero
failures. The native verifier took 1193.6 seconds overall; its build and CTest
reported 133.1 and 156.2 seconds respectively.

The verifier reports `[the tree changed during the run]`: only the lane handoff
was committed while awaiting its terminal result. Engine and question source
are unchanged between `0b0264c0` and `bbb19a24`; no duplicate full gate was run.

Focused camera tests: **52/52 PASS** on final source. Explicit-file fmt: exit 0
for control.rs, mod.rs, tests.rs and closure_tests.rs. Earlier fmt path/format
failures were corrected before the final check. A read-only process diagnostic
was sandbox-denied; it changed no inputs.

Evidence (all under this worktree's ignored `build-runs/`):

- `qbuild-test.log`, `qbuild-clippy.log`.
- `fc4verify-configure.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- `q/20261023-134500-37132.log`: final 52-test camera run.
- `q/20261023-133719-39712.log`: earlier 50-test fixture-discovery run; the temporary search was replaced by the fixed three-edge exhaustion case before final validation.
- `camera-gate-stats.py`: ANSI-aware Rust result summarizer.

Game runs: **none**, no request outstanding. The task explicitly excludes
plugin/C ABI wiring, so no real-game shadow or replacement fidelity is claimed.
Foreign callbacks and arithmetic are measured by core discriminators only.

## Questions and exact closure limits

New questions appended to `docs/specs/questions/camera.md` (next free ID CA8
confirmed from the index; the coordinator regenerates that index):

- **CA8, §7:** rotate-left/right and zoom-in/out arguments, returns, direction/wrapping, zoom increments, magnification and boundary behavior. These four commands remain Unsupported.
- **CA9, §7 / §2.6–2.7:** generic update kind validity, pre/post-message selection, optional-view bounds guard and setter-to-kind/clamp dispatch. Generic kind dispatch remains Unsupported; documented setter sequences are preserved.
- **CA10, §7 / §2.1:** control QI IIDs/output, AddRef/Release return/count edge rules and deleting-destructor flags. QI/Release/flag dispatch remain Unsupported; Rust allocation lifetime retains the specified no-camera-detach behavior.

Existing **CA7**, §§2.1/2.4: pre-init derived plane equation, target-plane point
and conversion scale remain provisional, explicitly commented in code. **CA1**
concerns geometric interpretation only; the §2.5 polygon formula is implemented
literally. **CA3** concerns cS3DCamera, outside this lane's target owner.

Next: spec writers resolve CA7–CA10, then implement those exact residual contracts
and update the owner table before promoting owner coverage. Native integration
and same-process shadow/replacement runs remain separate future work.
