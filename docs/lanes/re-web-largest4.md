| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-web-largest4: WWW native contracts, final round 3/3 | `re-web-largest4` / `.claude/worktrees/re-web-largest4` (RE, detached HEAD) | wip, PARK: 91/148 bodies, 26757/34049 B documented; round gain 20/8052 B; exact residual 57/7292 B; Q7/Q9 partial; docs checks PASS; cargo/clippy/ctest/game n/a; ABI unchanged | Coordinator review; retain partial contracts; no fourth round or whole-owner credit. |

## Final report: PARK after round 3/3

Preserved bdd8e715 and its 71 documented bodies / 18705 B without retracing those
contracts for credit. New work is in web-network-ftp-control.md and
web-network-http-lifetime.md, linked from the existing FTP companion/checklist.

- c833149e: seven new bodies / 4102 B: login, listing acquisition, format selection,
  list replacement/cleanup, job-value construction and disconnect; 17 independent
  discriminating prose vectors. Login can return true after a rejected reply and
  disconnect. Deferred login/listing/disconnect can dispatch into state-4 rejection.
- 6bad147d: thirteen further bodies / 3950 B: outer FTP scheduler, session constructor,
  shutdown/destruction, worker start/wrapper, job copying/final notification, deleting
  adapters and reached HTTP constructor. Ten further FTP vectors and nine HTTP vectors.
  Foreign HTTP worker/completion/stop/drain bodies are evidence only, not owner credit.
- Important correction included in 6bad147d: c833149e initially mistook synchronous
  listing's ESP+0xd0 for the second argument. time(nullptr)'s outstanding pushed argument
  makes it the intended first/path argument. Current spec fixes the claim and restores
  the null-output-list lifetime vector; do not implement the superseded interpretation.

The FTP worker has no retry loop; cancel at final boundary still yields a failed final
callback, while stop suppresses it at that check. Session shutdown Releases its target
before disconnect and does not join. HTTP completion copies resource fields, notifies,
then releases internal and external holds in order; stop bypasses that completion and
its releases. HTTP shutdown can start a replacement consumer before raw queue drain;
old worker exit can clear the replacement's published pointer. Callback suppression
is bounded by native checks, not an atomic global guarantee.

## Exact residual and questions

web-network-www-bodies.md retains all 148 unique snapshot addresses/sizes, with
C=91/26757 B, P=1/64 B, O=56/7228 B. P/O combined = 57 bodies / 7292 B.
Every residual address is explicitly marked P or O in that checklist. Largest remaining
individual bodies: 00815a60 717 B; 00814cc0 657 B; 00814f60/008153a0/008155f0
579 B each; 00815840 538 B; 008151b0 489 B; 00817600 399 B. Cookie constructor
008f9bad remains partial at 64 B. Remaining commands, initialization entry, scheduler
controls/containers and support/adapters receive no completion credit.

Q7/Q9 remain open and narrowed in docs/specs/questions/web-network.md; no full answer
was moved to answered. Existing Q8 answer and unrelated Q1/Q3 preserved. Questions
index regenerated (no resulting index content change). No fourth round is requested.

## Validation and limits

git diff --check PASS. Independent reconciliation against the retained native snapshot
build-runs/www-owner-inventory.tsv PASS: exact address-to-size equality, unique count
148, total 34049 B, and C/P/O counts and sums above. Validation was documentation and
inventory checking; no implementation/test output used as native behavior evidence.
Only local export/decompilation and executable bytes used; REA not needed. q.py Bash
startup failed E_ACCESSDENIED; bounded PowerShell worked. Detached commits succeeded
despite Git's packed-refs.lock housekeeping permission warning; no refs/config changed.

No engine/tools/maps/ABI-table edits, ABI proposal/export changes, game runs, network
contact, external communication, clipboard changes or subagents. No native gates are
required for these docs-only changes. No implementation, shadow, verification,
replacement or complete-owner credit. Preserve the worktree and PARK the residual.
