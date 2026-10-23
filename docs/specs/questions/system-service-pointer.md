# System-service pointer questions

## SSP2 — Persist clear-and-release source role

Section 3.2 establishes the exact normal clear-before-release body at
`0x00874DA3`. Is its source-level role a destructor, reset, or folded/shared
pointer helper? The imported Persist destructor label and heterogeneous direct
call metadata do not decide that. This asks for attribution only; its normal
field/call order is already specified and must not be replaced by D74 behavior.

## SSP3 — Exceptional and reentrant domain

If compatibility outside normally returning nonreentrant calls becomes required,
what original utility-owned behavior is observable when a reference primitive
throws or reenters assignment/destruction? The current evidence verifies normal
paths only; exception-frame scaffolding alone is not a cleanup guarantee.
Likewise no behavior is specified for malformed pointers. Keep foreign method
algorithms and global lifetime guarantees outside this utility research scope.
