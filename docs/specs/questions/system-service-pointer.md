# System-service pointer questions

## SSP2 — Persist clear-and-release source role

Section 3.2 establishes the exact normal clear-before-release body at
`0x00874DA3`. Is its source-level role a destructor, reset, or folded/shared
pointer helper? The imported Persist destructor label and heterogeneous direct
call metadata do not decide that. This asks for attribution only; its normal
field/call order is already specified and must not be replaced by D74 behavior.

**2026-10-23 evidence (re-system-service-pointer4):** Section 3.2.1 now proves
an output-address preparation use: calls `0x00995403`/`0x009954E6` receive
live fields `owner+0x54`/`owner+0x60`, then pass the returned full EAX field
address as a foreign output argument. Call `0x0093F371` likewise uses
`owner+0x28` as output to a separate interface's virtual `+0x00`. The WinIME
paths first guard for a null field; do not invent a nonnull Release on those
normal paths. This narrows the unknown to source-language attribution. Keep
SSP2 OPEN: no source spelling, source template or folding history is proved.

## SSP3 — Exceptional and reentrant domain

If compatibility outside normally returning nonreentrant calls becomes required,
what original utility-owned behavior is observable when a reference primitive
throws or reenters assignment/destruction? The current evidence verifies normal
paths only; exception-frame scaffolding alone is not a cleanup guarantee.
Likewise no behavior is specified for malformed pointers. Keep foreign method
algorithms and global lifetime guarantees outside this utility research scope.
