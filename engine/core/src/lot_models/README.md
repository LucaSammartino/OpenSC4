<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Lot model owners

Authority: `docs/specs/lot-models.md` sections 0–8.2. The pure Rust owners
implement the settled contracts, including published basic lifetime.
Unsettled inventory remains explicitly open below. No native execution,
ABI integration, native pool/layout or final GPU equivalence is claimed.

| Owner | Implemented bounded body | Foreign boundary |
|---|---|---|
| Foundation | Ordered five position/BGRA/UV quads, footprint-width side U, stored reciprocal top UVs; border enumeration, ground-before-water, truncating existing/required counts; geometry rebuild without relight; zero/mutable fallback keys and independent top draw | Individual terrain/normal-light outputs and acquired resource leases |
| Wall | Four biased cell ranges, inner/across mapping, raw query order and flat suppression; three emit branches, world UVs, seam positions/raw bounds; retained tails/full-buffer BGRA recolour; resource/sort/facing/draw decisions | Individual lot/terrain/light outputs and acquired leases |
| Cell | Native strip and orientation/postmirror tables; unsigned-byte placement, slot order/bounds/BGRA; first/later zoom and rotation updates, UV-only SetTexture; wrapped-key lookup; packed states, modes 0–3 combiner bytes/decoder and ordered draw requests | Context applies packed state/driver calls; terrain view draws nonflat cells |

## Domain and exclusions

Stored values are binary32; explicit binary64 register operations select
round-to-nearest/ties-to-even PC53. There is no ambient-control-word or
general PC64 equivalence claim. The wall lattice helper uses bias/store/bits,
not floor/ceil, for nonnegative coordinates through 2^24. Wall ledgers are
published PC53/64 analytical examples, not native observations. Cell world
coordinates are unsigned bytes. InvalidInput denotes unsupported nonfinite,
overflowing, invalid-mode, unbacked-element, foreign-error or allocation
domains, not guessed native false/clamp/rollback behavior. Foundation finite
scales include zero and negative values: reciprocal overflow, infinity and NaN
UVs are retained under section 2.3's unchecked division rule.

LM1 upstream authors/other mode meanings, LM2 remaining object/base inventory,
constructor fields and pool allocation, and LM8 exceptional arithmetic remain
open. `BasicModel` implements unsigned 16-bit references, QI, custom data and
owner-reference chronology. `LotModelLifetime` clears resources on terminal
Release without freeing the Rust allocation; further native calls are outside
contract. Each successful factory call supplies an acquired Rust lease;
`Texture::same_binding` identifies equal binding pointers so cells release the
extra acquisition. Owner callbacks use a stable Arc identity and separate
explicit native AddRef/Release accounting. Borrowing releases nothing.
Explicit constructor inputs cover unpublished fields; `FoundationModel::empty`
supplies the proven sentinel and default top scale. No ABI changed.

## Entry points and compatibility

Foundation `adjust_from_terrain` owns enumeration and `draw_lit`/`recolour` write BGRA
from integer light. Legacy `setup_vertices`/`draw` retain semantic floating
colour metadata; that compatibility metadata is not native packed-colour proof.

Wall `rebuild`/`setup_from_bounds` own traversal. `triangles()` returns the
full stored list including tails; `draw_lit` recolours it. Legacy `draw`
reports pending recolour when no raw light was supplied. Vec capacity is a
portable allocation choice, not a native heap guarantee.

Cell `from_record`, `update_vertices` and `draw_owned` are the settled path.
`Orientation::from_byte` accepts all record bytes: values above 3 retain UVs
then optionally mirror them on every SetTexture. `Orientation::new` remains
checked for 0–3. Unknown initial UV bytes are LM2; the standalone helper starts
with explicit zero storage. Retention is tested after a known orientation.
`CellRenderInitialization` models shared Init outputs and the absent-service
early return; `draw_owned` uses the specified completed-Init domain.
`CellCommand::State` selects the owner tuple; the foreign context owns
changed-bit device calls, default wrap/filter/transform-clear requests and
render overrides in section 4.3. Command errors retain the issued prefix.
Legacy `new(record,layout)` ignores the historical layout oracle; the
standalone layout helper remains for compatibility.
Legacy semantic `place`/`draw` are not the ordered callback path.

## Independent fixtures

The six original lm01–lm06 conformance files remain; lm01 now uses settled
truncation and lm06 uses the settled native strip table. DIFFERS comments
preserve the history of obsolete ceil/fallback/caller-cycle
expectations. Core fixtures include four sides times eleven height ledgers,
raw range/query/bounds/tails, nonsquare foundation records and border order,
zero/equal-fallback/wrapped-key lease chronology, all cell orientations,
first/later zoom and SetTexture, and section 8.2 state/combiner/unsigned-alpha/
pass/force/partial-prefix counterexamples. Expectations are public literals
and bit rules, never owner-generated goldens.

## Owner closure

Tests are in `tests.rs` and `lifetime_tests.rs`. Sections 5/7 describe foreign
users/open contracts, rather than additional owner algorithms. Section 8.1
applies only to walls, 8.2 only to cells, and sections 2/3/4 to their named owner.

| Owner | Spec section | Code item | Test or open question |
|---|---|---|---|
| Foundation | 0 | `FoundationModel`, `FoundationShape` | `foundation_quad_uv_order_and_raw_normal_colours` |
| Foundation | 1 | `sort_value`, `load_textures`, `DrawState::OPAQUE` | `foundation_draw_order_fallback_and_independent_zoom_rotation_caches` |
| Foundation | 1.1 | `BasicModel`, `LotModelLifetime`, `Drop` | `basic_defaults_owner_identity_and_release_before_acquire`; `terminal_release_and_drop_order_for_every_owner` |
| Foundation | 1.2 | Published setup/bounds/sort/draw/lifetime entries | open: LM2, remaining inventory/inherited slots, constructor fields, allocation failure and pool |
| Foundation | 2 | `FoundationShape::new`, `draw_lit` | `foundation_raw_draw_recolours_only_on_rotation_and_skips_highlight` |
| Foundation | 2.1 | `load_textures`, `sort_value` | `settled_owner_zero_same_fallback_and_key_mutation`; `foundation_unsupported_inputs_and_settled_independent_top_draw` |
| Foundation | 2.2 | `adjust_from_terrain`, `adjust_height_to_terrain`, `empty` | `foundation_border_order_exact_multiple_and_force_without_relight`; `foundation_unchecked_scale_and_signed_existing_count`; `terminal_release_and_drop_order_for_every_owner` |
| Foundation | 2.3 | `FoundationShape::new`, `recolour`, `set_top_scale` | `foundation_quad_uv_order_and_raw_normal_colours`; `foundation_unchecked_scale_and_signed_existing_count` |
| Foundation | 5 | `FoundationTerrain`, `FoundationLighting`, `TextureFactory` | `foundation_border_order_exact_multiple_and_force_without_relight` |
| Foundation | 6 | `adjust_height_to_terrain` | `foundation_exact_step_and_adjacent_f32_boundary` |
| Foundation | 7 | Unsupported unpublished/exceptional contracts | open: LM2, LM8 |
| Foundation | 8 | Steps, water, equality, fraction, force, UV, zero-key vectors | `foundation_public_step_vector_and_water_precedence`; `foundation_literal_existing_count_zero_allows_required_count_one`; `foundation_fractional_existing_depth_extends_only_downward`; `foundation_never_raises_base_and_force_rebuild_does_not_relight`; `foundation_quad_uv_order_and_raw_normal_colours`; `settled_owner_zero_same_fallback_and_key_mutation` |
| Wall | 0 | `RetainingWallModel`, `WallShape` | `wall_range_raw_order_bounds_and_tail_literals` |
| Wall | 1 | `sort_value`, `load_texture`, `DrawState::OPAQUE` | `wall_fallback_zoom_four_highlight_and_cull_order` |
| Wall | 1.1 | `BasicModel`, `LotModelLifetime`, `Drop` | `owner_qi_preserves_failure_output_and_word_refcount_wraps`; `terminal_release_and_drop_order_for_every_owner` |
| Wall | 1.2 | Published setup/bounds/sort/init/draw/lifetime entries | open: LM2, remaining inventory/inherited slots, constructor fields, allocation failure and pool |
| Wall | 3 | `init`, `setup_from_bounds`, `draw_lit`, `draw` | `sampled_wall_model_preserves_stored_tail_and_colours`; `wall_public_facing_vector_and_all_rotations` |
| Wall | 3.1 | `load_texture`, `load_owner_key` | `settled_owner_zero_same_fallback_and_key_mutation` |
| Wall | 3.2 | `WallRange::new`, `WallShape::{rebuild,emit,finish,recolour}` | `wall_range_raw_order_bounds_and_tail_literals`; `wall_empty_narrow_ties_flat_neighbour_and_missing_inner`; `settled_wall_literal_four_side_ledger` |
| Wall | 5 | `WallTerrain`, `WallLighting`, explicit caller box | `wall_range_raw_order_bounds_and_tail_literals` |
| Wall | 6 | `WallShape::emit` | `settled_wall_literal_four_side_ledger` |
| Wall | 7 | Unsupported unpublished/exceptional contracts | open: LM2, LM8 |
| Wall | 8 | Fallback, repeat failure, facing vectors | `settled_owner_zero_same_fallback_and_key_mutation`; `wall_public_facing_vector_and_all_rotations` |
| Wall | 8.1 | Four sides × eleven literal height ledgers | `settled_wall_literal_four_side_ledger` (positions, UV, BGRA, bounds and query order) |
| Cell | 0 | `CellTextureModel` | `cell_literal_orientation_postmirror_and_unsigned_placement` |
| Cell | 1 | `sort_value`, `load_texture`, `CellPackedState` | `cell_lookup_falls_back_to_unzoomed_key_and_keeps_texture_alpha`; `init_absent_service_preserves_state_and_success_packs_literals` |
| Cell | 1.1 | `BasicModel`, `LotModelLifetime`, `Drop` | `owner_qi_preserves_failure_output_and_word_refcount_wraps`; `terminal_release_and_drop_order_for_every_owner` |
| Cell | 1.2 | Published setup/bounds/sort/init/draw/lifetime entries | open: LM2, full inherited table, remaining inventory, constructor fields, allocation failure and pool |
| Cell | 4 | `draw_owned`, `CellRenderInitialization`, `cell_mode` | `cell_all_modes_literal_state_combiner_and_light_order`; `init_absent_service_preserves_state_and_success_packs_literals` |
| Cell | 4.1 | `update_native_corners`, `place_geometry`, `update_vertices`, `set_texture` | `cell_literal_orientation_postmirror_and_unsigned_placement`; `cell_first_zoom_later_recolour_settexture_and_pass_prefix`; `cell_invalid_orientation_retains_uvs_then_mirrors_each_update` |
| Cell | 4.2 | `load_texture`, `Texture::same_binding`, cached metadata | `cell_equal_binding_releases_extra_acquisition_and_retains_old_lease`; `cell_zoom_element_selection_and_fallback_zero_are_literal`; `cell_reload_before_null_and_pass_failures_and_foreign_partial_prefix` |
| Cell | 4.3 | `draw_owned`, packed states, combiner and `CellCommand` | `cell_all_modes_literal_state_combiner_and_light_order`; `cell_nonflat_fallback_keeps_original_alpha_and_force_flag`; open: LM1, other mode meanings |
| Cell | 5 | `CellPlacementSource`, `CellCommands`, explicit record | `cell_flat_placement_queries_lot_then_lighting_without_terrain` |
| Cell | 6 | `CellCommand::Terrain` | `cell_nonflat_fallback_keeps_original_alpha_and_force_flag` |
| Cell | 7 | Unsupported upstream/other modes, unpublished/exceptional contracts | open: LM1, LM2, LM8 |
| Cell | 8 | Mode, orientation, corners, bounds and wrapped-key vectors | `cell_mode_public_alpha_vector_and_precedence_boundaries`; `cell_literal_orientation_postmirror_and_unsigned_placement`; `settled_cell_wrapped_key_differs_from_old_overflow_error` |
| Cell | 8.2 | Literal owner requests and typed context/driver boundary | `cell_all_modes_literal_state_combiner_and_light_order`; `cell_first_zoom_later_recolour_settexture_and_pass_prefix`; `cell_reload_before_null_and_pass_failures_and_foreign_partial_prefix`; downstream driver caching is foreign as specified |
