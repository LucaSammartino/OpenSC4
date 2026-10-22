# text-edit: answered questions

<!-- from: re-textedit4 (2026-10-22) -->
- **TE4 - section 6:** Reconcile insertion-action quarter eviction and deletion record creation's absent limit check; does maximum zero keep growing?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — insertion recording erases the oldest floor(n/4) actions when n >= configured unsigned limit, then appends; deletion recording appends without checking that limit. Native range-erase/copy/destruction helpers establish survivor order and owned-string cleanup. Zero does not disable recording: insertion-only growth from empty stabilizes at four, while deletion-only growth is unbounded by this setting. Setter alone never prunes. See section 6 and four independent added vectors.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE2 - section 2:** What does primary hook +0x26C do for reasons 0, 1 and other editing failure reasons?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — section 10 specifies native 0x009BD9F2 as a sound-service adapter, indexing W+0x27C+4*reason, acquiring and playing through RZ::SS, releasing its temporary handle on success/failure, and returning false for missing ID/service. Its folded font-setter name is misleading. No error-dialog behavior is established.
