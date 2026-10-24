| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (model instance shadow load fix) | `glue-modelinst-shadow12` / engine lane | in progress | Diagnose shadow11 pre-city load blocker; apply MI14; gate and measure to 2032. |

## Handoff
- Goal: merge shadow11, fix menu load blocker, apply MI14 and measure shadows through 2032.
- Done: shadow11 merged in 2b8dc5bcf, preserving main's lighting additions.
- Diagnosis: pre-load diff only registers default-off ini key; fixture starts in city-load notification. MI14 proves missing constructor flag corrupts the x86 stack and allocation-start cast selects the container instead of instance interface.
- Applied MI14 constructor flag 1 and +0xA0 standalone interface adjustment in e8703c22d.
- Native verify PASS: qbuild-20261024-155017-build-ctest; build 263/263, CTest 81/81, logs build-runs/fc4verify-{build,ctest}.log.
- Outstanding game: modelinst-mi14-loadfix-2032-r1; Konradshohe, turbo, target 2032-01-01. Blocking wait session 74491; collect it, never duplicate the request. Next: inspect exact counts and implement scoped live call observation.
- Keep shadows default-off; use queued native builds and game runs; no direct game launch.
