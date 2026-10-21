| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span1555-4 | workflow / `re-span1555-4` | **ready** (specs only, no ABI rows): new `docs/specs/span-render-1555.md` — the five cSpanRender1555 methods exact, 14x12 RenderBlt table mapped, 9 blit bodies exact; questions SR1555-1..4 | SR1555-3/4: restate remaining blit bodies (keyed non-A1555 rows, alpha columns per source, tiled addressing) |

## Report

**What changed.** New spec `docs/specs/span-render-1555.md` for `nGZDrawSoftRender::cSpanRender1555`
(owner range `0x008310B0`-`0x0083DE50`, 172 functions, 50 817 bytes, colour types 4 and 6), new questions file
`docs/specs/questions/span-render-1555.md`, `docs/specs/QUESTIONS.md` regenerated. Commits `72e60d44`,
`36e028f6`, `9f3bc959`, plus this report.

**Exactly specified (every instruction read):**
- RenderSolid `0x008310B0` (+ fill helper `0x0090FFC8`), RenderAlpha `0x00831200`, RenderGradient `0x00831100`,
  RenderAlphaGradient `0x008312D0`, RenderBlt `0x00831470`: span/surface records, end-exclusive spans,
  `n = 0` underflow, alpha weight mappings (two different ones: `a8·0x818180 >> 26` vs `V·0x84 >> 26`, with
  threshold tables), rounded 5-bit lerp `floor((w·S + (32−w)·D + 16)/32)`, alpha bit (Solid verbatim, Alpha /
  AlphaGradient / blends force 1, Gradient forces 0), the 1555-only gradient quirk (start not shifted, unlike 565).
- Caller `DrawPreclippedSpanList` `0x008302E0`: method selection, source-format index per colour type, colour-key
  masks per format, `kind = alpha + 2·modulate (+8 tiled | +4 stretch)`; type 3 and unknown types draw nothing;
  row 13 (8-bit keyed) unreachable.
- Full 14x12 table with addresses; two slots folded into the 4444/565 ranges.
- Blit bodies exact: A1555 copy and keyed copy, all 7 source conversions (col 0), X555 Modulate /
  ConstantAlpha / ModulateAlpha, A8888 ConstantAlpha, X555 Stretch, blend helper `0x0083C490`.

**Unverified (marked [table]):** the remaining ~155 instantiations (other rows' alpha/modulate columns, keyed rows
other than A1555, all Tiled bodies and the stretch bodies of other rows).

**Tools:** REA was unavailable ("No app is open"); all work from `re_query.py asm` and raw `.exe` data reads
(`build-runs/dd.py`), Mac template names from `re/symport/mac_features.pkl`.

**Questions:** SR1555-1 (gradient record producer/scale), SR1555-2 (is a 1555 gradient ever drawn), SR1555-3
(remaining bodies), SR1555-4 (tiling/stretch caller parameters).

**Next:** SR1555-3/4 against bytes (`build-runs/core.sh <addr>` prints a compact loop listing).
