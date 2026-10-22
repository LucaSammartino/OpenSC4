| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances4: model instance Rust owners | `core-model-instances4` / (engine lane, detached worktree) | in progress | 1. Baseline gate. 2. Read contracts. 3. Implement and verify complete owners. |

## Handoff
- Goal: complete pure Rust model instance, processor, transform and sprite resource owners from published specs; core only.
- Done: 912f9519 implements published owners and initial independent vectors. Follow-up adds explicit factory creation, atomic Release results, 31 tests, normal division and repeated-sort/upward-pass literal corrections.
- Baseline all: cargo 4238 passed, clippy zero diagnostics, CMake 206/206; ctest 66/67 because source edits overlapped its cargo_test compile (missing modules). Final gate must run with frozen sources.
- Measured: two quick gates PASS (2393 core tests, dependents compiled, CMake 4/4). Latest full gate: cargo 4269/0, core 2402/0 incl. 31 owner tests, CMake 4/4, ctest 67/67; clippy failed one assign-op lint, now locally annotated to preserve published operand order.
- Format: seven new Rust files PASS qbuild fmt; shared lib.rs has pre-existing ordering/reformat differences (our change only appends one module).
- Next: run final qbuild all on frozen sources; update full report and commit, then queue ready if gate passes.
- Questions: MI3–MI10 in docs/specs/questions/model-instances.md; literal choices annotated in code. MI1 opaque headers retained, MI2 literal rotation negation including signed-zero bits.
- Constraints: clean room; no CAPI/plugin/ABI or coverage map changes, no game requests expected for core-only work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-model-instances4-r1-manifest.json.
