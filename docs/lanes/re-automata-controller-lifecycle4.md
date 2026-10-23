| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-automata-controller-lifecycle4 — inherited `cSC4AutomataController` base lifecycle spec (AT22, partial) | spec-writer / `.claude/worktrees/re-automata-controller-lifecycle4` | ready: spec rewritten to clean-room prose, doccheck ok | merge train (docs); AT22 residuals below |

## Report

- New `docs/specs/automata-controller-lifecycle.md`: C / S=C+4 views and field layout, construction defaults,
  QI/AddRef/Release, SetTemplate/SetOccupant, Init, Activate and flag accessors (raw-byte SetFlag quirk),
  PlaySFX, CheckEffectTriggers, HandleViewChanged, Shutdown vs direct destruction, animation tick and expiry,
  position/bounds/grid bytes, HandleRemoveAutomaton and IsFull, lot-facing point and within-radius test,
  state enter/exit/tick and behavior selection, secondary Read (non-transactional), attractor/generator
  override map, and a slot ledger.
- §14 holds 46 conformance fixtures (L1–L13, N1–N16, R1–R17); all proposed, none run against the original.
- `docs/specs/automata.md` §3.5 offset table corrected to the C and S views (flags C+0x40, not the list head).
- AT22 in `docs/specs/questions/automata.md` gets one partial answer; it stays open.
- Open: runtime value of `00B227CC`; slot 24/36 meanings and untyped tick arguments; CreateTemplateInstance;
  SC4ReadGZSerializable reference retention; secondary Write; failure/reentrancy/non-finite domains; runtime
  x87 precision; derived controller contracts.
