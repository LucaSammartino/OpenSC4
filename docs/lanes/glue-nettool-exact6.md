| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-exact6 | Codex / `glue-nettool-exact6` | in progress | Apply NT30/NT31 and measure the network tools shadow. |

## Handoff
- Goal: close six residual network tools probes using answered NT30/NT31.
- Base: detached HEAD 760cf23fd (train171); previous lane measured 8/14 exact.
- Done: NT30 two post-grading refresh passes implemented; controlled price/refresh
  fixtures pass, NT31 extended-precision powerline pricing fixed and tested.
- Done: shadow comparison lines now include the scripted date after `nettool shadow`.
- Focused cargo test `nt3`: 7 passed (four new NT30/NT31 tests); Rust 1.75 compatible.
- Done: record-only shadow grades into private terrain overlay, refreshes NT30 corners,
  then discards overlay for live post-native price reads. No game setters forwarded.
- Full qbuild all PASS (session 40130 collected terminal exit 0): cargo 6892/0,
  clippy 0 diagnostics, native 233/233, CTest 72/72; native check 346.7 seconds.
  Logs: build-runs/qbuild-{test,clippy}.log, fc4verify-{build,ctest}.log.
- DLL SHA256 d37c42349d4ad1f0fe28dd6365886aa285de10a78bdda5e6221a41ed267fe352.
- City nettoolexactsix20261024a collected terminal result: clean to 2032-01-06,
  refs=0, foreign_exceptions=0, 49.266 seconds; `nettool 8/14` (run 382).
- Same six differences: costs 110/113,170/192,90/98,55/61; powerline 65/0 valid13/0;
  bridge 974/0 valid17/0. New NT30 refresh/overlay did not change these city projections.
- No request outstanding. Next: trace retained replay cell/corner inputs for four costs;
  finish subclass resource/host replay. No exactness claimed, no READY queue entry.
- Follow-up diagnostic: f64 multiplication can cross an integer boundary; fixed using
  exact uint32 × float32 significand multiplication/truncation, with a discriminating test.
- NT31 controlled bridge selection/index and span-price fixture passes. Synthetic
  powerline recording placement now passes valid13/failure0/poles3/wires10/price65.
- Getter previously erased retained counted_poles after structural record release;
  corrected per literal NT31, older NG §11 wording questioned as NT32.
- New source after run382: final gate/game repeat pending. Focused NT31 tests 3/0;
  failed constructor/record-vector test evidence retained under build-runs/nt31-fixture-*.
- q.py Bash failed CreateFileMapping access (log build-runs/q/20261024-025713-33616.log);
  direct PowerShell cargo succeeded. fmt requires absolute file paths; fixes applied.
- Full native gate and unique 2032 city run required; no request outstanding.
- Do not edit tools/regress.ps1. Shadow label must be one lowercase word.
