| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| drawshadow29: 2D draw-stream oracle (engine) | Codex / drawshadow29 / detached HEAD; code 68cf4de64 | r5 continue; Rust 7966/7966, clippy clean, CTest 131/131; gen27-r5a clean; UI verified +2540 B; ABI unchanged; questions none | Complete remaining Draw profiles; Update and notification function credit withheld; queue wip. |

- Pilot: cSC4WinMapView (30,448 B); largest implemented candidate reached by the recorded runs.
- House reach: MapView 7/30 methods, 5,574 B, Update=0; build-runs/drawshadow29-house-r2a.reach.tsv.
- Gen27 reach: MapView 13/30 methods, 15,665 B, Update=24, DoMessage=178/32; build-runs/drawshadow29-gen27-r2a.reach.tsv.
- SubBudgetWindow, TreeView, ListBox: 0 reached methods/bytes in both traces.
- Budget action retained; discard its unspecified native return. Parser and ret-channel fixes pass at fc4verify-ctest.log:117,155; prior budget-r3b game clean, ref_warnings=0 (main panel).
- Reusable bounded Stream/Inputs/context replay records geometry, colours, clips/state, keyed images, text payloads and command events; owner/font/surface adapters must bind their outputs for credit.
- Default-off drawstream_shadow; no C ABI exports changed. Native queries execute once; core draws/messages/commands only record events.
- Final gate (68cf4de64): qbuild all PASS, Rust 7966, clippy 0 diagnostics, build 37/37, CTest 131/131; build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log. Owned Rust fmt PASS.
- Final game: drawshadow29-gen27-r5a clean through 2026-05-01, ref_warnings=0; main build-runs/game-results/drawshadow29-gen27-r5a.{verdict,log}; log:10337. Script and close_dialogs under [ini], parallel=1.
- Update 0x007A2F60: calls=4 complete=2 mismatches=0 resyncs=0; log:10111. Source-zero/no-custom/registered-overlay profile compares all owner/dynamic state, parent/child/rectangle queries, null overlay, clock and child release.
- Draw stream: 38/38 recorded operations exact; log:10110. Two complete draws each have 18 operations; first difference=none at log:8965,9928. Two other draws remain incomplete.
- Notification DoMessage 0x007A54D0: calls=51 complete=4 mismatches=0 resyncs=0; log:10112. Activation/deactivation and animation-tick collaborator profiles remain incomplete.
- UI DoMessage 0x007A56E0: calls=40 complete=40 mismatches=0 resyncs=0; log:10113. Inherited handler is a recorded/replayed boundary with the same O+8/message and its result/state; button commands remain unbound.
- Explained partial difference: activation owner O+0x14 refcount native=8/core=7, complete=0 (log:1604); map-view §1 registers the animation target, animation-tick-manager §4.1 AddRefs it, widget-toolkit §1 locates the inherited refcount. Replay deliberately omits that collaborator effect.
- Remaining: nonzero-source Update orchestration, overlay registration/custom-view calls, ticks, selection/legend/panel commands and complete font/surface widget adapters. Raster helper transport exists but calls=0 in this run; no raster-helper or whole-row credit claimed.
- Enqueue check accepted only UI DoMessage (+2540 B): function credit requires all calls complete. Update 2/4 and notifications 4/51 do not qualify. Queue downgraded to wip; no ready claim.
- Questions: none. Next: r6 diagnose the two unbound source-zero Update entries (log before/after +0x140/+0x141 and custom pointer), bind disabled-overlay/registration profiles, then rerun gate/game; tick/registration binding is also needed for notification credit.
