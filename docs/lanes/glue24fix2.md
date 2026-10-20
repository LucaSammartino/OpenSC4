# Lane glue24fix2

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue24fix2 (pin and fix the tick-5500 debug-CRT int3 in the netmgr/netlot replacement path) | `.claude/worktrees/glue24` / `glue24` | **in progress** | (1) install the RTC failure handler in every build and re-run `enable = netmgr,netlot` twice to 2032 plus the save round trip (`glue24fix2-*`); (2) read the named call site and fix the defect or file NM12; (3) flip the merge-queue entry to ready |

## Handoff

- Goal: the intermittent int3 at `FreeCity4.dll+0xB320CD` (about tick 5500, 3 of 5 runs of the glue24 build family) killed runs `glue24-rep-7`, `-rep-final`, `-rep-final2`. Name the site, fix or file it, then re-run the replacements clean.
- Diagnosis (from `build-runs/full-disasm.txt` and the run logs): the int3 is inside the statically-linked UCRT `failwithmessage` (DLL+0xB31E76), the Run-Time Check (RTC) failure reporter. With `_RTC_GetErrorFuncW`/`_RTC_GetErrorFunc` both null (they read two globals set by `_RTC_SetErrorFuncW`/`_RTC_SetErrorFunc`) and no debugger answering its 0x406D1388 probe, it falls to the shared `int3` at +0xB320CD with no report — which is why the `_CrtSetReportHookW2` hook (commit `d10b154b`) never fired: RTC failures bypass `_CrtDbgReport` entirely. esp 0x001AE8E4 identical every time = deterministic site.
- Root cause: `engine/build-x86/CMakeCache.txt` has `CMAKE_BUILD_TYPE=Debug` and `FC4_CHECKED=OFF`. MSVC's default `CMAKE_CXX_FLAGS_DEBUG` includes `/RTC1`, so the deployed plugin compiles run-time checks into every plugin TU — but `CheckedAttach()` installs `_RTC_SetErrorFuncW(&RtcHandler)` only under `#if FC4_CHECKED`. Checks compiled, no handler → first failing check ends the process.
- Fix being applied: compile `RtcHandler` (and its DbgHelp symbolizer) unconditionally in `engine/plugin/FC4Checked.cpp` and install it in every `CheckedAttach()`; keep the canaries and self-tests behind `FC4_CHECKED`. The handler logs `RTC ERROR #N`, the check kind, the call site with symbols (`FreeCity4.dll+0x… file.cpp:line`), the activity stack and the last interface calls, and returns 0 → `failwithmessage` skips the int3 (`cmp eax,1; jne` past it in the disassembly) and the run continues.
- Expected after the fix: the runs either complete clean or log `RTC ERROR #N … call site: …` naming the real defect; then fix that defect (uninitialized local / ESP mismatch through a game vtable in our glue) or file NM12 in `docs/specs/questions/` if it is a benign check the original also trips.
- Next: rebuild (`qbuild quick`), commit, then game runs `glue24fix2-a`, `-b` (turbo to 2032-01-01, Konradshohe, `enable = netmgr,netlot`) and the save round trip `glue24fix2-save`; grep the logs for `RTC ERROR`.
- Facts: worktree is the glue24 worktree on branch `glue24` (task says continue there, commit on top of `b2e676f9`); deployed DLL is `engine/build-x86/FreeCity4.dll` (harness copies it into slot 1); `abi_entries.inc` unchanged, `FC4_ABI_VERSION` not bumped; game queue slots shared (max one outstanding request).
