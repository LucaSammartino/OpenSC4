| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-exact6 | Codex / `glue-nettool-exact6` | in progress; shadow remains 8/14 | Corrected fixture needs final gate; finish city replay inputs. |

## Handoff
- Goal: exact network tools shadow; six residual probes remain, no READY entry.
- Detached base 760cf23fd; do not switch branches or update shared refs.
- 5c93779a6: NT30 corner-refresh populations, controlled price/refresh tests, dated shadow lines.
- 4825f1d46: private grading overlay; refresh uses own writes, price uses live terrain.
- 74fd22902: exact uint32 x float32 product/truncation; f64 can cross integer boundary.
- NT31 controlled bridge selection/span-price and synthetic powerline placement fixtures pass (3/0).
- Powerline getter retains counted_poles after structural release per newest NT31;
  older NG11 wording questioned as NT32 in docs/specs/questions/network-tools.md.
- Round1 all PASS: cargo6892/0, clippy0, build233/233, CTest72/72; session40130 terminal.
- Round1 logs: build-runs/r1-{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log.
- Run382 nettoolexactsix20261024a: clean2032-01-06, refs0, exceptions0, nettool8/14,49.266s.
- Run382 DLL d37c42349d4ad1f0fe28dd6365886aa285de10a78bdda5e6221a41ed267fe352.
- Costs remain native/core110/113,170/192,90/98,55/61; replay changed12,34,18,12 vertices.
- Powerline valid13/0,cost65/0; bridge valid17/0,cost974/0. No fitted correction.
- Round2 all FAIL, session24883 terminal: cargo6652/1, clippy0, build91/91, CTest71/72.
- Sole round2 diagnostic: legacy tests_power_line::cost_of_solution_shipped_values lacks counts.
- Corrected that fixture to supply counted_poles explicitly; final full gate still required.
- NEXT: preserve round2 logs before next all; fmt seventh owned tests_power_line.rs;
  run final all, then unique city label (run383+ only after grep docs/runs).
- Then trace four corner inventories per NT30; wire actual powerline and bridge resources/hosts.
- No operation/request outstanding. No ABI exports/version, regress/profile or shared refs changed.
- Focused failed fixture evidence: build-runs/nt31-fixture-*. q.py Bash fails here; direct cargo works.
