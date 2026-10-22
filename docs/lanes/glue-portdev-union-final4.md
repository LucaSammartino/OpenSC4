| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-portdev-union-final4: final AT24 ports union lifecycle | Codex / `glue-portdev-union-final4` (engine; detached HEAD) | in progress; final logical round 3/3 | Instrument actual aircraft ownership, prove native lifecycle differences, gate and controlled game checks; park if unresolved. |

## Handoff
- Goal: diagnose saved-port union aircraft HostRelease warnings using AT24 public contract; no promotion without paired clean runs.
- Done: reused detached worktree; AT24 5.2.1-5.2.2 and public report read. Instrumented HostRelease phase/manager/receiver/count and Read IID/index provenance; quick gate PASS. q.py Bash denied; bounded PowerShell used.
- Finding: current Shutdown only clears initialized flag; final Release skips Shutdown; destructor releases base and map together. Native specifies map-first initialized Shutdown, base cleanup separately.
- Baseline all PASS: cargo4707/0, clippy0 diagnostics, ctest68/68 (tree changed during baseline; final stable gate still required).
- Diagnostic paired runs pduf4-diag-ports-20261022-a and pduf4-diag-control-20261022-a clean2033/ref3, same DLL00213eb1 and immutable fixture/script.
- Proven root: aircraft Read base3+map3pairs=9 owned refs; Shutdown released none; destructor releases list planes2->1 then map planes1->0 (all three warnings). Airports2->1. Same common path in ports-disabled control.
- In progress: AT24 lifecycle bridge for public Shutdown/deactivate callbacks, native pointer order and separate base/map ownership; additive C ABI export, no version bump.
- Done: seven counted-interface families pass (capi527/0); fmt check PASS on both touched Rust files. Native plane slot13 uses SDK Remove(false).
- First corrected game pduf4-fixed-ports-20261022-a clean2033/ref0; runtime logs prove ascending airport-map pair releases during Shutdown, then base plane Shutdown/release; final Release calls empty Shutdown. Paired fixed-control pending/completing, session35355.
- Quick wrapper has a Ninja restat parsing issue (exit log1/4 despite no compiler error); final stable gate will include real C++ rebuild after final diagnostic wording change.
- Next: stable full gate; final same-DLL paired fixture checks; ordinary union and saved fixture shadow. No promotion before coordinator joint train PASS.
- Fixture SHA256 confirmed 38f234433b0c387393a7e25a971953c37b7127ea93c7d5d0a4cb2c789e78b0be.
- Evidence: build-runs/evidence-run4 contains supplied control/reload verdicts, script and warning excerpts.
- Fixture immutable: C:/Users/tlori/FreeCity4/build-runs/fixtures/glue-portdev-complete4/pdc4-replace-20261022-a.sc4.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-portdev-union-final4-r1-manifest.json.
- Constraints: no subagents, refs changes, private fields, RE inputs, tools/maps/default changes; max 100 calls between milestones, original family cap 3.
