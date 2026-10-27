| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-watercraft31: watercraft method comparisons (engine) | Codex / method-watercraft31 / detached HEAD; reserved branch method-watercraft31; code a1311aca0, 195c6b8cf, main merge 7f61fb182 | READY for bounded 704 B scope; full owner verification incomplete; cargo 8387/0, clippy clean, CTest 144/144; watercraft31-r3-moving-a clean, ref_warnings=0; record shadow 4/4 exact, four value methods exact, WC13 retained; ABI unchanged, no seam reservation | (1) Supervisor publishes detached tip and reviews 704 B method credit. (2) Route existing WC13 to spec writers. (3) Future work binds remaining reached outputs; no whole-owner credit. |

Merged main first (`7f61fb182`), preserving both Watercraft and Span565Probe harness actions. Existing straight-motion adapter, registry schema test and native boat fixture are finished; no new bindings this round.
Final `qbuild all` PASS: Rust 8387/0; clippy 0 diagnostics; CMake 227/227; CTest 144/144. Logs: `build-runs/qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`; owned Rust fmt PASS (1 file, 0 diff lines).
Qualified advancing house-shadows-settings queue run: `watercraft31-r3-moving-a`, clean, exceptions/stalls/RTC/overruns/ref_warnings all 0; 2026-01-27 to 2026-04-01, 189.407 s, 0.33 days/s. [verdict](C:/Users/tlori/FreeCity4/build-runs/game-results/watercraft31-r3-moving-a.verdict).
DLL SHA256 `12373B4D25D62D60119CDA3EDB81C2083CC1BE94E74E3927AFDF1800CE9BAE49`; shared slot 1; initial qwait timeout was queue latency, then the same request completed. No direct fallback or second game request was launched.
Fixture: seaport placement ok=1; three native boats created; all 19 actions completed, including budget window and QueryTool. [log](C:/Users/tlori/FreeCity4/build-runs/game-results/watercraft31-r3-moving-a.log):3772,3976,4645,4713,4879,5575.
Monthly per-method evidence below is cumulative at 2026-04-01, log:5598-5602; every listed method has resyncs=0. Literal ABI identities are emitted for coverage.
```text
address     bytes calls complete mismatches method
0x00566E00     35    97       97          0 FUN_00566e00 (Sign)
0x00567ED0    241   264      264         15 FUN_00567ed0 (AngularProximity; WC13)
0x00567FD0    478    19       19          0 FUN_00567fd0 (DirectionQuantization)
0x0056DC60      6    65       65          0 cSC4Watercraft::GetGZCLSID
0x00568230    185   163      163          0 FUN_00568230 (StraightIntegration)
```
Exact claim is four methods / 704 B only: every specified return/out-argument/changed owner field is compared; these helpers make no collaborator calls. No resyncs, dropped failures or weakened comparisons.
Movement qualified: shutdown straight-motion natural calls=153, compared=153, exact=153, refused/nested/outside=0; plus 16/16 fixture calls. This verifies straight integration, not the full AI/path/state methods (log:6173-6174).
Existing precise [WC13](../specs/questions/watercraft.md) asks for the missing receiver/precondition contract of 00567ED0. All 15 failures remain synthetic zeroed-receiver cases; shutdown native calls 254/254 exact. Item stopped, 241 B uncredited; no new question.
All 75 methods / 38073 B, ranked by observed reach then bytes, with counts and missing outputs: [inventory](../../build-runs/watercraft31-r3-methods.tsv). Reach is capped lower-bound evidence; hooked helpers are marked entry-changed-by-hook, never inferred unreachable.
Unbound reached: 39 methods / 20192 B. Largest: Engine_AIBoatMotion 00571880 (2779 B, entries>=219); LoadDriveParameters 00567310 (1845 B, >=4); DoAnimationTick 00572CF0 (1342 B, >=228); FUN_00569940 (1314 B, >=15); Init 0056DE40 (1274 B, >=8).
Left: 70 unbound methods / 37128 B plus WC13. Complete owner writes and outgoing terrain/RNG/state/model/effect/ferry/stream/QI/lifecycle transcripts remain missing; no complete comparisons or credit claimed for them. Largest sampled-zero methods 0056AF90 (3506 B), 0056A440 (2883 B) are not proved unentered.
Record/getter shadow is 4/4 exact (load plus three months), reachable boats 0 to 3; AI/path output comparisons remain incomplete. Fresh reach artifact: `C:/Users/tlori/FreeCity4/build-runs/game-results/watercraft31-r3-moving-a.reach.tsv`.
Earlier 002051 and r2 held-date/load-only results remain separate; this fresh corrected-source advancing result supersedes neither their failures nor WC13. Coverage-map/replacement lists untouched; coordinator owns credit changes.
Next: land the finished bounded implementation using the full detached HEAD queue tip; future whole-owner verification requires the missing bindings and WC13 specification clarification.
