| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-crash6: owners capture reference crash | Codex / `glue-automata-crash6` (engine; detached HEAD) | ready; gate a2c13e2af Rust 6888/0, clippy clean, CTest 73/73; runs 387-389 clean/ref0; automata and owners each 219/219 exact samples | 1. Land queued tip. 2. Add automata_owners_shadow to the house profile. |

## Changes

204ea940d fixes capture replay of temporary or adjusted native interfaces: automata captures retain QI-normalized serializables through replay, reject class/ID collisions before replay QI, and audit the replay references. Other capture users retain their existing default borrowing policy. The owners shadow remains default-off.

Round 2 validation found that deleting WriteCapture copies also suppressed the move assignment used by MySims' SimCapture. 208aa2159 adds explicit ownership-transferring move operations. The native DLL seam now destroys moved-from captures before replaying a writer-released temporary. Its regression covers adjusted interfaces, duplicate/null references, unknown IDs, class collisions, balanced lifetime, and moved-from destruction.

## Validation

- Initial source 204ea940d: Rust 6881 passed and clippy clean, but native requests `qbuild-20261024-023540-build-ctest` and `qbuild-20261024-023814-build-ctest` failed with C2280 at FC4MySimsShadow.cpp:1189. Both failed verdicts remain in `build-runs/build-results/`; these failures are not credited as passing checks.
- Corrected source 208aa2159: full all passed Rust 6881/0, clippy diagnostics=0, native build 208/208, CTest 73/73 (`qbuild-20261024-024340-build-ctest`). The separately requested queued native verify passed build 46/46 and CTest 73/73 (`qbuild-20261024-024436-build-ctest`).
- Required main merge: a2c13e2af incorporates main 760cf23fdcf03420d3678f829ed848f93ec933eb cleanly. FC4WriteCapture.cpp/.h and automata plugin code did not change. Main added Rust effects changes, so the full all gate was repeated on the integrated source.
- Full all after the effects merge: Rust 6888 passed/0 failed/0 ignored (summed across test-result lines), clippy diagnostics=0, native build 47/47, CTest 73/73, including automata_capture_test (`qbuild-20261024-030223-build-ctest`). Collected to terminal PASS. The tree-change note was the lane-record-only 0bb1527f7 commit; engine source stayed identical to a2c13e2af.
- The final mandated prequeue merge subsequently incorporated main 9b5c10647129e09af32b8f2dac03025d9f4efdcc, including the newly landed traffic lane. FC4WriteCapture.cpp/.h and FC4AutomataShadow.cpp/.h remained unchanged, so the round-two conditional capture/native rerun was not triggered. The measured gate above precedes this final main merge; no additional full-gate or game claim is made for the imported traffic changes.

Full build logs are in `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, and `build-runs/fc4verify-ctest.log`. Unique native request verdicts are retained in `build-runs/build-results/`.

## Real-city results

All three unique requests enabled automata_shadow and automata_owners_shadow, used turbo to end_date 2032-01-01, and completed clean with ref_warnings=0. Each shadow was exact on 72/72 monthly samples and 73/73 including load, for 219/219 exact samples per shadow across all three runs. Resynced lines, owner capture/roundtrip/unsupported diagnostics, foreign exceptions, watchdog stalls, rtc, overruns, Rust panics and other WARNING lines were all zero.

| Run | Label | Final date | Real seconds | Automata bytes compared |
|---|---|---|---|---|
| [387](../runs/automata-crash6-r2-2032-20261024-a.md) | automata-crash6-r2-2032-20261024-a | 2032-01-01 | 49.422 | 782013 |
| [388](../runs/automata-crash6-r2-2032-20261024-b.md) | automata-crash6-r2-2032-20261024-b | 2032-01-01 | 43.547 | 776199 |
| [389](../runs/automata-crash6-r2-2032-20261024-c.md) | automata-crash6-r2-2032-20261024-c | 2032-01-02 | 45.109 | 779584 |

The game-tested DLL SHA256 was ceebe86d4684c38fc31c2d0730b4591306f2885ece17663e53902c815e5b5484. These runs preceded the mandated main merges; capture/automata plugin source was unchanged by both merges. Full logs and verdicts remain under `build-runs/game-results/` with these exact labels. Every wait collected the original operation; the event-based waiter accepted existing completed verdicts and used no qwait. No requests remain outstanding. Run numbers were moved to 387-389 after the final merge revealed concurrent traffic and train172 records; filenames remain label-only.

## Questions and next steps

No questions filed and no C ABI export changes. This validates capture/replay tooling; it adds no replacement-owner claim. The supervisor should publish the detached HEAD to the reserved lane branch and land the READY queue tip. The coordinator can then add automata_owners_shadow to the house profile; that tooling change is outside this lane's clean-room scope.
