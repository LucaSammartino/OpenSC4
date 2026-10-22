| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lighting-complete4: answered lighting contracts | Codex / `core-lighting-complete4` (engine, detached checkout) | ready portable changes; tested code `e5cc4376`; cargo 4938/0, clippy 0 diagnostics, ctest 68/68, fmt 10 files PASS; LT12/LT13 filed; ABI unchanged; game/shadow n/a; no whole-owner native credit | Answer LT12/LT13 and regenerate question index in a spec lane; native binding/shadow requires a separately authorized integration task. |

## Report (2026-10-22)

Reconciled the portable lighting implementation against the public LT5–LT11 answers and the
re-lighting-contracts4 report. Commits: `04068164` implementation, `94484116` independent contract tests
and boundary corrections, `e5cc4376` final constructor-map correction. The existing detached checkout
was retained. No RE/native executable, prohibited source, external service or subagent was used.
No CAPI/plugin/CMake/ABI, runtime configuration, enable/default switch or coverage-map changes were made.

### Changes and independent evidence

- LT5: count starts at zero; QI acquires a reference; announced/cached/scaled colours start black,
  day/night fractions 1/0, sun (1,-1,-1). Added the actual initial shared property values separately
  from shipped exemplar values. Constructor map rotation is zero until a view update; Shutdown retains
  the established matrix. Unset native geometry/map slots still have documented safe empty Rust storage.
- LT6: RGB subtraction, blue-plus-green-then-red squares and square root use register arithmetic with
  an ordered strict threshold test. Tests distinguish equality, NaN, the public diagonal-distance case
  and a nonzero distance whose binary32 square would underflow.
- LT7: minimum-red scans the entire pixel vector, including padding, with the specified binary32
  constant. Missing images continue Init. Invalid foreign decoder data interrupts/latches Init rather
  than claiming native rollback/retry. Shutdown frees maps; destruction alone deliberately does not
  unsubscribe or free map allocations. Test-pattern Init assigns only the two normal maps.
- LT8/LT9: physical sweep stays X rows by Z columns at every rotation, stores the proved drift/drop,
  retains the previous rectangle on skipped sweeps, and uses (-1,-1,0,0) after an unchanged sweep.
  Terrain-normal upper bounds use binary64 0.001 before forwarding the stored coordinate. Spy tests
  distinguish exact copied vertices, rotation-1 world bounds (5,2,7,4), callback order 67 then 68,
  deferred dirty accumulation, repeated skipped-sweep notifications and no-change silence.
- LT10: replaced the gap stub/generic symmetric fit with the complete model-frame direction, basis,
  asymmetric register/memory sums, cofactors, coefficient/lift stores, entire flag-byte gate, inverse
  translation and reciprocal-scale order, and all sixteen column-major outputs. Integer register
  arithmetic models selectable 24/53/64-bit significands without a universal binary64 accumulator.
  Nearest-even and masked exceptions are explicit preconditions; callers supply precision.
- Literal public vectors 18–22 cover untouched output on count zero, vertical fallback, directions
  for flags 0/2 versus 4/6, all output rows for flags 0/1/2/4/6/128, signed determinant -1 from a product
  spill, and identical UV columns giving U=-16777215 versus V=-16777216. Finite LT10 vectors run at
  both 53 and 64 bits. Independent noncommuting shear products, zero scale, NaN determinant admission,
  literal positive-zero output entries and the 64-bit-register cancellation discriminator add coverage.
  Primitive checks compare 4,000 operand pairs against independent IEEE binary64 operations at PC53.
- LT11: replaced the gap stub with inclusive native rounding/intersection and all four ordered probe
  branches. Tests cover public vector 17, integral rounding ties, equality stop, zero components/NaN
  stop and a second edge probe continuing after the first reaches high terrain. The last extension
  starts at its own box face; stopped vertices remain included. No host computes owner results.

### Individual native inventory audit

The public inventory names **43 manager bodies / 21,714 bytes** and **2 image bodies / 517 bytes**.
The rows below account for those entries individually; they are portable behavior routes, not measured
original-method calls or native-address bindings. The two adjustment thunks remain adapter boundaries.
Shared AddRef and the attributed helpers are dependencies, not additional manager bodies. LT12/LT13 and
unexercised native binding prevent blanket complete-owner/numeric-exactness credit.

| Owner entry | Portable route / boundary |
|---|---|
| manager 007D65E0 QueryInterface | `query_interface`, acquired reference, typed interface identity |
| manager 007D6650 Release | `release` and `destroy` |
| manager 007D6670 GetTimeOfDay | `get_time_of_day` |
| manager 007D6680 IsNight | `is_night` |
| manager 007D68B0 BendShadowedNormal | `bend_shadowed_normal` |
| manager 007D6950 ShadowDepth | `shadow_depth` |
| manager 007D6A00 HeightAboveLand | `height_above_land` |
| manager 007D6AB0 SetLightingTint | `set_lighting_tint` |
| manager 007D6B00 ResetLightingTint | `reset_lighting_tint` |
| manager 007D6B20 GetGlobalLight | `get_global_light` |
| manager 007D6B50 GetShadowDirection | `get_shadow_direction` |
| manager 007D6B70 GetShadowParams | `get_shadow_params` |
| manager 007D6BB0 GetShadowStrengthFromLocation | `get_shadow_strength_from_location` |
| manager 007D6CC0 CalcShadowTerrainRect | `calc_shadow_terrain_rect`, LT11 probes |
| manager 007D73E0 GetColor(out) | `get_color_up` |
| manager 007D7420 GetColor(bytes) | `get_color_up_bytes` |
| manager 007D7460 GetColor(normal) | `get_color` |
| manager 007D7730 GetColor(normal,bytes) | `get_color_bytes` |
| manager 007D77C0 GetColor(normal,position) | `get_color_at` |
| manager 007D7840 GetColor(position,normal,bytes) | `get_color_at_bytes` |
| manager 007D7940 GetTerrainColor | `get_terrain_color` |
| manager 007D7AE0 GetTerrainColorFloor | `get_terrain_color_floor` |
| manager 007D7BF0 GetVertexColors | `get_vertex_colors` |
| manager 007D7F00 GetTerrainVertexColors | `get_terrain_vertex_colors` |
| manager 007D8340 GetTerrainNormalClamped | `terrain_normal`, LT8 forwarding boundary |
| manager 007D83C0 GetModelColor | `get_model_color`, surface dimensions/model storage |
| manager 007D8550 GetModelLight | `get_model_light`, raw bounds/interface answers |
| manager 007D8B20 GetTerrainLighting | `get_terrain_lighting` |
| manager 007D8C80 CalcShadowProjection | `calc_shadow_projection`, LT10 arithmetic schedule |
| manager 007D9820 SetTimeDependentLightingValues | `set_time_dependent_lighting_values` |
| manager 007D9DF0 SetTimeOfDay | `set_time_of_day`, LT6 register distance |
| manager 007DA0A0 constructor | `new`, explicit inherited register precision |
| manager 007DA3E0 QI adjustment thunk | typed MessageTarget routes to QI; physical base+4 thunk unbound |
| manager 007DA3F0 Release adjustment thunk | same primary release behavior; physical adjustment unbound |
| manager 007DA400 destructor | `destroy` and Rust `Drop`, no Shutdown/map cleanup |
| manager 007DA490 GetDecalShadowParams | `get_decal_shadow_params`, raw property overrides |
| manager 007DA670 deleting destructor | `deleting_destructor`, flag-bit-0 storage handling |
| manager 007DA690 Shutdown | `shutdown`, explicit subscriptions/resources protocol |
| manager 007DA7E0 ReadProperties | `read_properties`, persistent `LightingStatics` |
| manager 007DAE70 FindHillShadows | `find_hill_shadows`, LT8 physical scan |
| manager 007DB6A0 Init | `init`, continued missing loads and ordered subscriptions |
| manager 007DB840 DoZoomAndRotationChange | `do_zoom_and_rotation_change`, six caches/up-normal quirk |
| manager 007DBCF0 DoMessage | `do_message`, seven typed payloads and LT9 notifications |
| image 007D6690 SampleRowLerp | `ImageArgb8::sample`, linear row spill, checked invalid storage |
| image 007D6870 FindMinRedComponentValue | `find_min_red_component_value`, vector-end boundary |

Attributed helpers are separately routed: rectangle rotation to `changed_world_rect`, image copy to
`ImageArgb8::copy_from`, resource creation to `create_color_map`, test pattern to `make_test_pattern`,
view-vector rotation to `rotate`, perpendicular basis to internal `basis`/`model_basis`. None is credited
as a fabricated additional owner entry. Foreign providers supply only raw pixels, properties, bounds,
interface answers and terrain data/normal queries.

### Validation and limits

Final **unchanged-tree** `qbuild all` on code `e5cc4376`: cargo **4938 passed / 0 failed / 0 ignored**,
clippy **exit 0 / 0 diagnostics**, native build **4/4 edges**, ctest **68/68** (126.4 s;
fc4verify total 186.2 s), qbuild **PASS**. Core contributes **2931/0**. `qbuild fmt` with absolute paths
passes all **10** touched Rust files. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`. The canonicalization warning is not a formatter failure.

Earlier quick PASS: core 2930/0, dependents compiled, native 203/203 edges (146.3 s). The first quick
found the LT11 integer-tie bug and two obsolete expectations; these were corrected. A preceding full
run passed 4937 tests/clippy/68 native tests but was superseded when the constructor-map audit found
the final correction; only the unchanged-tree result above is the final gate. Baseline omitted per brief.

**Questions:** LT12 and LT13 appended only to `docs/specs/questions/lighting.md`, as authorized.
LT12 asks which inherited precision/rounding modes reach lighting; the core makes the supported
nearest-even precision explicit rather than selecting a game default. LT13 asks for NaN payload/sign
and signaling behavior; canonical quiet NaNs are a documented portable policy, not bit-exact native
output evidence. A spec lane must regenerate the shared question index. LT1–LT4 remain semantic questions
outside this completion and are still represented by raw setting/interface/property inputs.

**Game/shadow:** n/a. No hook, ABI export, runtime replacement or game request was added. No ref-warning,
same-process shadow, exercised replacement, default-enable or whole-owner native coverage claim is made.
Native physical interface thunks and numeric environment evidence require a separate integration task;
this bounded completion does not request another implementation round or restart.
