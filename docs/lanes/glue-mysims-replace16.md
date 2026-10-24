| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysims-replace16 (dispatch constructor ABI checks) | Codex / `glue-mysims-replace16` (engine lane) | Ready measured checks only; cargo 7503/0, clippy clean, CTest 84/84; `mysims16-final-2032d` clean, ref_warnings=0; mysims 73/73 and mysimdispatch 73/73 exact. CREDIT FLIP incomplete; native factories retained. | Implement all three replacement owners and their host adapters; then test the full house replacement profile before appending mysims. |

Adds twelve public dispatch constructor ABI checks per sample: reference counts, U-Drive-It guards, and QueryInterface ownership. No direct concrete trip calls or live owner mutation.
Positive and negative C++ fixtures cover rejecting and accepting the probe IID, reference balancing, output clearing, and failed drive guards; failure masks identify individual checks.
The spec leaves unsupported-IID rejection unspecified: the checker accepts either policy and releases successful query results through FC4_ACQUIRED/FC4_RELEASE. A SPEC-GAP comment records this literal interpretation.
Adds default-off `mysims` reservation and two plugin factory tests. Explicit requests report replacement unavailable; none of the three native factories is intercepted. Shadows remain independently default-off.

Final gate: `qbuild all`, cargo 7503 passed / 0 failed; clippy 0 diagnostics; queued native build 49/49 edges and CTest 84/84. Failed initial return-channel audit preserved in `build-runs/fc4verify-ctest-r1-ret-channel-fail.log`; explicit dispatch AL-return citation fixes the audit's MySim-name match.
Final game: Konradshohe, turbo, requested 2032-01-01; verdict clean at 2032-01-12, ref_warnings=0, panics=0, foreign exceptions=0; DLL SHA-256 prefix `a6604091`.
`mysims16-final-2032d 73/73` mysims.
`mysims16-final-2032d 73/73` mysimdispatch constructor/public-ABI/header scope; live trips and transport remain unverified.
Evidence: `build-runs/game-results/mysims16-final-2032d.log` and `.verdict`; request under `build-runs/game-queue/done/`.
Earlier diagnostic labels `mysims16-f958d2b09-2032a`, `mysims16-f958d2b09-2032b`, and `mysims16-f958d2b09-normalc` remain preserved. All reached 2032 clean with mysims 73/73; their dispatch probe expectation was corrected in final source.

Missing: actual cSC4MySim/cSC4MyStreetSim/cSC4MySimDispatch replacement fronts, core host integration, and the full house replacement-profile run. The dedicated Street contract is `docs/specs/my-street-sim.md`; its absence from the brief is not a spec blocker.
Do not append mysims to the house replacement list. This is measured preparatory work, not a replacement-credit claim.
Questions filed: none, per revised lane instructions. C ABI exports unchanged. No coverage maps, tools scripts, or run records edited.
