| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-effectsres30: effects resource; kind engine | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/replace-effectsres30; detached HEAD; reserved branch replace-effectsres30 | wip; code eb95b695b; ABI unchanged; cargo 8379 PASS, clippy 0, CTest 143/143; startup corruption reproduced; EF28 open in docs/specs/questions/effects.md; prior native shadow 4/4 exact per mode | 1. Resolve EF28 native audio/view lifecycle failure. 2. Fix and rerun full-list acceptance before enabling effectsres. |

- **NOT ready for the replace list.** cSC4EffectsResource (28,498 B) credit stays withheld; no ready enqueue.
- Merged main first (`bca6ee6ca`); prior replace-effectsres28/EF26 nonempty IO and EF27 implementation were already present and retained.
- R3 change is test-only: full train288 house-list registration with effectsres enabled/disabled, effects/regionalcity/gzfw membership, unique enumeration, no resource creation during enumeration. No runtime workaround was added.
- R3 logs/verdicts: `C:/Users/tlori/FreeCity4/build-runs/game-results/effectsres30-r3-<suffix>.log` / `.verdict`; exact shared requests in sibling `game-queue/done/`.
- R3 game DLL SHA256 `5ffd9d89b6428de369ebe045b1a4ef7a6ce589cebb545faf9ba066eeec768ac6`; final gate relink SHA256 `3050d732765bf448ee3b4470947b4e5b365af86c50719149b6e9fa38982c51f8` (same runtime source; not game-qualified).
- **(a) BLOCKED:** `house-on1` full house plus key reproduces corrupt at 2026-01-27 / 0.031 s, foreign_exceptions=1, ref_warnings=0; +0xE2C06 reading null (log 10988). No effects resource factory request/construction before the fault.
- **(a) Passing repeat, no fix claim:** unchanged-source `house-on2` clean to **2032-01-01**, foreign_exceptions/ref_warnings=0, 158.969 s. Key-off `house-off1` clean to **2032-01-01**, 172.500 s; `house-off2`/`house-off3` clean to 2026-04-01, all ref=0.
- EF28 records the native stack: audio view-level filter (audio-filters section 2), CanHandle/HandleCell/audio tick; null native 3D view. Failed on1 constructs two effects managers; passing on2/off1 construct one. Causality remains unresolved; no speculative global/timing patch.
- **(b) Historical R2 evidence retained, not remeasured after main merge:** `effectsres30-r2-shadowoff1`/`shadowon1` clean/ref=0, **4/4 exact name-membership samples each**, 13,860 queries each, mismatches=0/refusal=0/resyncs=0; payload/history excluded per EF22.
- **(b) Historical R2 saves:** `offsave3`/`onsave4` clean/ref=0, SaveCity(full) ok=1; `offload1`/`onload2` reverse-key loads clean/ref=0 to **2026-05-01**; `offcontrol1` same-save/key-off control clean. Export hashes and inputs remain in those verdicts.
- **(c) Historical R2 s0-grow:** `s0off1`/`s0on1` clean/ref=0 to 2028-01-30 / 2028-01-28, **216/216 actions each**. No R3 script rerun while (a) remains blocked.
- **(c) Historical R2 s1-grow:** `s1off1` corrupt at native +0xE2C06/ref=0 before advancement; `s1on1` clean/ref=0 to 2028-02-01, **216/216 actions**. Earlier code; not proof that current startup regression is fixed.
- Retained R2 failed attempts: `onsave3` plugin corrupt at native +0x1DF76E/no export; `onload1` corrupt at native +0xE2C06; setup rejections/failed starts supply no acceptance evidence. Successful retries did not establish causes.
- **(d) R3 qbuild all PASS once:** Rust **8379/8379**, clippy **0 diagnostics**, native build **30/30 edges**, CTest **143/143** including expanded full-house registration tests.
- Gate logs: WT `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; verdict `build-results/qbuild-20261027-025246-build-ctest.verdict`. No Rust edits; fmt n/a.
- No C ABI exports/version, coverage files, regress replace list, native audio/view globals, or game files changed.
- Next: obtain EF28's lifecycle/first-use contract (or coordinator routing to the audio/view owner), implement a justified regression fix, then repeat (a)-(c) on the final DLL. Supervisor publishes detached commits; queue state wip only.
