# nation: answered questions

<!-- from: core-nation10 (2026-10-24) -->
- **NQ1 - §1/§4, Init and filesystem methods:** please specify directory discovery/order, preference matching, AddNewRegion/DeleteRegion arguments, return values and failure/rollback behavior, tutorial reset/copy behavior, repeated Init/Shutdown behavior, and shutdown catalog cleanup. The slot names alone do not define these algorithms. [Core stores a supplied catalog and exposes directory/list getters; discovery, preference matching, add/delete and tutorial reset explicitly return Unsupported. Init/Shutdown implement the specified message subscriptions and once-per-process tuning only, without duplicate subscriptions/unsubscriptions. These owner behaviors remain open pending a spec.]

**ANSWERED 2026-10-24 (re-nation-nq3):** DIFFERS — nation.md §§6–8 specify immediate-root discovery, Downloads exclusion, template copying, preference matching, creation/deletion, tutorial reset and partial failures. Repeated Init is a no-op, but Shutdown releases the catalog and curves; subsequent Init rereads tuning. Only directory StaticInit is process guarded. §5.1 supplies conformance vectors.

<!-- from: core-nation10 (2026-10-24) -->
- **NQ2 - §4, SetActiveRegion/GetRegion:** what are the initial active index, invalid-index behavior, same-index selection behavior, and activation/deactivation failure semantics? [Caller supplies the initial optional active index; invalid indices return an error without mutation; a valid selection deactivates the old region then activates the new, even for the same index. Activation callbacks are infallible pending the contract.]

**ANSWERED 2026-10-24 (re-nation-nq3):** DIFFERS — nation.md §7: constructor index 0; Init chooses preferences or its first/tutorial candidate; same-index selection does nothing. A switch ignores callback failures and leaves the new index selected. Original indexing is unchecked; replacement InvalidRegionIndex precedes effects. §5.1 tests each case.

<!-- from: core-nation10 (2026-10-24) -->
- **NQ3 - §2 and demand §11.2:** for an explicitly empty ID list or absent ID 0 with an unknown demand ID, and for a difficulty vector shorter than the city's difficulty index, what safe replacement result is intended? The spec defines missing-property defaults and empty-curve zero, but the remaining cases describe undefined original behavior. [Return MissingCurve or InvalidDifficulty; do not fabricate a fallback curve or read beyond the vector. Missing properties use the explicit FreeCity4 defaults from demand §11.2.]

**ANSWERED 2026-10-24 (re-nation-nq3):** MATCHES — nation.md §2.1 adopts MissingCurve for a missing requested/default curve and InvalidDifficulty for a city index outside an explicitly supplied vector. MissingCurve takes precedence; no city means no difficulty access. Explicit empty data is not missing-property data; existing defaults and empty-curve zero remain. §5.1 gives numeric/error vectors.
