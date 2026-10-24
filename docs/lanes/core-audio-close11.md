| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-close11: SC4 audio owner closure | Codex, core-audio-close11 | in progress | Audit spec extensions against core owners, implement and gate |

## Handoff
- Goal: close specified SC4 audio owners in engine/core/src/audio, without ABI/plugin changes.
- Done: extended event-handler §7 owner algorithms implemented (lifecycle, radio/settings/dispatch, construction, alternate automata, modal/mouse, callbacks/torch/destruction). README now maps handler sections and explicitly lists remaining family work.
- Done commit: 45593631c (handler implementation, tests, README, AU45).
- Gate round 1: qbuild all workspace tests 7065 passed/0 failed/0 ignored; clippy fails only owner.rs:695 (7|8|9 must be 7..=9). Failed clippy evidence preserved in build-runs/qbuild-clippy-round1-failed.log.
- Native build/CTest still pending from SAME qbuild all, exec session 20824; queue label qbuild-20261024-110155-build-ctest. Collect actual terminal result before starting another all/verify. Do not duplicate it.
- Focused history: handler run 22 passed before final callback test; initial lifecycle 12 passed/1 failed (test asserted pre-Init state after Init), corrected then 13 passed.
- Formatting: qbuild fmt passed all four absolute owned paths after final test and lint fix.
- Corrected source commit: 4dba7adc7 changes mouse subtype pattern to 7..=9. Focused qbuild clippy session 23487 TERMINATED PASS, exit 0/diagnostics 0. Current qbuild-clippy.log is the successful check; round-1 failure preserved separately.
- Question: AU45 appended to docs/specs/questions/audio.md (completed §7.2 fifth accepted no-op vs §16.12 four-only fixture). Implement completed table literally with comment.
- Question AU46 added after reading §§8.1–8.3: sound-manager focus/reset/enumeration/volume/per-sound update/lifetime bodies are named but not defined. This is a reported spec gap, not a full-owner exemption or closure claim.
- Next: collect pending session 20824 (original all/native), commit results. Rerun all on corrected stable source after original operation terminates (do not duplicate native); then continue remaining owners from README pending rows.
- Relevant added spec: docs/specs/audio-owner-contracts.md, linked from audio.md §15; outlines §§17–21. Existing explicit gaps AU40–44, leaf AU5.
- Entire family NOT closed. Event handler is first milestone only. No merge-queue entry until all specified family behavior is implemented or explicitly spec-open.
- Local q.py failed to launch Bash (CreateFileMapping Win32 error 5); direct Cargo focused tests worked. qbuild all is running independently.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-audio-close11-r1-manifest.json.
