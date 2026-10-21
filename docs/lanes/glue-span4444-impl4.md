| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span4444-impl4: native bridge for cSpanRender4444 (`enable = span4444`, default off) | workflow / `glue-span4444-impl4` | **ready**: vtable `0x00AC185C` slots 1-5 bridged; constructed-fixture original-vs-ours 198 cases / 57024 pixels EXACT; live calls 0; SP5 filed; gate cargo 3947/0, clippy 0, ctest 64/64; runs span4444-impl4-b (shadow), -c (replace) clean, ref 0 | answer SP5 (cGZBuffer Init / pixel access / draw-system contracts) so a COM-buffer pixel test can run; SP4 runtime count in a 16-bit session |

## Report (2026-10-21)

**What changed**
- `engine/capi/src/span_render4444.rs` (new CAPI, declared at the end of `fc4.h`): `fc4_span4444_solid`, `_alpha`,
  `_gradient` (flag selects RenderAlphaGradient), `_blt` (table dispatch row `format*2+flag`, column `mode`) over
  `fc4_core::span_render4444` on a bounded byte region; 3 unit tests. Required: the plugin has no other way to run
  the core renderer.
- `compat`: `ExecutableVersion` gains `spanRender4444VtableVA` + `spanRender4444SlotVA[5]` (`0x00AC185C`;
  `0x00847CE0/0x00847D30/0x00847E40/0x00847F10/0x008480C0`, §1/§1.1) and `SpanRender4444Entries()` (image/code checks).
- `engine/plugin/FC4SpanRender4444.{h,cpp}`: subsystem `span4444` (no class, default off). At PostAppInit it checks
  entries 1-5 hold the published functions, runs the fixture, a COM probe, then patches entries 1-5 with
  `__fastcall` forwarders (= thiscall, `this` ignored, callee pops 16/16/20/16/24, no return; §1.1). Slot 0 and the
  4-byte objects are untouched, so the Shutdown/Release lifetime is the original's. Default: ours renders; the
  original runs when the core rejects the input. `span4444_shadow = 1`: the original renders, ours renders a copy
  of the touched bytes, compared. PreAppShutdown restores the entries and logs counts.
- COM probe: `cGZBuffer` made through GZCOM (CLSID `0xC470D325`, IID `0x20732180`, persistence.md §6.2),
  `FC4_ACQUIRED` + `FC4_RELEASE_CREATED`, unused: Init, pixel access, draw-system and context-setter contracts
  are unpublished, so no pixel test through it was attempted (question **SP5**, questions/span-render-4444.md).

**Measured**
- Pixel comparison, **constructed fixture, not a game path, not a COM buffer**: the five original slot functions
  called directly (published §1.1 contract) against ours on identical plug-in-owned 32x8 buffers (stride 72),
  6 spans with unaligned starts: RenderSolid 3, RenderGradient 3, RenderAlpha 21, RenderAlphaGradient 3, RenderBlt
  168 (all 14 rows x 12 columns, keyed rows with a hit key) = 198 cases, 57024 pixels compared, 0 differing,
  0 rejected, 0 faults -> EXACT (identical in runs a, b, c).
- Live calls through the bridge: 0 on every slot (Konradshohe, turbo, to 2026-06-01). Consistent with §1.2 (no
  shipped type-7 RenderToBuffer target), but the session's screen depth was not recorded, so SP4 stays open.
  Shipped producers remain unproved; no in-game hit is claimed.
- Game runs: span4444-impl4-a (shadow; ref_warnings=2 from the COM probe, fixed in ae98d5db), span4444-impl4-b
  (shadow) clean ref 0, span4444-impl4-c (replace) clean ref 0.
- Gate: baseline cargo 3944/0, clippy 0, ctest 64/64; final cargo 3947/0, clippy 0, build PASS, ctest 64/64.

**Questions**: SP5 filed (a-d: cIGZBuffer Init slot/args/pops, pixel base/stride access, cIGZDrawSystem
acquisition and slot-4 `out` reference rule, context slot 3 Init pops and the setters of `+0x74/+0x41/+0xC4/+0xB4/+0x44`).

**Next**: when SP5 is answered, extend the fixture to a COM-created type-7 buffer drawn through `cIGZDrawSystem`
slot 4 + context slot 22 (labelled "constructed buffer"); a 16-bit session run for SP4's live-call count.
