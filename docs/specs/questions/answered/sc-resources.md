# sc-resources: answered questions

<!-- from: re-persist-sl5 (2026-10-23) -->
- ~~**SCR6 (§8):** Audit constructor defaults, empty-stack OutputStream behavior,
  array separators, complete graph/reference literals, and surveyor creation
  failure. Existing scalar and graph order facts are not a byte-complete
  ASCII grammar (`0x00982F1C`, `0x009835ED`).~~

  **ANSWERED 2026-10-23 (re-persist-sl5):** MATCHES — section 8 specifies
  zero initial reference count, active-surveyor stream access, array trailing
  comma-space, graph/reference/type-info literals and LF ordering. Factory
  success is unchecked before surveyor use; there is no native successful
  result contract for a missing surveyor or empty stack. Float rendering is
  delegated to the typed output stream, not assigned invented precision here.
