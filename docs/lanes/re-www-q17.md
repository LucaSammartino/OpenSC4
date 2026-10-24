| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q17 (web and network contracts) | Codex / `re-www-q17` (RE lane, detached HEAD) | ready; partial owner contracts; doccheck check/links passed | Land documented answers; WWW Q7/Q9 and HTTP Q13 retain explicit residuals. |

## Report

Read core-www-close13's Owner closure table before resuming. The earlier round answered
Q1/Q3/Q10/Q11/Q15/Q16; their dated answers remain in
docs/specs/questions/answered/web-network.md. This resumed final round closes individual
blockers inside Q7/Q9/Q13, while leaving their incomplete whole-owner entries open.
Partial-ready is the brief's authorized final-round disposition; no whole-owner credit.

### Changes and evidence

- Earlier commits `448d45595` and `21d033ce5`: cookie Port/splitter/INI/header/settings
  rules, cache filename/INI/admission/orphan/writer rules, matched null-target WWW
  completion; 15 proposed/unexecuted vector groups. Report checkpoint `a75fee9b1`.
- `2648fec8d`: §4.4 WWW Cookie record construction and temporary string ownership;
  §4.14 shared HTTP request normalization, raw-text preservation with existing pairs,
  continuation/empty-final-line quirks and first-empty-value erasure. This resolves the
  HTTP Q9 normalization blocker and WWW Q9 cookie-record blocker.
- `3fcae5355`: FTP-control §9 specifies all seven remaining command bodies, exact
  reply-class acceptance, restored callback state, PWD extraction and unsafe malformed
  domains, async state-4 rejection and rename's double assignment; §9.1 specifies
  scheduler construction/admission. HTTP §5.1 qualifies native QueryInterface slot 0
  and deleting wrapper slot 3. Full destructor cleanup remains open.
- Current WWW inventory, independently recounted from rows: **102 C / 31532 B;
  46 O / 2517 B; 148 total / 34049 B**. Previous summary still counted the already
  qualified 64-byte cookie constructor as P; corrected it and credited ten new bodies
  totaling 4711 B. C means documented, not implemented or dynamically equivalent.
- Added 21 proposed/unexecuted fixture groups: WWW-22a–c (including empty-value
  variant), NORM01–NORM08, FTPC01–FTPC08, HTTP-SLOT01–02. No engine vectors or code.
  These are constructed fixtures with expected dependency calls/outputs, not game runs.

Evidence uses main-checkout re_query.py function/byte-instruction queries, bounded decomp
reads and read-only executable literal decoding. No REA session was used. Numeric and
layout claims cite instruction addresses; no decompiled code/listings were copied into
specs. Research ended at measured context **95966/150000**, below the two-thirds bound;
the remaining round was documentation, review and validation.

### Validation and limits

doccheck check PASS; doccheck links PASS; git diff --check PASS. Questions index regenerated.
Cargo/clippy/CTest n/a for this specification-only change. Game runs/shadow n/a; no request
outstanding. No new question IDs filed; no newly complete whole-question entry was moved
in this resumed round. Q7/Q9/Q13 progress is recorded in the open question file instead.
Q12 is outside the brief. Previous six archived answers remain intact.

No engine/, plugin, capi, ABI table, coverage-map, game-file or external/ edits. No proposed
ABI rows and no C ABI change. Git reports inability to create shared packed-refs.lock during
commits, but each detached-HEAD commit succeeds; shared refs/config were not repaired.
Ready queue uses kind re and the full actual HEAD; supervisor publishes the reserved branch.

## Handoff

- WWW Q7: 00817af0, scheduler lifecycle/control/container helpers and remaining O rows.
- WWW Q9: remaining O support/adapters; Cookie record construction and HTTP normalization are settled.
- HTTP Q13: wire generation/retry/redirect, ReadMessage 00819800, finalization 0081a470,
  date grammar 00818490/00818680, timezone adjustment 00b18ba8 and destructor 0081cee0.
- Slot 0 is ID-1-only QueryInterface; slot 3 is the deleting wrapper. Do not infer a join.
- Exact remaining WWW inventory lives in docs/specs/web-network-www-bodies.md; no full closure.
- Q16 was supplied on core-www-close13 and already archived here; do not reopen it on merge.
- No ABI changes, game labels, pending operations or uncommitted work are intended at ready tip.
