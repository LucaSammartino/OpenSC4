| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-web-largest4: cGZWWWSystem behavior (RE), round 2/3 | re-web-largest4 / .claude/worktrees/re-web-largest4 (RE, detached HEAD) | wip: 71/148 bodies, 18705/34049 B documented; Q8 answered; Q7/Q9 partial; docs checks pass; native/game gates n/a | Coordinator decides round 3; trace 00816ee0, 008163d0, 00815d30, then remaining checklist/Q9. |

## Earlier checkpoint (historical; superseded by round 2 report below)
- Goal: specify only cGZWWWSystem (reported owner 34049 B), native evidence, no engine/tools/ABI edits.
- Worktree reused at detached HEAD; initially clean. Prior re-webnet4 covers URL and partial cookies only.
- Done: inventory 148 bodies/34049 B from main build-runs/run4-owners-current.tsv; sorted list in build-runs/www-owner-inventory.tsv.
- Done: substantial web-network section 4: lifecycle, QI IDs, load dispatch, synchronous file/cache boundaries, duplicate admission, status/cancel; 12 independent prose vectors.
- Round 2 milestone f949602d: Q8 answered in §4.12, 14 independent vectors.
- Done: front-end completion/progress/FTP boundary and 20 independent vectors; corrected misleading pseudocode for PostInit, visited query and cancel-all.
- Milestones: f25760eb admission/lifecycle; cfe33e39 completion/order/ref ownership.
- Done: native message slots and unsigned conversion thresholds checked; path escape/directory rules specified; 24 constructed prose vectors.
- Done: exact committed checklist docs/specs/web-network-www-bodies.md: 148/34049 B total; C=47/7239 B, P=8/12691 B, O=93/14119 B. No complete-owner credit.
- Questions: Q2 capacity answered in section 3.6 and moved to answered; new Q7 FTP, Q8 cache policy, Q9 remaining owned scheduler/support bodies; questions index regenerated.
- Validation: git diff --check PASS; checklist count/sum assertions PASS. Docs only; cargo/clippy/ctest/game n/a. No ABI proposal or exports changed.
- Next: remaining task is Q7-Q9 (largest unresolved FTP bodies first); check checklist before any claim. Do not duplicate existing URL/cookie contracts. Max one round means coordinator decides disposition; no self-restart.
- Key fixes: HTTP Init target008179f0 (not Release); IsVisitedURL calls cache presence; cancel-all snapshots IDs and calls individual cancel (007ebd30 is append, despite destructor name).
- Job destructor releases containers only, not notification target; cancellation and foreign admission failure retain unmatched target reference.
- Exact root table offsets: 2c=008f6bbc HTTP progress,30=008f8c5a HTTP final,34=008f6c47 FTP progress,38=008f815f FTP final,3c=008f6a9a notify wrapper.
- Limit: full 34049 B owner task remains incomplete; no coverage-map proposal or merge-queue entry. Latest guard before checkpoint was 147403/150000.
- Shell: q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell commands.
- Round 2 resumes ad775348; exact baseline remains C=47/7239 B, P=8/12691 B, O=93/14119 B; no completed bodies will be retraced for credit.
- Round 2: FTP spec companion covers listing parser, RETR/STOR, 13 helper/landing-pad bodies; native async dispatcher does not reset state 4, so transfer worker rejects without notification absent an external mutation.
- Next: Q9 admission/cancel contracts traced but not yet published; then exact checklist/validation and final report. Q7 login 00815d30, listing 008163d0 and outer scheduler 00816ee0 still open.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-web-largest4-r2-manifest.json.
- No network requests, no subagents, no game runs; max one guarded round.

## Round 2 report

Preserved ad775348 and its exact 148-body / 34049-byte inventory. Added **24 documented
bodies / 11466 B**, without repeating the original 47 / 7239 B for credit. Current checklist:
C=71 / 18705 B; P=4 / 4403 B; O=73 / 10941 B. The task remains wip: Q7/Q9 and 77 bodies
are unfinished. No complete-owner, implementation, verification or replacement credit.

- f949602d: §4.12 native cache policy, Q8 answered DIFFERS and moved to answered file;
  14 independent constructed vectors. Whole-token directives ignore ordinary max-age arguments;
  case-sensitive Pragma substring rejection, Date+259200 wrap and RAM/file insertion flags specified.
- ff9195ad: §4.13 / web-network-ftp.md, listing parser (3607 B), RETR/STOR (1878/1741 B)
  and 13 helpers/landing-pad bodies. Selector table, destructive parsing, unsafe fixed columns,
  commands/replies, file paths, progress, failures, ownership and callback reentry specified.
  14 listing and 17 transfer/helper vectors; none are live-network observations.
- a72140de: §4.14 HTTP admission/cancel, queue copying, method conversion and request Release;
  11 vectors. Queued cancel frees only the node, retaining the request reference. Active cancel
  sets flags and clears active pointer without joining; successor activation can occur under lock.

Native async finding: RETR/STOR set state 4 before starting dispatcher 00816990. Neither that
dispatcher nor trampoline 0092083d restores state 2. Absent intervening mutation, the worker
rejects on entry, clears +0xa15, leaves state 4 and emits no terminal notification. The spec
does not silently repair this. Outer scheduler behavior remains Q7. Unsafe listing cases
have no fabricated deterministic bytes; these input-domain limitations are explicit.

Validation: git diff --check passed; independent inventory reconciliation verifies 148 unique
addresses, exact byte sizes, total 34049 B and the C/P/O sums above. Questions index regenerated.
Docs only: cargo/clippy/ctest/game/shadow n/a. No ABI exports or proposal, no engine/tools/ABI-table
edits, no game/decompilation files committed, no network contact, no subagents. Local export,
decompilation and executable data sufficed; REA was not called. q.py cannot start Bash, so bounded
PowerShell was used. Git reports packed-refs.lock permission denied during housekeeping but the
detached HEAD commits succeed; no refs/config/maintenance changes were made.

Q8 answer: docs/specs/questions/answered/web-network.md. Q7/Q9 remain open and narrowed in
docs/specs/questions/web-network.md. Unrelated Q1/Q3 remain untouched.

Exact next state, only if coordinator authorizes round 3 (no self-restart):

1. Largest P: outer FTP scheduler 00816ee0 (1628 B), listing acquisition 008163d0 (1466 B),
   login/format selection 00815d30 (1245 B), cookie constructor 008f9bad (64 B).
2. Largest O: 00812390 (825 B), 00815a60 (717 B), 00814cc0 (657 B),
   00814f60/008153a0/008155f0 (579 B each), 00815840 (538 B), 008151b0 (489 B),
   resource constructor 00818210 (481 B); follow all remaining rows of the complete checklist.
3. Q9 callback suppression/final resource release needs worker 0081ba80 (3868 B, other owner)
   and shutdown/drain callers. Do not add that worker to the fixed WWW inventory.
4. build-runs/read-www.py pages native functions/data; checklist-www.py knows new mappings but
   emits a stale overview paragraph. Preserve the reconciled public overview when regenerating it.
5. No outstanding game requests/labels or ABI proposal; coordinator decides further disposition.

Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-web-largest4-r2-manifest.json.
