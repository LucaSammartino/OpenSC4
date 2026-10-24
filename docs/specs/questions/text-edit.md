# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE5 - sections 7-8; text-edit-layout.md and text-edit-adapters.md:** **PARKED final round 3/3; no fourth round.** Owner paths now proved: pages measure current X while arrows reuse stored X; cache survives all four vertical success paths; single-line strict three-quarter threshold; complete owner AddScrollBar ordering/failure; SelectCurrentWord and insertion-repair asymmetry; composition owner's erase/previous-value loop. Exact residual: GetPreviousUTF16Character 0x0091163F, GetUTF16Character 0x009115F0 and UTF16GetThaiCharLevel 0x00911A40 results for each selector/malformed/context case, plus composed-length helpers, are foreign inputs. Do not infer their contracts from names or promote their owners. Preserve this worktree; no restart.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** **PARKED final round 3/3; no fourth round.** Actual attribution remains 150 bodies / 19,999B. Direct entries now have 97 P / 10,136B and 37 P/B / 9,279B, zero U, each with a method contract; the inventory's final reconciliation names every P/B address and bounded dependency. The sixteen additions have eight small P contracts and eight B roles: 009BDE3E allocation, 009BE9ED initial line, 009BEA0D/009BF518 range destruction, 009BFA74 storage cleanup, 009BFA92/009BFABB copy construction, 009BFAE3 destructor. Exact residuals: generated-action copy/allocation failure/unwind; foreign base/font/drawing/bar/timer/dispatch/clipboard outcomes beyond documented call sites; lib::_strncpy padding and search primitives 004427F0/005C23E0 edge/error behavior. TE5 separately names conversion/composition residuals. No complete-owner or widget-family credit from inventories or these bounded roles. Preserve worktree; no restart.

<!-- from: core-widgets5 (2026-10-23) -->
- **TE9 - section 1 constructor / section 6 history limit:** what initial values
  are stored in the maximum-undo-level word W+0x184 and preferred-X W+0x170?
  The listed constructor defaults and setters specify neither initial value
  nor an Init assignment. [The portable byte/state stage requires explicitly
  resolved history-limit and preferred-X values;
  no native constructor default is inferred. This does not reopen the parked
  TE5/TE8 foreign-helper research or request another owner research round.]

<!-- from: core-widgets-close12 (2026-10-24) -->
- **TE10 - section 1 runtime constructor defaults:** what are the initial caret,
  highlight-fill and outline color words, four sound IDs, horizontal byte/pixel
  offsets, visible metrics and line-height words? [The portable runtime uses
  explicit zero placeholders pending caller resolution or layout calculation;
  these are not asserted native defaults. Six cleared font colors and the
  white fill follow the specified constructor. No new RE round requested.]

<!-- from: core-widgets-close12 (2026-10-24) -->
- **TE11 - section 8 shared automatic-reflow counter:** what exact counter
  comparison limits automatic scrollbar recursion, and which calls increment
  and decrement it? [Literal shared guard permits one active automatic check
  and skips nested checks, with exception-safe reset. The stated shared scope
  is preserved; no larger unspecified recursion threshold is inferred.]
