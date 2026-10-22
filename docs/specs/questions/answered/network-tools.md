# network-tools: answered questions

<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NT19 - network-geometry §11, the power-line parameter defaults:** only `+0x3A4` has a stated default (20). What
  are the defaults of `+0x3A8` (`0x098B27D0`), `+0x3AC` (`0x4A5F5953`) and `+0x3B0` (`0x0A5F596B`) when the exemplar
  lacks them, and what do `+0x3A4`/`+0x3A8` control? The core uses 0. [missing]
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — network-geometry §11/power-line-tool §§1,4:
  fresh Init defaults uint32 20, float32 7.0, uint32 0, float32 0.0; spacing in drag steps at pivots;
  clearance raises ray endpoints. Re-Init resets only spacing/clearance. Missing cost properties retain
  prior values. Evidence `0x006506AA–0x00650704`, `0x00650A43–0x00650A62`, `0x00650CE8–0x00650D9C`.

- **NT20 - §9 step 4 and §11 NT3, two literal readings:** (a) "When `real`, `AttemptNeighborConnections` is followed
  by `PlaceNeighborConnections`": does `AttemptNeighborConnections` also run in draft mode, and does
  `PlaceNeighborConnections` depend on its result? The core runs Attempt whenever the conditions hold and Place
  after it when real, regardless of the result. (b) In `CheckForReservedTiles`, a zoned (1) or road (3) last cell
  leaves `last` at the previous clear cell, so a line ending on a zoned cell loses that cell (v = last + 1); is that
  the original's behaviour, and what does a `CellIsOKForPlacement` result above 3 do (the core ignores it)? [ambiguous]
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — network-tools §9: Attempt runs in both modes;
  Place requires success and real=true (`0x00652A34–0x00652A46`); gate counts input anchors, not poles.
  Network-geometry §11/power-line-tool §6: zones/roads preserve last except final road after result 0,
  which stops at currentStep-1. Clear sets last; initial last=count-1 explains an all-zone walk retaining
  count. Results above 3 preserve last/failure but replace previous result; first previous is uninitialised.

- **NT21 - power-line-tool.md §§5,9, connection helpers:** prove `0x0064E3E0`'s connection geometry and
  optional wire-vector ownership. Enable/restore `0x00649910` and reference-copy `0x0064B6E0` now have
  instruction-derived contracts in §5.2; these two helper obligations are resolved.
  Reach attributes these pole-occupant helpers to this tool. Observed ordering is specified, numeric
  geometry is not. [not proved]
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — §§5.2,5.3,5.5 specify unsigned
  endpoint ordering, four attachment pairs, parameter defaults/properties, x87 control and cubic
  evaluation, N=truncate(max(minimumCount,length/sampleDistance)), and independent geometry storage.
  Optional wire-vector copying takes AddRefs only at the geometry-building endpoint, leaves the
  input unchanged and transfers their release responsibility to the stored connection list.

- **NT23 - regional Attempt:** full type-5 AttemptNeighborConnections `0x00628BD0` endpoint/direction
  derivation, failure/consent behavior, vector mutations and preview effects need instruction checks.
  NT20 resolves the wrapper pairing only. [not proved]
  **ANSWERED 2026-10-22 (re-powerline-completion4):** MATCHES — §5.4 traces all 1438 B for type 5:
  final-anchor/step direction, candidate border and validation, regional type 4, failure returns
  without failure-code writes, append-only paired vectors and no consent UI or preview creation
  inside Attempt. The wrapper's separate Place call remains governed by NT20.
