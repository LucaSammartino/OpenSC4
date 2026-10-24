| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-te13b: TextEdit and FileBrowser closure | Codex; re-widget-controls-te13b | ready: doccheck passed | Coordinator lands queued tip; no further lane work |

## Report

All requested questions are closed at the owner/service boundary. The
requested owner closure tables were read first. Only specifications, question
records/index and this lane report changed; no engine, ABI, plugin, capi or
coverage-map edits. RE evidence came from re_query.py, selected cached
decompilation and numeric instruction checks with address citations. REA
was not called; no live Ghidra session or REA fallback was needed.

| Milestone | Result |
|---|---|
| `5000f7668` | Reconciled duplicate open TE9–TE12 with existing answered records from re-widgettk-wt13; preserved attribution. Added history/preferred-X default vector and verified constructor/counter instructions. |
| `71ba3ab06` | UTF16 adapters; browser wildcard polarity, unsigned column capacity/widths, sorted restoration/empty-grid requests and paint ordering. Six adapter and six browser vectors. |
| `f3dbc714d` | Completed TE5 conversion selector/count/error/alias rules, Thai class/level tables, composed lengths and fourteen more vectors. |
| `63ca06dc7` | Completed TE8 owned action-copy/growth, absent local cleanup/rollback, unsafe null-allocation quirk, strncpy NUL padding and forward/reverse byte-search edges; seven vectors. |
| `b01830010` | Completed FBQ2/FBQ5: twelve eager child controls plus lazy popup, factory/fallback/configuration, every message route/base delegation, directory/component/drive/file population, display fitting and popup sizing/focus. Ten more browser vectors. |

Notable instruction-verified corrections: popup columns are span+8 at
`009A47E9–47FB` (cached pseudocode said +2); indentation is the logical
shift of wrapped icon-width*9 at `009A492D–4946`; popup fitting subtracts
(depth+1)*icon-width and conditional bar width at `009A49E1–4A05`.
Primary slot 92 (`00949ADE–AE0`) only returns true, so paint's slot-92 call
is not itself invalidation. TextEdit growth/copy has no local unwind handler;
forward search's position+count is wrapping unsigned arithmetic, while the
owner rejects end-position empty matches. Specs describe these as quirks.

Browser inventory now has 66 original entries / 15,194 B with complete
owner-side contracts, or 67 / 15,904 B including its separately found
constructor. Remaining FBQ1/FBQ4/FBQ6 total 108 original entries / 13,713 B;
their existing pixel/property/ownership scope was not started or expanded.
TextEdit attribution stays 150 / 19,999 B with its conservative P/B counts.
Neither count claims foreign-owner implementation, whole-widget-family
coverage, executed conformance or pixel/shadow equality.

**Questions:** TE5, TE8, TE9, TE10, TE11 and TE12 are in
`docs/specs/questions/answered/text-edit.md`; FBQ2 and FBQ5 are in
`docs/specs/questions/answered/win-filebrowser.md`. No requested ID remains
open; no new or narrower question was filed. TE8 closes documented editor
behavior for controlled foreign inputs, without claiming the font, base,
allocator, drawing, dispatch, timer or clipboard owners. Questions index
regenerated with `python tools/questions.py index`.

**Gate:** one actual `python tools/doccheck.py check` run passed, saved in
`build-runs/doccheck-re-widget-controls-te13b-r3.log`. Both working-tree and
round-commit `git diff --check` passed. The earlier q.py wrapper attempt
never started doccheck: Bash failed CreateFileMapping with Windows error 5;
its distinct log is `build-runs/q/20261024-194818-28468.log`. The successful
check ran directly in the foreground via a local log-capture helper.
Cargo/clippy/ctest and game runs are n/a. No game labels or ABI proposals.
Git reported packed-refs.lock permission warnings, but detached-HEAD commits
succeeded; no shared refs/config/hooks were changed manually.

## Handoff

- Goal finished: all eight requested question IDs have answered records.
- Done: milestones above; doccheck passed on the finished spec changes.
- Next: coordinator publishes/lands the queue's full HEAD; no lane restart.
- Queue: build-runs/merge-queue/re-widget-controls-te13b.json, kind re/ready.
- Detached HEAD is intentional; supervisor publishes to the reserved lane.
- No game requests, ABI proposals, new questions or owner work outstanding.
- Preserve existing FBQ1/FBQ4/FBQ6 and foreign-service boundaries.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-widget-controls-te13b-r3-manifest.json.
