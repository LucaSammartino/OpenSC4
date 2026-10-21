| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-span4444-4: pure-Rust span render 4444 core (nGZDrawSoftRender::cSpanRender4444) from docs/specs/span-render-4444.md | `core-span4444-4` / (engine lane) | ready, no ABI change; core commit `d9d84a7e`; gate cargo 3662/0, clippy 0, ctest 62/62; no game runs (core lane); SP1-SP2 filed | 1. coordinator: regenerate QUESTIONS.md (tools/questions.py index) for new prefix SP (span-render-4444). 2. coverage map: spanrender4444 core path `engine/core/src/span_render4444`. 3. glue only once a real consumer (soft render-to-buffer context) exists. |

## Report

What changed: new module `engine/core/src/span_render4444/` (mod.rs types/blend, methods.rs slots 1-4, blt.rs slot 5 +
kernel table, tests.rs 26 tests) and one `pub mod` line in `engine/core/src/lib.rs`. No capi, plugin, CMake or ABI change.

Implemented (whole owner, by the spec's model): RenderSolid (section 3), RenderGradient (5, incl. B >> 4 drift and
R carry into alpha, U2), RenderAlpha (4, weight `((alpha>>27)+1)>>1`), RenderAlphaGradient (6, weight `acc>>27`
with acc = A*0x88), RenderBlt dispatch (7, row `format*2+flag` added literally, column `geometry*4+op`) and all 168
table entries as one parameterised kernel (8, 9: seven formats x keyed, direct/stretch/tiled, copy/constant
alpha/modulate/modulate+alpha, 9.2 truncation, 9.3 replication, `(C*K+128)>>8`, `+8` blend rounding, per-format
key masks incl. A8 "key == 0").

Bounded API (documented choices, not verified original behaviour): byte slices + origin + signed byte stride,
little-endian pixels; every destination and source address is checked before any write (all-or-nothing).
Errors: EmptySpanList (U1 count 0), DestOutOfBounds / SourceOutOfBounds (U5, wrapped widths), BadKernelIndex
(U3: row >= 14 or mode >= 12), ZeroTileSize (U7 DIV fault). Wrapping 32-bit maths kept for gradients, stretch
accumulators and tiled `x + u0`.

Tests: worked example 0xF842, weight maps both ways, exhaustive blend for a 0..16, op-1 cap (15 over 0 = 14),
gradient truncation/steps/absolute xy/U2 overflow, every 9.2/9.3 conversion, modulate rounding, key masks per
format, A8 keyed degenerate in all 12 columns, stretch half-texel walk with absolute x, tiled wrap and 2^32 wrap,
168-entry table bijection, odd flag byte, untouched neighbours and untouched target on error.

Questions: SP1 (blend bit result for a > 16 in RenderAlphaGradient), SP2 (spans running past the row end) in
docs/specs/questions/span-render-4444.md. QUESTIONS.md not regenerated (tools/ off limits for this lane).
