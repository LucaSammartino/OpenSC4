| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view4 (bounded portable MapView owner) | `Codex` / `core-map-view4` (engine lane, detached HEAD) | ready for bounded source integration: cargo 5286/0, clippy 0 diagnostics, CMake 4/4, full CTest 68/68; 24 MapView fixtures; new-file fmt 7/7, inherited lib.rs formatting differences retained; QV5-QV11 appended; ABI unchanged; game/shadow n/a | (1) coordinator combined integration gate; (2) obtain missing contracts before extending partial owner bodies |

## Delivery report (2026-10-23)

Added the production pure-core `engine/core/src/map_view` module and one final module declaration in
`engine/core/src/lib.rs`. No existing simulator, grid, UI, CAPI, plugin, default, map or coverage owner changed.
This delivery implements bounded algorithms and partial prefixes, with explicit Unsupported boundaries.
It does not complete SetDataView, Update, UpdateHighlights or the native owner. The owner total
**87 functions / 31396 bytes**, and the brief's **26 named functions**, remain attribution context only.
Per-body byte sizes and implemented-body byte credit are unknown; no whole-owner credit is claimed.

### Supported algorithms and exact domains

- `MapView` owns active/current/requested/source/config/resource/dirty/last-redraw/notification/subscription,
  highlight-observation and overlay-registration state. Getters use shared references, with a Send + Sync
  fixture and no RefCell. Portable construction accepts explicit clock/notification baselines and starts
  with unknown current view; native constructor defaults remain unpublished (QV9).
- `init` emits the published ordered generic init/subscription/marked-cell intents with an explicit native
  allocation boundary. Activation/deactivation own registration intents and the active flag. Hook/unhook
  own the exact source-message table, register-then-subscribe order and source76 priority 0xC0 (others 0).
  Repeated Init/Hook semantics are refused rather than invented.
- `set_data_view` preserves everything on inactive/current-view no-ops. The active changed-view prefix drops
  update subscriptions before raw exemplar lookup; missing resource is represented separately. It decodes
  and stores supported fields, then **stops at owner ramp generation**. Current view is never committed by
  this prefix. None stops at default-filter construction (QV1), retaining later source/config/highlight
  state instead of fabricating filter/reset calls. No custom view is created or released by this module.
- Exemplar inputs are typed raw resource properties. Supported decoding includes checked resource-key
  arithmetic, source, raw ramp pairs, scale range `1 < s < 2^24` else 256, default layer 1, required typed
  interpolation/highlight fields and block-property presence (including argument zero). Independent raw
  helpers retain nonzero occupant types, map terrain modes 0/1/2 to 1/3/2, read typed bools, and keep at most
  eight travel entries already in byte range. Missing/malformed consumed fields, duplicates, invalid modes,
  oversized/high-bit travel lists and key overflow are Unsupported. Remaining fields stay raw; no host can
  provide a normalized MapView configuration, filter, palette or complete owner decision.
- Source selection covers all published simulator slots/arguments/value types and own-cell source29.
  Sources 11, 13-23 and 31-63 select DrawNothing; this is no bitmap-clear acknowledgement (QV9). All twelve
  source64-75 developer types are distinguished: layer2 slot14/u16, layers3/4 slot13/i16, debug flag forcing
  layer6's slot11-entry-times12 request. Layer1/unknown layers, layer5 construction and layer6 grid layout
  remain unsupported. Layer selection does not build a developer grid or provide its conversion.
- Source notifications and changed-value 0xA6B79602 own dirty decisions. Every delivered tick collects raw
  highlights, even while clean/throttled. Known dirty redraws become due at elapsed >=1000 ms, or >=250 ms
  with source76 always dirty. Unresolved highlight comparison cannot prove a clean map. Partial Update
  intents retain dirty and last-redraw time; no public redraw-acknowledgement callback bypasses the boundary.
  Times earlier than the supplied last-redraw baseline are refused; clock wrap/commit behavior remains QV9.
- Palette helpers implement full-domain u8 identity and i8 +128, finite floats whose truncated result is
  0-255 (including -0.75 -> 0 and 255.99 -> 255), direct lookup/emission from an explicitly supplied raw
  256-entry palette, and rounded per-channel alpha multiplication. Nonfinite/out-of-range truncated floats
  are refused before output writes. Both entire 16-bit conversion domains are Unsupported (QV5).
  `emit_grid` reuses unchanged `TractGrid` raw reads in its documented x-outer/z-inner order. Supplied
  palettes/grids exercise helpers only; they never construct the owner's palette or complete Update.
- Highlight helpers select the documented raw collection/coverage slots for kinds1-4 and6-9, collect
  matching individual building records, select residential/fire-half/police-half radii for1-4, retain
  unspecified radius for6-8 and explicit none for9. Candidate enumeration retains input order only as an
  observation. **Native comparison/order/equality, dirty changes and complete UpdateHighlights remain
  Unsupported**, including repeated identical observations. Kind0 and crime hotspot kind5 are unsupported.
- Overlay helper state emits renderer slot32/layer5/priority1000 registration once and one verbatim mark
  per explicitly supplied coloured cell with complete finite raw origin/extent/altitude/colour. Invalid
  geometry is refused before registration. It does not infer cell inclusion, alpha thresholds, geometry,
  depth transforms, manager behavior or damaged-road effects, and is not a complete Update.

### Bodies still unimplemented

Owner ramp generation, coarse-grid interpolation, BlockMapByLot, developer layer5/debug-grid construction,
crime/traffic markers, source76 traffic collection/rendering, native allocation/failure handling, default
filter/reset calls, custom-view lifecycle, child-flag application and current-view completion remain open.
Buttons/exclusion/groups/remembered values, tutorial commands, panels, ResetDataView, 3D filter options,
legend UI and crime-hotspot effects retain their summary/QV boundaries. There is no opaque host callback
returning SetDataView, Update, GetHighlights, comparison results or completed owner stages.

### Validation and retained failures

| Check | Measured result and evidence |
|---|---|
| Preimplementation | Inventory, questions and independent literal fixtures committed before production code in `b59d16e7`. Production algorithms/24 fixtures in `f1c71dd6`; report checkpoint `6dd6c14b`; engine-style formatting in `5af24714`. |
| Initial all | Cargo 5262 passed/0 failed, clippy 0 diagnostics, CMake 215/215; CTest 67/68. Source was changed before CTest finished, causing E0277 in the new alpha closure and a tree-changed warning. **Not a clean baseline.** Fixed before substantive source commit; full failure logs retained at `build-runs/baseline/`. |
| Focused | 24/24 MapView tests, 3220 filtered, via q.py direct argv/native shell; `build-runs/focused-map-view.log`. Earlier single-string Bash launch failed E_ACCESSDENIED before testing, retained under `build-runs/q/`. |
| Quick, once | Raw i686 core results 3244 passed/0 failed/0 ignored (including 24 MapView), dependents compile-only, CMake 4/4. Wrapper remained outstanding after completed logs and was interrupted (exit1, no final PASS); retained at `build-runs/quick/`. It was not repeated. |
| Formatting | Final scoped qbuild fmt passes all seven new files. Initial wrong-relative-path failure and 100-column mismatch retained in `build-runs/format-observations.txt`. Eight-file fmt attempt reported 1630 diff lines before correcting new-file width; inherited lib.rs/module-child formatting was retained. Read-only `audit-lib-format.py` proves base/current root differ from engine style and current lib.rs is exactly base plus one final module declaration. No broad reformat. |
| **Final all** | **qbuild PASS, exit0** against frozen committed source `5af24714`: summed Cargo **5286 passed/0 failed/0 ignored** over 22 result rows, MapView **24/24**, clippy **0 diagnostics**, CMake **4/4 edges (67.3 s)**, full CTest **68/68** (listed68, actual122.56 s; wrapper122.7 s), fc4verify PASS847.6 s without a tree-changed warning. Current `build-runs/qbuild-*.log`, `fc4verify-*.log`; compact final audit `build-runs/q/20261023-023427-26416.log`. |

Discriminating tests include full eight-bit domains with distinct palette entries, signed -128/0/127 and
unsigned0/255 emission, float truncation/refusals, the independent arithmetic exposing QV5, typed resource
defaults/ranges/refusals, active/current/none preservation, all empty ranges and simulator/developer cases,
source-message changes, every-tick polling and 999/1000/249/250 thresholds with retained dirty state,
raw radii/unsupported repeated-highlight comparison, and once-only overlay registration/atomic refusal.

### Source, build flags and artifacts

The final audit confirmed the same frozen lib.rs/new-module combined SHA256
`ba2d859f871fb8552258cd36e58c5bc151fb2d605d47a2a32bfdf4f97cafcafb` before and after the final gate.
The release core fingerprint is i686-pc-windows-msvc with `-C target-feature=+crt-static`; generated C++
Ninja rules use sccache and `-MT`, CMake cache is Debug/Ninja with FC4_COMPILER_CACHE=ON.
The build reports `engine/tests/abi_entries.inc` unchanged (185 interfaces); it was not edited.

- `engine/build-x86/FreeCity4.dll`: 18185728 bytes, SHA256 `dae8b8916b2a586bc42c5699c5a2409ddf38eb027a71629da61784013873d3eb`.
- `engine/build-x86/cargo/i686-pc-windows-msvc/release/fc4_capi.lib`: 79184404 bytes, SHA256 `57b40c8df01e895e08cc834d155c55684835153ed7f6a53438275a4b3eb9791c`.

Report-only commits after the frozen-source final gate do not require repeating it. No game/native probe,
game request, live service or shadow run was made, as this brief forbids them. No native compatibility,
ref-warning count, replacement/default promotion or full-owner result is claimed. C ABI exports unchanged.

### Questions and next steps

Appended **QV5-QV11** to `docs/specs/questions/map-view.md`, preserving QV1-QV4 and leaving the generated index
untouched: contradictory16-bit arithmetic/default factors; missing/malformed raw fields/failures; highlight
comparison/radius semantics; ramp/resampling rules; initial/reset/clock/empty-source state; float/overlay
geometry limits; developer default/debug layout. These contracts are required before extending the listed
unsupported bodies. The coordinator's combined integration gate remains required before landing with other
lanes. This lane is ready for review/integration of the explicitly bounded tested production source only.

## Preimplementation owner-method inventory (2026-10-23, before production source)

Owner total **87 functions / 31396 bytes** and the brief's **26 named functions** are attribution context,
not implementation credit. No per-body byte sizes are published here; implemented-byte credit is unknown.
The rows below cover every named owner operation and distinguish external primitive names. Addresses identify
published contracts, not inspected code. No complete SetDataView, Update or UpdateHighlights is planned.

| Published operation | Intended bounded algorithm or precise missing contract |
|---|---|
| Init (0079E400) | Ordered generic-window init, three subscriptions, marked-cell create/init/depth/water intents; native allocation/failure and repeat behavior unsupported (QV6/QV9). |
| Activation / deactivation messages | Register/unregister animation intent and owned active flag, without inventing activation priority. |
| ShowWindow (007A6290), HideWindow (007A64D0), ShowLargeWindow (0079DE70), small-map return (0079E050) | Unimplemented panel summary, QV3. |
| SetDataView (007A49B0) | Inactive/current no-op; drop owned update subscriptions before raw exemplar lookup; missing resource outcome; parse/store coherent raw fields; stop at owned ramp generation. None stops at default-filter construction (QV1), with no fabricated reset calls or current-view commit. |
| UnhookUpdateMessages (0079FB20) | Drop recorded source subscription and animation registration as a group; no inferred within-group native call order. |
| InterpolateRampToPalette | Unimplemented owner algorithm, QV8. A supplied palette never completes this stage. |
| Alpha-multiplied palette copy | Independent per-colour rounded channel-times-alpha / 255 helper; no ramp construction. |
| OnViewModeButtonSelected (007A0F30) | Unimplemented: exclusion/remembered values/panel decisions, QV3. No static button table promoted to full selection. |
| DoMessage (007A56E0), tutorial command 287259F6 | Button, group and tutorial handling unimplemented, QV3. |
| ResetDataView (007A5610) | Unimplemented current-mode-button and visible-panel summary, QV3. |
| Update (007A2F60) | Inactive no-op; owned source/developer selection and clear/draw-nothing intents. Other paths explicitly stop at palette/interpolation/lot-block/markers/QV2 boundaries; no supplied helper grid/palette completes Update. |
| MakeMapColorBuffer | u8 identity, i8 +128, finite representable truncated float indices and explicit raw 256-palette emission helpers. Both 16-bit conversions unsupported (QV5). |
| SC4InterpolateGrid | Unimplemented owned coarse-grid algorithm (QV8), not a host callback. |
| BlockMapByLot (0079FEF0) | Unimplemented owner algorithm, QV4. |
| Developer layer-5 builder (007A2080) | Unimplemented owner algorithm, QV4. Layer-6 raw entry layout also unpublished; selection only. |
| Crime-incident markers (0079E600), damaged-road overlay | Unimplemented marker geometry/effects decisions, QV4. |
| UpdateHighlights (007A1A00) | Select/collect raw kinds 1-4 and 6-9; select documented radius input for 1-4 and explicit none for 9. 6-8 radius unspecified. Retain candidates only; comparison/dirty decision unsupported (QV7). |
| Highlight comparison (007A1390) | Unimplemented equality/order/duplicate/radius semantics (QV7), even for repeated raw collections. |
| GetHighlights (brief boundary) | No published complete operation; no callback or invented result. |
| Crime hotspot / effects | Unimplemented kind-5 summary. |
| DisplayLegend (007A04F0) | Unimplemented title/row UI summary and missing-property behavior; no full legend claim. |
| HookUpdateMessages (007A17B0) | Own exact source message/animation priority mapping and register-then-subscribe intents. |
| DoMessage (007A54D0) | Activation, source notification and changed-value dirty decisions. Each tick polls highlights; schedule known dirty at >=1000ms or source76 always dirty at >=250ms, retaining state at unsupported redraw boundary. |
| Overlay registration/marked-cell emission | Independent helper: once-only renderer slot32/layer5/priority1000 registration and one mark per supplied coloured cell with explicit raw geometry. No cell inclusion/geometry/depth inference (QV10), no full Update claim. |

External primitive names in the spec: window-generator Init, SC4MarkedCellView, SC4DoButtonExclusion,
GetCrimeIncidentArray, GetCongestionMap, GetPollutionGrid, GetOccupantCoverage and GetCrimeHotspot are
boundaries, not additional implemented owner methods. Raw grids may use existing `TractGrid` reads; existing
UI/View3D owners lack the missing filter/panel contracts and will not be changed to manufacture them.

## Coordinator provenance correction (report only)

Both successful operations include exact coordinator client-only interventions, omitted from the original
delivery report. The inherited baseline's read-only cache-statistics client552 (created1792716645.6393185)
was terminated01:57:38.118890 BST/exit15 after independent ownership review; the SAME validator2396 and
daemon17880 survived and proceeded to CTest22792. Baseline5262/native215 does not validate later owner source.

The stable owner operation at source5af24714866385f0c66c644f14503d1feda9e442 later waited on exact read-only
stats client9364 (created1792718461.2334836). Root terminated ONLY that client02:31:56.677711 BST/exit15;
the SAME validator3120 and daemon17880 survived and proceeded to CTest24004. The actual completed final
5286-test/68-CTest proof remains tied to its unchanged source and raw logs, not inferred from the intervention.

Both statistics queries were interrupted/unavailable; their underlying RPC cause is UNKNOWN. No daemon,
compiler, source, validator, check or cache-policy restart occurred. No additional gate or implementation round
was started. Exact structured/textual evidence was copied into own ignored build-runs; see
coordinator-stats-client-intervention-mapview-r1-baseline.json and
coordinator-current-final-stats-client-intervention.json. These case-specific root actions give no general
worker process-control permission. This correction changes report provenance only; original1/3, QV gaps,
unsupported/native/default/whole-owner limits and actual historical failures remain unchanged.
