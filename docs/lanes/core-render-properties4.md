| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-render-properties4: render properties core | `core-render-properties4` / engine lane, detached reserved worktree | Ready scoped core; f12f6207 implementation; 77 entries, slots 3–17; whole owner incomplete (COM slots 0–2 pending RP3). Cargo 4183/4183, clippy 0 diagnostics, ctest 67/67. No game/shadow/replacement claim. Filed RP3–RP5. ABI unchanged. | 1. Resolve RP3–RP5. 2. Implement COM lifecycle once specified; plan separate glue/game verification. |

## Changes

Commit `f12f6207` adds `engine/core/src/render_properties/` and one additive module registration at the end of `engine/core/src/lib.rs`. No plugin, CAPI, build-system, generated ABI entry, or ABI export changes.

`RenderProperties` owns independent boolean, signed-i32 and f32 tables containing the exact 38/24/15 registrations of render-properties.md §3. Construction copies immutable registration data, drops the leading `k`, derives capital-letter acronyms, and lowercases full names (§1.3). Every entry retains its index, names, current value and two extra values; unlisted extra values are zero. The boolean extras represent the two zero bytes without assigning them a purpose. Rust ownership drops all vectors and their strings, and cloning duplicates all mutable state and strings. Records are portable core data, without claiming the game's binary layout.

Implemented property slots, individually:

- 3, 4, 5: boolean, integer and float counts.
- 6, 7, 8: boolean, integer and float reads at valid indices.
- 9, 10, 11: boolean, integer and float writes, ignoring extra/range values.
- 12, 13, 14: typed name/acronym find, lowercase query, first matching entry wins, missing returns -1.
- 15, 16, 17: stored lowercase full names at valid indices.

All four acronym collisions preserve the lower-index winner, while the later entry remains accessible by full name. No texture/GPU/terrain/clock/resource provider or original-owner output is used. Immutable getters and exclusive mutable setters are Send/Sync compatible; a future glue owner must supply synchronization when exposing concurrent reads and writes.

## Completeness and questions

**Whole-owner completeness is not claimed.** Exactly `QueryInterface`, `AddRef`, and `Release` (slots 0–2), including their interaction with the deleting destructor/two interface views, remain unimplemented pending RP3. Constructor/table-copy, acronym helper, registry contents, owned-table cleanup, and the 15 property slots above are implemented. There is no family-wide implementation, verification, shadow, or replacement credit.

Appended to `docs/specs/questions/render-properties.md`, with matching source comments:

- **RP3:** initial reference count, accepted IIDs, failed-query output, Release-at-zero and deleting-destructor behavior. No synthetic COM result or reference count is supplied.
- **RP4:** original unchecked indexing has no defined out-of-bounds result in the spec. The safe Rust boundary returns None for invalid read/name indices and false for invalid writes, leaving state unchanged. This is not claimed as original failure behavior.
- **RP5:** query encoding/locale and C-string NUL boundaries are unspecified. The Rust API folds ASCII and compares the whole string. Non-ASCII folding/C-string truncation is not claimed.

Existing RP1/RP2 remain open. Extra values are preserved as uninterpreted metadata; no additional consumers or boolean-byte semantics are invented. The generated questions index is unchanged under this lane's edit restrictions.

## Validation

- **Initial `qbuild all`: invalid pre-edit baseline.** Workspace tests and clippy passed, but I edited while its CTest phase was running. `cargo_test` encountered the new module before tests.rs existed; native CTest finished 66/67. This transient failure is recorded, not represented as a clean baseline or an existing repository defect.
- **`qbuild quick`: PASS.** Core 2326 passed, 0 failed; CAPI/conformance/formats dependents compiled with --no-run; native build 4/4 edges. The wrapper noted a tree change during this iteration (test formatting/implementation commit); the final full gate ran after these changes.
- **Final `qbuild all`: PASS.** Workspace 4183 passed, 0 failed/ignored; clippy exit 0, 0 diagnostics; native build 2/2 edges; CTest 67/67, 0 failed (109.1 s). Final code is the implementation commit `f12f6207`; subsequent changes are this report only.
- **Formatting:** `qbuild fmt` passes for all three new .rs files. Checking the touched shared lib.rs fails on pre-existing module order/formatting (first difference commands/cmdline at line 31; 122 diff lines), including the end-appended registration. Its only committed change is the required one-line additive registration; no mass reformat was performed.
- **Conformance:** 10 new core tests execute a 77-row spec-derived registry fixture and the six §7 cases. Coverage includes every name/default/acronym/extra value; all four collisions; exact/typed misses; all 77 writes isolated from other entries; ignored ranges; preserved float bits (signed zero, infinities, NaN payload); independent objects/clones and drop; invalid signed indices without mutation; ASCII/string policy; second-thread immutable getters. Fixtures and tests stay under the authorized module directory, without conformance-crate edits.

Full final logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`. Quick logs: `build-runs/fc4verify-quick-test.log` and `build-runs/fc4verify-quick-deps.log`.

## Game and delivery

No game request or outstanding label. This pure core addition has no CAPI/plugin entry path, so in-game shadow exactness and reference-warning counts are n/a. No runtime behavior is promoted and no game files were committed. The branch remains detached as reserved by the supervisor; commits are ready for publication to `core-render-properties4`, with ABI false and an empty game_runs list.

Next work requires spec answers for the exact remaining COM methods and boundary contracts, followed by a separately scoped glue/verification lane. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-render-properties4-r1-manifest.json`.
