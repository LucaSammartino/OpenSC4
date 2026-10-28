# view-input-controls: open questions
<!-- Questions about [view-input-controls.md](../view-input-controls.md). Append new entries at the END; when one is answered, move it to answered/view-input-controls.md. Rules: README.md. -->

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- **VIC3 - sections 2.3, 3, 4.4, 5.2: numeric primitive boundaries:** what are the
  rules for negative/equal/reversed scroll thresholds, nonfinite/out-of-city terrain
  picks, a missing road tile after a successful mask-0x44B lookup, and fractional
  projected sign drag offsets? Does avenue "orthogonal" mean connected kind 2,
  including when kind-4-only inner sides occur? [reject invalid thresholds and
  invalid level vertex ranges/missing road tile with explicit Unsupported;
  orthogonal is literally connected kind 2; sign mouse-plus-offset truncates at
  the typed integer-pixel boundary; no claim about native rounding]

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- **VIC5 - sections 1-6: complete owner inventory:** please publish the complete
  per-owner method inventory and byte ranges/sizes for CityScroll, LevelTerrain,
  LabelTool, SignTool, SignLabelDemolish and Label/Sign/SignOrLabel filters,
  including inherited overrides, destructors, reference/message-interface methods
  and any unnamed helpers. The current prose gives entry addresses and selected
  fields but no method sizes or complete slot inventories. [implement only named
  published supported algorithms; list every published method/field locally;
  byte coverage and full-owner/base ownership explicitly unclaimed]

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- **VIC6 - sections 2-6: failure/lifecycle contracts:** what happens on picture,
  window, script, field or prop allocation failure, repeated SetMouseScroll after
  deleting its child, repeated selection of the same prop, or shutdown mid-level
  drag? Is there a leveling Shutdown override, and how is capture/cursor restored?
  [resource failure is explicit Unsupported, acquired picture retained until
  shutdown; same-prop selection is a no-op; no invented level-shutdown behavior
  or hidden GUI/service lifetime; native COM adapters remain unsupported]
