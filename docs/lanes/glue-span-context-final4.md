| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span-context-final4: native draw-context route for span1555/span4444 bridges | `glue-span-context-final4` / engine lane | ready: span-render-4444.md 1.3 steps 1-5 through the real cGZDrawContextRenderToBuffer (vtable 0x00AC17D0, slot 22) for types 4, 6 (1555) and 7 (4444), all five methods and all seven call sites; full arenas (29x8, stride 60, padding included) byte-exact 126/126, 126/126, 122/126 (4 = open SP1, alpha outside 0..255); lifetime balanced; shipped calls 0 (none observed); gsctx4-r2-union2032 clean ref 0; gate cargo 4099/0, clippy 0, ctest 65/65; SP6 filed | 1. RE answers SP1 (packed red/blue blend for weights 17..31), then the core follows. 2. SP6 (SetBltSource x/y/sx/sy and rect meaning). 3. Shipped calls stay 0: SP4 / SR1555-6 still need a natural producer; no whole-class claim. |

## Report

What changed
- engine/plugin/FC4SpanContext.{h,cpp} (new; CMake additive): the span-render-4444.md 1.3 fixture, once at the first
  city load when span1555 and/or span4444 is on. Destination cGZBuffer (GZCOM 0xC470D325) Init(29, 8, type, 16),
  textures types 4-9 (64x32, 16/32 bpp) filled between Lock/Unlock with the same flags; draw system service
  0xEB2EA3CB / IID 0x4B2EA3B3, slot 4 (buffer, &ctx), vtable checked = 0x00AC17D0, draw system released; ctx slot 3
  Init, +0xB0 renderer vtable checked (0x00AC182C / 0x00AC185C), +0x78 = pixel base checked; per case the path state
  per the 1.3 table (slots 7, 8, 9, 12, 13, 14, 15), then slot 22 with the original renderer entries and again with
  our forwarders from the same arena; whole stride x height compared. Shutdown, Release, buffer Release; lock count /
  flags and remaining reference counts logged. SEH-guarded slot-22 calls.
- Caller classes (by forwarder return address, 1..7 bytes after one of the seven 0x008302E0 call sites, SP6b):
  constructed_native_context / constructed_direct / shipped / other, per renderer and slot, logged after the fixture
  and at shutdown (cFC4Director, before the 4444 entries go back).
- FC4SpanRender4444 / FC4Span1555Shadow: `*UseOriginal(bool)` (entries 1-5 = originals or forwarders), forwarders
  report return address and refusals; 4444 renders ours on the fixture thread even under span4444_shadow; 1555's
  "game call" notice ignores the context fixture. compat: `SpanContextAddresses` (identified build only).
- No core change: the only difference is the open SP1 case (not contract-backed).

Measured (gsctx4-r2-union2032: default keys + span4444,span1555, turbo to 2032-01-17, clean, ref_warnings=0;
r1 gsctx4-r1-union2032 also clean/ref 0, before the SP1 split)
- span1555 type 4: 126 cases exact 126; type 6: 126/126; route per site solid 4, gradient 10, alpha 6, alpha
  gradient 10, tex plain 24, stretch 48, tiled 24; route_mismatch 0, ours_refused 0, faults 0, no_renderer_call 0.
- span4444 type 7: 122/126 exact; the 4 others are alpha-gradient ramps with alpha outside 0..255 (SP1, open; 1-2
  bytes, R nibble); all 4 in-range alpha-gradient cases and every other method exact.
- Lifetime: dst lock count 0 -> 1 (flags 0x8040) after Init, back to 0 / 0x0000 after Shutdown; ctx refs 1, Release
  -> 0; every destination and texture Release -> 0; texture lock counts unchanged across all cases.
- Counts (session): span1555 native-context 252, direct 158 (its own fixture through the vtable), shipped 0, other 0;
  span4444 native-context 126, direct-through-vtable 0 (its fixture calls the slot functions directly: 198 cases),
  shipped 0, other 0. Shipped = 0 means no natural call was observed; it is not evidence of absence.
- Gate: cargo test 4099/0, clippy 0, fc4verify build + ctest 65/65.

Not covered / open (no whole-class claim)
- SP1 (weights 17..31), SP6 (SetBltSource x/y/sx/sy and rect meaning; site vs return address), U1/U7 inputs (count
  0, wide spans), colour-key through the context (no published slot sets it), SP4 / SR1555-6 (no shipped producer
  seen). Default promotion is the coordinator's decision.
