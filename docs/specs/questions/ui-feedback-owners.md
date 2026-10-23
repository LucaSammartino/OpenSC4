# UI feedback owners: open questions

## UIFO-1 — Exact foreign string grammar

Spec: [ui-feedback-owners §3.3](../ui-feedback-owners.md#33-text-branch-raw-string-boundary).
What exact SplitToken/Find character and resource-string semantics are needed
for end-to-end text vectors? The current owner contract takes the single raw
primitive outputs and proves the branch and call order; it does not invent a
tooltip target cache or foreign text renderer.

## UIFO-4 — Progress normal-mode image helper and three-string drawing

Spec: [ui-feedback-owners §4.7](../ui-feedback-owners.md#47-repaint-and-bounded-residual).
What exact normal-mode image composition is performed by internal helper
`00776C30–00776F49` (794 bytes), called at `00777503`, and what rectangle and
per-line arithmetic/call schedule is used by the remaining three-string loop
in `00777508–00777631`? This publication qualifies the update's state algorithm
and the obliteration draw path, but neither delegates this own helper to a
host oracle nor claims a complete normal-mode final image. The newly attributed
update, QI and destructor paths are documented in §4, not left identity-only.

## UIFO-3 — Library effects and selector domains for end-to-end progress text

Spec: [ui-feedback-owners §4.3](../ui-feedback-owners.md#43-buffer-colors-and-string-sequence),
[§4.5](../ui-feedback-owners.md#45-resource-change-message),
[§4.6](../ui-feedback-owners.md#46-update-elapsed-threshold-and-string-consumption).
What exact seeded permutation does library shuffle `00777700` produce, what
string copying/clearing semantics do its foreign string primitives provide,
and what selector domains restrict the six literal fallback-resource rows?
The owner contract supplies raw primitive outcomes and proves loading, threshold,
consumption and reference ordering. It does not extend into region algorithms
or silently clamp out-of-range selector indices.
