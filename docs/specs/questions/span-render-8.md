# span-render-8: open questions
<!-- Questions about [span-render-8.md](../span-render-8.md). Append new entries at the END; when one is answered, move it to answered/span-render-8.md. Rules: README.md. -->

<!-- from: re-span8-4 (2026-10-21) -->
- **SR8-1 - sections 2, 3, 6:** what byte does a colour-type 2 (8 bpp) target store for a solid colour? RenderSolid
  and RenderAlpha write the low byte of `ctx+0xC4`, which `SetColorTable` (`0x0082FF70`) fills from the target
  buffer's vfunc `+0x78(r, g, b)`. Is that a palette index, a grey level or an alpha value, and does it share a
  meaning with the blit/gradient bytes (which are pure source alpha, section 7.2)? Needs the type-2 buffer class.
- **SR8-2 - whole spec:** does any shipped path create a colour-type 2 render target (`Init` `0x00830710`, jump
  table `0x008308B0` entry type 2 → vtable `0x00AC188C`)? No caller analysis was done; if one exists, name the
  caller and what it draws (texture formats, kinds), so the reachable slots of section 7 can be listed.
