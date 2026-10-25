| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| nettool-cost27 (NT36 Tempelhof road cost), kind engine | Codex / `nettool-cost27` / reserved branch `nettool-cost27`, detached HEAD | ready; code `e2484c179`, vectors `8ebf830bd`, main merge `c772fe20d`; ABI unchanged, no seam reservation; cargo 7848/0, clippy 0, CTest 117/117; `nettool-cost27-tempelhof-nt36-2` clean, both shadows exact 1/1; `nettool-cost27-konrad-nt36-1` clean, both shadows exact 73/73; both ref_warnings=0; NT36 answered | 1. Coordinator: review and land the ready engine queue entry. |

Merged main's NT36 answer: network-geometry §§9.1/9.8.1 and network-tools §8.1.5.
The satisfier now alternates signed-id ordered propagation sets until no affected vertices remain before its first hash pick.
Equality groups propagate allowed bounds while preserving individual initial heights/preferences; singleton visits queue height pairs for LIFO picks.
Tempelhof's sole road cell now retains four corners at 293.308929443359375; earthwork sum 8.198638916015625 rounds to 1, total 11.
Added four unit tests: admitted post-plane fixture/retry/pick order/cost, live clear-land replay, flat/gentle single-cell controls, gentle eleven-cell road control.
Updated singleton-order tests and two existing conformance vectors for the specified propagation; the anchored ramp still tests smoothing failure.
Pricing and powerline code are unchanged. Plane windows remain the existing host boundary; the admitted unit fixture supplies the specified windows.
Prior diagnostic plugin logging remains in place; no new questions. NT36's Tempelhof filing is resolved by the landed spec answer.
Owned Rust fmt: four files, exit=0, diff_lines=0. Focused turbo-off conformance: 2/2 passed.
Final qbuild all PASS: cargo 7848 passed/0 failed, clippy 0 diagnostics, native build 46/46, CTest 117/117.
Final native request: `qbuild-20261025-211628-build-ctest`; source commit `8ebf830bd`.
Gate logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
Earlier full gate failed two stale conformance expectations; they were corrected before the final passing gate.
Final Tempelhof: `nettool-cost27-tempelhof-nt36-2`, seconds=150/timeout=400, date stayed 2000-01-01; shadow julian=0.
Tempelhof shutdown: network exact 1/1 (12 vectors), powerline exact 1/1 (6 vectors), mismatches=0, unavailable=0.
Final Konradshohe: `nettool-cost27-konrad-nt36-1`, turbo/end_date=2032-01-16, reached 2032-01-20.
Konradshohe shutdown: network exact 73/73 (876 vectors), powerline exact 73/73 (438 vectors), mismatches=0, unavailable=0.
Both final verdicts clean, foreign_exceptions=0, ref_warnings=0; same final DLL SHA256 prefix `66430a40`.
Game evidence: `build-runs/game-results/<final label>.log` and `.verdict` for each label above.
Pre-final `nettool-cost27-tempelhof-nt36-1` was also clean/exact 1/1; repeated because the gate rebuilt the DLL afterward.
Scope remains clear-land draft status/cost. All required work is complete; queue kind engine, state ready, ABI unchanged.
