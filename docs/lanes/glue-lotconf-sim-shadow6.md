| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotconf-sim-shadow6: lot configuration and simulator shadows | Codex, `glue-lotconf-sim-shadow6` | in progress | Inspect contracts and existing shadow infrastructure; implement and measure to 2032 |

## Handoff
- Goal: default-off record-only lotconf_shadow and simulator_shadow; real-city measurements to 2032.
- Done: independent Rust configuration field/object derivation and C ABI projection; default-off lotconf/simulator probes wired into existing observer callbacks.
- Done: initial focused Rust tests 2/2; final owned-file formatting 4/4. Added ABI null/layout test and literal raw-byte vector handling after focused tests; full gate next.
- Next: run qbuild all, then queued verify if all cannot submit native checks; resolve diagnostics, measure original owners to 2032.
- GLUE22 filed in docs/specs/questions/lot-configuration.md: native capacity input contract and absent configuration cost getter. Occupancy/cost remain unmeasured.
- Simulator measures native Write/core Read bookkeeping plus calendar/speed/time getters; no independent scheduler evolution. Native inTick is ignored on Read by spec; all other record bytes compared.
- Lot configuration compares every loaded ID; cache independently derived from effective exemplar properties, rebuilt at city load/detail change. PL23 unspecified appended-foundation tags reported separately.
- Evidence: q wrapper failed Bash CreateFileMapping (build-runs/q/20261024-022755-9140.log); direct focused cargo completed successfully. No duplicate operation started.
- Constraints: clean room; no tools/ reads or edits; queued native verify and game only; no branch/ref changes.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lotconf-sim-shadow6-r1-manifest.json.
- No game request outstanding.
