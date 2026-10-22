| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-callbacktimer-final4 (RE: timers.md TM3 a/b/c, cRZCallbackTimer ctor/dtor/slot 11/Stop refcounts) | `re-callbacktimer-final4` / `.claude/worktrees/re-callbacktimer-final4` | **ready to merge** (docs only, no ABI, no game run). TM3 answered DIFFERS (a, b) / MATCHES (c); spec §2.1/§2.2/§2.3/§2.3.1, quirks 10-11, §6 test 7; questions index regenerated | engine: dtor must gate the service release on `+0x04`; default scheduler uncounted; null slot 3 stored; orphan entry on running switch |

## Report

Proof from the instruction bytes (re_query.py asm; REA not used):
- Constructor `0x00932C23`: `+0x20 := 0`, then the default `0x00B62AB0` stored at `0x00932C86` with no AddRef,
  `+0x04 := 0` at `0x00932C8D`, direct call to slot 11 (result discarded). `+0x0C` is never written by the ctor.
- Destructor `0x00932AFE`: Stop if `+0x18`; release `+0x24`; clear `+0x08/+0x0C`; if `+0x20` non-null, Release only
  when `+0x04` true (`0x00932B71`), then `+0x20 := 0`.
- Slot 11 `0x00932A5D`: found path releases old (non-null, including the default), stores slot 3's result, AddRefs only
  if non-null (`0x00932AB7`); never touches ID/running.
- Scheduler AddRef/Release `0x009334B1`/`0x009334C2` count only when byte `+0x09` is set; the static default is never
  marked; the framework service ctor `0x009304A6` marks its heap scheduler (`0x009334E7`, 1) and AddRefs it (count 1).
  Release at count <= 1 calls slot 8 (`0x009337DD`) with 1.
- Handler `0x00932B87` checks only set membership, so an entry left in the old scheduler keeps firing after Stop.

Answers: TM3 (a) DIFFERS (dtor gated on `+0x04`, default uncounted), (b) DIFFERS from "not found": null stored,
(c) MATCHES plus the orphan-entry consequence. Moved to answered/timers.md; QUESTIONS.md regenerated.
Vectors: timers.md §6 test 7 (default replacement, no framework, leak, null slot 3, running switch).
Engine follow-up (not done here, docs-only lane): implement the `+0x04` gate in the destructor and the uncounted default.
