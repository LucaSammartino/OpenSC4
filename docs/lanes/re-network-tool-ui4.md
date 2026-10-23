| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-network-tool-ui4 (NetworkToolUI front end) | Codex / `re-network-tool-ui4` (reserved detached worktree) | ready: bounded partial docs; questions/doccheck/links/literal checks pass; no ABI or game runs | Review spec; resolve NUI1–NUI3 without granting whole-owner coverage |

## Report

Published [network-tool-ui.md](../specs/network-tool-ui.md), the first dedicated
front-end contract for cSC4NetworkToolUI. It covers ordinary constructor/Init/
Shutdown/ref ownership, adjusted interfaces and return widths, selection,
press/cancel/lock changes, numeric picking/conversion, test/preview orchestration,
simple movement, partial extra-anchor chronology, release/dialog branches,
button activation and deferred messages. A body/family ledger distinguishes
Supported conditional behavior from Partial, shared and foreign behavior.

Native DoPlacement confirms existing network-tools §8.2 as true partial input:
raw cost zero-extension, sound timing, withdrawal gate, draft/real setup,
low-byte neighbor/failure predicates, ignored placement returns, paid-only
intersection query, exact cleanup order and no refund. Added only a narrow
public link/qualification to that subsection. No existing backend algorithm was
replaced or credited to this UI spec. In particular raw negative cost does not
itself force the budget's returned result; paid cleanup is 17 then 16, while
abort is 16 then 17. QueryInterface defines AL, not all EAX.

The independent PROPOSED UNEXECUTED trace ledger contains 28 literal cases:
asymmetric cell/anchor coordinates, repeats/cancel/release, same/changed bridge
selection, QI hit/miss/output sentinel, lifecycle/ref/lock effects, raw costs
25/0/FFFFFFFB/FFFFFFFF, paid/unpaid and neighbor/failure combinations,
real-failure no-refund and window-creation failure without automatic cleanup.
It is not a portable-output oracle or executed conformance result.

### Attribution and admission

- Source base: manifest `09a92fed66001510669086751a2bc1d736b851a8`; reserved
  detached worktree, lane re-network-tool-ui4, RE round 1. No branch switch/ref
  update, merge, rebase, push, extra worker or restart.
- Same-owner history audit used exact spec-path history, owner commit search,
  re-citytools4 and re-specs14b reports. Citytools listed 41/8005 only in its
  catalog; its published scope was base/PlaceLot. Specs14b published §8.2
  DoPlacement within a backend spec. No prior dedicated UI attempt was found.
- Canonical main Windows export queries first; matching native instructions/
  bytes and caller/vtable evidence govern the prose. Decompiled owner text was
  used for navigation. A capped lane-directory history search incidentally
  returned one generated INDEX row; INDEX was not otherwise opened or used as
  evidence. No engine/tests/portable-output reads, REA, RPC, game/native execution,
  process/cache operations or new services.
- Native PE constant bytes confirm float32 1.0, 50.0 and 0x3DCCCCCD; numeric
  requirements retain x87/control-state and conversion-domain limits.
- Milestones: admission `2dd9a0af`; first substantive contract `1da5b1d0`;
  findings handoff `b9df54c5`; expanded publication/questions `ce7d1d44`.

### Validation and errors

- `python tools/questions.py index`: generated index; `check`: PASS.
- `python tools/doccheck.py check`: PASS; `links`: PASS.
- `git diff --check`: PASS. Final tracked changes are documentation only.
- Ignored `build-runs/check-ui-literals.py`: PASS, two public arithmetic
  conversion examples, two raw-cost literal values, float32 offset/altitude
  example and presence of all 28 trace IDs. It does not execute a UI model.
- Logs: `build-runs/ui-doccheck-{1,2,3}.log`; PE constants read helper and literal
  validation helper stay ignored. No ABI rows proposed; ABI exports unchanged.
- q.py failed because Bash could not create its instance (E_ACCESSDENIED).
  Continued with capped local PowerShell. Initial PE helper failed because
  pefile was unavailable; a standard-library read succeeded, no installation.
- Git reported packed-refs.lock permission errors alongside successful detached
  commits; HEAD/log checks confirmed the commits. No shared Git configuration,
  references or maintenance were changed to work around that warning.
- Cargo/clippy/ctest: n/a (RE docs lane). Game/native/shadow runs: none; no
  equivalence or replacement/default verdict.

### Questions and limits

[NUI1–NUI3](../specs/questions/network-tool-ui.md) remain open: numeric helper/
extra-anchor completion; bridge-entry/enumeration/indicator/layout completion;
foreign low-byte argument signatures and any callers inspecting residual return
registers. No existing question was answered/archived. Backend solve/route/
occupant/geometry/rules and NetworkPreview remain foreign. This is coherent
bounded partial documentation, not all 41 entries/8005 bytes or full owner credit.
Ready for normal docs review; follow-up should close these precise gaps without
restarting capped foreign owners.
