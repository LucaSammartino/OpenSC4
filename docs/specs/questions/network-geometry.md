# network-geometry: open questions
<!-- Append-only engine questions; index regeneration belongs to the coordinator. -->

- **NG15 — §9.2 nonflat straight-run cross-sections and constraint insertion.**
  The spec gives the three-section shape and slope/smoothness equations, but not the
  exact vertex pairs returned by `GetCrossSections` for each side/edge byte, its
  diagonal flag, or insertion/traversal order when adjacent cells share vertices.
  Please specify those details and an admitted nonflat width-one road fixture with
  its per-level constraints and solve result, so these can be distinguished from a
  missing piece/plane input rather than choosing a propagation order to fit a price.
  Blocking case: `gen27-spandau-s1-default`, julian 2452276, road `(65,7)→(69,7)`
  and reverse: native/core return=1/0, valid=5/0, price=144/0 (same ordered cells and
  counts). The vertical five-cell controls return true on both sides, price=66/75.
  Evidence: `build-runs/suite/plans-cities29-2/gen27-spandau-s1-default.log:8936`.
  The current implementation's inferred section mapping is retained; no constraint
  is removed or relaxed. Fresh public-terrain diagnostics are queued under
  `plans-cities29-r3-spandau`; that run has not produced a verdict yet.
