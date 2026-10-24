| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-house11 (countable house disaster shadow) | Codex / `glue-disasters-house11` | in progress | Inspect shadow and harness; implement deterministic schedule; gate and measure house to 2032 |

## Handoff
- Goal: deterministic scripted disaster house shadow, monthly countable exact summaries through 2032.
- Worktree reused at reserved detached HEAD; no branch changes permitted.
- Done: added opt-in disasters_scripted fire/riot schedule and cumulative monthly dated comparison lines; native gate next.
- Implementation commit: 6345f0625. DS47 filed for observation contracts for short-form disasters; DS28 remains open.
- Gate: qbuild all Rust PASS 7119/0, clippy PASS 0 diagnostics. Native queued as qbuild-20261024-121859-build-ctest; unified exec session 64395 still active (collect this operation; do not duplicate).
- Spec diagnostic: riot leader spawning can sustain the emergency pause. After the current native operation terminates, bound the fixture by removing leaders via occupant manager slot 21 after 200 riot samples. Keep AddRef holds until teardown so the mirror can inspect removed leaders after native releases them; do not force a phase/resume.
- Native request is still outstanding after extended waiting; do not start another check. Current C++ source still has the unbounded schedule; the correction above is NOT implemented yet.
- Correction detail: snapshot/pin leader occupants with Keep into a separate owned set, RemoveOccupant(occ, false, 0), retain the set through the original and mirror ending ticks, DropAll at EndCity. Include cISC4OccupantManager.h. Then gate corrected source and run the prepared request.
- Prepared request: build-runs/house11.req.pending, label house11-fire-riot-01, not submitted yet. build-runs/wait-house11.py waits without qwait.
- No game request outstanding and no DLL/game measurement yet. build-runs/compare-house11.py compares dated mismatch counts against the explicitly named main build-runs/regress-train193-shadows.log.
- Next: collect current gate, implement bounded fixture, final queued gate; then submit prepared house request with train193 switches plus disaster keys, timeout 1800.
- Monthly failures include numbered differences, refused ignition resyncs, unfollowable inputs, missing leaders, call-result differences and guarded-core failures.
- Schedule uses original manager start APIs and waits for natural lifecycle completion, with turbo held off then restored; no forced resume.
- Constraints: no tools/ reads or edits; propose regress profile change in report only. Native verify uses queued qbuild verify. No qwait for game requests.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-disasters-house11-r1-manifest.json.
