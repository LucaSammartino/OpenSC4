# lot-records: open questions
<!-- Questions about [lot-records.md](../lot-records.md). Append new entries at the END; when one is answered, move it to answered/lot-records.md. Rules: README.md. -->

<!-- from: core-lotrecords-final4 (2026-10-22) -->
- **LR6 - section 7.1 GetLotWallEdgeFlags step 4:** when lot manager slot 11 `GetVerticesAtEdgeOfLot(R, 1 << i, V)`
  returns false (its return is not tested), which vertex range does the loop walk: what V holds before the call
  (initialised by the caller, or left from the previous edge), or what slot 11 writes on its false path?
  [the host always fills V and the loop walks whatever it filled]

<!-- from: core-lotrecords-final4 (2026-10-22) -->
- **LR7 - section 7.2 step 3:** when occupant manager slot 33 returns true with `w` null, the update path calls
  `QueryInterface` through null. Is that reachable (does slot 33 ever return true without an occupant)? [treated as a
  host contract violation: the core panics, no null guard is synthesised]

<!-- from: replace-lot29 (2026-10-27) -->
- **LR17 - section 13.2.1 slot 26 SetUnit ownership (replace-lot29 LR9):** the adapter follows the specified sequence:
  if different, Release old retained unit, store new, AddRef new when nonnull. An on-save run reports
  13 final releases of native business units (class `0x4A232DA8`) at this boundary. Is final Release
  legitimate during native regrouping, or is a retain/caller obligation missing? Please specify the
  ownership sequence and permitted final-release boundary. Evidence: main `build-runs/game-results/`
  `lot29-r5-on-save.log`, first warning line 15124. Zero-warning save acceptance stays open; the audit
  must not be suppressed without a specified ownership justification.
  Follow-up: section 4 already specifies that Remove(last member) destroys the unit inside SetUnit(0).
  The adapter now checks GetMembers().empty() at this null-assignment boundary and logs each final release;
  other SetUnit releases retain the warning. Please confirm whether this is the intended boundary.
  Measured follow-up: `lot29-r6-on-save.log` reaches 2032 with ref_warnings=0 and exactly 13 logged final
  releases at the empty-member/null-assignment boundary. Its exported save then loads with `lotrec` off:
  `lot29-r6-on-to-off.log`, clean to 2032, ref_warnings=0, owner shadow exact 1482/1482.
