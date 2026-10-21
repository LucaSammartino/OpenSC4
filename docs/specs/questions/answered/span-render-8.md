# span-render-8: answered questions (archive)
<!-- Archive for [span-render-8.md](../../span-render-8.md). Lanes do not read this file. Open questions are in ../span-render-8.md. -->

<!-- from: re-span8-4 (2026-10-21) -->
- **SR8-1 - sections 2, 3, 6:** what byte does a colour-type 2 (8 bpp) target store for a solid colour? RenderSolid
  and RenderAlpha write the low byte of `ctx+0xC4`, which `SetColorTable` (`0x0082FF70`) fills from the target
  buffer's vfunc `+0x78(r, g, b)`. Is that a palette index, a grey level or an alpha value, and does it share a
  meaning with the blit/gradient bytes (which are pure source alpha, section 7.2)? Needs the type-2 buffer class.
  **ANSWERED 2026-10-21 (re-span8-bridge4):** DIFFERS — section 10.4: every shipped cIGZBuffer colour mapping (+0x78: 0x00825DA0, 0x0082AD30) returns 0 for colour type 2, so SetColorTable leaves ctx+0xC4 = 0 and RenderSolid/RenderAlpha store 0x00 whatever the colour; only blit (texel alpha) and gradient bytes can be non-zero.
