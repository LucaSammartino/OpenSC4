| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-query-ct5 (query behavior CT14, CT18–CT20) | Codex / detached `re-query-ct5` worktree | **wip**; verified supplements committed, doccheck passed; query owner remains incomplete | Complete residual CT14/CT18/CT19/CT20 before owner closure. |

## Report

Published verified query subcontracts in city-tools-input §8.16 and corrected the existing hospital-list,
wealth-constant and commute-summary text. Added 16 conformance vectors covering table dispatch, funding
lists, developer selection/ties, wealth geometry, commute bytes, curve endpoints, airport resource identity,
neighbour scripts, debug keys and writer order. No engine, plugin, capi, ABI or coverage-map changes.

Findings that change implementation assumptions:

- Hospital staff's count-four related-ID list includes adjacent-data words `4` and `0x3F800000` after
  `0x1023`, `0x1024`. School staff has four ordinary IDs; the other four funding sliders have no related IDs.
- Occupant filler slots `0x1026`–`0x1028` are genuinely null; zone dispatch is separate.
- Best-developer selection has zone-specific candidate order, signed16 desirability ranking, truncated
  demand as the tie-breaker, and an initial `(-1000,-1000)` eligibility sentinel.
- Wealth size uses binary32 `0x3EAAAAAB`, moves only the left edge, and preserves a reference coordinate
  across refreshes. The 90-pixel/wealth-one vector exposes truncation at left 69 rather than 70.
- Commute samples are unsigned bytes; zero becomes 255 before the curve. Constructed ordered curves clamp
  at endpoints and interpolate using stored slopes. Property-to-curve construction remains unverified.
- Lost-luggage constants form one complete resource key, not a list of three candidate airport exemplars.
- Neighbour kinds use last-successful type priority. Kinds 4/5 choose scripts by list count, with no special
  global-window lookup. All one/two-record script IDs are now specified.
- Debug enable requires the application functionality byte plus Shift/Ctrl/Alt. Building-pick bookkeeping
  and subsequent cell-writer phases are distinct; the writer order is now explicit.

### Validation

- `python tools/questions.py index`: passed; generated index already had the same question counts.
- `python tools/doccheck.py check`: **passed**, `doccheck check: ok`.
- `git diff --check`: passed at the spec milestone.
- Cargo, Clippy and CTest: n/a, documentation-only RE lane. Game runs/shadow comparisons: n/a; none requested.
- Static evidence: main-checkout `re_query.py` export queries, selected per-class pseudocode, and executable
  data bytes. Numeric instructions were checked for the published ranking conversion, commute byte handling,
  wealth constant/geometry and curve evaluator. No REA calls were needed or made. No decompiled output is
  committed. Ignored helper `build-runs/read-query.py` retains the bounded function/data reader.
- Commits succeeded on detached HEAD despite Git printing a denied shared `packed-refs.lock` maintenance
  attempt. No branch switch or shared-ref update was attempted by this lane.

### Questions and limitations

This is a partial answer, **not owner closure**. Only the resolved subcontracts were archived as answered
portions in `docs/specs/questions/answered/city-tools-input.md`; residual entries retain CT14, CT18, CT19 and
CT20 in `docs/specs/questions/city-tools-input.md`. No new IDs were filed.

Still required: neighbour row fillers and complete MySim branches; empty-cell sound conflict; airport noise
instruction verification; aura endpoint verification; curve property parsing/slope construction and malformed
knots; missing records/zero capacity/exceptional conversions/budget overflow; both complete traffic-tip
writers and trip-volume helpers; complete description/debug-file writer bodies and network backing fields.
Inventories in §8.16.7 are explicitly incomplete and cannot be treated as native-owner callback contracts.

Read the core-query-tool5 report from `main`. The requested engine module README was not read because the
task also explicitly forbids reading `engine/`; the report's Unsupported list supplied the closure scope.
Research stopped at measured context 105843/150000, slightly beyond the requested two-thirds research limit;
the remaining work was publication and validation. The queue is **wip**, despite the requested ready target,
because a ready query-closure claim would hide the remaining unanswered contracts.

## Handoff

- Goal: answer city-tools-input CT14 (query variant), CT18, CT19 and CT20 in prose only.
- Done: initialization `f98f46f4`; verified supplements and 16 vectors `7f540322`; doccheck passed.
- Done: archived only resolved portions of CT14/CT18/CT19/CT20; narrowed residual questions without claiming owner closure.
- Next: query `GetBuildingOccupantTrafficInfo` (`0x004D0190`), `GetNetworkOccupantTrafficInfo` (`0x004D07C0`),
  and trip-volume helpers (`0x004CFB30`, `0x004CFD20`); finish the residual contracts listed above.
- Verified: hospital staff IDs include `4`, `0x3F800000`; wealth k `0x3EAAAAAB`; commute byte 0 -> 255;
  developer sentinel (-1000,-1000), signed16 desirability then truncated-demand ranking.
- Budget: research stopped at measured context 105843/150000 after crossing the requested two-thirds limit.
- Unfinished: neighbour row fillers/MySim, full writer bodies/traffic tips, empty-cell sound and remaining numerical edge cases.
- Facts: main source checkout `C:/Users/tlori/FreeCity4`; no engine source reads/edits, ABI changes or game requests.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-query-ct5-r1-manifest.json`.
