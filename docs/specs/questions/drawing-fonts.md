# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN18 - section 7, advance conversion domain:** FN16 specifies finite discriminating inputs but does not publish `_ftol2`'s result for NaN/infinite scales or a rounded product outside signed i64. What low-word value is stored in the loaded-glyph and missing-box branches on those paths (including the narrow-band space adjustment), and are unmasked x87 exceptions part of the contract? [Core emulates finite products and all four rounding controls exactly when the truncated integer fits i64; conversion failures are explicit typed errors, and the legacy font entry points panic before inventing a stored advance. No coverage claim for these unpublished cases.]

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN19 - section 9, slot 52 paragraph result and pen:** The signature lists a pen argument but the paragraph body describes only line placement and does not state a return rule. Does it pass pen through to each DrawTextLine (thus overriding computed alignment), when is pen written, and does it propagate line-lock failure or always return initialised? [Existing core entry point has no paragraph pen, returns false on line failure and otherwise initialised. Retained provisionally; no complete-owner claim for this slot. Trailing CR/LF are trimmed as converted characters, the literal published rule, including UTF-16.]

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN20 - section 10, AA helper floating precision:** The AA table rule describes float temporaries and truncation of v*255 but not x87 PC/rounding controls, intermediate stores, or the foreign pow/log call precision. Does FN16's ambient PC24/53/64 rule also affect this helper, and at exactly which rounding/store points? [Existing literal Rust float temporaries and double final product retained; tests exercise published examples only, not an exact-native rounding claim.]

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN21 - section 6, incomplete input:** What are the observable return/output rules for odd byte-length UTF-16 and a truncated multibyte sequence (including source-length mapping)? [Existing safe core treats a lone UTF-16 byte as a zero-extended low byte and bounds the foreign multibyte length to available input; outside the published valid-input contract, no exact-owner claim.]

<!-- from: core-fonts-contract-complete4 (2026-10-22) -->
- **FN22 - sections 8.1/8.4, valid backing storage and progress domain:** Are nonnegative advances, a readable unit beyond a three-unit Thai cluster, and positive progress after Thai map-back required caller preconditions? The spec explicitly documents no bounds/progress guard; what public input domain can a bounded portable API faithfully promise, and should native nontermination/fault be exposed as a typed fault outside it? [Existing core bounds the mode-2 walk at text end, treats unavailable Thai lookahead as zero, and stops line counting on zero progress. Backing units supplied beyond logical n are honoured by thai_cluster_len. These paths remain explicit residuals, not native-owner equivalence.]
