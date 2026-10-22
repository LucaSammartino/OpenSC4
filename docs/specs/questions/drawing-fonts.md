# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN18 - section 7, advance conversion domain:** FN16 specifies finite discriminating inputs but does not publish `_ftol2`'s result for NaN/infinite scales or a rounded product outside signed i64. What low-word value is stored in the loaded-glyph and missing-box branches on those paths (including the narrow-band space adjustment), and are unmasked x87 exceptions part of the contract? [Core emulates finite products and all four rounding controls exactly when the truncated integer fits i64; conversion failures are explicit typed errors, and the legacy font entry points panic before inventing a stored advance. No coverage claim for these unpublished cases.]
