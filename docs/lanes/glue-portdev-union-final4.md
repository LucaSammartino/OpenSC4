| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-portdev-union-final4: final AT24 ports union lifecycle | Codex / `glue-portdev-union-final4` (engine; detached HEAD) | in progress; final logical round 3/3 | Instrument actual aircraft ownership, prove native lifecycle differences, gate and controlled game checks; park if unresolved. |

## Handoff
- Goal: diagnose saved-port union aircraft HostRelease warnings using AT24 public contract; no promotion without paired clean runs.
- Done: reused detached worktree; AT24 5.2.1-5.2.2 and public report read. Instrumented HostRelease phase/manager/receiver/count and Read IID/index provenance; quick gate PASS. q.py Bash denied; bounded PowerShell used.
- Finding: current Shutdown only clears initialized flag; final Release skips Shutdown; destructor releases base and map together. Native specifies map-first initialized Shutdown, base cleanup separately.
- Next: collect instrumented saved-city paired controls before lifecycle correction; baseline all still running (session 58942, build PASS208 edges; cargo/clippy logs complete).
- Fixture SHA256 confirmed 38f234433b0c387393a7e25a971953c37b7127ea93c7d5d0a4cb2c789e78b0be.
- Evidence: build-runs/evidence-run4 contains supplied control/reload verdicts, script and warning excerpts.
- Fixture immutable: C:/Users/tlori/FreeCity4/build-runs/fixtures/glue-portdev-complete4/pdc4-replace-20261022-a.sc4.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-portdev-union-final4-r1-manifest.json.
- Constraints: no subagents, refs changes, private fields, RE inputs, tools/maps/default changes; max 100 calls between milestones, original family cap 3.
