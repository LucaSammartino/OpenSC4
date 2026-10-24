| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-regionalcity9 (regional city core) | `Codex` / `core-regionalcity9` (engine lane) | Ready; cargo 7001/0, clippy clean, CTest 79/79; verify PASS; RY7; no ABI changes | 1. Land core owner. 2. Qualify native seam and RR3 before game integration. |

## Implementation

Implemented the cSC4RegionalCity core owner in `engine/core/src/regional_city`
from regional-city, regional-city-records and foundations §11: owned fields and
interface identity/lifecycle, statistics and growth, preferences/cache/message
handling, commodity/deal lifecycle, connection reconciliation and edge callbacks,
record codecs and package/path helpers. World longitude/latitude and base elevation
have separate accessors. Raw wire decoding is separate from object Read normalization.

Implementation milestone: 31700cf87. Final source milestone: 803244736, including
the slot audit and foundations §14.20–22 conformance vectors. The module README
maps every owner section to code and tests; no specified owner method is deferred.
Round 2 reviewed remaining rows: none are unimplemented outside the documented
native integration work. This round changed reporting only.

## Verification

- Focused owner tests: 13 passed (`build-runs/focused-1.log`).
- Focused conformance: 4 passed (`build-runs/conformance-final.log`).
- Explicit-file formatting: 5 Rust files, zero diff lines.
- Final stable-source `qbuild all`: Rust 7001 passed / 0 failed (summed across
  43 result lines), clippy zero diagnostics; native label
  `qbuild-20261024-091216-build-ctest`, build 47/47 (88.6 s), CTest 79/79
  (180.8 s), total native 271.4 s. Rust evidence: `build-runs/qbuild-test.log`
  and `build-runs/qbuild-clippy.log`; native verdict retained under `build-results/`.
- Collected the existing follow-up verify request
  `qbuild-20261024-092447-build-ctest`: PASS, build 46/46 (1.6 s), CTest
  79/79 (74.9 s), total 78.6 s. Original session 16861 was unavailable after
  transport restart; waited for that same request's terminal verdict with session
  76950. No duplicate build request was launched. Evidence:
  `build-runs/build-results/qbuild-20261024-092447-build-ctest.verdict`,
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- Earlier passing gates remain recorded in `build-runs/build-results/`;
  the initial borrow-check diagnostic was fixed before the passing gates.

## Questions and limits

Filed RY7 in `docs/specs/questions/regional-city.md`: unspecified malformed Read
counts, map defaults/duplicate keys and failure atomicity. Literal choices are
documented in code; the codec round-trip does not claim normalized object equality.
Existing RR3 concerns native network arguments and needs qualification before
replacing the game's edge-rebuild path.

No C ABI exports or plugin behavior changed. No game requests were made; no
in-game shadow, reference-warning count or native equivalence is claimed.
Native integration needs a future read-only plugin seam; application save
traversal and thumbnail rendering belong to other owners.

## Next steps

Land the tested core owner. Then qualify RR3 and capture interface/record/deal
snapshots through a native seam before enabling replacement. No verification
requests remain outstanding. The worktree stays detached; the supervisor publishes
the actual committed HEAD to the reserved lane branch.
