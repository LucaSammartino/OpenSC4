| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-watercraft31: watercraft method comparisons (engine) | Codex / method-watercraft31 / detached HEAD; reserved branch method-watercraft31; code a1311aca0, 195c6b8cf | WIP; cargo 8256/0, clippy clean, CTest 142/142; 002051 load-only 4/5 value methods exact; no new coverage claim; WC13; ABI unchanged, no seam reservation | (1) Restore advancing game harness and qualify moving boats. (2) Capture monthly evidence/reach and bind remaining outputs. (3) Resolve WC13; coordinator publishes detached commits and reviews helper credit. |

Merged main first: `4541447a0`. Added straight integration binding, actual-registry schema test, and `watercraft auto|x z` scripted native fixture action; no native result is replaced or resynced.
Final corrected-source `qbuild all` PASS: 8256 Rust tests, clippy 0 diagnostics, CTest 142/142; logs `build-runs/qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`. Focused quick: capi 684/0, build 64/64; owned Rust fmt PASS; the earlier all gate predates the schema fix.
Final direct run `002051` (slot 9, house shadows settings): DirectDraw error, held at 2026-01-27, no advancing days/actions/months; fallback clean is NOT a qualified clean verdict/ref-audit result. DLL SHA256 `48D370946E3E9B5D0F633532EB89EA960CBFF9E9087D6DE10F04CBACD2CF3C6C`.
Load-only cumulative comparisons in `C:/Users/tlori/FreeCity4/build-runs/002051.log` (all resyncs=0; these are inplace summaries, NOT monthly method credit):
```text
address     bytes calls complete mismatches method
0x00566E00     35    16       16          0 Sign
0x00567ED0    241    16       16         15 AngularProximity
0x00567FD0    478    16       16          0 DirectionQuantization
0x0056DC60      6    22       22          0 GetGZCLSID
0x00568230    185    16       16          0 StraightIntegration
```
Exact value outputs span 704 bytes (+185 load-fixture bytes); no new coverage credit claimed. Monthly labels now use literal ABI identities FUN_00566e00/FUN_00567ed0/FUN_00567fd0/FUN_00568230 and cSC4Watercraft::GetGZCLSID; fresh monthly credit is unmeasured.
All 75 methods / 38073 bytes, descending by size, with counters/missing observations: [inventory](../../build-runs/watercraft31-r2-methods.tsv); zero unbound counters do not mean zero native entries.
Left: 70 methods / 37128 bytes unbound, plus failing 241-byte WC13; complete runtime writes and outgoing terrain/RNG/state/model/effect/ferry/stream/QI/lifecycle transcripts remain missing.
No usable native reach census or moving-boat fixture; largest unbound: 0056AF90 3506 B, 0056A440 2883 B, Engine_AIBoatMotion 00571880 2779 B. Record/getter load fixture: 4 samples, 3 probes, 856 bytes, mismatches=0; reachable=0.
Earlier r2 direct runs `001023`, `001423` (main build-runs logs) used pre-fix DLL f07a99b7, held on the same date; straight binding rejected its schema. Corrected `002051` installs it and matches 16/16. No later result is substituted for earlier failures.
Shared request `watercraft31-r2-moving-a` timed out after one nine-minute qwait without starting; its pending request was removed before direct fallback. No outstanding request remains.
Fixture prototype/settings: `build-runs/watercraft31-r2-moving.script`, `watercraft31-r2-moving-b.ini`; port creation, boat creation, budget/query actions never ran because the date did not advance.
Existing [WC13](../specs/questions/watercraft.md) remains precise/value-only and unanswered; 15/16 heading mismatches retained; item stopped. No new question. Prior house-g evidence remains in `build-runs/regress-watercraft31-house-g.out`; prior coverage 3029 bytes/36 methods is not fresh r2 credit.
Next: run the corrected DLL through a working advancing shared harness; qualify the script and a seaport/ferry; collect monthly method lines and reach; bind remaining reached methods without excluding failures. WIP queue only; ready/merge-train enqueue withheld.
