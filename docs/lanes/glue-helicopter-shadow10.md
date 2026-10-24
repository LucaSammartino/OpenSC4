| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-helicopter-shadow10: partial helicopter metadata shadow | Codex, glue-helicopter-shadow10 (reserved detached HEAD) | in progress: round 2 merge and ID fix-up | Re-gate merged source; retain clean helicopter 73/73 verdict; resolve AF11 adapter/live-spawn contract later. |

## Handoff

- Goal: round 2 fix-up only; merge main, unique AF/run IDs, re-gate, READY queue.
- Done: main merge conflicts resolved additively; helicopter adapter question is AF11; run 10480 becomes max+1 = 10481.
- Next: regenerate question index, commit merge, run stable-source qbuild all (test, clippy, verify).
- Prior clean acceptance: helicopter10-r2-20261024, helicopter 73/73 through 2032, ref_warnings=0. No game request outstanding.
- Main AF10 belongs to airplane non-finite physics; preserve it. Detached HEAD must stay detached.
- Full report below retains earlier measured evidence; replace this handoff with final round 2 results.

## Delivered scope

Default-off `helicopter_shadow` (environment fallback `FC4_HELICOPTER_SHADOW`) adds an independent metadata comparison to the automata observer, also usable without the other automata shadows. One dated `helicopter shadow YYYY-MM-DD: ... mismatches=N` summary appears at city load and once per later month. Every compared field contributes separately to mismatches.

A fresh Rust `helicopter::Helicopter` writes the expected record through the additive `fc4_helicopter_shadow_fixture` export. No native output enters that owner. Comparisons cover class identity and four nested u16 versions: helicopter 1, aircraft 1, automaton 8, occupant 4. All remaining bytes are excluded; this is not a record roundtrip or rotor simulation.

The adapter scans the aircraft manager's saved references with retained capture pins. This does not prove complete live enumeration. With no helicopter in those references, a private COM factory object supplies metadata only: never Init, tick, manager insertion or a claim of live city spawning. Factory acquisition is audited before FC4_RELEASE_CREATED; capture acquisitions are separately balanced. No hardware debug registers are armed. The simulator date getter supplies the load date before PublishedJulian is populated.

## Open owner coverage and questions

New **AF11**, [questions/aircraft.md](../specs/questions/aircraft.md), requests native adapter IDs/signatures/ownership, complete enumeration, independent reconstruction inputs and a safe deterministic live-spawn scenario/group/exemplar contract. No native signature or group ID was guessed. Only `Helicopter::class_id` and the four version fields of `write` are compared. Uncompared owner methods/outputs:

- Drive-record import/ancestor selection, all thirteen load_drive_parameters fields and presets (AF11 native adapter; AF9/AF4 defaults).
- All remaining serialized base/goal/home fields, read, init_placement, phase getters and state reconstruction; native outputs are never seeded into expected values.
- get_altitude, clip_to_world, set_helicopter_state, pursue_goal, push_mission_loiter, chase geometry and state transitions.
- physics_init, init_instance, next_frame, yaw/pitch/roll/slide, MoveXZ/MoveY and integration (AF2/AF4).
- AI reset/hover/move-XZ/building-lift, complete goal/chase/dispatch handling and messages (AF3/AF6).
- Rotor sounds, spotlight, takeover, player drive tick/commands and weapons (AF4/AF5).

**READY is for measured metadata work, not full cSC4Helicopter verification. AF2-AF6/AF9/AF11 stay uncompared.** Regression profiles and coverage maps were not edited by this lane; full owner coverage remains open. Round 2 explicitly authorizes merging main and running the question index generator. Coordinator review decides any profile append after landing.

## Gate

Final implementation source **f23e231fb** (initial d3f449106). Focused C ABI test passed: exact versions, null/short stream rejection and write-failure propagation. Explicit owned Rust-file fmt passed; diff check clean. Stable corrected-source `qbuild all` PASS: **7012 tests passed, 0 failed, 0 ignored; clippy 0 diagnostics**. The same operation handed native verification to queue **qbuild-20261024-103348-build-ctest**, terminal PASS: **47/47 build edges, CTest 80/80** (95.9 s overall, CTest 88.1 s). No duplicate verify request. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log.

The pre-fix gate also passed 7012 Rust tests and CTest 80/80 (qbuild-20261024-101914-build-ctest); the game diagnostic below justified the corrected-source repeat. Its tree-change note was a lane-doc commit only. Initial relative fmt paths failed; absolute paths passed. q.py Bash launch hit Win32 access error 5; direct focused cargo passed. One additive C ABI export; **ABI changed**, version not bumped; generated abi_entries.inc untouched.

## Game results

**`helicopter 73/73`** — **helicopter10-r2-20261024**, [run 10481](../runs/helicopter10-r2-20261024-10481.md). Main build-runs/regress-train189-shadows.ini house switches plus helicopter_shadow=1; Konradshohe, turbo through **2032-01-01**, full quit/shutdown. Clean: **foreign_exceptions=0, watchdog_stalls=0, ref_warnings=0, rtc=0, overruns=0**. City time 827.937 s / wall 844 s; DLL SHA256 prefix c6befc64. First sample **2026-01-27**, last **2032-01-01**: 73 unique monthly summaries, **365 fields exact**, zero resynced lines. Every line has reachable=0, fixtures=1: no saved-manager helicopter observed; every sample used the private metadata fixture, not a live city spawn. Shutdown reports 73/73 exact. No turbo difference, so no normal-speed repeat required. Counts come from retained logs and the harness summary; tools/coverage.py cannot be run in this clean-room lane.

Failed evidence: **helicopter10-r1-20261024**, [run 10479](../runs/helicopter10-r1-20261024-10479.md): `helicopter 13/13`, stalled at 2027-02-20, foreign exceptions 0, reference warnings 39, one recovered watchdog stall (6469 ms), no clean shutdown. Warnings identified missing factory acquisition audit bookkeeping; that and the missing load-date observation were fixed in f23e231fb. R1 is not acceptance evidence. Both unique logs/verdicts remain under build-runs/game-results/.

Requests were sequential. The blocking FileSystemWatcher wait used no qwait or polling; its nine-minute timeout resumed the same r2 verdict wait without resubmission. The initial r1 wait needed its results directory created. No game/build request remains outstanding.

## Next

Resolve AF11 and existing AF2-AF6/AF9 before claiming native parameter/full-record/goal or live-spawn coverage. Keep the metadata key default-off; coordinator reviews the partial READY entry and any eventual profile append.
