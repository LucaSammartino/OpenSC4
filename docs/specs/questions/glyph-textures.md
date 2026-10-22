# glyph-textures: open questions
<!-- Questions about [glyph-textures.md](../glyph-textures.md). Append new entries at the END; when one is answered, move it to answered/glyph-textures.md. Rules: README.md. -->

<!-- from: re-glyphs (2026-10-19) -->
- **GT1 - §0, §1.3:** what `RZ::WCM()` (`0x00913C72`) returns, and whether its slot 5 is a font lookup by ID as this
  spec infers.

<!-- from: re-glyphs (2026-10-19) -->
- **GT2 - §1.2:** which framework services `Init` registers the glyph manager with through framework slots 3 and 9
  (`0x005D7D50`); the tick list is inferred from `OnTick`.


<!-- from: core-glyph-textures4 (2026-10-22) -->
- **GT4 - complete owner/body inventory and lifecycle:** the brief assigns 43 functions/4475B to cSC4GlyphTextureManager, cSC4GlyphTextureInfo and cSC4StringTextureInfo, but the published spec does not enumerate every body. Please reconcile the exact inventory, including manager constructor/destructor (`0x005D8900`, deleting wrapper `0x005D8D40`), initial reference counts/last-sweep time, system-service slots 3–7/11–12, auxiliary-view slots 4–5, message-target QI/refcount thunks, and glyph/string slot 0 if present. What destructor cleanup order and flags behavior apply? Does manager-link assignment AddRef the new manager before releasing the old, and what does glyph-map removal do if its key is absent? [Core implements published paths only; explicit caller-supplied initial manager references, zero initial sweep time, provisional shutdown-then-cache context teardown; no complete-manager or 43-body credit claimed.]

  **Partial proof 2026-10-22 (re-small-renderer-contracts4):** §1.1.1 establishes constructor at 005D8CA0, initial refs/sweep=0, inherited slots/thunks, storage-only destructor distinct from Shutdown, acquire-before-release manager assignment, absent-pointer removal no-op; §3.1 establishes info slot 0. Residual: exact legacy 43/4475 inventory scope versus 38 named bodies/4047B plus shared/unnamed slots; no complete-manager count claim.
