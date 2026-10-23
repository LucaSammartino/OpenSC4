| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-player-drive4 (first portable drive-tool owner) | `Codex` / `core-player-drive4` (engine) | READY: final all PASS exit0; cargo5168/0, clippy clean, CTest68/68, exact owner26/26; new-file fmt PASS, shared173-line order drift separately retained; old quick terminal UNKNOWN; DR1-DR8; ABI unchanged | (1) Land the bounded portable subset; (2) resolve DR1-DR8 before native/full-owner integration or promotion |


## Report

Implemented the first bounded portable player-drive owner in `engine/core/src/player_drive`, registered by the single additive declaration at the end of `engine/core/src/lib.rs`. No C API, plugin, other owner, default, map, generated index or ABI export was changed. No game/native probe or shadow was requested; game labels are not applicable.

Milestones: 1ebdc0a1 audited the published methods and committed independent literal fixtures before implementation; df9c801a added the owner and tests; 2c43a25c tightened literal message/domain policies; 0e117e5a recorded the non10-state/lifecycle questions; bb25fc0e made callback registration/removal carry explicit tool identity; eab6c1ad registered the module and completed the 26 fixtures. These are implementation milestones, not claims of executed conformance.

### Completed final validation — original round 3/3

Exactly one final `qbuild.py --wt <WT> all` ran on committed `72be9027` from 00:29:34Z to approximately 00:37:17Z. The captured original terminal session78746 was collected after its durable completion record existed: **wrapper exit0, q exit0, `qbuild: PASS`, `fc4verify: PASS`**. No final check was repeated for later documentation commits.

- Workspace Rust: **5168 passed / 0 failed / 0 ignored**, summed across **18 test summaries**; **four doctest summaries, 0 passed / 0 failed**. All **26/26 exact expected PlayerDrive fixture names** passed, with no missing or unexpected names. Compilation finished in **2m46s**.
- Strict workspace clippy: **exit0, diagnostics0**, finished in **2m20s** through the required all wrapper (`-D warnings`).
- Native build: **PASS 2/2 edges (0.6s)**, release Rust reused source-corresponding output in **0.03s**. This is the final incremental build count; R2's separate source-matched build was **4/4**. Neither grants native PlayerDrive-body equivalence.
- Full CTest: **68/68 passed, 0 failed**, **98.91s** (`fc4verify` reports99.0s). CTest's cargo_test independently executed all26 PlayerDrive fixtures and the same5168 workspace tests; these reruns are not added to the workspace total above. Native/source checks ran through the full unfiltered CTest command, with no skip or tolerance changes.
- Fresh bounded absolute-path fmt: **two new files exit0**. Shared `lib.rs` separately **exit1, diff_lines173**, against the retained pre-existing171-line module-order drift plus the additive declaration. No mass formatting; no whole-file formatting success is claimed. The earlier wrong-config985-line fmt, Bash/CIM/apply_patch-context failures remain separate historical attempts, not implementation/compiler/test defects.
- `fc4verify` printed **`[the tree changed during the run]`**. The only changed tracked path between gate-start72be9027 and the later checkpoints/report is `docs/lanes/core-player-drive4.md`. The committed engine tree, source files, working engine diff and production comparison stayed identical. This valid source-matched gate is retained for the docs-only commits.

Gate source: production `eab6c1ad`; engine tree **`c96747fe52d09dcbd607457fd6c8a4934c60603e`**; player_drive tree **`704f92dc902e17f841c13c8353a9130f1e577ea6`**. Gate-start whole-tree ID `f0e1d92929c915a854161d0866c7a470c139ddf7` includes the then-current report. Source hashes remained unchanged:

| Source | SHA256 |
|---|---|
| player_drive/mod.rs | `552123994542f44f6263a6b2607d7d019b964ab435b81e566b2c41b9aeccffd6` |
| player_drive/tests.rs | `49f9d4132275d0a961c6f7f46960e4d70040cbeaf4212462684eb812b0cc1ec1` |
| player_drive/fixtures.md | `25e3c7be8a8741e3a80669e51c70b50142d1e8ab10125fbe41d3b19d7d85eaf9` |
| core/src/lib.rs | `f401503862d43cf02cb145e4d1192d4c1576e5313f787b1b23f1305fd1058c4d` |

Final evidence manifest: `build-runs/source-r3-native-evidence.json` records full log/source/artifact hashes, source cleanliness, exact fixture names, CTest commands and flags. Native configuration stayed **Debug, i686-pc-windows-msvc, compiler cache ON, FC4_CHECKED OFF**, with the own-worktree CMake source directory. Generated `abi_entries.inc` reported unchanged185 interfaces and was not edited.

| Evidence | SHA256 |
|---|---|
| final-r3-terminal.log | `4fa4433de8d9813aa26d78063c6523827f2ac2a656b65bef61aaf3396e443e7c` |
| qbuild-test.log | `25297644be8d0a5c10864aabed7c4852cb3a39c319ff548f0d8cab4e4384a21b` |
| qbuild-clippy.log | `be47dd4e6ebbcc542d1a43f6cf540e678357c9a3d50f71801d2364118d47bb58` |
| fc4verify-build.log | `ef23e50e244f2bf670e06321481e49b30b37ec0da3925009fe861980d9a5bd1d` |
| fc4verify-ctest.log | `d9c429765e224b41bfb9b312e453ea560001b874a4e74785c3f41f17ba673b92` |
| LastTest.log | `f93722cc7cc0b82d37eb8e2825eaf8d1b2e152e9175e38e21c401203cf82aca7` |
| FreeCity4.dll (18110976B) | `0dca05743ef3a34b7f7beec4fc8c9d728a652c120932bd4b27ec773841bbceff` |
| fc4_capi.lib (78734052B) | `02194755fa713f4df58c5cc8899f5b36a917089f9a5f96b7d9841c95bd9cc44e` |

### Retained incomplete attempts and process provenance

Original pre-registration baseline remains **incomplete**: Rust5142/0 across22 summaries, clippy clean4m32s, native215/215, CTest18/68 only; terminal exit/cause **UNKNOWN**. It supplies no final acceptance or owner credit. All six original logs remain under `build-runs/baseline-r1/` with their original archive manifest. The exact root intervention against only old read-only stats client11484 remains in `build-runs/coordinator-stats-client-intervention.md`; it grants no authorization for another process/cache action.

R2's source-corresponding quick components remain valid: Rust3126/0 across2 summaries, all26 fixtures, dependent crates compile-only (no execution credit), native4/4. **Quick terminal exit/cause remains UNKNOWN**. Its four remaining component logs were archived to `build-runs/quick-r2/` before final replacement; every SHA256 matches `build-runs/archive-r3-manifest.json`. No unchanged quick was repeated to repair its missing terminal record.

Fresh read-only process inspection at00:24:33Z found old quick36008, verifier25864, read-only stats37432 and creation-bound continuation37316 absent; no surviving attributable checker/compiler/output owner or TCP4721 listener. Direct CIM was denied; the native process inventory recovered creation/parent/argv and own cwd. Foreign cwd denials(error5) remain explicitly recorded and were classified by other-worktree compiler output paths; cache-service children targeted train-integration. Evidence: `build-runs/process-r3-preflight.json`. Absence establishes neither success nor disappearance cause; the old continuation was not replayed. No client/compiler/daemon was terminated, no cache liveness RPC issued and no cache/environment/provider/policy changed.

Final qbuild26020(created00:29:34.384666Z), runner12324(.226582Z), q.py29060(.180281Z), and later verifier32128/CTest30348 were separately attributed by creation, parent, cwd, argv and compiler paths in `process-r3-final-started.json` and `process-r3-clippy.json`. Their successful final result is independent of the old unknown exits and other lanes' checks.


## Pre-implementation method audit

The published attribution is 40 functions / 12400 bytes; the reported inventory is 32 entries (30 tool entries plus two foreign junction-view entries). Neither number is implemented-body/byte credit. The table records published portable method boundaries, not a claim to all inventory bodies. Inventory names absent from the behaviour spec cannot acquire invented contracts.

| Method | Bounded portable source status | Published portable boundary |
|---|---|---|
| Construction | partial | IDs, raw camera-speed bits, toggles, empty identities/callbacks, cell scales; no native layout/timer construction |
| Init | unsupported | Native service acquisition, omitted table IDs/counts and DR1 objects unresolved |
| Shutdown | unsupported | Summary cannot establish resource/COM release ordering |
| EnterChoosingMode | partial | Ordered cleanup, hover clear, conditional simulator pause, four manager pauses |
| PauseEverything | supported subset | Conditional simulator pause/resume; four manager effects |
| PickAutomaton | primitive boundary only | Raw renderer/occupant query, no foreign body credit |
| PickNetworkOccupant | primitive boundary only | Raw renderer/IID query, no foreign body credit |
| OnMouseMove | partial | Terrain/cell and hover/drive-field selection; hover reference policy unresolved |
| OnMouseDownL | partial | Accepted network pick wins; form selection; mission/group gate |
| EnterDrivingMode | partial | Only explicit portable subset; dashboard load, audio table, settings selection remain visible gaps |
| IsDriveAutoValid | supported | Each of flags 0x08000000 and 1 independently excludes |
| StopTracking | supported subset | Ordered raw effects and state-0/slot-53 decision, held driven reference only |
| EndInput | partial | Callback, slot18/13, stop, hover, disaster, base stack removal; unknown callback values block full method |
| OnKeyDown | partial | Repeat suppression, Escape, mission/validity/keymap; key effects Unsupported |
| OnKeyUp | partial | Mission/validity/keymap; key effects Unsupported |
| HandleKeyEffects | unsupported | No published policy; no host outcome hook |
| SetDriveCommand table | supported subset | Numeric table only; automaton slot60 body foreign |
| OnMouseDownR | supported subset | Validity/mission gate and command8 down |
| OnMouseUpR | supported subset | Validity/mission gate and command8 up |
| OnMouseUpL | supported subset | Clear selected cell, command7 up/event4 stop |
| ShowUI | unsupported | DR1; dashboard ID selection alone supported |
| Update | unsupported | DR1; gauges/tracking cadence not published |
| DoWinProcMessage | supported subset | Four literal button/command pairs |
| DoMessage | partial | Numeric routing; panel direction/model details unresolved |
| AutomatonRemoved | partial | Matching identity only, StopTracking then partial EndInput prefix |
| AddCallbackTarget | partial | Ordered list, no guessed initial counts/dedup/ref policy |
| SendCallbackMessage | partial | Literal message and supplied two values; reentrant mutation unresolved |
| SC4TrainJunctionView construction / Shutdown | excluded | Separate owner, zero credit |

Foreign Automata/Gauge/Audio/Camera/dispatch/junction/state10/keymap bodies are excluded throughout. Existing camera APIs use different zoom units and do not expose this window's SetZoomLimits; use raw window operations rather than reinterpret values. Existing automata PlayerDrive is serialized state only; city_situation slot32 remains a raw foreign predicate.

## Bounded implementation

The new `engine/core/src/player_drive` owns current/hover identities, selected/hover cells, choosing/driving state, both toggles, camera-speed words, callback planning order and timer markers. Its foreign inputs are individual picks, flags/group/type/state/property/drive fields, raw script command lookup, simulator pause state, mission slot32 and zoom predicates. Its outputs are ordered primitive operations, including the explicit acquired/released driven reference. No primitive returns a same-owner algorithm result.

`enter_driving_mode` is an explicitly partial projection, not native entry: it marks omitted ShowUI, audio-table and settings-selection steps and always returns their unsupported gaps. It selects an ID without loading a dashboard. Light/button effects require an explicitly attached, already-created window. Normal keys stop at Unsupported HandleKeyEffects after raw lookup; direct numeric commands and the published mouse paths exercise the owned table. Unknown/nonboolean/missing primitive domains fail visibly, keeping already-emitted prefixes instead of inventing rollback or releases.

`fixtures.md` was committed before production implementation (1ebdc0a1). The 26 tests discriminate table/order/state decisions rather than roundtripping serialization or asking a host for an owner outcome. No new-owner native equivalence, implemented bytes, shadow measurement, replacement/default promotion or complete Init/Shutdown is claimed. This is the first bounded portion of this distinct owner; no foreign-owner bodies count toward its credit.

Questions DR3-DR8 are appended only in `docs/specs/questions/drive-tool.md`; DR1 and DR2 remain open and unchanged. The generated question index is deliberately not edited or regenerated under this lane's restrictions.

## Next steps

1. Land this validated bounded portable subset; no new game request or native/shadow/default promotion is claimed.
2. Resolve DR1-DR8 before completing native lifecycle, ShowUI/Update, key effects, resource/reference/reentrancy and default domains. Foreign automata, gauge/audio/camera/junction/state10/route bodies remain unsupported and receive no implementation credit.

Final-round checkpoints:75ccd5a8 process ownership;72be9027 exact log archive and bounded fmt;9aa6d5ff final validator identity;fbb6794e source review;76184b3a workspace/clippy/native milestones. Production and fixtures were unchanged throughout rounds2/3. This report closes original round3/3 after the actual completed final gate.
