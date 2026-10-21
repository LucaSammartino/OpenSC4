| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain3 (enable = terrain glue over the TR21 3D view contract, terrain.md §15.5) | `glue-terrain3` / (engine lane) | blocked on TR38, TR39, TR40 (and TR33): `enable = terrain` still faults in the game's 3D view before the first month; full gate PASS | wait for answers to TR38/TR40 (edge-layer set-up) and TR39 (who calls slot 96 Load), then run gt3-repl-6 |

## Report (run4 round 1, 2026-10-21)

Goal: `enable = terrain` clean to 2032 under turbo with ref_warnings=0. Not reached. This is not ready to
merge or count as a replacement.

Code on the branch: c1b06da3 (creates the view and calls its Init, raw T+0x6C / W+0x18 mirrors, GetView =
base+0x0C), 25a4ec76 (slot 96 Load reads the misc / altitude / property records), 44a73e47 (slot 96 Load logs
when it is called; TR38/TR39 filed), plus this round's tip (a constexpr switch on the TR33 whole-city redisplay,
left on, and TR40).

Game evidence (no RE material read; registers and stacks from the harness logs only):
- gt3-repl-4 (TR33 whole-city redisplay on): the first queued 0x278E6003 after DoPostCityInit faults in the
  view's edge-layer refresh FUN_00755B60 at 0x00755C2B (write 0x16B23FF8, caller 0x00756AAC in FUN_007569F0),
  before the first game date. Question TR38.
- gt3-repl-5 (the same build with the post off): it reached 2026-01-27, then the first DrawTerrainLayers faults
  at 0x007540A5 (reads 0x00000008, 0x00B4C760 edge-layer statics on the stack, called from 0x0075BFA3 under
  Update). Question TR40.
- Both runs: slot 96 Load is never called with a segment, so the replaced city runs on the flat 300 m default
  rather than its saved altitudes. Question TR39 (the "Load: slot 96 called" log line was added for
  gt3-repl-5 and does not appear). Both runs: ref_warnings=0, no Rust panics.
- Diagnosis: the view's city-edge layer state is not set up when the replacement terrain is in place. The spec
  does not name what sets it up: §15.5.1 step 7's 0x00743B60/0x007490B0 (TR33), slot 67 DoLighting (a no-op
  here) or something else. Per the brief, no value was guessed.

Gate (tip): qbuild all PASS: cargo 3576/0, clippy 0, ctest 62/62.

Questions filed in docs/specs/questions/terrain.md: TR33-TR37 (earlier rounds), TR38, TR39 and TR40 (this
round). QUESTIONS.md still lists TR33 as the next free ID; the coordinator must regenerate it (next free: TR41).

Next steps: (1) once TR38/TR40 are answered, implement the edge-layer set-up (step 7 pair / slot 67) and drop
or keep the TR33 post as the answer says; (2) once TR39 is answered, make Load run on a city load; (3) send
gt3-repl-6 (enable = terrain, turbo, end_date 2032-01-01) and then a normal-speed run; (4) Save (slot 97) is
still unwired; (5) the coordinator adds `terrain` to the regress replace list and the coverage replace key.
