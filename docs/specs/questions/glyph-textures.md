# glyph-textures: open questions
<!-- Questions about [glyph-textures.md](../glyph-textures.md). Append new entries at the END; when one is answered, move it to answered/glyph-textures.md. Rules: README.md. -->

<!-- from: re-glyphs (2026-10-19) -->
- **GT1 - §0, §1.3:** what `RZ::WCM()` (`0x00913C72`) returns, and whether its slot 5 is a font lookup by ID as this
  spec infers.

<!-- from: re-glyphs (2026-10-19) -->
- **GT2 - §1.2:** which framework services `Init` registers the glyph manager with through framework slots 3 and 9
  (`0x005D7D50`); the tick list is inferred from `OnTick`.

<!-- from: core-glyph-textures4 (2026-10-22) -->
- **GT3 - §§1.2–1.5 numeric boundaries:** please publish the input/output integer widths and signedness for font dimensions, `P(n)` at zero/overflow, the point of 16-bit narrowing versus UV division, and whether cache bytes use stored string dimensions or render-result dimensions. What precision, rounding and zero-budget behavior apply to `B/L > 1.1` and `B - 0.1L`, and are budget multiplication/byte counters wrapping? [Core uses unsigned u32 metrics, checked P(width+2), u16 assignment afterwards; UVs divide by original renderer dimensions; string stored dimensions for w*h*4; f64 target truncated after subtraction, IEEE zero-budget division, wrapping u32 field arithmetic. Unrepresentable P is an explicit error, not a claimed native result.]

<!-- from: core-glyph-textures4 (2026-10-22) -->
- **GT4 - complete owner/body inventory and lifecycle:** the brief assigns 43 functions/4475B to cSC4GlyphTextureManager, cSC4GlyphTextureInfo and cSC4StringTextureInfo, but the published spec does not enumerate every body. Please reconcile the exact inventory, including manager constructor/destructor (`0x005D8900`, deleting wrapper `0x005D8D40`), initial reference counts/last-sweep time, system-service slots 3–7/11–12, auxiliary-view slots 4–5, message-target QI/refcount thunks, and glyph/string slot 0 if present. What destructor cleanup order and flags behavior apply? Does manager-link assignment AddRef the new manager before releasing the old, and what does glyph-map removal do if its key is absent? [Core implements published paths only; explicit caller-supplied initial manager references, zero initial sweep time, provisional shutdown-then-cache context teardown; no complete-manager or 43-body credit claimed.]

<!-- from: core-glyph-textures4 (2026-10-22) -->
- **GT5 - §1.6 restore outputs/failure:** does restore change only the glyph binding or also glyph/texture dimensions and UVs? Is map order the re-render order? After releasing the old binding, what happens if font lookup or rendering fails? What source rectangle is passed? [Literal core: clear string cache first, ordered map traversal, release old binding before lookup/render, unit source rectangle, replace binding on success and leave it empty on failure, retain metadata. No factory leaves glyphs untouched.]

<!-- from: core-glyph-textures4 (2026-10-22) -->
- **GT6 - §§1.3–1.4, 2–3 output/reference and invalid-input contracts:** does glyph slot 7 return a borrowed binding or AddRef it; what are glyph/string slot 0 QI IDs and output mutation rules on failed lookups/renders; what happens on a null font in GetCachedStringTexture (specified always true); how does StringToGlyphTextureArray handle GetCharLength returning 0 or a length beyond remaining bytes; and does an initial empty source string attempt a glyph? [Core exposes a borrowed binding getter, token-returning lookups, explicit MissingFont error for strings, stops at invalid character boundaries, and makes no glyph call for empty input. No native QI/failure-output contract invented.]
