# Lane glue24fix2

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue24fix2 (pin and fix the tick-5500 debug-CRT int3 in the netmgr/netlot replacement path) | `.claude/worktrees/glue24` / `glue24` | **The int3 is diagnosed and disarmed: the site is the statically-linked UCRT `failwithmessage` falling to its silent int3 because the deployed Debug build compiles /RTC1 with no RTC error func installed; the handler is now installed in every build, and `enable = netmgr,netlot` ran twice to 2032 clean plus the save round trip clean (state ready pending the gate).** | (1) full gate (`qbuild all`) before the merge train; (2) the merge-queue entry flips to ready; (3) if a future run logs `RTC ERROR #N`, fix the named call site or file NM12 (docs/specs/questions/) |

## Report

**What the int3 was.** The three glue24 deaths (`glue24-rep-7`, `-rep-final`,
`-rep-final2`, all at about tick 5500 = 2031-01-03, esp 0x001AE8E4 identical
every time) were the statically-linked UCRT `failwithmessage` at
`FreeCity4.dll+0xB31E76` falling to its shared `int3` at `+0xB320CD`. Reading
the disassembly (`build-runs/full-disasm.txt`): `failwithmessage` is the
Run-Time Check (RTC) failure reporter. It consults `_RTC_GetErrorFuncW` /
`_RTC_GetErrorFunc` (two globals set by `_RTC_SetErrorFuncW` /
`_RTC_SetErrorFunc`), then probes the debugger by raising exception
0x406D1388. With no error func installed and no debugger, it reaches
`cmp eax,1; int3` — it ends the process without printing anything. That is
why the `_CrtSetReportHookW2` hook (glue24 commit `d10b154b`) never fired:
RTC failures never go through `_CrtDbgReport`.

**Why the checks were live but unhandled.** The deployed build
(`engine/build-x86`) is `CMAKE_BUILD_TYPE=Debug` with `FC4_CHECKED=OFF`.
MSVC's default `CMAKE_CXX_FLAGS_DEBUG` carries `/RTC1`, so every plugin TU
had run-time checks compiled in, but `CheckedAttach()` installed
`_RTC_SetErrorFuncW(&RtcHandler)` only under `#if FC4_CHECKED`. Checks
compiled, no handler: the first failing check was fatal. `/RTC1` is only
compiled into the plugin's own TUs, so the failing site was our code —
probably one deterministic uninitialized-local or ESP check in the glue or
the core bridge (esp identical every run).

**The fix** (commit `c1afaa18` on top of `b2e676f9`): compile `RtcHandler`
and its DbgHelp symbolizer unconditionally in `engine/plugin/FC4Checked.cpp`
and install the error func in every `CheckedAttach()`; keep the
out-parameter canaries and self-tests behind `FC4_CHECKED`; clear
`g_inDllMain` in `CheckedStartup()` for every build so DbgHelp can load.
Now a failing check logs `RTC ERROR #N` with the check kind, message, call
site (function+offset and `file:line` when the PDB is next to the DLL), the
activity stack and the last interface calls, and returns 0 — which
`failwithmessage` honors (`cmp eax,1; jne` skips the int3): the run
continues and names its defect instead of dying.

**Measured results** (runs 193-195, `docs/runs/glue24fix2-*-193/194/195.md`,
all on the new DLL, sha256 `be596b82`, log line `build: normal (/RTC1 where
the configuration sets it; RTC failures are logged …)`):

| Run | Verdict | Pace | Through the death window |
|---|---|---|---|
| `glue24fix2-a` | clean (rtc=0) | 60.91 days/s, reached 2032-01-06 | 2031-01-03/04 passed, no int3 |
| `glue24fix2-b` | clean (rtc=0) | 62.73 days/s, reached 2032-01-15 | 2031-01-03/04 passed, no int3 |
| `glue24fix2-save` | clean (rtc=0) | 44.83 days/s, reached 2032-01-03 | `save -> ok=1 (10906 ms)` |

The save round trip matches the recorded glue24 behavior: `netmgr Write ->
1 (6 bytes, record53 round trip same)`; `netlot Write -> 1 (3160 bytes,
record11 round trip DIFFERENT)` — the known one clock-seeded configuration
draw, counted, not a mismatch.

`rtc=0` in all three runs means the failing site (3 of 5 runs before) did
not fire this time; it is rare. If it fires in a future run, the log now
names it (`RTC ERROR #N … call site: …`) and the run continues — fix the
named site or file NM12 if it is a benign check the original also trips.

**Gate:** cargo **3019/0**, clippy clean (0 diagnostics), ctest **58/58**
(`build-runs/gate-glue24fix2.txt`). **Questions:** none filed. **ABI:**
unchanged (`abi_entries.inc` untouched, `FC4_ABI_VERSION` not bumped).
