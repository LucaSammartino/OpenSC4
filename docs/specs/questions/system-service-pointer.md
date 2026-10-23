# System-service pointer questions

## SSP1 — Actual Draw destructor identity

The label at `0x0082AE40` is a four-word arithmetic body with a stack argument,
not a service pointer destructor (spec section 4.2). Which original native
entry or inlined path actually implements destruction for the stored Draw
wrapper constructed at `0x00449720`? No uniform destructor rule is proposed
for that nominal pair until its identity is established. Do not expand the
arithmetic owner or region callers to answer this utility attribution question.

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
