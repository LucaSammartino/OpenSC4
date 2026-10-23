| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-proplot-shadow5: PL23 prop/lot shadows | Codex, detached glue-proplot-shadow5 | ready: cargo6695/0, clippy0, CTest70/70; normal2032 clean/ref0, propmgr220/220 and lotcfg293/293 defined fields | Coordinator adds exact keys to regress shadows profile; PL21 remains open |

## Result

PL23 defined-field comparisons are exact in a real city through **2032-01-01**. Raw comparisons remain unequal and are retained separately; this is not raw-profile, raw-array or whole-owner exactness.

Implementation commit: `f2a5f8701`. The profile helper excludes only +0x15..+0x17 after independently confirming absent or three-value OccupantSize. The array helper excludes only +0x24 of a final appended type-3 record, confirmed by a foundation property, and still checks type, flags, geometry, order, foundation type and empty ID lists. Every ordered record is checked, including records after a difference. Ordinary property-derived IDs remain significant. No native bytes are imported, and no replacement zero defaults or core algorithms changed. Both ini keys remain default-off. No C ABI exports changed.

The synthetic CTest covers PL23-P/Z/F/D, disallowed padding masks, ordinary-record IDs, nonempty foundation lists, differing foundation types and geometry. Full gate `qbuild all` passed: cargo **6695 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, queued native build **222/222 edges**, CTest **70/70**. The same asynchronous operation completed; no duplicate verify was launched. Native queue label: `qbuild-20261024-001556-build-ctest`. Only lane documentation changed during the gate. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Earlier standalone test attempt failed before compilation because Git Bash CreateFileMapping returned Win32 error5; preserved at `build-runs/q/20261024-001230-29316.log`; the new test subsequently passed in CTest.

## Game measurements

Both runs used native owners in Konradshohe and the identical DLL SHA256 `709727b9c95d525032855e1e792ca48084ef18a4d438925fac0a4b248cce0561`. Both reached 2032 cleanly with ref_warnings=0, exceptions=0, stalls=0, rtc=0, overruns=0, incomplete=0 and resynced=0.

| Run | Pacing | Exact shadow lines |
|---|---|---|
| [proplot5-defined-2032-20261024-r1](../runs/proplot5-defined-2032-20261024-r1.md) | turbo | propmgr 219/220; lotcfg 293/293 |
| [proplot5-defined-2032-20261024-r2-normal](../runs/proplot5-defined-2032-20261024-r2-normal.md) | normal | propmgr 220/220; lotcfg 293/293 |

```text
propmgr 220/220
lotcfg 293/293
proplot5-defined-2032-20261024-r2-normal propmgr 220/220
proplot5-defined-2032-20261024-r2-normal lotcfg 293/293
propmgr shadow (catalogue) 2026-01-27 load: mismatches=0 types=1544/1544 families=112/112 (orig/ours) keysDiffering=0 validDiffering=0 profilesDiffering=0 padOnly=1544 members=1136 familiesDiffering=0 data2=1544 (types 1544) catalogue=1544/112/1544 scope=defined-fields rawProfilesDiffering=1544 fixups=1544 first: -
lotcfg shadow (arrays) 2026-01-27 load: mismatches=0 configurations=1909 objects=25210 detail=1 foundations=1908 unbound=0 configurationsDiffering=0 scope=defined-fields rawArraysDiffering=1908 fixups=1908 first: -
```

Raw native verification still reports prop catalogue pairs4862/exact3318/different1544 and complete lot arrays pairs1909/exact1/different1908, both refused0/fallback0/resync0. These raw differences are precisely the documented PL23 scope exclusions. The record shadows bootstrap from native records and earn no independent-evolution credit. Lot configuration loading/getters remain C++ glue with the Rust object decoder, not a whole Rust configuration owner.

The turbo run separately reproduced existing **PL21**: one GetLotProps list differed on 2026-08-01 at lot(14,38), native4/core3, first @2 1F8AE488/1F8AE3F4. That result is preserved; it was not removed or normalized. The required normal-speed follow-up was exact. PL21 remains open in `docs/specs/questions/prop-lotconfig.md`; no new question or guessed fix was added.

## Coordinator action

Add `propmgr_shadow = 1`, `propmgr_shadow_cells = 32`, and `lotcfg_shadow = 1` to the shadows profile in `tools/regress.ps1`. This lane did not access or edit `tools/`, as the clean-room prohibition explicitly covers every part of that directory. The exact normal-speed evidence above supports those profile additions within defined-field scope; keep the turbo PL21 limitation in the record. Coverage maps are untouched; coordinator credits after review.

No game request remains outstanding. Full requests, logs and verdicts are retained under this worktree's ignored `build-runs/game-queue/done/` and `build-runs/game-results/`.
