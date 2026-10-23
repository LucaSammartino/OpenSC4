# Answered system-service pointer questions

**SSP1 — Actual Draw destructor identity**

The label at `0x0082AE40` is a four-word arithmetic body with a stack argument,
not a service pointer destructor (spec section 4.2). Which original native
entry or inlined path actually implements destruction for the stored Draw
wrapper constructed at `0x00449720`? No uniform destructor rule is proposed
for that nominal pair until its identity is established. Do not expand the
arithmetic owner or region callers to answer this utility attribution question.

**ANSWERED 2026-10-23 (re-system-service-pointer4):** DIFFERS — The actual
Draw local constructed at `0x0099D041` (W=`EBP-8`) is directly cleaned up by
`0x004495C0` at `0x0099D054`; the local constructed at `0x0099D118`
(W=`EBP-4`) reaches the same callable body at `0x0099D153` on its normal
post-construction exits. This body's MessageServer2 label does not restrict
its callers: it conditionally releases [W] through virtual `+0x08`, without
clearing W or a usable pointer return. It is not inlined arithmetic cleanup.
Spec section 3.4 gives receiver adjustment, branches and independent owner
fields; section 5.4 supplies proposed, unexecuted traces. Source-template and
linker-folding identity and exceptional paths are not inferred.
