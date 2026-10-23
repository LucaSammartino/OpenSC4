# GZ graphics questions

Scope: [gz-graphics.md](../gz-graphics.md), Windows 1.1.641.

- **GZG1 — sections 2–3, generic geometry and convenience blits:** settle the
  signatures/geometry of context slots 36–37 (`0x0082C100`), kinds 2/4/5, line
  endpoint and polygon closure/coverage, gradient conversion, and exact
  Blt/BltTiled source rectangles and scaling. Basic filled result-1/result-2
  dispatch is specified in section 3.2; full tessellation coverage remains open.
  Evidence next: helpers around `0x0082CC60`,
  `0x0082D040`–`0x0082E130`, `0x0082F970`–`0x0082FC30`; verify float widths,
  rounding and intermediate precision against instruction bytes.
- **GZG2 — section 4, hardware/SimGL and AA geometry:** finish the
  hardware extension-slot inventory, hardware base defaults/Shutdown, exact
  source bounds and texture/gradient subdivision, buffer AA edge coverage,
  SimGL Draw specialized/fallback routes and HWSetImage failures. Export
  pseudocode for `0x009922D0` and `0x00992F80` failed; use paged assembly.
  Relevant base helpers `0x00A64490`, `0x00A646F0`, `0x00A64A80`,
  `0x00A64DA0`, `0x00A660F0`; buffer AA `0x008308D0`.
- **GZG3 — section 5, graphics IO/presentation and startup:** name and type
  remaining graphics-interface entries in the table `0x00ADB050` (slots 0–68),
  including bitmap/file IO helpers; settle each failure/output-ownership rule.
  Complete driver discovery/INI precedence, mode-selection and partial-init
  rollback, screen buffer allocation/format defaults, SetUIRenderingMode's
  exact render state/projection and RenderBufferToScreen copy/present rules.
  Audit GetGDriverByIndex and ByID failure/ownership and folded driver slots
  17/20. Relevant owners are `cGZGraphicSystem` and `cGZGraphicSystemW32`,
  including unnamed helpers in region `0x00980000`.
- **GZG4 — section 7, full Windows canvas event contract:** finish the unnamed
  primary slots and subinterface thunk mappings; settle HWND ownership, window
  creation/reconfiguration failure cleanup, event pump ordering and every
  Windows keyboard/mouse/focus/resize/quit translation. Confirm names/units of
  the two words set by message `0xFA0C6390`, IID `0x1A0C615F`, subtype 13.
  Evidence next: `0x0098C690`, `0x0098CDB0`, `0x0098D080`, `0x0098D230`,
  `0x0098D9C0`, `0x0098DC40`–`0x0098DD70`; instructions are needed where
  adjusted-this and message locals are incorrectly typed in the export.
- **GZG5 — section 6, standalone alpha operations and loader details:** settle
  GenericBufferClip's exact rectangle/output mutation and all clipped versus
  NoClip alpha/shade/shadow/bright/stretch/half-source formulas, supported
  formats, locks, return channels and failure cleanup. Specify the legacy
  indexed surface loaders' argument widths, alpha quantization and mask byte
  meaning, malformed/short-read behaviour and resource ownership on open/lock
  failure. Next evidence: standalone helpers in `_global.c`, especially
  `0x00930BE0`, `0x00930CF0`, `0x00985D40`, `0x009888E0`, `0x00988A80`;
  confirm numeric conversions in assembly before extending the spec.
- **GZG6 — section 2.1 context destruction versus section 3.4 replacement:**
  Does final context destruction call engine Shutdown before releasing it, or
  only release the independently retained engine and target? Section 2.1 says
  destruction releases both; section 3.4 explicitly requires Shutdown only on
  replacement. The core follows that literal distinction: replacement invokes
  Shutdown, destruction releases engine then target without adding a Shutdown.
  This matters because the buffer engine's final Release does not unlock (§4.2).
  Please settle the context-destructor call sequence; no native destructor
  layout or implicit shutdown guarantee is assumed.
