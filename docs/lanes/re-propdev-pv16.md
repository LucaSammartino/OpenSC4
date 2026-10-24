| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv16 (PV16/PV17 callback effects) | Codex / `re-propdev-pv16` (detached HEAD) | ready: PV16/PV17 answered; seven conformance vectors; doccheck PASS | Land spec; engine lane capture native shuffle and dated effects at specified boundaries |

## Report

Answered PV16 and PV17 in `docs/specs/prop-developer.md`, with seven prose conformance vectors in §12
(PV16-01–03 and PV17-01–04). Behavior milestone: `b4f243a4e`. Recovered the full question entries from the
explicitly named engine worktree, archived them with answers dated 2026-10-24, and regenerated QUESTIONS.md.
No new question remains. The owner report has no Owner closure table or `open:` rows; its Report/Handoff
and both filed questions supplied the closure requirements instead.

PV16: §§4.8 and 5.3 establish the bounded native shuffle scope. The dawn/dusk branch has no prop callback,
notification send or first-batch work; all n−1 CRT draws occur within its shuffle. Later broadcast recipients
can alter the same thread's CRT and night membership before a later observer. The four-token fixture starting
at CRT 1 yields [A,B,D,C], CRT `0x18BE873A`, batch 1 and cursor 0. Capture at the real shuffle or developer
callback, accounting for thread/nesting; equal late counts cannot validate list identity.

PV17: §3.3.1 specifies recurrence dates, inclusive transitions, one advancement per call (365 is calendar-year
recurrence), timer mask writes and chance behavior. Activation draws from the separate occupant MT at
`0x00B231B0`. A changed placed state posts `0xE9C86DDB` with prop/state payload and held prop; model updates
and date writes finish before the next dated entry. The queued receiver does not run merely on posting.
Ordinary expiry neither deletes the prop nor changes developer list membership. §4.4 bounds service return,
hold/holder/tag/release: no developer pump intervenes, but custom virtual calls or wrappers can invoke observers.

Evidence: queried the export first, then read bounded owner decomp sections; numeric widths, constants,
date comparisons, CRT arithmetic and call order were checked against `re_query.py asm --bytes`. Relevant
instruction ranges are cited in the spec. REA was not used or needed. No decompiled source or assembly
listing was committed. The transcripts are synthetic instruction-derived expectations, not game measurements.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` PASS;
`git diff --check` PASS. Cargo/clippy/CTest: n/a, documentation only. Game runs: none. The engine owner's
existing selector-stream verdict is not promoted to full-operation exactness. Native observation work and
fresh output verification remain for that lane. No engine/plugin/capi/tools/coverage-map.tsv, ABI tables,
version, game files, shared refs, or main checkout were edited; no proposed ABI row file is needed.

## Handoff

- Goal complete: PV16/PV17 behavior, conformance vectors, archive and generated index delivered.
- Milestone `b4f243a4e`: behavior and seven vectors; final documentation gate PASS.
- Next: supervisor publish detached tip and land; retain archived answers if the owner's question filing lands later.
- Engine next: capture CRT/list at real shuffle boundary, separate queued state messages from synchronous dated effects, then verify outputs.
- No game request or asynchronous command is outstanding; no full-operation exactness claim.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv16-r1-manifest.json.
