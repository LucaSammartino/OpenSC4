| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances-contractfix4: MI3-MI10 corrections | Codex / `core-model-instances-contractfix4` (engine lane) | in progress | 1. Reconcile published contracts and independent vectors. 2. Gate and report. |

## Handoff
- Goal: correct the portable model-instance subset against revised MI3-MI10, preserving exclusions and MI7.
- Base: detached HEAD 1c4c54d2; reserved worktree reused, initially clean.
- Done: 965e8949 initial MI3-MI10 corrections; added 11 independent-vector/boundary tests (42 module tests total), owner callback ordering, exact count policies, native/reference-only binding separate from frame convenience; no native/analysis sources accessed.
- Initial all: Rust 4707 PASS, clippy 0 diagnostics, build 208/208; ctest 67/68 but cargo_test overlapped edits (tree changed), so native baseline invalid. Logs preserved with baseline- prefix.
- Quick PASS: 2721 core initially, then 2732 core, dependent crates compiled, native build 4/4. Formatting helper mistake fixed before expanded quick PASS. Final ownership-order review keeps saved originals through destructor container release.
- Questions: MI11 precision/store boundaries; MI12 vector 15 stage ambiguity; MI13 wrapped interior-zero keys; docs/specs/questions/model-instances.md. Index stale MI3 slot was not reused because revised spec already assigns MI3-MI10.
- f778bcf4: 42 module tests covering the 20 vectors (vector 15 staging qualified by MI12), exclusions and MI11-MI13 recorded.
- Stable all: Rust 4718 PASS, native build 4/4 PASS, ctest 68/68 PASS; clippy rejected two uses of try_update (stable 1.95 vs MSRV 1.75). Replaced with compare_exchange_weak loop; no count-policy change.
- Next: repeat final all after this specific MSRV fix, no edits during the run; final report/queue afterward.
- Quiet q.py Bash launcher fails E_ACCESSDENIED; use bounded PowerShell reads.
- No game requests; pure core task, no runtime or whole-owner claims.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-model-instances-contractfix4-r1-manifest.json.
