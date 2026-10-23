| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view4 (bounded portable MapView owner) | `Codex` / `core-map-view4` (engine lane, detached HEAD) | in progress | (1) inventory published methods and fixtures; (2) implement coherent bounded algorithms; (3) required gates |

## Handoff
- Goal: first pure-core MapView implementation; preserve QV1-QV4 and missing palette/ramp/grid/highlight/summary boundaries.
- Base: detached `4706fd83`, reserved worktree reused; initial status clean.
- Done: read map-view §§1-10, QV1-QV4, question policy, grid raw access and ui_windows boundaries. No prior attributable qbuild/fc4verify logs; baseline all launched once.
- Next: commit preimplementation inventory/questions/discriminating fixtures, then implement bounded algorithms.
- Scope: new map_view module/tests, additive lib.rs declaration, appended map-view questions, this report only.
- No game/native probes, ABI/plugin/CAPI/default changes, subagents, branch/ref mutation, or forbidden paths.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-map-view4-r1-manifest.json`.
- Owner accounting: 87 functions / 31396 bytes is context only; named operations and unknown body sizes will be recorded independently.

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
