| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-player-drive4 (first portable drive-tool owner) | `Codex` / `core-player-drive4` (engine) | in progress | Audit contracts, fixtures, bounded implementation, required gates |

## Handoff
- Goal: first portable player-drive owner from drive-tool sections 1-10; no native/ABI work.
- Done: audit/questions/literal fixtures committed as 1ebdc0a1 before implementation.
- Done: production owner/tests df9c801a; tightening 2c43a25c; explicit callback tool identity bb25fc0e. Module now registered; 26 new tests not yet executed.
- Next: collect baseline operation (exec session44795/PID37080), then serial quick and final all on this committed production source. Never duplicate an active build.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell commands.
- Process inventory: native command-line/handle query succeeded: baseline qbuild PID37080 (102 handles), attributable fc4-core test compiler PID8136 (126 handles); compiler later completed. No duplicate check started.
- Baseline all PID37080 / exec session44795: Rust complete 5142 passed/0 failed across 22 summaries; clippy finished clean (4m32s). CMake configured, build still active; collect it before another build.
- Formatting: first new-file check failed (wrong formatter config, 985 diff lines); corrected new files using engine/rustfmt.toml via apply_patch. Untouched lib.rs baseline fmt fails 171 diff lines (pre-existing module order); do not broadly reformat it.
- New-file absolute fmt passed (2 files); baseline lib.rs fmt failure remains separate. No game requests/native probes; no ABI changes. DR1/DR2 retained; appended DR3-DR8.
- Actual CMake cache: Debug, Ninja, FC4_CHECKED=OFF, compiler cache ON, Rust target i686-pc-windows-msvc; no source/default flags changed.
- Callback39/40 plans now carry explicit tool identity as well as automaton; commit before registering module/checking it.
- Registration followed baseline Rust/clippy completion and its native core compilation; active native baseline uses earlier objects and is not an owner-source gate. Final quick/all remain mandatory.

## Pre-implementation method audit

The published attribution is 40 functions / 12400 bytes; the reported inventory is 32 entries including two foreign junction-view entries. Neither number is implementation credit. The behaviour spec names the following tool methods/handlers; inventory names absent from it cannot acquire invented contracts.

| Method | Planned status | Published portable boundary |
|---|---|---|
| Construction | partial | IDs, raw camera-speed bits, toggles, empty identities/callbacks, cell scales; no native layout/timer construction |
| Init | unsupported | Native service acquisition, omitted table IDs/counts and DR1 objects unresolved |
| Shutdown | unsupported | Summary cannot establish resource/COM release ordering |
| EnterChoosingMode | partial | Ordered cleanup, hover clear, conditional simulator pause, four manager pauses |
| PauseEverything | supported subset | Conditional simulator pause/resume; four manager effects |
| PickAutomaton | partial | Raw renderer/occupant query, no foreign body credit |
| PickNetworkOccupant | partial | Raw renderer/IID query, no foreign body credit |
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
| AutomatonRemoved | supported subset | Matching identity only, StopTracking then EndInput prefix |
| AddCallbackTarget | partial | Ordered list, no guessed initial counts/dedup/ref policy |
| SendCallbackMessage | partial | Literal message and supplied two values; reentrant mutation unresolved |
| SC4TrainJunctionView construction / Shutdown | excluded | Separate owner, zero credit |

Foreign Automata/Gauge/Audio/Camera/dispatch/junction/state10/keymap bodies are excluded throughout. Existing camera APIs use different zoom units and do not expose this window's SetZoomLimits; use raw window operations rather than reinterpret values. Existing automata PlayerDrive is serialized state only; city_situation slot32 remains a raw foreign predicate.

## Bounded implementation

The new `engine/core/src/player_drive` owns current/hover identities, selected/hover cells, choosing/driving state, both toggles, camera-speed words, callback planning order and timer markers. Its foreign inputs are individual picks, flags/group/type/state/property/drive fields, raw script command lookup, simulator pause state, mission slot32 and zoom predicates. Its outputs are ordered primitive operations, including the explicit acquired/released driven reference. No primitive returns a same-owner algorithm result.

`enter_driving_mode` is an explicitly partial projection, not native entry: it marks omitted ShowUI, audio-table and settings-selection steps and always returns their unsupported gaps. It selects an ID without loading a dashboard. Light/button effects require an explicitly attached, already-created window. Normal keys stop at Unsupported HandleKeyEffects after raw lookup; direct numeric commands and the published mouse paths exercise the owned table. Unknown/nonboolean/missing primitive domains fail visibly, keeping already-emitted prefixes instead of inventing rollback or releases.

`fixtures.md` was committed before production implementation (1ebdc0a1). The 26 tests discriminate table/order/state decisions rather than roundtripping serialization or asking a host for an owner outcome. No new-owner native equivalence, implemented bytes, shadow measurement, replacement/default promotion or complete Init/Shutdown is claimed. This is the first bounded portion of this distinct owner; no foreign-owner bodies count toward its credit.

Questions DR3-DR8 are appended only in `docs/specs/questions/drive-tool.md`; DR1 and DR2 remain open and unchanged. The generated question index is deliberately not edited or regenerated under this lane's restrictions.
