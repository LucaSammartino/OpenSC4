| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rz-sparsematrix4: first sparse matrix owner contract | Codex, detached worktree `re-rz-sparsematrix4` (RE) | ready: bounded partial docs, original final 3/3; docs checks pass; SM1–SM5 open; ABI/game n/a | 1. Coordinator lands detached docs tip. 2. Retain explicit partials and unreconciled 39/3358 attribution; no automatic fourth round. |

## Final round 3 report (2026-10-23)

Published [rz-sparse-matrix.md](../specs/rz-sparse-matrix.md), a bounded primary-evidence contract,
with a minimal exclusion/link update in framework-rz-files and five indexed open questions.
Ready means these partial documents can land, not that the matrix owner is complete.
This closes the original third and final round; no fourth round or automatic restart is requested.

### Authority and identity

Initial detached HEAD was clean at `b1304240`. The copied immutable proof is
`build-runs/evidence-run4/coordinator-sparsematrix-final-host-proof.json`; it pins that source,
next round 3, full host framework services (RZ) admission exit 0 and dependency ancestor exit 0.
The actual `C:/Users/tlori/FreeCity4/build-runs/run4-re-rz-sparsematrix4-r3-manifest.json`
matches lane, kind re, round 3 and this worktree path. The prior manifest's SHA256 matches the
proof. Supplied Catalog READY `904e55b6` result/host-proof metadata establishes the supported
dispatcher context only; no Catalog algorithms or native outputs were used. No isolated host
spawn preflight was rerun. Normal runtime guard remained active; latest milestone status was
within budget (93599/150000).

Canonical export queries preceded matching decomp support. Current binary SHA256 and export-summary
hash both equal `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`;
binary resource version is 1.1.641.0, original filename SimCity 4.exe, export base 00400000.
Live REA target availability remains UNKNOWN: no retry, open/reset, added tool or server action.
R1 NoApp `281dd777` and R2 isolated preflight failure `b1304240` remain preserved below, with
their then-applicable instructions. R2's inaccessible npm path does not establish physical host absence.

### Supported publication and limits

Instruction-backed rules cover signed column/row keys, inclusive insertion bounds and range queries,
lookup address/output effects, occupancy counts, row-major 12-byte range records, pointer and forty-byte
payload replacement order, wrapped removal endpoints and erase scheduling, extrema/sentinel tie
behavior, fixed-column iteration, and exact comparator return channels. Foreign helper success and
valid tree invariants are explicit conditions. Thirteen small independently deduced literal vectors
are **UNEXECUTED**; none is a native-exact comparison, coverage measure or default promotion.

The two entries labeled GetCellPtr `009a5bf2` and E col_end `009a5c36` initialize records; they
are not lookup/end bodies. `009a7577` is iterator equality. The ledger separates supported,
partial and foreign/shared entries and includes auxiliary MatrixRow/tree dependency boundaries.
Eighteen matrix-labeled entries sum 1784 bytes, iterator 1/82, foreign AutoPtrs 2/82: exact named
entry total 21/1948. The public matrix 38/3276 plus iterator 1/82 = **39/3358 attribution remains
unreconciled**, not a unique-body or whole-owner denominator. No byte-coverage fraction is claimed.

SM1–SM5 in [questions/rz-sparse-matrix.md](../specs/questions/rz-sparse-matrix.md) retain attribution,
foreign allocation/eraser failures, constructor/default/dimensions/resize/clear/copy/lifetime,
invalid-pointer/iterator invalidation and specialization-equivalence gaps. No previous question was
answered. No ABI proposal was needed; no docs/abi table/version/map edits were made.

### Validation and work record

- `python tools/questions.py index`: exit 0; generated docs/specs/QUESTIONS.md.
- `python tools/questions.py check`: exit 0, questions: ok.
- `python build-runs/check-sparsematrix-docs.py`: exit 0; own admission/manifest/prior hash and
  fixture metadata, local links, thirteen vector rows, five indexed questions and diff check pass.
  Its first run failed with KeyError because the fixture result is wrapped in structured_output;
  the local checker was corrected and passed. This was metadata parsing, not a native failure.
- `git diff b1304240 --check`: exit 0; only five documentation files changed.
- One staging attempt named nonexistent questions/INDEX.md and failed; corrected to generated
  docs/specs/QUESTIONS.md before commit. Detached commits succeeded despite restricted Git's
  packed-refs.lock diagnostic; no shared refs/config/hooks were modified.
- Commits: `d2a95cfd` admission handoff; `78ccfb31` substantive contract/questions/link;
  `7268e16e` E copy-order audit; `496a132a` exact returns and dependency ledger; final report follows.

Cargo, clippy, ctest, build/game/shadow checks are n/a. No engine source/tests, original runtime,
game/build/cache RPCs, subagents, external messages, merge/rebase/push, branch switch or target
control. Ignored docs queue records the actual detached final tip with abi=false and no game runs.

## Round 2 report (2026-10-23)

The coordinator explicitly authorized primary export/instruction research without successful live
pseudocode. This supersedes round 1's research-method prerequisite, but does not waive the normal
full REA guard and server handshake. Round 1 at `281dd777` remains a failure with zero behavior
credit; its original report is preserved below, including its then-applicable resume instructions.

### Blocking result

Ran the normal full check, without a handshake bypass:
`python C:/Users/tlori/FreeCity4/.claude/lane-bin/laneguard.py preflight re-rz-sparsematrix4 --kind re`.
It reported:
**`REA-DOWN C:\Users\tlori\AppData\Roaming\npm\node_modules\rea-agents\scripts\rea.mjs does not exist`**.
The configured entrypoint check fails before a successful server handshake can be obtained.
No repair, new server, target open/close, repeated NoApp query, or alternate configuration was attempted.
This is a distinct round 2 configuration blocker, not evidence that round 1's live target was repaired.

### Bounded preliminary observations; no behavior credit

- Reused the reserved detached worktree, initially clean at `281dd777`.
- The export summary identifies `SimCity 4.exe`, SHA-256
  `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, image base `00400000`.
  The main working binary's version resource reports `1.1.641.0`, original filename `SimCity 4.exe`.
  A current binary hash comparison was not completed; identity verification remains partial.
- Queried the export name inventory first; it returns 18 matrix-named entries and one iterator-named
  entry, plus two foreign AutoPtr entries. These names are not an audited unique-body inventory.
- A single export instruction query for `009a5bf2` was included in the same shell invocation as
  preflight. No contract is published from it. Attribution needs instruction/caller reconciliation;
  a method name alone is insufficient.
- Matrix **38 attributed functions / 3276 bytes**, iterator **1 / 82**, combined **39 / 3358** remain
  unaudited public denominators. Named/folded/foreign/uncertain per-body classification is unfinished.
  Completed bodies, supported behavior rules and validated literal vectors: **zero**.
- No spec, question resolution, ABI proposal, source/test/tool changes, engine reads, builds, games,
  subagents, restart, merge/rebase/push, branch switch or shared ref change.
- q.py again failed Bash startup with `Bash/Service/CreateInstance/E_ACCESSDENIED`; capped reads
  then used PowerShell. This did not cause the REA failure.

### Validation and next step

Documentation only: `git diff --check` is the applicable check; cargo/clippy/ctest and game/shadow
checks are not applicable. No question was filed or answered, so no question index changes are needed.
The ignored merge-queue entry remains `wip`.

Coordinator must restore the configured REA entrypoint and obtain a full guard/handshake pass.
Only then resume the authorized primary export workflow: complete binary identity comparison,
audit exact bodies/aliases, establish coherent native algorithms and literal vectors, then publish
the bounded contract and unresolved questions. Do not repeat the unchanged live NoApp query.
This round stops here; it does not self-restart or consume the remaining round automatically.

Evidence root: `C:/Users/tlori/FreeCity4`; round 2 manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-re-rz-sparsematrix4-r2-manifest.json`.

## Preserved round 1 report (281dd777)

## Report

The first-owner behavior publication is blocked before substantive research. The normal configured
`mcp__rea__procedure_pseudo_code` check for `0x009a8fec` returned
`target_unavailable` (category `unavailable`, `retryable: false`): **"No app is open."**
Its suggested remediation requires `open_binary`, which is outside this lane's two permitted REA
operations. No target was opened, no additional tool was added, and no static-only research was
substituted: the brief expressly requires REA availability and disallows that downgrade.

### Work and attribution

- Reused the reserved detached worktree; initial tracked status was clean and `main..HEAD` was empty.
- Initial export name search found matrix template entries and the named ColIterator increment;
  `re_query.py func 009a8fec` reports the iterator entry as 82 bytes. This is inventory only,
  not proof of its behavior or of unique body ownership.
- Matrix **38 attributed functions / 3276 bytes**, iterator **1 / 82**, combined **39 / 3358**
  remain the brief's unaudited denominators. No named/folded/foreign/uncertain classification,
  per-body coverage, completed-body credit, or validated vector is claimed.
- No `docs/specs/rz-sparse-matrix.md` was created; no native behavior was established.
- No ABI proposals, ABI table/version edits, implementation/source/test/tool changes, engine reads,
  builds, games, subagents, merges, branch switching, or ref updates.
- `q.py` could not start Bash (`Bash/Service/CreateInstance/E_ACCESSDENIED`); subsequent capped
  export/status reads used direct PowerShell. This is secondary to the REA blocker.

### Validation and questions

Documentation-only blocker report. `git diff --check` is the applicable check; cargo, clippy,
ctest and game/shadow checks are not applicable. No behavior question was answered or filed,
and no question-index regeneration is needed.

### Resume instructions

1. Coordinator restores the original SimCity 4 Deluxe GOG Windows 1.1.641 target in the shared
   REA session. The lane must first obtain a successful normal pseudocode response.
2. Audit exact matrix/iterator body attribution, then establish a coherent bounded contract,
   operand widths/return channels, caller-supported semantics and literal vectors.
3. Publish the spec and its precise unanswered list; retain unsupported and foreign distinctions.

Evidence root: `C:/Users/tlori/FreeCity4`; manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-re-rz-sparsematrix4-r1-manifest.json`.
Ignored merge-queue entry is `wip`, never `ready`, while this prerequisite is unavailable.
