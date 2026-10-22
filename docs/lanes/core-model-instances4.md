| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances4: model instance Rust owners | `core-model-instances4` / (engine lane, detached worktree) | in progress | 1. Baseline gate. 2. Read contracts. 3. Implement and verify complete owners. |

## Handoff
- Goal: complete pure Rust model instance, processor, transform and sprite resource owners from published specs; core only.
- Done: read model-instances §§1–6,10 and narrow dependency sections; implemented requested owner families plus table/factory and independent test vectors.
- Baseline all: cargo 4238 passed, clippy zero diagnostics, CMake 206/206; ctest 66/67 because source edits overlapped its cargo_test compile (missing modules). Final gate must run with frozen sources.
- Next: finish first quick gate (session 12999), inspect failures, format touched Rust, audit whole-owner contracts, final all with frozen sources.
- Questions: MI3–MI8 in docs/specs/questions/model-instances.md; literal choices annotated in code. MI1 opaque headers retained, MI2 literal rotation negation.
- Constraints: clean room; no CAPI/plugin/ABI or coverage map changes, no game requests expected for core-only work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-model-instances4-r1-manifest.json.
