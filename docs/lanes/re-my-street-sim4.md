| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-my-street-sim4 — MyStreet native owner contract | Codex / re-my-street-sim4 | ready — docs; owner completeness remains bounded/partial | Review own contract and ST1–ST5; no automatic continuation |

## Report

Created [my-street-sim.md](../specs/my-street-sim.md), the first dedicated
Street-owner contract found by the bounded prior-history audit. Existing
mysims1/cores26 references were consumer/shadow attribution. Historical round
results and MS8–MS16 remain unchanged; no family counter is reset.

The contract covers class `0x0BF606A5`, concrete registered factory provenance,
six interface identities, shared reference lifetime, own constructor stores,
Init/Shutdown/destruction and serialization wrappers. It publishes scheduling
gates, ShowLocation/HideLocation retention, detach/attach ordering and raw
returns, every Street message branch and inherited forwarding, trip/environment
grid outputs, lot counting, landmark selection/funding, disaster scanning,
script finalization/cleanup, and tool creation. All host boundaries are typed
single operations. No engine files, portable output, SDK code or native runtime
oracle were read or used.

The main correction during review was budget-call chronology: pending stack
arguments belong to GetBudgetItemForPurpose(holder,purpose,item&), not to the
intervening no-argument property-holder retrieval. A wording correction also
preserves the literal `trip_on_subway` key. The MySim overrides are
ShowLocation/HideLocation, not balloon methods.

### Evidence and measured checks

- Source: main canonical Windows export and executable table words, GOG
  1.1.641, source base `9b2335b42e1b120740918a15031a1e0f91561266`.
  Executable SHA-256 is recorded in spec §1. Export/decomp were available;
  REA was unnecessary and no live query/session operation was attempted.
- Native table comparison: MySim differs from the ordinary table only at
  slots 0–2, 34,35. Generic active/slot/name/portrait/residence/status hooks
  are inherited, not newly recovered Street stubs or algorithms.
- Native-window inventory: **38 entries / 5481 bytes**, each ledgered exactly
  once by the read-only analytical helper. Metadata **33 / 5373** remains a
  different, unresolved attribution membership (ST1); neither count is whole
  owner coverage, and no new coverage/native/default/runtime credit is claimed.
- **67 distinct proposed UNEXECUTED literal rows**: lifetimes and identity,
  alternating handles, repeated transitions, raw AL/full-EAX discriminators,
  each message path, list order, signed widths, boundary coordinates,
  false-return prefix effects, and explicit Unsupported domains. These are
  analytical proposals, not executed test or native verdict counts.
- `python tools/questions.py index`: regenerated the index;
  `python tools/questions.py check`: **ok**.
- Direct `python tools/doccheck.py check`: **ok**; `links`: **ok**.
  `git diff --check`: clean, with normal LF-to-CRLF Git notices.
- Ignored `build-runs/street-doc-audit.py`: all 38/5481 ledger rows match native
  export sizes and membership, 67 unique literal rows, ST1–ST5 present, no
  assembly listing detected in the spec. `street-audit.py` retains the
  read-only PE-table/constant inspection helper. Neither models portable
  behavior or executes proposed traces.
- Earlier q.py attempts for doccheck failed **before launching the checks**:
  `Bash/Service/CreateInstance/E_ACCESSDENIED`; direct Python reruns passed.
  Git emitted `packed-refs.lock: Permission denied` during detached commits,
  but each commit completed and HEAD advanced. No refs/config were changed.

### Commits and scope

- `d859cbaf`: first substantive identity/lifecycle/serialization contract.
- `82dcf31d`: own behavioral chronology and typed boundaries.
- `3dfddee2`: body ledger, literal traces, five questions and index.
- Final report commit includes the reviewed key wording and checks above.

Only the new spec, its question file, generated question index and this report
are committed. No ABI rows are proposed; no C ABI export changed. Cargo,
clippy, CTest and game runs are **n/a** for this docs-only RE lane. No native
execution, implementation, glue, coverage-map changes or game requests.

### Questions and next steps

[ST1–ST5](../specs/questions/my-street-sim.md) remain open: metadata membership;
inherited defaults/copy/persistence; numeric/x87 domains; semantic IDs/units;
exceptional/reentrant lifetimes. No existing question was silently closed or
moved to an answered archive. Full owner completeness remains partial at those
explicit boundaries; the own normal-path decisions are published.

Coordinator: review/land this docs result and retain the residual limits for
any later independent owner work. No automatic new round, implementation,
capped-family reopening, native execution or live-query prerequisite is
requested. Queue is the normal own `kind=docs`, `abi=false`, `game_runs=[]`.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-my-street-sim4-r1-manifest.json`.
