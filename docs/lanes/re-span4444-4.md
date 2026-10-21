| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span4444-4 | workflow / `re-span4444-4` | **ready** (specs only, no ABI rows): `docs/specs/span-render-4444.md` for `nGZDrawSoftRender::cSpanRender4444` (0x00847CE0-0x00854E10, 172 fn, 51,809 B); round 2 closed U4: all 168 blit kernels verified by execution | coverage "specified" for the class; optional: extend the check to U7 ranges |

## Report (round 2, 2026-10-21)

**What changed.** Spec section 9.5 rewritten, U4 closed, U6 extended, U7 added, section 11 updated.
- Byte identity: Capstone disassembly of `bin/SimCity4.exe` vs export instructions for all 168 table entries:
  0 mismatches; 91 in-loop alignment fillers (`lea r,[r]`, `nop`, `mov edi,edi`) are absent from the export.
- Normalized text comparison (ordinal labels, immediates/widths kept): no two kernels identical, row-to-row diffs
  5-91 lines from register allocation/scheduling, so a textual template proof is not possible. Constant inventory
  per row matches the format (documented in 9.5 item 2).
- Execution check: x86 interpreter for the 35 mnemonics used, kernels run on their real bytes against the spec
  model 9.1-9.4: random 150 trials/kernel, 168/168 kernels, 747,927 pixels, 0 mismatches, ESP and callee-saved
  registers preserved; exhaustive texel sweep 25 kernels (cols 0-3 of rows 0/2/4/6/12, col 0 of keyed rows
  1/3/5/7/13), 1,312,000 pixels, 0 mismatches. Mutation sensitivity: five single-rule model changes each fail
  39-78 kernels (12/12 for the key-mask mutation).
- No rule of sections 1-9.4 changed: five methods, 14x12 table, conversions and rounding stand as written.

**Unknowns left.** U1-U3, U5, U6 (caller meaning), U7 (input ranges the check did not exercise: spans >25 px,
stretch steps >=1.0 or accumulator wrap, tiled `x+u0` wrap, width/height 0, negative strides).

**Questions.** None filed (no ambiguous contract found).

**Tools.** REA not used. Scripts (ignored, build-runs/): norm.py, emu.py, check.py (`MUT=n`, `EXH=1`).

**Commits.** 935fc914 spec verification, 408968d1 handoff, final commit with this report.

## Report (round 1)

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
