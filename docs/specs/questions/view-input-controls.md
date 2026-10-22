# view-input-controls: open questions
<!-- Questions about [view-input-controls.md](../view-input-controls.md). Append new entries at the END; when one is answered, move it to answered/view-input-controls.md. Rules: README.md. -->

<!-- from: re-viewtools (2026-10-20) -->
- **VIC1 - §3, §4.1, §5.1, §6:** which commands of the city view's `ProcessCommand` (`0x007F1C10`) make the label,
  sign and sign-bulldozer tools, which buttons send them, and what the other cases of `RunGlobalTerrainTool`
  (`0x007E75B0`, commands 0–6 beside levelling's 7) do. [tools specified from their own code; makers named only]
- **VIC2 - §5.1, §5.4:** what message `0x0BE10A20` is (the sign tool answers it with a screen point while typing)
  and what framework service `0x27EBFFFD` (interface `0x07EC0010`, slot 18 reads text) is. [read as an input-method
  position request and the clipboard]

<!-- from: core-view-inputcontrols4 (2026-10-22) -->
- **VIC3 - sections 2.3, 3, 4.4, 5.2: numeric primitive boundaries:** what are the
  rules for negative/equal/reversed scroll thresholds, nonfinite/out-of-city terrain
  picks, a missing road tile after a successful mask-0x44B lookup, and fractional
  projected sign drag offsets? Does avenue "orthogonal" mean connected kind 2,
  including when kind-4-only inner sides occur? [reject invalid thresholds and
  invalid level vertex ranges/missing road tile with explicit Unsupported;
  orthogonal is literally connected kind 2; sign mouse-plus-offset truncates at
  the typed integer-pixel boundary; no claim about native rounding]
- **VIC4 - section 5.3: encoded text:** which code-page decoder and lead/trail
  validation rules are required, what happens to pending bytes on control keys,
  Escape, a new edit, or shutdown, and does pasted "newline" include CR/CRLF or
  only LF? Can OnCharacter receive values beyond one byte? [complete glyphs are
  raw code-page byte strings; primitive codec supplies segmentation only;
  backspace removes a complete glyph; literal LF is skipped; pending bytes are
  retained unless decoding completes; invalid decoding is Unsupported; byte
  input API makes no claim for wider character values]
- **VIC5 - sections 1-6: complete owner inventory:** please publish the complete
  per-owner method inventory and byte ranges/sizes for CityScroll, LevelTerrain,
  LabelTool, SignTool, SignLabelDemolish and Label/Sign/SignOrLabel filters,
  including inherited overrides, destructors, reference/message-interface methods
  and any unnamed helpers. The current prose gives entry addresses and selected
  fields but no method sizes or complete slot inventories. [implement only named
  published supported algorithms; list every published method/field locally;
  byte coverage and full-owner/base ownership explicitly unclaimed]
- **VIC6 - sections 2-6: failure/lifecycle contracts:** what happens on picture,
  window, script, field or prop allocation failure, repeated SetMouseScroll after
  deleting its child, repeated selection of the same prop, or shutdown mid-level
  drag? Is there a leveling Shutdown override, and how is capture/cursor restored?
  [resource failure is explicit Unsupported, acquired picture retained until
  shutdown; same-prop selection is a no-op; no invented level-shutdown behavior
  or hidden GUI/service lifetime; native COM adapters remain unsupported]
- **VIC7 - section 5.2 release while typing:** a new-sign press sets state 3
  (typing); is its subsequent release meant to leave state 3, or does the stated
  "otherwise idle" apply to it as well? [literal published release rule sets
  idle outside the pressed/empty-drag cases; no exception invented; typing
  operations independently tested from state 3 before such a release]
