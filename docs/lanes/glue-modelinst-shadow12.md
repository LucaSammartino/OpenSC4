| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (model instance shadow load fix) | `glue-modelinst-shadow12` / engine lane | in progress | Diagnose shadow11 pre-city load blocker; apply MI14; gate and measure to 2032. |

## Handoff
- Goal: merge shadow11, fix menu load blocker, apply MI14 and measure shadows through 2032.
- Done: shadow11 merged in 2b8dc5bcf, preserving main's lighting additions.
- Diagnosis: pre-load diff only registers default-off ini key; fixture starts in city-load notification. MI14 proves missing constructor flag corrupts the x86 stack and allocation-start cast selects the container instead of instance interface.
- Applied MI14 constructor flag 1 and +0xA0 standalone interface adjustment; next: queued native verification, then game to 2032 to test load and exact fixture results.
- Keep shadows default-off; use queued native builds and game runs; no direct game launch.
