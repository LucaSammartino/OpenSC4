| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| drawshadow29: 2D draw-stream oracle (engine) | Codex / drawshadow29 / detached HEAD; code 1101959dd | ready; Rust 7967/7967, clippy clean, CTest 131/131; gen27-r6b clean, refs 0; Update 4/4 and UI 40/40 exact; ABI unchanged; questions none | Coordinator: credit Update and UI DoMessage; notification and whole-row credit withheld. |

- Pilot: cSC4WinMapView (30,448 B); largest implemented candidate reached by the recorded runs.
- House reach: MapView 7/30 methods, 5,574 B, Update=0; build-runs/drawshadow29-house-r2a.reach.tsv.
- Gen27 reach: MapView 13/30 methods, 15,665 B, Update=24, DoMessage=178/32; build-runs/drawshadow29-gen27-r2a.reach.tsv.
- SubBudgetWindow, TreeView, ListBox: 0 reached methods/bytes in both traces.
- Budget action retained from earlier rounds; no new action or window implementation in this final round.
- Reusable bounded Stream/Inputs/context replay records geometry, colours, clips/state, keyed images, text payloads and command events; owner/font/surface adapters must bind their outputs for credit.
- Default-off drawstream_shadow; no C ABI exports changed. Native queries execute once; core draws/messages/commands only record events.
- Final gate (1101959dd), run once: qbuild all PASS; Rust 7967, clippy 0 diagnostics, build 36/36, CTest 131/131; build-runs/qbuild-{test,clippy}.log and fc4verify-{build,ctest}.log:267. Owned Rust fmt PASS.
- Recorder test: draw_stream_oracle PASS, build-runs/fc4verify-ctest.log:66; core oracle tests cover registration once, custom-before-clock order and ignored false results.
- Final game: drawshadow29-gen27-r6b clean through 2026-05-01, ref_warnings=0; C:/Users/tlori/FreeCity4/build-runs/game-results/drawshadow29-gen27-r6b.{verdict,log}; log:10337. Script/close_dialogs under [ini], turbo=1.
- Draw-equivalent Update 0x007A2F60: calls=4 complete=4 mismatches=0 resyncs=0; final log:10120. Map-view spec 4.4 defines its bitmap/presentation output rather than a context argument.
- Draw stream: 100/100 operations exact, complete=4/4; log:10119. Per-draw counts 33/18/31/18, first difference=none; log:8963,8965,9919,9921.
- New binding: renderer slot 32 registration and custom-view slot 5; compare owner/dynamic state, parent/child/rectangle queries, null overlay, custom entry/exit, clock and child release. Same borrowed objects; no second foreign execution.
- Notification DoMessage 0x007A54D0: calls=38 complete=4 mismatches=0 resyncs=0; log:10121. Activation/deactivation and animation-tick collaborator profiles remain incomplete; no function credit.
- UI DoMessage 0x007A56E0: calls=40 complete=40 mismatches=0 resyncs=0; log:10122. Inherited handler is a recorded/replayed boundary with the same O+8/message and its result/state; button commands remain unbound.
- Explained partial difference: activation owner O+0x14 refcount native=8/core=7, complete=0 (log:1604); map-view §1 registers the animation target, animation-tick-manager §4.1 AddRefs it, widget-toolkit §1 locates the inherited refcount. Replay deliberately omits that collaborator effect.
- Remaining/uncredited: nonzero-source Update orchestration, ticks, selection/legend/panel commands and complete font/surface widget adapters. Raster helpers have calls=0 (log:10123-10124); no raster-helper or whole-row credit claimed.
- Diagnostic drawshadow29-gen27-r6a was clean, refs 0, but Update only 2/4 complete; final r6b supersedes that evidence after registration/custom binding.
- Questions: none. Next: coordinator lands the ready tip and applies only the measured Update/UI method credit; this final round starts no further implementation.
