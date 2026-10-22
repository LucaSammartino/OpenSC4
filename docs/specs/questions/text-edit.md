# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE5 - sections 7-8; text-edit-layout.md and text-edit-adapters.md:** **PARKED final round 3/3; no fourth round.** Owner paths now proved: pages measure current X while arrows reuse stored X; cache survives all four vertical success paths; single-line strict three-quarter threshold; complete owner AddScrollBar ordering/failure; SelectCurrentWord and insertion-repair asymmetry; composition owner's erase/previous-value loop. Exact residual: GetPreviousUTF16Character 0x0091163F, GetUTF16Character 0x009115F0 and UTF16GetThaiCharLevel 0x00911A40 results for each selector/malformed/context case, plus composed-length helpers, are foreign inputs. Do not infer their contracts from names or promote their owners. Preserve this worktree; no restart.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** Attribution is reconciled in text-edit-attribution.md: actual coverage assigns 150 bodies / 19,999B, including the 134 direct entries and sixteen measured thunks/helpers. Complete remaining U/B checklist entries (thin text/caption adapters, color/outline bounds, exact line/index conversion, scroll helpers, image setters and generated action helpers) from native bodies before complete-owner credit. [partial specification only; no whole-owner or widget-family credit]
