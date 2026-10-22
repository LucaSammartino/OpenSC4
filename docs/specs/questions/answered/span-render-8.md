# span-render-8: answered questions (archive)
<!-- Archive for [span-render-8.md](../../span-render-8.md). Lanes do not read this file. Open questions are in ../span-render-8.md. -->

<!-- from: re-span8-4 (2026-10-21) -->
- **SR8-1 - sections 2, 3, 6:** what byte does a colour-type 2 (8 bpp) target store for a solid colour? RenderSolid
  and RenderAlpha write the low byte of `ctx+0xC4`, which `SetColorTable` (`0x0082FF70`) fills from the target
  buffer's vfunc `+0x78(r, g, b)`. Is that a palette index, a grey level or an alpha value, and does it share a
  meaning with the blit/gradient bytes (which are pure source alpha, section 7.2)? Needs the type-2 buffer class.
  **ANSWERED 2026-10-21 (re-span8-bridge4):** DIFFERS — section 10.4: every shipped cIGZBuffer colour mapping (+0x78: 0x00825DA0, 0x0082AD30) returns 0 for colour type 2, so SetColorTable leaves ctx+0xC4 = 0 and RenderSolid/RenderAlpha store 0x00 whatever the colour; only blit (texel alpha) and gradient bytes can be non-zero.

<!-- from: glue-span8-impl4 (2026-10-22); coordinator scope clarification retained -->
- ~~**SR10 - sections 1..7, core-to-plugin ABI prerequisite:** which public C ABI is approved for the landed
  `engine/core/src/span_render8` implementation? `engine/capi/src/lib.rs`, all CAPI source modules and
  `engine/capi/include/fc4.h` expose no span8 adapter or exports. The bridge needs solid, gradient, alpha,
  alpha-gradient and blit entry points with bounded destination origin/signed stride, the 0x30-byte gradient,
  source descriptor and blit record contracts, error-before-write results, and alias handling preserving
  ascending read/write order. The Rust core has these operations, including `render_blt_aliased`, but cannot
  be called by the C++ plugin through an existing export. [Stop the replacement path rather than invent an
  unreviewed ABI; retain SR8-3 row13 refusal and SR8-2 reachability limits when this prerequisite lands.]~~
  **ANSWERED 2026-10-22 (coordinator):** MATCHES — the glue task includes implementing the missing C adapter for the landed `engine/core/src/span_render8` operations. This is our public ABI design, not an unknown native-game contract. Define bounded, checked exports in `engine/capi` and document them in `fc4.h`, with the train consolidating the ABI bump. Preserve the published native behavior, signed-stride/origin bounds, gradient/source descriptors, error-before-write guarantees and ascending alias order; validate those boundaries with meaningful tests. Native-game layouts, virtual calls and calling conventions still require reconciled evidence under SR9 before enabling or exercising the bridge. This clarification approves no guessed native ABI and provides no replacement credit. SR8-2 reachability and SR8-3 row13 restrictions remain. Spec §9 records this scope clarification.

<!-- from: glue-span8-impl4 (2026-10-22) -->
- ~~**SR9 - section 10.1 / 10.5, reconciled native ABI prerequisite:** can the ABI owner publish the six
  `cSpanRender8` vtable `0x00AC188C` rows (slots 0..5, thiscall, callee pops 4/16/16/20/16/24;
  destructor returns this in EAX, render slots return void) and reconcile the native-context rows before this
  bridge is enabled? `docs/abi/interfaces.tsv` has no rows for this renderer, `cGZDrawContextRenderToBuffer`,
  `cGZDrawSystem` or `cGZBuffer`. The 17 rows in `docs/abi/proposals/re-span-context4.tsv` are explicitly
  unapplied per its adjacent report; in particular slot 22 still says `unknown` while section 10.5 says void.
  The constructed route needs draw-system slot 4 (pop 8, AL), context slots 3/4/7/8/9/12/13/14/15/22
  (pops 4/0/12/4/4/12/24/0/4/8), and buffer Init/Lock/Unlock plus the descriptor getters specified in
  section 10.3. [Stop this path under the lane's no-unreconciled-ABI instruction; no table edits, new native
  forwarders or fixture calls. Existing sibling infrastructure is unchanged.]~~
  **ANSWERED 2026-10-22 (re-span8-abi4):** DIFFERS — §§10.1, 10.5, 10.7 confirm the six renderer rows and public setter contracts, strengthen the earlier proposal's context slot-22 unknown return to void (pop 8; eight native exits; callers ignore EAX), and specify the used buffer lifecycle/descriptor calls. The bounded 40-row proposal is `build-runs/abi-rows-re-span8-abi4.tsv` in worktree re-span8-abi4; exact-key reconciliation finds all 40 absent from interfaces.tsv, with 14 overlaps against the unapplied earlier proposal (13 matching, slot 22 strengthened). This resolves native contract uncertainty, not ABI application: coordinator review and gated absent-key application remain required before enabling or exercising the bridge. No native row remains unknown within this bounded route; SR8-2 and SR8-3 remain open.
