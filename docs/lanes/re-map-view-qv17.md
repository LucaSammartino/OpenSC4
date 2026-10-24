| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv17 (map-view QV1/5/11/16 closure) | Codex / `re-map-view-qv17` (RE lane; detached HEAD) | ready, partial: merged dependency `8caf1b84c`; QV5/QV16 `4276e6bd5`, QV11 `82b5541ed`, QV1 `7b7614a9e`; doccheck passed; ABI unchanged; game n/a | (1) Finish QV2 traffic volume; (2) finish QV3 panels/groups; (3) finish QV14 ring precision. |

## Report

Merged the requested `core-map-view-close12` dependency first. The only conflict was the generated
`docs/specs/QUESTIONS.md`; regenerating it with `tools/questions.py index` resolved the conflict.
The worktree remains on detached HEAD as instructed. No engine, plugin, capi, coverage-map or ABI-table
edits were made by this RE work; predecessor changes are present through the expressly requested merge.

Extended `docs/specs/map-view.md` with four complete behavioral closures and conformance vectors:

- **QV1, §§2/3.1, vector 35:** E3 controls the city level, plumbing coverage and surface water;
  subsequent F1 controls filter options only. E6 is the current zone-opacity byte, defaulting to
  the configured zone-manager alpha; none restores that default and normal terrain display.
- **QV5, §4.2, vector 1:** absent/out-of-range scale initializes both factors to 256, whereas
  explicit valid 256 computes factors 65535/32767. Signed 64-bit products use floor on right shift;
  explicit scale 1024 and signed −512 give index 64. Added default, explicit-scale and reset vectors.
- **QV11, §4.1, vector 34:** default/unknown developer layers draw no grid, layers 3/4 share a path,
  and layer 6 uses slot-13 geometry, fresh zero fill and ordered signed triples with wrapping
  multiplication by 12, last duplicate overwrite, availability gating and temporary interpolation disable.
- **QV16, §4.3, vector 21:** integer division truncates the fixed-point step before its integer
  half-step origin is formed. The disputed 2×2-to-3×3 centre is 0. A separate 2×2-to-1×1 case gives
  2 and distinguishes two-stage integer rounding from a single bilinear rounding.

All four original entries were moved to `docs/specs/questions/answered/map-view.md` with dated DIFFERS
answers; the questions index was regenerated after each closure. No new question IDs or ABI rows.
QV2, QV3 and QV14 remain open without newly invented splits. QV15 is the pre-existing separate open
question about missing lot-mean prose and was outside this lane's requested IDs.

Validation: `python tools/doccheck.py check` **passed** (`doccheck check: ok`), foreground log
`build-runs/doccheck-re-map-view-qv17.log`. Numeric rules were checked against instruction bytes via
the local export; citations are in the spec. No REA query was attempted because this session has no
binary open, as stated in the brief. Cargo/clippy/ctest and game/shadow runs: n/a for this spec lane.
No claim of runtime measurement or native ring edge-pixel equivalence was added.

Stopped new investigation at measured context **102791/150000**, following the requested roughly
two-thirds checkpoint. This is a passing partial-ready result, not completion of all seven questions.

## Handoff
- Goal remaining: complete QV2, QV3 and QV14; preserve the four committed closures above.
- Next exact step: query source-76 Update instructions, then read main decomp/cSC4WinMapView.c lines
  1902–2360 in ranges of at most 80 lines; this is the unspecifed traffic-grid construction/drawing path.
- QV2 facts: three fresh shift-1 zero grids (one u16 and two s16); ordinary travel types are remapped
  to capacity network types, then traffic slots 23/12 supply capacity and directional records.
- QV2 facts: selected view 0x54 takes a separate compressed ferry-route branch; +0x9AC selects
  alternate volume counts. Remaining drawing/overlay below line 2235 still needs tracing.
- QV3 next: cSC4WinMapView.c lines 3219–3900 (buttons/show/hide); helpers ShowSmallWindow and
  group selection may be in decomp/_unnamed/region_00790000.c or region_007a0000.c. Do not infer names.
- QV14 facts: ring helper 0x0079E130 uses x87 and does not set CW; CRT __setdefaultprecision
  0x009F6C30 calls controlfp with value 0x10000/mask 0x30000 (PC=53). Establish runtime RC/PC
  inheritance and explicit store schedule before choosing a threshold-discriminating vector.
- No ABI reservation, game request or REA operation is outstanding. All RE sources are under
  C:/Users/tlori/FreeCity4; edit only the reserved lane worktree and never read engine/.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv17-r1-manifest.json.
