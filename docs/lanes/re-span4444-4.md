| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span4444-4 | workflow / `re-span4444-4` | in progress (round 2: U4 structural proof): new `docs/specs/span-render-4444.md` for `nGZDrawSoftRender::cSpanRender4444` (0x00847CE0-0x00854E10, 172 fn, 51,809 B) | line-by-line read of the 144 sweep-only kernels (U4); coverage "specified" for the class |

## Handoff
- Goal (round 2): resolve U4 (144 sweep-only kernels) by normalized structural comparison of export instructions.
- Done round 1: 62184d4d spec sections 1-11 (see git log). Round 2 started at 62184d4d.
- Next: build-runs/norm.py (normalize kernels, diff vs fully-read references), then spec 9.5/U4 update.
- Facts: table rows at 0x00AC2100, kernels from instructions.asm; cls.py = old feature sweep (weak).

## Report

**What changed.** New spec `docs/specs/span-render-4444.md` (sections 1-11):
- Class/vtable `0x00AC185C`, creation by `cGZDrawContextRenderToBuffer::Init` for colour type 7; methods take all
  inputs as arguments.
- Span record (`(y<<16)|x` start/end, exclusive), target descriptor (base, stride), no clipping, count>=1 loop.
- RenderSolid (16-bit fill), RenderAlpha (`a = ((alpha>>27)+1)>>1`, 0..16; `floor((16d + a(s-d) + 8)/16)`,
  alpha out 0xF), RenderGradient (8.16 B/G/R ramps, truncation to top nibble, B step loses low 4 bits, alpha
  nibble = top of R accumulator), RenderAlphaGradient (alpha weight `acc*0x88 >> 27` = floor(A*17/256)).
- RenderBlt: `table[src.format*2 + keyed][mode]`, row pointers `0x00AC2100`, 168 entries `0x00AC1E60-0x00AC20FF`.
  Rows: 7 source formats (4444, 1555, X555, 565, 8888, X888, A8) x keyed; columns: {direct, stretch, tiled} x
  {copy, constant alpha, modulate, modulate+alpha}, matched to the Mac `cBlitter` method names.
- Conversions (truncate to 4444; bit-replicate to 8888), op formulas (constant alpha weight 0..15 so opaque
  constant alpha gives 14 for 15-over-0; modulate `(C*K+128)>>8`), colour-key masks per format and the A8
  degenerate key (key 0 draws nothing).
- Two table entries are folded copies living in the 1555/565 ranges (`0x00833730`, `0x00842C50`); range =
  5 methods + 166 kernels + helper `0x0084BD40`.

**Measured.** 172 functions in range (functions.tsv), 168 distinct table entries; read in full: 5 methods, the
helper, ~24 kernels; the other 144 kernels by a feature sweep (build-runs/cls.py, not committed).

**Questions.** None filed. Unknowns U1-U6 in the spec (count 0, R overflow into alpha in RenderGradient, unchecked
indices, sweep-only kernels, unclamped stretch, unread descriptor fields).

**Tools.** REA not used (export + raw image table reads sufficed).

**Commits.** fc5a4174 lane file, 7d4cf8d5 methods + dispatch, e0706d70 kernels, final commit with this report.
