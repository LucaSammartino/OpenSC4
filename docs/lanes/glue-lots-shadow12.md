| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lots-shadow12 (lots and lot records verification) | `Codex` / `glue-lots-shadow12` (engine lane) | Ready partial closure: implementation 8089668c3; cargo 7214/0, clippy clean, native build 262/262, CTest 81/81; run 512 clean to 2032, lotrec 73/73 and lottrip 73/73 exact, ref_warnings=0; run 511 turbo stalled; LR15 open; public ABI unchanged | (1) Coordinator credit measured owners and append lottrip_shadow to the existing regress shadows string; (2) specify LR15 for the three excluded owners. |

## Report

Added `lottrip_shadow` (ini/environment, default off), a separately labelled observation of
`cSC4TripData` inline in the spec's master-lot inventory. It shares the existing LR12 native
capture and Rust full-record decode/re-encode when both keys are enabled. Trip counts are
returned through the existing private static-link transport; no public C ABI export changed.
Only complete byte-exact containing lots credit trips. Failed capture/read/consumption or
inventory errors remain mismatches, including when a trip denominator cannot be recovered.
Empty inventories are explicit and supply no nonempty evidence. No new game interface is called.

The subsystem remains partially verified. Section 14.1 explicitly excludes business-unit payloads,
base textures and retaining walls and requires a separate discovery contract for additional
inventories. LR15 requests the missing exhaustive discovery signatures, bounds/filter arguments
and reference ownership for these three owners. No speculative traversal or owner shadow was
activated. The existing LR12 scope is preserved; this does not claim off-list trips, simulation
mutation, or complete-owner replacement.

| Owner | Measured evidence / remaining contract |
|---|---|
| cSC4Lot | lotrec 73/73 exact load/month lines through 2032-01-01; 49,811 record instances |
| cSC4TripData | lottrip 73/73 exact load/month lines through 2032-01-01; 66,343 inline instances |
| cSC4BusinessUnit | Not measured; separate inventory and getter ownership requested in LR15 |
| cSC4LotBaseTextureOccupant | Not measured; exhaustive occupant enumeration/ownership requested in LR15 |
| cSC4LotRetainingWallOccupant | Not measured; exhaustive occupant enumeration/ownership requested in LR15 |

### Validation

- Focused Rust tests: 2 passed, covering count, complete consumption, truncation, empty evidence,
  negative-zero bits and the 0xCCCCCCCC sentinel. Explicit owned-file fmt passed.
- Stable-source `qbuild all`: PASS, workspace cargo 7,214 passed / 0 failed / 0 ignored;
  clippy 0 diagnostics; coordinator native build 262/262; full CTest 81/81.
- The same all operation delegated native verification as
  `qbuild-20261024-144300-build-ctest` and was collected to terminal PASS; no duplicate verify.
- Logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- The Bash q.py wrapper failed with Windows CreateFileMapping error 5 before launching the
  focused test; PowerShell subsequently ran it successfully. Failed wrapper evidence remains
  in `build-runs/q/20261024-143509-26960.log`.

### Real-city evidence

`glue-lots-shadow12-house2032-normal-r2 146/146`

[Run 512](../runs/glue-lots-shadow12-house2032-normal-r2-518.md): Konradshohe at normal speed,
clean through 2032-01-01, 114.890 real seconds. All 73 observations for each shadow were nonempty
and exact; `foreign_exceptions=0`, `watchdog_stalls=0`, `rtc=0`, `overruns=0`, `ref_warnings=0`.
Final lines: lotrec `exact=668/668 differences=0 errors=0 mismatches=0`;
lottrip `owner=cSC4TripData lots=668/668 exact=820/820 mismatches=0`.

[Run 511](../runs/glue-lots-shadow12-house2032-r1-517.md) preserves the turbo attempt:
`glue-lots-shadow12-house2032-r1 118/118` exact shadow lines, but watchdog stalled at
2030-11-28 before the target. Last heartbeat: main thread outside FreeCity4, last OnTick
7531 ms ago; `ref_warnings=0`. It is not credited as a clean 2032 run. Both runs used DLL
SHA256 prefix `c1f97ad6`. No game request remains outstanding.

### Coordinator follow-up

The immutable clean-room prohibition prevented reading coverage-map/shadows TSV files or editing
the regression profile under `tools/`; it also prevented establishing a per-owner byte ranking
from those files. The permitted coverage report had aggregate 73/73, verified part. The counts
above are direct log accounting, not an official invocation of tools/coverage.py. Reconcile
per-owner coverage mappings and credit the two measured record scopes; append `lottrip_shadow`
to the current single regress shadows string after review. Keep the other three owners open
under LR15 and default off until their discovery contracts are specified.
