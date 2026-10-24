# network-geometry: open questions
<!-- Append-only engine questions; index regeneration belongs to the coordinator. -->

<!-- from: glue-nettools-shadow12 (engine lane, 2026-10-24) -->
- **NG8 - section 3.3 underground drag extension, slot 65 override:** what is
  the precise occupant lookup/filter/layer and acceptance rule of the pipe/subway
  `TestForAutoConnectOccupant(x,z)` override (`0x00659470`), and how does the
  one-cell reserved-tile result participate in extending a drag? Section 1.1
  supplies the callable slot signatures, but section 3.3 does not specify that
  selection body. Clean run 512, `nettools-shadow12-draw-2032-20261024-aeaff8388`,
  observes type 4 `(16,44)-(10,40)` with snap false and true: native/core return
  true/true, cells 12/10, straight 0/0, total 7/6, repeated through 2032-01-01.
  The dimension-only recording host intentionally omits `extends_diagonal_end`;
  its empty-world answer is an adapter difference, not proof the native is wrong.
  Which public occupant queries reproduce the underground predicate without
  reading native tool fields or substituting the observed total? [retain the
  missing extension input and all differences; no narrow-drag exact credit;
  wide-drag public projection remains separately measured, not whole-owner proof]
  Normal-speed run 513, `nettools-shadow12-draw-normal-2032-20261024-aeaff8388`,
  is also clean to 2032-01-01 with ref_warnings=0 and reproduces both differences.
