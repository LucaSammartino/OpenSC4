| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-ms21 (dispatch route and constructor rules) | `re-mysims-ms21` / `re-mysims-ms21` (RE lane, detached HEAD) | ready; MS21/MS22 answered in `b4326190d`; eleven normative cases; questions check and doccheck pass; no ABI change or game run | (1) Coordinator land; (2) engine lane implement the specified route selection and auxiliary comparisons. |

## Report

Resolved both blocking questions in [mysims](../specs/mysims.md), §4.6.3,
§6.2.1, §7.2.3 and new §12.30. The question paths with repeated `docs/specs/`
in the brief were interpreted as the existing canonical mysims files.

- MS21: inclusive start S at +30, exclusive end E at +2C; R[S] is the route-ready
  marker. Specified equal-mode partitioning, ordered unsigned x/z network probes
  with arguments `(x,z,0x1FCF,0)`, start trimming, absent-run progression, FE and
  exhaustion retention, explicit jump relocation to R[S−1], current-range dispatch
  extraction and callback-5 selection order. Ten worked normative cases include
  successful dispatch/skip, rejected FE, exhaustion and all-zero network results.
- MS22: constructor +FC is null and +100 is zero; +10 is unwritten before Init.
  One normative case checks prefilled flag preservation and semantic reference
  decoding. The ordinary Init path later clears +10; no allocator-zero assumption.
- Moved both complete entries to [answered questions](../specs/questions/answered/mysims.md)
  with dated answers and regenerated the question index: mysims now 0 open / 22 answered.
  No new questions. Implementation files and public ABI tables were not read or changed.
  No proposed ABI rows are needed.

Evidence came from the main checkout's `re_query.py` export and narrowly read
`decomp/cSC4MySimDispatch.c`. Numeric widths, arguments, bounds and stores were
verified against byte-bearing instruction queries, cited in the spec. No REA
session was used. The named engine lane report has no Owner closure table in
this checkout; its MS21/MS22 and uncompared-scope paragraphs were read first
alongside the open question entries.

Validation: `questions.py index`, `questions.py check`, `doccheck.py check` and
`git diff --check` passed. Foreground gate logs are under this worktree's
`build-runs/{questions-index,questions-check,doccheck}.log`. Cargo, clippy and
CTest are n/a for this spec-only change. No game run was requested or measured;
the new cases are normative vectors in prose, not executable engine fixtures or
live owner equivalence. The public input-observation limitation in §7.2.3 remains.

Milestone: `b4326190d` contains the answers, cases and question-index update.

## Handoff

- Goal: answer MS21/MS22 with behaviour and conformance cases; no engine edits.
- Done: `b4326190d` answers both, adds eleven normative cases and archives questions; gates pass.
- Next: coordinator land; engine lane consume §4.6.3 and §12.30 for previously unsupported paths.
- Facts: S=+30, E=+2C; route-ready uses R[S]; network probes unsigned x,z,1FCF,0.
- Constructor +10 unwritten; +FC null; +100 zero. No REA used.
- Source main: C:/Users/tlori/FreeCity4; export queried read-only.
- No game requests, ABI edits, or engine reads.
