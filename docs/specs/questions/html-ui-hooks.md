# html-ui-hooks: open questions

<!-- Questions about [html-ui-hooks.md](../html-ui-hooks.md). -->
<!-- from: re-html-ui-hooks4, 2026-10-23 -->
- **HU1 — §3/§4/§6, foreign URL and conversion domains:** What are GetScheme/GetFullURL's exact raw-text normalization, scheme splitting, percent/fragment/query and non-ASCII encoding rules? What does RZAtoUint32HexA return on malformed, prefixed, overlong or overflowing input, including embedded zero bytes? Owner gates and argument routing are specified; raw URL strings are not automatically equivalent to the scheme/full-string boundary vectors. Settle from an independently scoped foreign codec contract, not by inventing additional schemes or running links.

<!-- from: re-html-ui-hooks4, 2026-10-23 -->
- **HU2 — §2.2/§5, foreign service semantics:** Confirm the public names and reference guarantees of class `0x056B906E` / IID `0x656B8EFC`, service slot `+0x80`, and returned segment slot `+0x70`. What exactly do their return values and in/out size mean, and what bytes are valid after a failed or short second read? The owner unquestionably uses first EAX as requested output size, checks only `FFFFFFFF`, and ignores second-read result; this is not a claim of successful resource contents. Settle with the service's own interface/body evidence in a separately authorized scope.

<!-- from: re-html-ui-hooks4, 2026-10-23 -->
- **HU3 — §2/§4/§5, partial failure domain:** What allocation/exception guarantees apply to string copy, token allocation, resize and native constructor invocation? Null retained service, success-with-null-segment, null URL/string/output, freed objects, gigantic sizes and unreadable buffers are unchecked native domains, not specified false returns. Native hardware faults, allocator failure and unwinding require independent contracts; no runtime probes were performed.

<!-- from: re-html-ui-hooks4, 2026-10-23 -->
- **HU4 — §2, foreign ownership and reentrancy:** Does the registering service retain H in `0045111C`, and does framework lookup return one owning reference even when its boolean is false? What reentrancy is permitted during service Release and destruction? Owner order is known (Init AddRef/store/Release/temp-Release; Shutdown clear-before-Release; destructor Release without clearing), but no foreign retention or safe reentrant lifetime is implied. Settle from registration/lookup interface contracts without treating constructor success as proof of retained lifetime.
