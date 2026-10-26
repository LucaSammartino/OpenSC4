| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-watercraft31: watercraft method comparisons (engine) | Codex / method-watercraft31 / detached HEAD; reserved branch method-watercraft31 | WIP; +6 verified bytes; cargo 8207/0, clippy clean, CTest 142/142; house-g clean/ref=0; 3/4 methods exact; WC13; ABI unchanged, no seam reservation | (1) Resolve WC13. (2) Obtain moving-boat/entry fixture and bind remaining methods. (3) Coordinator: question index and anonymous-helper credit identities. |

Added four value-only bindings, all-call comparison for this owner, strict getter bits, and monthly method evidence. No native result is changed or resynced.
Gate: final `qbuild all` PASS, 8207 Rust tests, clippy 0 diagnostics, CTest 142/142; owned Rust fmt PASS. Logs: `build-runs/qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`.
Final house: `regress-watercraft31-house-g-shadows` clean to 2026-04-01, ref_warnings=0, foreign_exceptions=0; record/getter shadow 3/3, reachable boats=0 throughout; DLL SHA256 prefix 162f0afb.
Cumulative method counters on 2026-04-01 (all resyncs=0):
```text
address     bytes calls complete mismatches method
0x00566E00     35    16       16          0 Sign
0x00567ED0    241    16       16         15 AngularProximity
0x00567FD0    478    16       16          0 DirectionQuantization
0x0056DC60      6    34       34          0 GetGZCLSID
```
Exact outputs span 519 bytes; coverage credits only GetGZCLSID (6). Sign/direction have literal ABI names FUN_00566e00/FUN_00567fd0, unlike their logged descriptive names; neither is claimed verified.
Measured coverage: 3023/35 method bytes/functions initially -> 3029/36; see `build-runs/coverage-final.txt` and unmodified captured command output `build-runs/regress-watercraft31-house-g.out`.
All 75 methods / 38073 bytes, sorted by size, are in [the inventory](../../build-runs/watercraft31-methods.tsv); shadow_calls=0 means unbound, not zero native entries. No usable entry census was produced.
Left: 71 methods / 37313 bytes unbound, plus the failing 241-byte angular helper; missing native movement/path fixture, complete runtime writes, and outgoing terrain/RNG/state/model/effect/ferry transcripts. Codec/QI/lifecycle-specific missing channels are in the inventory.
Largest unbound: 0x0056AF90 3506 B; 0x0056A440 2883 B; 0x00571880 Engine_AIBoatMotion 2779 B. Reach-based ranking remains unavailable.
Earlier evidence retained: house-c clean to 2027-01-28 but class-ID 70/66 before sampling fix; census-b stalled at deferred save; census-d clean early at 2026-04-17, seaport site occupied; census-e no-load; queued census-a timed out and was removed before direct fallback.
Question: [WC13](../specs/questions/watercraft.md), native heading writes disagree on 15/16 fixtures; item stopped, all mismatches retained. Coordinator must regenerate the question index.
Game logs: `C:/Users/tlori/FreeCity4/build-runs/game-results/regress-watercraft31-house-{c,g}-shadows.log` and main `build-runs/watercraft31-census-{b,d}.log`; value-only cases are under this worktree's `build-runs/compare-cases/`.
Code commits: 7d067a6a5, addbdf407, 65aeb7a49, 3ef9c6624. First full gate's clippy lifetime failure is distinct in `build-runs/first-gate-clippy.txt`; corrected gates passed. No public C ABI changes.
Held WIP: no ready enqueue. Next: resolve WC13, qualify a moving seaport/ferry fixture, capture entry counts and remaining outputs, then repeat house checks and coverage; coordinator reviews helper identities and publishes detached commits.
