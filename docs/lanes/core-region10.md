| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-region10: cSC4Region core | Codex / core-region10 (reserved branch; detached HEAD) | ready; core owner implemented, RM16 out-of-range height open | Review closure; publish reserved branch; integrate host and read-only shadow |

## Implementation

Module: `engine/core/src/region/mod.rs`; codecs: `region/codec.rs`; tests:
`region/tests.rs`; section-by-section **Owner closure** table: `region/README.md`.

Implemented cSC4Region from region-model.md §§1–8 and region.md: identity and counted
lifetime, every state field and interface operation, config reads/writes and localization,
layout decoding/scanning/fallback, cell boxes and reference retention, city version gates,
exact placement and overlap/overflow order, idempotent lifecycle, recursive deletion,
creation/templates/starter naming, file naming, city reload/delete/move, tutorial reset,
terrain tuning and water queries, and the serial-number helper. The four INI writes remain
independent failure points. City moves preserve unrelated package data through the host.

`RegionHost` isolates filesystem, nation, image/resource, regional-city codec, time/RNG
and tutorial services. Regional-city serialization belongs to that external owner; this
module supplies its application-version gate and calls rather than inventing another
regional-city record format. The 12-byte location record, INI and bitmap codecs have
same-input round trips. Getters use no RefCell. Tests check Send + Sync and holder identity.
Overwritten and zero-area boxes retain their own counted references even after destruction,
as specified. Safe stale BoxIds cannot dereference freed memory (RM17).

Conformance coverage includes all region-owner vectors in region.md §11 and
region-model.md §12: tile colors/fit/visited scan, six synthetic exact-cover layouts with
the specified city-count distributions, duplicate Timbuktu cell/overflow, degenerate
bounds, config ranges/localization, folder creation, Init result and serial-sensitive
file naming. Synthetic layouts do **not** establish shipped bitmap identity, filesystem
enumeration order, or runtime first-load behavior. Terrain-map/view owners and §12.8
are outside cSC4Region; no coverage is claimed for them.

No plugin/capi wiring, ABI exports, ABI version, generated ABI entry file, or coverage map changed.

## Validation

- Focused implementation run: 21/21 region tests passed before the final additions.
- Final module: 25 focused tests, included in the passing workspace run.
- Explicit owned-file formatting: PASS, 3 files, zero diff lines.
- Final stable-source `qbuild all`: PASS on beeb2efbf, **7036 passed, 0 failed, 0 ignored**;
  clippy **0 diagnostics**; native build **210/210 edges**; CTest **80/80**.
  Request `qbuild-20261024-103734-build-ctest`; session 31717 collected to completion.
  Evidence: `build-runs/final-all-{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log`.
- Explicit `qbuild verify`: PASS on 5a43e768e (same engine source); native build
  **46/46 edges**, CTest **80/80**, 97.0 seconds total.
  Request `qbuild-20261024-105100-build-ctest`; session 29765 collected to completion.
  Evidence: `build-runs/fc4verify-{build,ctest}.log` and its unique build-results verdict.
- `git diff --check`: PASS. All tests honor the workspace's Rust 1.75 language baseline.

The first full gate failed clippy's derivable_impls check after **7035 tests passed**;
its native phase passed **255/255 edges and 80/80 CTests** but reported the tree changing.
It is retained as failed evidence, not final validation, under
`build-runs/pre-correction-{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log`
and verdict `qbuild-20261024-102643-build-ctest`. Corrections 09747d671 and 99a96cd07
derive Default, preserve orphaned references and check malformed BMP header arithmetic
on 32-bit targets. The final gate ran only after these diagnostics changed the source.

## Questions and remaining open contract

Appended **RM15–RM18** to `docs/specs/questions/region-model.md` and **RQ3** to
`docs/specs/questions/region.md`. The coordinator must regenerate the question index;
this engine lane did not run tools/ scripts or edit the generated index.

- RM15: hexadecimal prefix versus whole-string parsing; implemented literal prefix parsing.
- RM16: out-of-range base-height lookup. **region-model.md §8.1 / region.md §7 remain
  open only for terrain types outside 0..1**: the spec describes arbitrary process-memory
  reads, so `Unsupported::BaseTerrainType` is returned for this unsettled input. The setter
  still stores every integer; both valid entries and defaults are implemented.
- RM17: dangling reload/remove handles and tutorial reload failure; stale IDs remain stale,
  resolve to None and cannot cause a freed-pointer dereference. Tutorial activation still
  follows successful export. All defined operations are implemented.
- RM18: collision naming after truncation removes the last dot; append the suffix if no dot remains.
- RQ3: older/newer naming and return-value conflicts. Implemented the detailed model's
  suffix starting at 2, MoveCity false on post-rename read failure, ignored write result,
  and tutorial true on export failure.

Other existing inferred contracts remain tagged in the specs. They have literal implementations;
no other owner method is left Unsupported.

## Game and next steps

Game runs: **none**, shadow: **n/a**. No request is outstanding. An in-game read-only shadow
is feasible next once a host adapter and regional-city references are available; compare
config, row-major locations, bounds, occupied cells and overflow decisions before enabling
filesystem mutation. This task expressly excluded plugin/capi integration.

Coordinator: review the owner-closure table, publish this detached tip to the reserved
`core-region10` branch, regenerate the question index and claim coverage after review.
Then integrate the regional-city owner/host adapter and request a separate shadow run.
