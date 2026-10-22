| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-application4: portable application spans, verification round 2 | Codex / `core-application4` (engine lane, reserved detached HEAD) | ready for bounded portable scope: Rust 5064 passed/0 failed/0 ignored (21 result lines), application 25/25, fmt 5/5, clippy clean, CMake build passed, CTest 68/68; ABI unchanged; no game requests; AP6-AP9 retained | (1) Land the reserved lane tip. (2) Resolve AP3-AP9 and independently assigned owners before claiming application integration or replacement coverage. |

## Verification and source correspondence

Resumed `35a8ef64`. Round 2 made no production source corrections: the published
contract audit and actual checks found none necessary. Production Rust remains
as committed in `a7d4967c`; the per-method README last changed in `6269658c`.
Earlier implementation/test milestones are `2ddc32c8` and `71c3daa9`.

Exactly one new full `qbuild all` ran, with no overlapping baseline/quick/all
and no engine edits. Its supervisor recorded source head
`e0f9fb1973e6e617a7a2fef058eeccc15572890c`, engine tree
`17997eb8d4b21bceaed53cbe89cf070c4fa31b77`, and identical before/after SHA256
maps for tracked engine files. It ran 22:09:39-22:48:25 BST on 2026-10-22;
fmt and all exited 0. The final checked HEAD was `6fc7141f`.

Measured final results:

- Formatting: all five application Rust files passed. No mass reformat of the
  existing shared module list was made.
- Workspace Rust: all 21 result lines parsed with suite attribution;
  **5064 passed, 0 failed, 0 ignored, 0 measured, 0 filtered**. All **25 application
  tests passed**. ANSI escapes were stripped for parsing; raw logs were retained.
- Clippy: `--workspace --all-targets -- -D warnings`, exit 0, zero diagnostics;
  finished in 10m23s.
- CMake: fresh configure and release/native build passed, 4/4 edges, 788.2s.
  Generated ABI entries remained unchanged (185 interfaces).
- Full CTest: **68/68 passed**, 0 failed, 459.60s. Its Cargo workspace test is
  included among those 68 cases and is not added to the separate Rust total.
- Every required final log is fresh, with zero parsed diagnostics and zero NUL
  bytes. Canonicalize-path environment warnings remain visible in the raw logs.

`fc4verify` reports "the tree changed during the run" because milestone lane
documentation commits advanced HEAD. No engine source changed: the supervisor's
before/after file hashes agree, and `git diff e0f9fb19 HEAD -- engine` is empty.
Subsequent report commits likewise change documentation only. This establishes
the correspondence between checked source and the landing tip.

Raw final logs, every parsed Rust result and CTest case, log hashes/freshness,
and all readiness predicates are retained in
`build-runs/application-r2-gate/{source.json,result.json,summary.json}` and the
copied qbuild/fc4verify logs there. `summary.json` reports all checks true and
`ready: true`. The ordinary step logs also remain in `build-runs/`.

## Earlier artifacts and supervision

Before submitting the full gate, process metadata inspection found no live
wrapper, orphan or delegated compiler attributable to this worktree. Recorded
old PIDs 21192/27000/21876 were absent; old session 6992 was not treated as proof
of liveness. Actual old logs and audit metadata were preserved under
`build-runs/application-r2-interrupted` before the wrapper replaced step logs.

The old all stopped mid-automata_records. Old clippy/CTest were baseline logs;
that baseline had Rust 5039/0 and CTest 67/68 after source editing overlapped its
Cargo check. Earlier overlapping quicks contained NUL/stale errors. None of
those artifacts supplies final source evidence or final counts above.

The first round-2 supervisor stopped at a fmt input-path error before invoking
all. Its unchanged-source result is retained separately under
`build-runs/application-r2-final`; absolute fmt inputs corrected the invocation.
The real full gate was supervised by `verify-application-r2.py` (supervisor
26328, qbuild 36964). Process metadata also attributed native/cache work and the
live full CTest; it advanced normally. Wait-window timeouts did not fail or
restart the gate. Audit, Rust, clippy and native-build milestones were committed
as `bd924120`, `162a99d1`, `94f02603` and `6fc7141f` respectively.

## Published implementation and withheld scope

`engine/core/src/application/README.md` retains the per-method map for the
primary vtable, cISC4App slot groups and published sections 1-10. Implemented
portable spans include service/director ordering and temporary-reference
discipline, lifecycle ordering around missing owners, both distinct Init
failures, message routing, focus pause, strict idle timers, command option
effects, resolution rules, and byte-path/lazy-directory behavior.

The 25 tests assert literal operation traces, failure stops, created versus
borrowed references, timer edges, parser effects and persistent owner blocking.
Tests acknowledge boundaries only to inspect subsequent published spans.
Acknowledgment does not implement a missing body. Host supplies primitive
operations, never a fabricated final result for an app-owned method. The v1
quit-request boundary still means RequestQuit(false, false).

All 26 unsupported continuations remain explicit:

- Startup: BuildVersion, ResolveUserDataDirectory, RestartWait,
  FindAndPossiblyFixIniFiles, CursorAndOsSettings, PluginDateChecks,
  AddCodePlugins, OpenLogs, FindAndRegisterLanguages, PreferencesChanged.
- Init/post-init: DriverConfiguration, SetupResources, LanguageManagerHookup,
  RenderPropertyVector, SlOnAppInit1, InitAlphaSystem, FindTestScript.
- Persistence/shutdown/quit: SavePreferences, ShutdownMessageServer1,
  ShutdownHelperOrder, QuitService, CloseCity, RequestQuit, CityObliteration.
- Validation: ResolutionAcceptance and UserDirectoryAcceptance; neither commits
  guessed preference/path acceptance.

AP3 aliases/flag, AP4 interface/primary-method identity and AP5 restart,
configuration, shutdown and quit-service gaps remain. AP6-AP9 remain filed in
`docs/specs/questions/application.md`: startup/failure/freeing, path append,
numeric/parser/validation and cheat ambiguities. No new question was filed in
round 2, and none is declared answered.

Section 12 remains withheld: resource/font/window setup, code plugins and
dynamic libraries, preferences read/save/change bodies, DB callbacks,
LoadRegistry, city save-path/scoped-object/unknown slots, residual app inventory,
and cWinSC4App, COM-director bodies, cSC4Preferences, cSC4GameDataRegistry and
cSC4AutoGamePause. Section 11 request/load/save/new-city/close/obliteration and
all assigned region/new-region/import/establish/city-transition owners remain
excluded. A passing portable gate does not implement those owners or establish
native integration, native getter synchronization or default/replacement
application coverage.

No ABI, native/plugin, tools, maps or default changes were made by this round.
No game request or real user/game file change was made; game verdict, shadow and
reference-warning evidence are n/a for this unbound portable module. The next
step is to land this bounded portable work, keeping those owners and questions
separate. No additional family round is needed for this verification task.
