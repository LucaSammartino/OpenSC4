| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-player-drive4 (first portable drive-tool owner) | `Codex` / `core-player-drive4` (engine) | in progress | Audit contracts, fixtures, bounded implementation, required gates |

## Handoff
- Goal: first portable player-drive owner from drive-tool sections 1-10; no native/ABI work.
- Done: audit/questions/literal fixtures committed as 1ebdc0a1 before implementation.
- Done: production owner and 24 discriminating tests drafted; unregistered until baseline all finishes so it remains a baseline.
- Next: collect baseline operation (exec session 44795), register module, commit, run absolute fmt and serial quick/final all.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell commands.
- Process inventory via CIM denied; psutil unavailable; Get-Process measured active process IDs/start times/handles. No existing local qbuild/fc4verify logs before baseline; no duplicate check started.
- Baseline all started once; local qbuild-test.log shows compilation only at last observation. Do not launch another build while it is active.
- No game requests; no ABI changes. DR1/DR2 remain unresolved.

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
