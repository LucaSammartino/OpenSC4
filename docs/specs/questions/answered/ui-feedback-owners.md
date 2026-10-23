# UI feedback owners: answered questions

## UIFO-2 — Progress unnamed destruction and repaint details

Original question: Which unnamed table entries are actually Progress-owned
teardown and update bodies, and what ordinary-return paths do they establish?
Five attributed names do not prove vtable extent or whole-object completion.

**ANSWERED 2026-10-23 (re-ui-feedback-owners4):** DIFFERS — The actual primary
table has four inspected slots; the adjacent refcount table has an adjusting
deleting-destructor thunk at `007770A0`. Direct destruction is `00777800`,
deleting wrapper `00777B90`, update `00777640`, and its repaint dispatcher
`007773B0`. They are additional unnamed owner bodies, not part of the five-name
1874-byte catalog total. Their qualified state/reference/return contracts are
in [§4.1](../../ui-feedback-owners.md#41-construction-and-caller-boundary),
[§4.4](../../ui-feedback-owners.md#44-shutdown-and-direct-destruction) and
[§4.6](../../ui-feedback-owners.md#46-update-elapsed-threshold-and-string-consumption).
No whole-object/render claim follows; the precise remaining image-composition
and three-string-draw work is UIFO-4.
