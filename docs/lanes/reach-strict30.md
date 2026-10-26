| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach-strict30 / span565 strict suite reach; kind engine | Codex / reach-strict30 / detached HEAD; supervisor publishes reach-strict30 | ready final; code 3b36cd04f, merge fdb75ae40; ABI/seam none; cargo 8098 passed, clippy 0 diagnostics, CTest 141/141; final4 clean, span565 498/498 exact, ref_warnings=0; questions none | 1. Attach coverage credit and land the ready entry at its full detached tip. 2. Other owners deferred beyond this final round. |

- Final round finished the existing span565 set only; no new span4444/1555/8 or remaining-owner driver was started.
- Merged main first again (fdb75ae40, main ed9d973d8); retained BudgetWindow and appended Span565Probe in the enum/name lists. Earlier merge: 31fb6a9d8.
- Original driver/tests/scenario: 95fc54e19; diagnostic fix: 193a00ced; copied-window verifier fix: 3b36cd04f. No further renderer implementation changes this round.
- Acceptance: `span565-buffer` is in `full,reach30`; two dated `span565probe` passes exercise 249 guarded native/Rust cases each, including teardown/reinstallation.
- 3b36cd04f fixes the verifier: one allocation contains independently guarded source/destination preimages, keeping the C ABI hull below 256 MiB; all byte, source, guard and failure checks remain.
- Dated `samples=` shadow lines now enter the official parser; final `per_key.tsv:86` records `span565` 1/1 exact, zero resyncs.
- Official `run --set all --scenario span565-buffer --profile shadows --reach --dry-run` staged immutable inputs; main `suite.py resume <tag> --slots 2` submitted through the shared main queue (no private queue).
- Final gate `qbuild all` ran once: PASS, cargo 8098/8098, clippy 0 diagnostics, CTest 141/141; `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`. No Rust files owned/edited.
- Final DLL SHA256 `5de4f8554361872ef49985c0db7923e54ae8cf1c8731533bb299b316a3daa7f0`; final4 verdict identifies this DLL/hash after the gate.
- Final shared-queue label `reach-strict30-final4`: clean, end 2026-01-30, both actions ok, ref_warnings=0; `C:/Users/tlori/FreeCity4/build-runs/game-results/reach-strict30-final4.{verdict,log}`.
- Final4 log lines 3552/3704: 249/249 exact twice; mismatches/different/refused/unaccounted/contract_failures/unchanged_unexpected/resyncs=0. Actions ok at 3560/3712; reach at 3889: saved=1, ignored/aborted/slot_exhaustions=0.
- Final4 spec/public reach versus sh28-census3 (178 traces): 0 -> 133 functions / 36,753 bytes; public missing=0; `build-runs/reach-strict30-final4.reach.tsv`, `measure-span565.py`.
- Final4 strict attribution: 0 -> 134 functions / 36,854 bytes; reached set identical to suite3; `build-runs/measure-final4.py`, `span565-final4-metrics.txt`. Official suite3 coverage remains span565 0.0% -> 94.4% (`strict-after.md:138`).
- Attribution's extra 101 bytes are entry `00845450` (`FUN_00845450`), outside the spec dispatch/wrapper roster; the two foreign ICF bodies remain excluded from the spec-mapped sum.
- Fixed-census union: 329,798 -> 380,643 strict verified bytes (+50,845; +0.91049 pp); `build-runs/gaps-reach-strict30-after/metrics.json`. Union gain includes other enabled keys; span565's own gain is 36,854 bytes.
- Preserved failed `suite1`: actions 0/2, 339/498 exact, 159 WINDOW refusals from separated copy allocations; `suite1/span565-buffer.log`. Corrected `suite2`: actions 2/2, 498/498 exact; post-gate relink required final suite3.
- Four non-span finding keys persist through final4: gzmsg unavailable=1; viewutil camera-angles=8, city-width=7, aggregate=15; final4 log 3002/3032/3035/3059; outside this owner's credit.
- Final4 waited 905 s for a shared slot, then ran 174 s. Verdict notes hidden/held 127 s at the reached end date; both probes completed beforehand. No comparison was relaxed or scenario excluded.
- Scope: constructed public software buffers; spec section 1.2 establishes no shipped type-5 producer. Remaining 11 functions / 2,197 bytes are section 7 keyed 8-bit row 13, unavailable through the public context.
- No ABI export changes, spec edits, coverage-map edits, replacement-list edits or generated ABI-entry edits; questions none.
- Next: attach checked coverage credit and land the final ready engine entry; no full census was rerun and no further work is in flight.
