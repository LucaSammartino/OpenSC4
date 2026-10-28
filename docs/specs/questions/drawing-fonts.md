# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN20 - section 10, AA helper floating precision:** The AA table rule describes float temporaries and truncation of v*255 but not x87 PC/rounding controls, intermediate stores, or the foreign pow/log call precision. Does FN16's ambient PC24/53/64 rule also affect this helper, and at exactly which rounding/store points? [Existing literal Rust float temporaries and double final product retained; tests exercise published examples only, not an exact-native rounding claim.]

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN21 - section 6, incomplete input:** What are the observable return/output rules for odd byte-length UTF-16 and a truncated multibyte sequence (including source-length mapping)? [Existing safe core treats a lone UTF-16 byte as a zero-extended low byte and bounds the foreign multibyte length to available input; outside the published valid-input contract, no exact-owner claim.]
  PARTIAL 2026-10-27 (re-x-ui): the converting routines count units (UTF-16 copies `2n` bytes, so an odd length cannot occur); UTF-8 decodes one unit per sequence and stops silently at the first invalid or truncated sequence, keeping the units decoded so far; `Find` counts bytes in every encoding (section 6). Other multi-byte code pages and the system code page were not traced.

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN22 - sections 8.1/8.4, valid backing storage and progress domain:** Are nonnegative advances, a readable unit beyond a three-unit Thai cluster, and positive progress after Thai map-back required caller preconditions? The spec explicitly documents no bounds/progress guard; what public input domain can a bounded portable API faithfully promise, and should native nontermination/fault be exposed as a typed fault outside it? [Existing core bounds the mode-2 walk at text end, treats unavailable Thai lookahead as zero, and stops line counting on zero progress. Backing units supplied beyond logical n are honoured by thai_cluster_len. These paths remain explicit residuals, not native-owner equivalence.]

<!-- from: re-fonts (2026-10-23) -->
- **FN23 - section 15:** the glyph-cache holder (class `0x8B3255F9`, labelled `cGZDrawFontRenderer`) in full:
  its state, the cache surface and table, the colour table per pixel format, the flush rule's caller contract and
  the hardware-surface path (its owner accounting is font-renderer.md FR3). [summary]
