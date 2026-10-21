| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span565-impl4: plugin glue enable=span565 (cSpanRender565 vtable 0x00AC1844 bridge) | `glue-span565-impl4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: enable=span565 five-method bridge (spec span-render-565.md 1.1/1.2), constructed-buffer native-vs-ours trace in game.
- Done: core extents 7d646510; capi fc4_span565_* (5 entries, decline codes) 04b3cea8; plugin FC4Span565Bridge.{h,cpp}
  (vtable 0x00AC1844 slots 1-5, checks all 6 expected entries, PostAppInit install, PreAppShutdown restore + counters,
  span565_trace fixture) + wiring (CMake, FC4Subsystems "span565" default-off, cFC4Director, FC4Config key) -> next commit.
  Questions SR565-7 (fixture conventions/setters), SR565-8 (texture record) appended.
- quick gate PASS (cargo 2642/0 capi+core, build 144/144). Baseline `all` before work: cargo 3980/0, ctest PASS (tree
  changed during run), clippy 2 errors in my own new test (fixed).
- span565-t1: clean, trace EXACT 20/20 cases, shipped 0 calls, but ref_warnings=6 (missing FC4_ACQUIRED for
  created buffer/context; fixed in next commit). Next: game label span565-t2 same request; then qbuild all; finish.
- Old next: game request label span565-t1 (enable = span565, span565_trace = 1, turbo, end_date 2032-01-01);
  grep log for "span565"; then qbuild all; then finish (lane file report, merge-queue json).
