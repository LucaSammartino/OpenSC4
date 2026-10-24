# nation: open questions
<!-- Questions about nation.md. Append new entries at the end. -->

<!-- from: core-nation10 (2026-10-24) -->
- **NQ1 - §1/§4, Init and filesystem methods:** please specify directory discovery/order, preference matching, AddNewRegion/DeleteRegion arguments, return values and failure/rollback behavior, tutorial reset/copy behavior, repeated Init/Shutdown behavior, and shutdown catalog cleanup. The slot names alone do not define these algorithms. [Core stores a supplied catalog and exposes directory/list getters; discovery, preference matching, add/delete and tutorial reset explicitly return Unsupported. Init/Shutdown implement the specified message subscriptions and once-per-process tuning only, without duplicate subscriptions/unsubscriptions. These owner behaviors remain open pending a spec.]

<!-- from: core-nation10 (2026-10-24) -->
- **NQ2 - §4, SetActiveRegion/GetRegion:** what are the initial active index, invalid-index behavior, same-index selection behavior, and activation/deactivation failure semantics? [Caller supplies the initial optional active index; invalid indices return an error without mutation; a valid selection deactivates the old region then activates the new, even for the same index. Activation callbacks are infallible pending the contract.]

<!-- from: core-nation10 (2026-10-24) -->
- **NQ3 - §2 and demand §11.2:** for an explicitly empty ID list or absent ID 0 with an unknown demand ID, and for a difficulty vector shorter than the city's difficulty index, what safe replacement result is intended? The spec defines missing-property defaults and empty-curve zero, but the remaining cases describe undefined original behavior. [Return MissingCurve or InvalidDifficulty; do not fabricate a fallback curve or read beyond the vector. Missing properties use the explicit FreeCity4 defaults from demand §11.2.]
