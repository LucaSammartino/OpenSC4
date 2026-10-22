| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-textedit4 (cGZWinTextEdit final round 3/3) | `re-textedit4` / (RE lane, detached worktree) | ready: bounded native specification; TE1 answered; 64 proposed vectors; TE5/TE8 PARKED; no whole-owner credit | Coordinator may merge docs; preserve this worktree. No fourth round, restart, implementation or coverage promotion authorized. |

## Final report (round 3/3)

This final round specifies only cGZWinTextEdit, the export owner requested
as `cGZWinTextEdit19999B`. It preserves the round-2 attribution and all
44 previously committed vectors. No cGZWin, cGZLegacyControl, Grid,
FileBrowser or whole-widget-family credit is claimed.

Changes committed this round:

- **a8095773:** [byte-helper contracts](../specs/text-edit-encoding.md):
  exact numeric selector/lead tables, malformed forward/backward/count
  outcomes, pending-input acceptance and its differing tail-count rule.
  TE1 answered DIFFERS. Also caret-in-view strict three-quarter stepping,
  AddScrollBar attempts/resource/failure order and word-selection extension
  of existing endpoints. Eight new proposed vectors.
- **c728de2f:** [layout extension](../specs/text-edit-layout.md) proves
  pages use newly measured X while arrows use cached X, with vertical cache
  retention. [Adapters/repair](../specs/text-edit-adapters.md) specifies
  text/caption/state getters, coordinate clamps and asymmetry, period bounds,
  signed sound indexing, lifecycle/input adapters, LF filtering and the
  owner composition erase/previous-value loop. Eight further proposed vectors.
- **1db2459e:** remaining direct flag/image/color, scrolling, geometry,
  search and line-vector body contracts; unsafe domains and callback order;
  final per-body reconciliation and exact parked questions. Four further
  proposed vectors. Highlight setter's measured conversion slot and the
  pixel-only ScrollToStart no-op are distinguished explicitly.

## Evidence and measured scope

Primary evidence is original Windows 1.1.641 export/decomp plus native
instructions queried through main-checkout re_query.py. Numeric comparisons,
widths, operation order and argument loads were checked with asm --bytes;
addresses are cited in the specs. Examples: byte helpers 009106A9/0091073C/
009107D8, pending 009C0996, page/arrow loads 009C0B7C/009C0DC3/
009C0EBB/009C146C, caret view 009BE302, AddScrollBar 009BE477,
composition loop 009BE614. No implementation or constructed runtime output
was used as behavioral evidence. REA was not needed or queried.

The [inventory](../specs/text-edit-inventory.md) retains all **134 direct
addresses / 19,415B**, with **97 P / 10,136B**, **37 P/B / 9,279B**,
and zero U. Every formerly U row gained its own primary-body contract;
none was promoted from inventory membership. P/B remains bounded proof,
with every row's address and unclosed dependency grouped in the final
checklist. The largest key/paint bodies remain P/B.

The [actual attribution](../specs/text-edit-attribution.md) still contains
**150 bodies / 19,999B**. Its sixteen additions total 584B: **8 P / 202B**
for small adjustment/predicate/correction/copy contracts and **8 B / 382B**
for bounded allocation/generated-action/destructor roles. Metadata, strides,
call roles and thunks do not establish full callee/failure contracts.
The folded maximum setter has a contract but remains outside that actual
owner set. No complete-owner, implementation, verified or replacement credit.

## Checks and questions

- PASS: questions.py index/check and git diff --check.
- PASS: ignored build-runs/audit-textedit-r3.py compares all direct addresses
  and sizes to entry commit 8b37770a, checks the disjoint sixteen additions,
  and verifies all **44 original vector rows unchanged**, followed by **20
  new cases**, **64 proposed vectors total**. These cases were not executed.
- Cargo/clippy/ctest/game/shadow: n/a for this docs-only lane; no engine,
  tools, maps, ABI-table or C ABI changes. No ABI proposal file.
- TE1 moved to [answered questions](../specs/questions/answered/text-edit.md),
  marked ANSWERED 2026-10-22 (re-textedit4): DIFFERS; index regenerated.
- [TE5 and TE8](../specs/questions/text-edit.md) are explicitly **PARKED**.
  TE5 retains exact foreign UTF16/Thai/composed helper outcomes. TE8 retains
  bounded base/font/draw/bar/timer/dispatch/clipboard outcomes, search/copy
  edge/error behavior and generated-action allocation/copy/unwind proofs.
  Their owner-side navigation/view/resource/adapter loops are now documented;
  those bounds are not guesses about foreign implementations.

## Final disposition

Ready to merge as bounded documentation. **Final round 3/3: no fourth round
or automatic restart. Preserve this original worktree and its ignored native
query/audit artifacts.** Coordinator review/merge does not authorize owner
promotion. No live clipboard, game, network or external communication actions
were performed. q.py could not start Bash (E_ACCESSDENIED); bounded PowerShell
was used. Commits succeeded despite a packed-refs.lock permission warning;
shared refs/config/hooks were not changed and no Git maintenance was run.

Merge queue: own-worktree build-runs/merge-queue/re-textedit4.json, kind docs,
state ready, ABI false, no game runs, actual detached HEAD tip. The supervisor
publishes the reserved lane branch; this agent did not switch/update refs,
merge or push.
