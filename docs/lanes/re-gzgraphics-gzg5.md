| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzgraphics-gzg5: GZG graphics evidence | Codex, `re-gzgraphics-gzg5` (detached HEAD) | wip: GZG6 answered; partial GZG1/GZG3/GZG5; doccheck passed | Finish GZG1–GZG5 before closing the engine lane's declared boundaries |

## Report

The full GZG1–GZG6 task is **incomplete**. GZG6 is answered and moved to the
answered file; GZG1–GZG5 remain open, with explicit partial-resolution notes.
The merge queue is held as `wip`, not a claim that all renderer contracts are
ready. Research stopped at the instructed two-thirds context allocation;
remaining context was reserved for specification, checks and this report.

### Changes and evidence

- **GZG6 / §2.1: DIFFERS.** Final context destruction explicitly calls engine
  Shutdown, frees six temporary vector allocations, releases the engine, then
  independently releases the target. Instructions at `0x0082CA40`–`0x0082CADD`
  establish the call order. The engine lane must correct its final-context
  release behavior; direct buffer-engine final Release still has its own
  no-unlock quirk.
- **GZG1 / §§3.2–3.3, 4.2:** both folded Ellipse convenience slots accept four
  integers and submit filled kind 5. Kind 2 consists of independent cubic
  curves. Added filled-ellipse raster spans, result-1 polygon subdivision and
  its repeated-first-pair quirk, line fixed-point input conversion, signed
  span endpoint handoff, convenience-blit placement/scaling/cleanup, and
  buffer gradient conversion/rounding. Full outline/line/polygon coverage
  remains open.
- **GZG3 / §5.3:** ByIndex/ByID return borrowed stored pointers or null; neither
  adds a reference, creates a driver or substitutes another one.
- **GZG5 / §6.3:** exact ordinary clipping mutations, low-byte return and lack
  of rollback; ordinary AlphaBlt's local copies/destination point; legacy
  loader argument widths, unsigned two-division quantization and raw mask
  bytes, bottom-up copies, unchecked reads, leaked references/header storage,
  and the one-past-end free attempted after failed destination lock.
- **17 new declarative vectors**, GZG-V45–V61. These are specified expected
  results from static evidence, not executed native or game measurements.
- No new behavior claims for GZG2 or GZG4. Their relevant functions were
  located/partly read, but not enough to settle their contracts.

Used the main checkout's read-only `re_query.py` export before matching decomp
reads; numeric claims were checked in instruction pages. No REA session was
opened or queried, and no REA error/fallback occurred. No game files,
decompilation, assembly listings or external sources were committed. No engine,
plugin, capi, tools or ABI tables were edited. No proposed ABI rows or exports.
The core lane's own report supplies the open-section list; the owner table is
inside forbidden engine/, so it was not read.

### Validation and commits

- `python tools/questions.py index`: passed; generated QUESTIONS.md updated.
- `python tools/doccheck.py check`: passed after both behavior and final-report
  edits. `git diff --check`: passed.
- Cargo, Clippy, CTest and game/shadow: n/a, specification-only work.
- `c955a568`: initial lane journal.
- `64af2ec9`: behavior additions, GZG6 answer and vectors.
- Git emitted `packed-refs.lock: Permission denied` during commit cleanup;
  each commit nevertheless completed successfully on detached HEAD. No shared
  refs/config were modified or retried.

### Remaining work

1. GZG1: finish integer-line clipping/diagonal ties, cubic forward differences,
   outline subdivision, and simple-polygon closure/coverage/tessellation.
   Start with `0x0082D040`, `0x0082D5A0`, `0x0082DB00`, `0x00932700`.
2. GZG2: hardware extension slots/base shutdown, device subdivision, buffer AA
   coverage, SimGL specialized/fallback paths and HWSetImage failures. The two
   failed-export bodies still require paged assembly, not REA.
3. GZG3: remaining IO inventory, driver discovery/INI/rollback, screen defaults,
   projection, presentation and folded control-slot semantics.
4. GZG4: adjusted-interface slots/thunks, HWND lifecycle, queue/event translation
   and message coordinate units. The pseudocode has seriously broken locals;
   do not infer payload signatures from it alone.
5. GZG5: stretch clipping and all standalone pixel kernels, supported formats,
   lock ordering and return/failure contracts. Loader details are now settled.

## Handoff

- Goal: answer GZG1–GZG6 in gz-graphics.md from Windows 1.1.641 export.
- Done: `64af2ec9` adds GZG6 answer and partial GZG1/GZG3/GZG5; 17 vectors.
- Next: finish GZG1 integer-line/polygon coverage from the addresses above.
- GZG1–GZG5 remain open; queue must remain wip until those are answered.
- Numeric loader/gradient details and source references are in §§4.2 and 6.3.
- Research stopped at two-thirds context allocation; full task not achieved.
- Export/main: C:/Users/tlori/FreeCity4; worktree is this directory.
- No game runs requested; no REA binary open, use re_query.py and decomp.
