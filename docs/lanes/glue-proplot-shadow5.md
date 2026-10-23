| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-proplot-shadow5: PL23 prop/lot shadows | Codex, glue-proplot-shadow5 | in progress | Apply PL23, gate, measure through 2032 |

## Handoff
- Goal: compare specified prop/lot bytes, report unspecified bytes as fixups, obtain exact shadows through 2032.
- Done: PL23 comparison helpers and synthetic P/Z/F/D checks added; raw native verification retained; defined-field shadow counters now separate. Arrays walk every record, even after a difference.
- Next: run final qbuild all and queued verify, then submit unique game run to 2032.
- Focused local compiler attempt could not run: q wrapper's Git Bash CreateFileMapping failed with Win32 error 5; evidence build-runs/q/20261024-001230-29316.log. No compiler is available on PATH; new CTest will run via queued verify.
- No replacement defaults/core algorithms changed; no real defined-field difference established yet.
- Constraints: no tools/ access; coordinator must apply any regress profile addition there.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-proplot-shadow5-r1-manifest.json.
- No game request outstanding.
