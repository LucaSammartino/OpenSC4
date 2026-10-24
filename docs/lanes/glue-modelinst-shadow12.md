| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (model instance shadow load fix) | `glue-modelinst-shadow12` / engine lane | in progress | Diagnose shadow11 pre-city load blocker; apply MI14; gate and measure to 2032. |

## Handoff
- Goal: merge shadow11, fix menu load blocker, apply MI14 and measure shadows through 2032.
- Done: shadow11 merged in 2b8dc5bcf, preserving main's lighting additions.
- Diagnosis: pre-load diff only registers default-off ini key; fixture starts in city-load notification. MI14 proves missing constructor flag corrupts the x86 stack and allocation-start cast selects the container instead of instance interface.
- Applied MI14 constructor flag 1 and +0xA0 standalone interface adjustment in e8703c22d.
- Native verify PASS: qbuild-20261024-155017-build-ctest; build 263/263, CTest 81/81, logs build-runs/fc4verify-{build,ctest}.log.
- Game run 520 modelinst-mi14-loadfix-2032-r1: clean to 2032-01-02, ref_warnings=0; static 73/73 exact, 144 fields/line. DLL fa30f030. Completed verdict is valid; file-watcher wait session 74491 has not noticed it yet and still needs terminal collection (no repeat game request).
- Live static GetTransform destination counter/padding observer implemented locally (first 4096 natural calls/city, no added native calls); focused cargo test session 51921 still compiling. Next: collect it, final all gate, then measure live/static together.
- Keep shadows default-off; use queued native builds and game runs; no direct game launch.
