# glyph-textures: answered questions

<!-- from: core-glyph-textures4 (2026-10-22) -->
~~- **GT3 - §§1.2–1.5 numeric boundaries:** please publish the input/output integer widths and signedness for font dimensions, `P(n)` at zero/overflow, the point of 16-bit narrowing versus UV division, and whether cache bytes use stored string dimensions or render-result dimensions. What precision, rounding and zero-budget behavior apply to `B/L > 1.1` and `B - 0.1L`, and are budget multiplication/byte counters wrapping? [Core uses unsigned u32 metrics, checked P(width+2), u16 assignment afterwards; UVs divide by original renderer dimensions; string stored dimensions for w*h*4; f64 target truncated after subtraction, IEEE zero-budget division, wrapping u32 field arithmetic. Unrepresentable P is an explicit error, not a claimed native result.]~~

**ANSWERED 2026-10-22 (re-small-renderer-contracts4):** DIFFERS — §3.2: signed P and metric interpretation, low-u16 storage, signed original dimensions for UVs, stored dimensions for cost, float32 constants/rounded B with x87 target arithmetic and wrapping counters.

<!-- from: core-glyph-textures4 (2026-10-22) -->
~~- **GT5 - §1.6 restore outputs/failure:** does restore change only the glyph binding or also glyph/texture dimensions and UVs? Is map order the re-render order? After releasing the old binding, what happens if font lookup or rendering fails? What source rectangle is passed? [Literal core: clear string cache first, ordered map traversal, release old binding before lookup/render, unit source rectangle, replace binding on success and leave it empty on failure, retain metadata. No factory leaves glyphs untouched.]~~

**ANSWERED 2026-10-22 (re-small-renderer-contracts4):** DIFFERS — §3.2: map-order restore clears old binding and supplies its slot directly to the renderer even on failure, without metadata updates; no manager-level rollback.

<!-- from: core-glyph-textures4 (2026-10-22) -->
~~- **GT6 - §§1.3–1.4, 2–3 output/reference and invalid-input contracts:** does glyph slot 7 return a borrowed binding or AddRef it; what are glyph/string slot 0 QI IDs and output mutation rules on failed lookups/renders; what happens on a null font in GetCachedStringTexture (specified always true); how does StringToGlyphTextureArray handle GetCharLength returning 0 or a length beyond remaining bytes; and does an initial empty source string attempt a glyph? [Core exposes a borrowed binding getter, token-returning lookups, explicit MissingFont error for strings, stops at invalid character boundaries, and makes no glyph call for empty input. No native QI/failure-output contract invented.]~~

**ANSWERED 2026-10-22 (re-small-renderer-contracts4):** DIFFERS — §§3.1–3.2: both QI slots always fail with untouched output, binding getter borrows, glyph failure preserves output, null string font is dereferenced, character lengths are trusted and empty input makes no glyph request.
