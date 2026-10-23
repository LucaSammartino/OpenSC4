# lua-host: open questions
<!-- Append-only questions from clean-room implementer lanes. -->

No open entries. LH1/LH2 are in [answered/lua-host.md](answered/lua-host.md).

<!-- from: core-lua-host5 (2026-10-23) -->
- **LH4 - sections 1.2, 3.1, 6.1, 6.2: thread lifecycle closure:** what are the cGZLua5/cGZLua5Thread constructor fields, reference-count destruction rules, main-thread ID, child ID allocation/reuse, registry rooting, CreateThread argument transfer and factory-failure cleanup, GetThread ownership, ResumeThread status/result-count/error semantics, and Pause/Status/IsValid/Destroy/OnCreate/OnDestroy contracts? The spec lists signatures but not these decisions. What state survives failed thread-registry initialization? [Implement the stated main-thread backpointer and base Open order; retain an allocated state on registry-allocation failure as the literal sequence has no cleanup. Leave unspecified lifecycle decisions explicitly Unsupported, rather than treat the original host as a foreign interpreter primitive.]

<!-- from: core-lua-host5 (2026-10-23) -->
- **LH5 - sections 1.2 and 6.3.2, cGZLua5CommandGenerator:** please specify the constructor/defaults, generic Init, remaining setup after accepting/creating the Lua global, parameter-set creation, generated callbacks/command enumeration/execution, repeated Init and failure/destructor cleanup. Section 6.3.2 specifies endpoint retention, global validation and Shutdown but calls the success tail only "remaining setup". [Implement the specified validation, registration, retention and Shutdown sequence; stop explicitly Unsupported at the unspecified setup, without claiming a successfully initialized generator.]
