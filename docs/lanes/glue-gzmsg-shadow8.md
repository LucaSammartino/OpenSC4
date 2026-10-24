| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-shadow8 (queue manager shadow) | Codex / `glue-gzmsg-shadow8` | READY: default-off diagnostic; main merged; full gate PASS; shadow unverified | Land diagnostic; separate RE lane answers GZ11/GZ12. |

## Round 2 result

READY to land as a default-off diagnostic, as explicitly requested by the coordinator. The shadow remains unverified; no coverage credit is claimed.

- Merged main (03482006f) in 808a5612d. Resolved `engine/capi/src/lib.rs` and `engine/plugin/FC4Config.cpp` additively: retained main's `scres_file_shadow` and the lane's `gzmsg_shadow` module/key entries. Main's public exports, including nettool exports, remain intact. No ABI version bump or generated ABI list edits.
- Final stable-source `qbuild all`: **6962 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 48/48 edges; CTest 78/78; qbuild PASS**. Native request `qbuild-20261024-064821-build-ctest` collected to terminal PASS. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; terminal verdict in `build-runs/build-results/qbuild-20261024-064821-build-ctest.verdict`.
- First round-2 check `qbuild-20261024-064225-build-ctest` passed Rust/clippy and native build 91/91, but CTest was 77/78: stale `capi_smoke` object compiled ABI 95 from train-integration while current header/Rust both declare 96. It relinked without recompiling that object. Invalidated exactly that compiler artifact with Ninja's clean rule and repeated the full check; no source change or ABI bump was needed. Failed evidence preserved in `build-runs/round2-failed-fc4verify-{build,ctest}.log` and the unique failed request's verdict.
- No new game run: merge conflict resolutions only retain existing module/config declarations and do not change diagnostic behavior. Existing completed labels `gzmsg-shadow8-r1-2032` and `gzmsg-shadow8-r2-2032` remain clean with ref_warnings=0; gzmsg 0/73. No requests outstanding.
- No new questions. Existing GZ11/GZ12 in `docs/specs/questions/framework-gz-services.md` remain for the separate RE lane.

Prior implementation and measurements follow below.

## Result

Native state could not be captured at all. Both real-city turbo runs produced **`gzmsg 0/73`**, not the required 73/73 for a verified shadow. No coverage-map/shadow verification credit is requested. There are no outstanding game requests. Round 2 authorizes landing this default-off diagnostic while its questions remain open.

Implemented `FC4GZMsgShadow.*`, default-off through `gzmsg_shadow = 1`, with load/month/shutdown observer hooks, a known config key, and a private static-link Rust transport using the production `QueueManager2`. The diagnostic's explicit domain is IDs 0..255, service ID 0x64FA8434, IID 0x652294BC, and UINT32_MAX (259 probes). Its intended comparison is lookup membership, Boolean return and numeric queue identity against an immutable load baseline. It never resyncs, mutates native queues, registers notifications, posts messages, or substitutes engine behavior. Tokens are numeric only; all acquired native references are released within Capture. No C ABI DLL exports or ABI version changed.

**GZ12 blocks even that bounded comparison:** the final run reports `framework=1 lookup=0 pointer=0 service=64FA8434 iid=652294BC` at load and every month. Missing capture is counted as mismatches=1; it is never passed as an exact empty map. The SDK's EnumSystemServices explicitly leaves its callback signature unknown; no fallback signature or object layout was invented. **GZ11** independently blocks complete registry enumeration, queue contents, pending/deferred posts, notification registrations and mutation replay. Both questions are appended to [framework-gz-services questions](../specs/questions/framework-gz-services.md); the coordinator must regenerate the question index using its permitted tooling.

## Verification

- Implementation `488be884c`; diagnostic/config correction `9545111e0`. Focused tests: 2/2, including insertion/removal/identity/false-return differences and transport bounds. Explicit owned-file fmt passed.
- Final stable-source `qbuild all`: **6940 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native build 206/206 edges; CTest 78/78**. Coordinator native request `qbuild-20261024-061652-build-ctest` collected to terminal PASS. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No source changes during the final gate.
- Initial all gate also passed: Rust 6940/0, clippy clean, native 250/250 edges, CTest 78/78. It noted documentation-only tree changes. The subsequent full gate was justified by the first game's config-key warning and added acquisition diagnostics; no redundant native check was started.
- Initial relative-path fmt failure was corrected with an absolute path. q.py failed due sandbox Bash CreateFileMapping; its evidence remains in `build-runs/q/20261024-060422-11764.log`. Direct focused Cargo succeeded.

| Run record | Queue label | Exact dated lines | End date | Shutdown safety | Resyncs |
|---|---|---|---|---|---|
| [434](../runs/gzmsg-shadow8-r1-2032-454.md) | gzmsg-shadow8-r1-2032 | gzmsg 0/73 | 2032-01-08 | clean; foreign_exceptions=0; ref_warnings=0 | 0 |
| [435](../runs/gzmsg-shadow8-r2-2032-455.md) | gzmsg-shadow8-r2-2032 | gzmsg 0/73 | 2032-01-14 | clean; foreign_exceptions=0; ref_warnings=0 | 0 |

Each run has one load and 72 monthly samples through 2032-01-01; all are unavailable captures with probes=0, baseline=0, mismatches=1. Final run: 35.125 real seconds, 56 s wall, DLL SHA256 prefix `647e4899`, zero other WARNING lines. The first run had three unknown-key warnings, fixed in final source. No normal-speed comparison is required to explain this gap: acquisition already fails at load, before turbo advancement. Full logs/verdicts remain under `build-runs/game-results/<label>.{log,verdict}`. Counts were independently extracted from dated log lines; tools/coverage.py was neither read nor run.

## Coverage-map owner-row disposition

Owner names below identify the relevant tools/coverage-map.tsv row groups from the brief. That forbidden file was neither read nor edited; the coordinator can map its exact row identifiers. **No rows are wholly compared.**

| Owner / specified entries | Wholly compared rows | Remaining gap |
|---|---|---|
| cGZMessageQueueManager2, section 3 GetQueue slot 3 | None | Bounded lookup diagnostic implemented but never reached: GZ12 acquisition failure; GZ11 prevents full ID enumeration and live mutation replay. |
| cGZMessageQueueManager2, QI/base, Add/Create/Remove/RemoveAll and destructor | None | No native state captured; read-only sampling does not exercise mutations, lifecycle or reference-count traces. Existing GZ8-GZ10 also remain open. |
| cGZMessageQueue2, section 2 | None | No queue acquired; no ordered message contents, equality, blocking signal or ownership traces compared. |
| cGZMessageServer2, section 4 | None | Pending/deferred posts and notification registrations have no specified read-only capture or complete observation hook (GZ11). These are server state, not queue-manager state. |
| cGZMessageServer, section 4A | None | Legacy state/replay not implemented by this diagnostic. |

## Next steps

1. Answer GZ12 with a supported existing-manager acquisition/availability/ownership contract. Do not create or insert a native service merely to make the shadow pass.
2. Answer GZ11 with complete read-only state or mutation observation contracts; widen beyond the explicit diagnostic domain before requesting owner-row credit.
3. Resume the shadow, rerun the gate after source changes, then obtain 73/73 real-city exact samples. Until then keep the shadow default-off. The diagnostic is READY to land per the round 2 instruction; verification remains incomplete. tools/regress.ps1, tools/coverage-map.tsv and tools/coverage-shadows.tsv were not touched; no successful shadow exists to add to a profile.
