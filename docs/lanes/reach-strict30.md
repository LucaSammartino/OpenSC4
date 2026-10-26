| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-strict30 / span565 strict suite reach; kind engine | Codex / reach-strict30 / detached HEAD; supervisor publishes reach-strict30 | ready first set; code 3b36cd04f, merge 31fb6a9d8; ABI/seam none; cargo 8075 passed, clippy 0 diagnostics, CTest 135/135; suite3 clean, span565 498/498 exact, ref_warnings=0; questions none | 1. Land the ready engine entry at its actual detached tip. 2. Software-renderer/depth scenarios and remaining owners. |

- Merged main first (31fb6a9d8); retained UI actions and appended Span565Probe; original driver/tests/scenario are 95fc54e19 and diagnostic fix 193a00ced.
- Acceptance: `span565-buffer` is in `full,reach30`; two dated `span565probe` passes exercise 249 guarded native/Rust cases each, including teardown/reinstallation.
- 3b36cd04f fixes the verifier: one allocation contains independently guarded source/destination preimages, keeping the C ABI hull below 256 MiB; all byte, source, guard and failure checks remain.
- Dated `samples=` shadow lines now enter the official parser; final `per_key.tsv:86` records `span565` 1/1 exact, zero resyncs.
- Official `run --set all --scenario span565-buffer --profile shadows --reach --dry-run` staged immutable inputs; main `suite.py resume <tag> --slots 2` submitted through the shared main queue (no private queue).
- Final tag `reach-strict30-suite3`: clean, end reached 2026-01-30, actions 2 ok/0 failed, ref_warnings=0; `C:/Users/tlori/FreeCity4/build-runs/suite/reach-strict30-suite3/summary.tsv` and main game-results verdict.
- Final DLL SHA256 `88bc6b5ca0386daf16cd00a2704a8039858ad59ce609c40e9c9fcbf53e80d504` matches the post-gate artifact and suite manifest.
- Suite3 log lines 3551/3703: 249/249 exact twice; mismatches/different/refused/unaccounted/contract_failures/unchanged_unexpected/resyncs=0. Log line 3887: saved=1, ignored/aborted/slot_exhaustions=0.
- Spec/public reach versus sh28-census3 (178 traces): 0 -> 133 functions / 36,753 bytes; public missing=0; final `.reach.tsv`, `build-runs/measure-span565.py`, `span565-final-metrics.txt`.
- Strict coverage attribution: 0 -> 134 functions / 36,854 bytes, span565 0.0% -> 94.4%; `build-runs/strict-after.md:138`, `strict-functions.tsv`, `measure-strict-span565.py`.
- Attribution's extra 101 bytes are entry `00845450` (`FUN_00845450`), outside the spec dispatch/wrapper roster; the two foreign ICF bodies remain excluded from the spec-mapped sum.
- Fixed-census union: 329,798 -> 380,643 strict verified bytes (+50,845; +0.91049 pp); `build-runs/gaps-reach-strict30-after/metrics.json`. Union gain includes other enabled keys; span565's own gain is 36,854 bytes.
- Ranking/after reports: `build-runs/gaps-reach-strict30-{before,after}/owners.tsv`; next ranked exact-owner gaps include AutomataScriptSystem 25,481 B, AdvisorSystem 7,781 B and ExemplarResourceView 7,635 B; netocc is not exact census-wide.
- Final `qbuild all`: PASS, cargo 8075/8075, clippy 0 diagnostics, CTest 135/135; `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`. Native build followed 3b36cd04f; no Rust files owned/edited.
- Preserved failed `suite1`: actions 0/2, 339/498 exact, 159 WINDOW refusals from separated copy allocations; `suite1/span565-buffer.log`. Corrected `suite2`: actions 2/2, 498/498 exact; post-gate relink required final suite3.
- Four non-span finding keys persist identically across suite1/2/3: gzmsg unavailable=1; viewutil camera-angles=8, city-width=7, aggregate=15; retained in final `findings.tsv`, outside this owner's credit.
- Scope: constructed public software buffers; spec section 1.2 establishes no shipped type-5 producer. Remaining 11 functions / 2,197 bytes are section 7 keyed 8-bit row 13, unavailable through the public context.
- No ABI export changes, spec edits, coverage-map edits, replacement-list edits or generated ABI-entry edits; questions none.
- Next: land this first verified engine set; software display/depth scenarios for span4444/1555/8, then remaining owners. No full census was rerun.
