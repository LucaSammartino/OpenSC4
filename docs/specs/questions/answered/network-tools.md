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

<!-- from: glue-nettool-shadow5 (engine lane, 2026-10-23) -->
- ~~**NT27 - same-process scripted placement result observation:** specify a public-interface
  observation contract for native solved cells with network types, piece IDs and orientation
  after slot 7, and confirm slots 23/25/26/27 expose the ordered dragged cells and current
  valid/straight/total step counts after real placement, before Shutdown. Section 2 names
  internal offsets but does not define a public solved-record getter or its ownership/layout.
  [compare only public drag/status/cost getters; never invent class-method signatures or
  read undocumented object storage; solved piece/type coverage remains unverified]
  The existing plugin host also leaves bridge types/models and powerline-specific placement
  unwired. The diagnostic replay uses the existing adapter's literal zero underground Init
  values, shipped geometry parameters and zero demolition cost: its differences in these
  paths are recorded, not suppressed or credited as exact whole-owner coverage.~~
  **ANSWERED 2026-10-24 (re-nettool-nt27):** MATCHES — network-tools §2.1/§14 and tests 20/30:
  slot 23 assigns a caller-owned coordinate-vector copy; 25/26/27 return current valid/straight/total
  step counts before Reset/Shutdown. No public solved-record getter exists; coordinates do not
  certify type/piece/orientation or missing bridge/power-line adapters. Original entry recovered
  from glue-nettool-shadow5 because this lane's base predates its filing.

<!-- from: glue-nettool-shadow5 (engine lane, 2026-10-24) -->
- ~~**NT28 - observed public result differences in Konradshohe:** run
  nettool-shadow5-r1-20261024-a is clean through 2032-01-02, but the original road
  (20,4)-(28,12) getter returns 17 dragged cells where the core returns 9 (both
  total=9, straight=0, valid=9). Avenue (4,28)-(16,28) returns 26 cells in a
  different order from the core's 26. What is the ordered slot-23 projection
  relative to geometry's drag cells/extra cells, including width ordering?
  Rail (4,24)-(16,24) and avenue (4,28)-(16,28) return false, failure=1,
  valid=0; the core returns true, failure=0, valid=0. Which failure path yields
  literal 1 (rather than the section 5.3 F000xxxx codes), and what rule is
  missing from the documented validation contract? The lowered-strip bridge
  attempt has native valid=17 versus core 0; powerline has 13 versus 0
  (known unwired adapter paths). [retain all differences, no exact owner credit;
  costs are sampled against post-commit terrain in subsequent source so that
  pre/post terrain phases are not mistaken for an engine cost difference]~~
  **ANSWERED 2026-10-24 (re-nettool-nt27):** DIFFERS — network-tools §3.5/§5.3, tests 21–24:
  slot 23 includes all staircase turn cells, grouped in major-axis steps; minor coordinates
  increase within each step even on reversal. Extra cells are a separate inventory. Slot 14
  is a normalized Boolean, not the internal F000xxxx code: every nonzero reason yields 1.
  A refused nonempty run returns false with default stop-on-failure. Boolean logs cannot identify
  the specific rejecting pass. Original entry recovered from glue-nettool-shadow5.

<!-- from: glue-nettool-shadow5 (engine lane, 2026-10-24) -->
- ~~**NT29 - cost/height replay observations:** corrected-source turbo run
  nettool-shadow5-r1-20261024-b is clean to 2032-01-02. Both cost getters now
  read the live post-commit terrain, yet road (4,4)-(14,4) costs original/core
  110/113, and street (64,8)-(74,8) costs 55/61; a repeat of the first road
  costs 110/110. The two diagonal road runs cost 170/102 and 90/54 (native
  slot 23 has additional cells, NT28). Which retained height/terrain records
  must the post-commit GetCostOfSolution sample, and is the extra-cell
  projection also the cost-cell inventory? [retain earthwork/cell-cost
  differences; no tolerance or compensating cost is applied; bridge and
  powerline costs remain explicitly uncovered by the existing host]~~
  **ANSWERED 2026-10-24 (re-nettool-nt27):** DIFFERS — network-tools §8.1/§8.1.1, tests 25–29:
  price retained solved entries (including appended neighbours), their cached network corners
  refreshed from the network satisfier during commit, and live terrain at stored vertex IDs.
  Count shared corners per entry and round each running sum addition to float32. Neither slot 23
  nor the extra-cell vector is the general pricing inventory. Cell +0x52 excludes intersection
  cells, not already-built cells; placed +0x51 does not suppress the charge. Original entry
  recovered from glue-nettool-shadow5; no guessed per-run height residual or compensating price.
