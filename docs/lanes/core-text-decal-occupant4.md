| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-text-decal-occupant4 (FIRST portable TextDecal owner) | Codex / `core-text-decal-occupant4` (engine lane) | Ready portable partial: source6e493859; cargo5517/0/0, clippy0 diagnostics, native4/4, CTest68/68, fmtRC0; ABI unchanged; no game runs or new questions. | 1. Coordinator review and land bounded core implementation. Keep TDO1 open. |

## Portable owner implementation

## Handoff
- Goal: bounded FIRST portable TextDecal owner; implementation and final checks complete.
- Done: 07020c67 algorithms; f1ef930e lifecycle/records/18 fixtures; 6e493859 final code; c810a1d4 full report.
- Final frozen source6e493859: cargo5517/0/0, clippyRC0/0 diagnostics, native4/4 RC0, CTest68/68 RC0; qbuild all RC0. All handles terminal.
- Guard denied queue creation after report commit: context budget exceeded. No merge-queue file was written.
- Exact next step: coordinator review full report below and publish own ready queue for actual final HEAD if authorized; no source edits or check reruns needed.
- Engine SHA values and prior failures are recorded below. No ABI changes, no game runs, no new questions; TDO1 remains open.
- This is maxRounds1; no new implementation round/refund/rename requested. Guard requires transport state continue solely for budget handoff.

## Report

Source: engine/core/src/text_decal_occupant.rs and its tests.rs fixture module.
Base detached HEAD482b170d; train115 publication and full engine admission
were established by the brief. Existing consumer owners stay unchanged.

FIRST core family round 1/3, separate from the capped original RE FINAL 3/3.
Portable Rust only; no native binding, C ABI, factory hook, default, region,
replacement, runtime/shadow, coverage-map or full-class credit.

Implemented a dedicated TextDecal owner with production PropOccupant and
OccupantBase state, not a second private prop model. Existing lot_records,
prop_manager/LabelTool and effects consumers are unchanged. O/O+12/O+16 facts
map text, separate center, IDs and orientation to portable named fields; no
native pointer/layout emulation.

| Methods | Status and boundary | Authority / evidence |
|---|---|---|
| Constructor / GetText | Supported portable defaults and output-string assignment; existing prop owns its own fields and bounds | spec §§2–3; constructor G1 fixture |
| SetText / UpdateTexture | Supported ordinary non-reentrant path: assign, null/release old source, raw factory transfer, pointer-only rebuild decision; failed factory retains old IDs | §§3–4; R3 |
| RemoveDecals | Supported with valid manager for each live ID; A then B, sentinel store after callback; high-bit IDs valid | §4; G2/G4, R4/R5 |
| UpdateOverlayManager / SetDynamic | Supported bounded raw lookups; B 1/3 before A 0/2; AddRef-new/store/Release-old; equal pointers quiet but rebuild retained; raw byte2 differs from1 | §§3–4; R1/R2/R5 |
| UpdateDecal | Partial: owns q, stores, swapped extents, table, box, A then B add/color/alpha, notification and no partial-Add rollback | §5; literal G1–G4/R4 |
| Height / color / alpha / orientation | Supported storage/comparison branches, with rebuild inheriting geometry limits; nearest24 masked-exception comparison domain | §3; setter boundary fixture plus G2/R2 |
| SetPosition / MoveDecal | Partial: owns center, prefix/terrain ordering and dynamic/static choice; shared box translation is one typed raw base operation | §3; P1 |
| Init / SetAllFlags / DoMessage | Partial: owns guard, config/flag decisions, routing/source ordering, subscriptions, filter ownership and delegation; shared base operations remain foreign | §6; init, flags and message fixtures |
| Shutdown / destruction | Supported under completed-base-Shutdown protocol: initialized guard, unsubscribe, A/B remove/null/release, source/base; Drop separately B/A/source/text/base, never implicit Shutdown | §6; R6 and null-manager fixture |
| Read / Write extension | Partial: complete valid records, v1 unsigned numeric height, v2 extra word, v3/future shape, duplicate center, orientation byte; no setters or resource reset; final error AL only, base-false suffix skip | §7; S1–S3 literal byte fixtures |
| Embedded prop state / record | Reused: PropOccupant, OccupantBase, PropRecord::of/apply, read_prop_record/write_prop_record; no generic codec fixes | existing prop contract; S1 real-codec roundtrip plus literal extension suffix |
| Base Init/Shutdown/flags/translation/message | Foreign: individual typed raw base calls only; no TextDecal outcome supplied by host | §§3/6; instrumented primitive calls |
| Foreign replies/effects | Source identity/dimensions/transfer, raw altitude/view level, manager lookup, single overlay add/remove/color/alpha/position, references, filters/config and subscription primitives | §§4–6; trace fixtures |
| Unsupported | Other arithmetic policies, geometry subnormals/nonfinite/overflow, dimensions outside1..=2^24, table indices outside0..3, missing live-ID/add manager, invalid borrowed filter, record truncation; no silent false/default/rollback | explicit Unsupported enum and domain fixture |

TDO1 remains OPEN. Nearest24Normal is explicitly caller selected, nearest/even
at each operation and each binary32 store; q is retained after storing size.
Geometry supports exactly loaded dimensions1..=2^24. Height, center, raw-f64
altitude, intermediate operations and rounded stores must be zero or finite
values with magnitude in the binary32 normal range (f32::MIN_POSITIVE through
f32::MAX). Raw altitude is one foreign value, not a precomputed placement.
Existing x87::r24 is reused only inside this bounded normal finite domain;
x87::u32_to_float supplies the v1 unsigned numeric conversion. No global
helper semantics change, arbitrary-bit/native-x87 or universal PC/RC claim.
NaN/epsilon/signed-zero setter branches follow the published masked-exception
rules; nonfinite state can be stored while a later geometry operation is
explicitly unsupported.

Owner retains Host through Rust Drop so source/reference final effects are not
lost. Call Shutdown before dropping a live initialized subscription owner.
Callbacks can inspect state; mutation/reentry, OOM and foreign exceptions are
unsupported. Temporary filters release locally; borrowed message filters do
not. No implicit Shutdown, manager fallback, add rollback or generic-stream
repair was invented.

18 independent fixture functions use literal G1/G2/G3 dyadic results/bit
patterns, exact epsilon neighbors and S1/S2/S3 bytes. Raw hosts supply only
primitive replies and record state during callbacks. No host-computed geometry,
whole-owner mock, sampled expectation or two-identical-path numeric oracle.

## Prior checks and retained failures
- Baseline all RC1: cargo5499/0, clippy0 diagnostics, native217/217; CTest67/68,
  cargo_test failed because tests module was absent while tree changed.
  Baseline-* logs retained. This is not a clean baseline.
- Initial quick RC0: eight new fixture groups; cargo3463/0/0 in5 result lines,
  dependents compiled, native4/4. Tree changed due milestone commit.
- Expanded compile quick RC1: private-record imports / fixture missing Copy;
  native1/4. Handle66740 returned terminal after689.2s; no process/service kill
  or restart. failed-expanded-quick-*.log retains all three logs. Fixes use
  existing public reexports and Copy. Do not erase this failed attempt.
- Corrected focused quick handle74016 terminal RC0 on f1ef930e: cargo3473/0/0
  in5 result lines, all18 new fixture functions passed; dependents compiled,
  native4/4 (167.2s), fc4verify1069.7s. corrected-focused-quick-* logs retained.
- First frozen all on80a37737 terminal RC1: cargo5517/0/0, native4/4 RC0,
  CTest68/68 RC0, clippy101 with manual-non-exhaustive lint and follow-up warning
  (2 diagnostics); fc4verify397.6s. Standard #[non_exhaustive] replaces the private
  unit construction seal without changing owner algorithms. failed-canonical-*
  logs retained; this failure is not treated as success.
- Fmt RC0 for both new Rust files; additive lib.rs module-list line only.

No new questions. Existing TDO1 stays open. No game runs requested or claimed.
No C ABI exports/version changes.

The first quick detailed logs were replaced by the expanded compile attempt;
actual terminal output is preserved in initial-quick-console.txt. Subsequent
failed, corrected and final logs remain distinct. No cache RPC, process kill,
daemon/compiler/client/validator restart or duplicate concurrent check occurred.

## Final canonical gate and exact tested source

Serial qbuild all ran on frozen committed source
6e49385928ca5fd5aa7fd53d1c415dcb91568451; handle12017 terminal RC0:

- Workspace cargo RC0: **5517 passed, 0 failed, 0 ignored**, summed over all26
  result lines, build-runs/qbuild-test.log.
- Clippy -D warnings RC0: **0 diagnostics**, qbuild-clippy.log.
- Native build RC0: **4/4 edges**, 64.6s, fc4verify-build.log.
- CTest RC0: **68/68 passed, 0 failed**, 125.6s, fc4verify-ctest.log.
- fc4verify RC0, 845.5s; outer qbuild RC0. No source changes during the run.
- Touched owner/fixture rustfmt checks RC0; lib.rs is one additive module-list
  line, without mass root/module-list formatting.

Engine SHA-256 values, unchanged by the final report commit:

| File | SHA-256 |
|---|---|
| text_decal_occupant.rs | 503b16d0890b48d4b35a531d0e9a4bceccf8b9e856401a36ac6d75f7e82326e8 |
| text_decal_occupant/tests.rs | 83603324ea952e9a3fac7c1b0df10aa905690e7690ff195cb3d16fe9bc99731d |
| lib.rs | edfd9fb57c0ac4a8dfd277cac127511a867c3134d7ab71ec4ab14886a4f800ec |

All long-command handles are terminal. No game requests; game_runs=[]; no
native owner execution, shadow, replacement or full-class credit. TDO1 remains
open in docs/specs/questions/text-decal-occupant.md; no new questions filed.

## Commits and next step

07020c67: state/source/routing/geometry and initial literal fixtures.
f1ef930e: lifecycle/setters/record extension and18 fixture functions.
80a37737: equivalent finite-range predicate and focused gate record.
6e493859: standard non-exhaustive construction after clippy feedback.
The final report commit changes the lane file only; engine hashes exactly
match the frozen tested source above. The queue records the actual final HEAD.
Coordinator review/landing is next; no continuation/refund/rename requested.
