| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| spike-inplace26 (run-7 compare harness, round 4) | `Codex` / `spike-inplace26` (engine lane) | READY. Code through 70b460c4a; evidence 1098a022a. cargo 7844/0, clippy clean, CTest 117/117. inplace26-r4-final-2 clean/ref_warnings=0; 25335/25335 comparisons exact, 95/95 mutations detected/located. Two private C bridge symbols added; ABI version unchanged. Questions none. | Land the default-off facility; class-binding lanes follow docs/ops/compare.md. |

SPIKE VERDICT: INCONCLUSIVE (historical S3 rate criterion; this round delivers the approved harness, with no new rate/coverage claim).
Registry: engine/plugin/compare/*.tsv; exact run-7 columns, generated entry shells/dispatch, published-contract validation when functions.tsv exists.
Adding a binding needs one registry row and Rust adapter entry; optional authored fixtures stay in Rust. No plugin C++ edit per binding.
Selectors: compare names/class:<Class>/all, inplace alias; default-off compare/record/replace modes and explicit override.
Corpus: schema-filtered values, pointer words removed, image pointers persisted as RVAs; bounded records and contract validation.
City-load corpus replay uses isolated copies; compare_dense makes one/two nearby scalar changes, preserving declared count/stride relationships.
First-K JSONL differences include both outputs and mutation labels; engine/tests/compare_case_to_rust.py emits engine/tests/vectors/*.rs.
Stable registry/ini/summary contract is documented in docs/harness.md; docs/ops/compare.md is the 49-line binding/fix workflow.
CompareAttach provides per-row outgoing-call callbacks for S4; no S4 files changed.
All 28 requested prior functions retained, plus two historic declined rows: 30 rows, 26 installed.
Declined: rotation, visible-edges, lighting-night, system-country; unsupported entry relocation, no speculative execution.
Final game inplace26-r4-final-2: DLL 23e6068b; clean/ref_warnings=0/foreign_exceptions=0, zero ordinary differences/float-only differences/outside signals.
Natural: 37444 calls, 2410 sampled, 2409/2409 exact; one clip3 output-alias refusal. Call volumes depend on rendering cadence; old absolute counts were not reproduced.
Fixtures: 640/640 exact, reproducing prior 512 view-utility + 128 round-3 comparisons. Mutations: natural 17, fixture 26, corpus 26, dense 26; all 95 detected/located.
Corpus: 2478/2478 exact despite lower current recording cap. Dense: 19808/19808 exact from 19824 attempts, 16 explicit zero-language fallback-domain refusals.
Mode checks: inplace26-r4-record-2 clean/ref_warnings=0, 497 records/no comparisons; inplace26-r4-replace-2 clean/ref_warnings=0, 1944 Rust-only calls/no comparisons (earlier DLL dd354cea).
Earlier failures kept separately: capture-1 nested-hook lock stall fixed; dense-1 had 7 corpus/56 dense view-box differences from omitted published ground-line fields, fixed in snapshot schema.
No production Rust algorithm mismatch was found this round; the earlier failing values and all unattributed neighbourhood-write exclusions remain in evidence.
Gate after last engine fix: cargo 7844 passed/0 failed, clippy clean, CMake built, CTest 117/117; owned Rust fmt 3 files/0 diff. No engine changes after the gate.
Native tests cover modes, isolated replay, nested sampling, larger previous write cap, and pointer/padding/strided-field sanitization; the converted Rust vector compiles.
Limits: confinement observes net changes in containing private allocations (64 MiB cap), excluding active stack; misses undone writes, unrelated allocations and image globals; concurrent changes are unattributed.
Evidence: engine/tests/spike-evidence/spike-inplace26/round-4/ (recomputable per-function/source CSV/JSON and requests); raw mirrors/logs in build-runs/spike-evidence/round-4/ and game-results/.
Two private static-link C bridge symbols added; fc4.h and FC4_ABI_VERSION unchanged. No coverage-file or spec edits. Questions: none. Next: coordinator lands the harness.

