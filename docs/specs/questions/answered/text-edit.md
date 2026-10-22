# text-edit: answered questions

<!-- from: re-textedit4 (2026-10-22) -->
- **TE2 - section 2:** What does primary hook +0x26C do for reasons 0, 1 and other editing failure reasons?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — section 10 specifies native 0x009BD9F2 as a sound-service adapter, indexing W+0x27C+4*reason, acquiring and playing through RZ::SS, releasing its temporary handle on success/failure, and returning false for missing ID/service. Its folded font-setter name is misleading. No error-dialog behavior is established.
