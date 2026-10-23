# font-renderer: open questions
<!-- Questions about [font-renderer.md](../font-renderer.md). Append at the END; answered entries move to answered/font-renderer.md. -->

<!-- from: re-font-renderer4 (2026-10-23) -->
- **FR1 - section 2, slots 3 and 4:** What interface-level operations and argument types do the two one-dword, folded true-return slots name? [The small renderer ignores the argument and returns true in both; semantic names are deliberately unknown. No inference from the folded body's unrelated auto-label.]

<!-- from: re-font-renderer4 (2026-10-23) -->
- **FR2 - sections 3 and 5, source/palette backing domain:** Which original producers guarantee byte coverage below 128 for the font's 128-entry palette, and sufficient source backing when record source dimensions differ from destination extents? [The renderer demonstrably trusts destination extent and raw byte palette index, without source-bound clipping or 127 clamp. The literal renderer contract requires addressed storage; producer/cache algorithms remain foreign and are not reopened in this lane.]

<!-- from: re-font-renderer4 (2026-10-23) -->
- **FR3 - section 1, inherited owner accounting:** Which exact bodies constitute the inherited 4,444-byte cGZDrawFontRenderer label, after separating class 8B3255F9/450-byte holder from class 08CDA30B/8-byte renderer? [This spec audits 12 small-renderer-path bodies totaling 1,666 bytes plus two shared bodies totaling 12; it claims neither a whole-owner reassignment nor the holder's remaining canvas/cache algorithms. That broader accounting is deliberately unresolved.]
