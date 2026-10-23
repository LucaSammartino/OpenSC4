// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Spec-derived conformance scenarios plus ownership/failure cases. Seeding
//! private records tests the known state machine without laundering unknown
//! placement/model math through a host. These are not add/placement coverage.

use super::*;
use std::sync::Mutex;

struct Texture;
impl TextureBinding for Texture {
    fn size(&self) -> [u32; 2] {
        [64, 32]
    }
}
impl TextureSource for Texture {
    fn size(&self) -> [u32; 2] {
        [64, 32]
    }
}

struct Factory {
    calls: Mutex<Vec<(u32, BindingLookup)>>,
    texture: Option<Arc<Texture>>,
}
impl TextureFactory for Factory {
    fn binding(&self, key: u32, lookup: BindingLookup) -> Option<Arc<dyn TextureBinding>> {
        self.calls.lock().unwrap().push((key, lookup));
        self.texture.as_ref().map(|b| b.clone() as Arc<dyn TextureBinding>)
    }
}

struct Terrain {
    cells: [u32; 2],
    width: f32,
}
impl TerrainProvider for Terrain {
    fn cells(&self) -> [u32; 2] {
        self.cells
    }
    fn cell_width(&self) -> f32 {
        self.width
    }
}

struct View {
    terrain: Option<Arc<Terrain>>,
    factory: Option<Arc<Factory>>,
    quality: i32,
    debug: i32,
    messages: Mutex<Vec<(bool, u32)>>,
}
impl ViewProvider for View {
    fn terrain(&self) -> Option<Arc<dyn TerrainProvider>> {
        self.terrain.as_ref().map(|t| t.clone() as Arc<dyn TerrainProvider>)
    }
    fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>> {
        self.factory.as_ref().map(|f| f.clone() as Arc<dyn TextureFactory>)
    }
    fn shadow_quality(&self) -> i32 {
        self.quality
    }
    fn shadow_debug(&self) -> i32 {
        self.debug
    }
    fn subscribe(&self, message: u32) {
        self.messages.lock().unwrap().push((true, message));
    }
    fn unsubscribe(&self, message: u32) {
        self.messages.lock().unwrap().push((false, message));
    }
}

fn view(quality: i32) -> Arc<View> {
    Arc::new(View {
        terrain: Some(Arc::new(Terrain {
            cells: [8, 4],
            width: 16.0,
        })),
        factory: Some(Arc::new(Factory {
            calls: Mutex::new(Vec::new()),
            texture: Some(Arc::new(Texture)),
        })),
        quality,
        debug: 4,
        messages: Mutex::new(Vec::new()),
    })
}

fn manager(dynamic: bool) -> OverlayManager {
    let mut manager = OverlayManager::new();
    manager.init(view(3), dynamic, false).unwrap();
    manager
}

fn rect(min: [u32; 2], max: [u32; 2]) -> CellRect {
    CellRect { min, max }
}

#[test]
fn public_ov2_rectangle_copies_every_word_without_clipping_or_handle() {
    let mut m = manager(false);
    let matrix = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0,
        8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let bounds = rect([7, 3], [10, 5]);
    assert_eq!(m.add_overlay_for_rect(None, bounds, matrix, 8), Ok(Handle::NONE));
    let h = m.add_overlay_for_rect(Some(Arc::new(Texture)), bounds, matrix, 8).unwrap();
    assert_eq!(h, Handle(0));
    let o = m.snapshot(h).unwrap().unwrap();
    assert_eq!(o.flags, 9);
    assert_eq!(o.cells, bounds);
    assert_eq!(o.primary, matrix);
    assert_eq!(m.handles.iter().count(), 0);
    assert_eq!(m.take_changed_rectangles(), vec![bounds]);
    let mut output = Placement([42.0; 9]);
    m.get_placement(h, &mut output).unwrap();
    assert_eq!(output, Placement([42.0; 9]));
    m.set_placement(h, Placement([99.0; 9])).unwrap();
    assert_eq!(m.snapshot(h).unwrap().unwrap(), o);
}

#[test]
fn public_ov2_city_matrix_uses_only_signed_nx() {
    for nz in [32, 48] {
        let mut v = view(2);
        Arc::get_mut(&mut v).unwrap().terrain = Some(Arc::new(Terrain { cells: [64, nz], width: 7.0 }));
        let factory = v.factory.as_ref().unwrap().clone();
        let mut m = OverlayManager::new();
        m.init(v, true, false).unwrap();
        let h = m.add_city_decal(77).unwrap();
        let o = m.snapshot(h).unwrap().unwrap();
        assert_eq!(o.flags, 0x13);
        assert_eq!(o.cells, rect([0, 0], [64, nz]));
        assert_eq!(o.primary, [0.0, 0.0, 0.0, 0.0, 0.0, 1.0/32.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(factory.calls.lock().unwrap().last(), Some(&(77, BindingLookup::Slot5)));
        assert!(m.take_changed_rectangles().is_empty());
        assert_eq!(m.handles.iter().count(), 0);
    }
}

#[test]
fn public_ov3_settings_message_is_quiet_and_copies_raw_values() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(flag::SHOWN));
    let before = m.snapshot(h).unwrap();
    let index = m.cell_index.clone();
    assert!(m.handle_settings_message(7, [0, 7], 0));
    assert_eq!(m.shadow_settings(), [3, 4]);
    assert_eq!(m.selection_byte, 1);
    assert!(m.handle_settings_message(CHANGE_MESSAGE, [0, 7], 0));
    assert_eq!(m.shadow_settings(), [0, 7]);
    assert_eq!(m.selection_byte, 0);
    assert_eq!(m.snapshot(h).unwrap(), before);
    assert_eq!(m.cell_index, index);
    assert!(m.take_changed_rectangles().is_empty());
}

/// Explicit initial state, not claimed constructor defaults.
fn overlay(flags: u32) -> Overlay {
    Overlay {
        primary: [0.0; 16],
        flags,
        cells: rect([1, 1], [3, 3]),
        alpha: 0.75,
        color: [0.25, 0.5, 1.0],
        draw_state: 2,
        texture_scale: None,
        binding: None,
        source: None,
    }
}

fn seed(manager: &mut OverlayManager, overlay: Overlay) -> Handle {
    let rectangle = overlay.cells;
    let index = manager.overlays.insert(overlay);
    if let Some(cells) = &mut manager.cell_index {
        for cell in rectangle.cells() {
            cells.entry(cell).or_default().push(index);
        }
    }
    Handle(index as i32)
}

fn indirect(manager: &mut OverlayManager, target: Target) -> Handle {
    let index = manager.handles.insert(HandleRecord {
        placement: Placement([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]),
        target,
    });
    Handle((index | 0x8000_0000) as i32)
}

fn group(manager: &mut OverlayManager, flags: u32) -> (Handle, [Handle; 8]) {
    let parts = std::array::from_fn(|_| seed(manager, overlay(flags)));
    let index = manager.groups.insert(parts.map(|h| Some(h.index())));
    let handle = indirect(manager, Target::Group(index));
    (handle, parts)
}

const FRAME: FrameOptions = FrameOptions {
    snapshot: false,
    decals: true,
    shadows: true,
    use_second_stage: true,
};

#[test]
fn conformance_null_handle_is_ignored_by_every_handle_method() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(0));
    let before = m.snapshot(h).unwrap();
    m.set_position(Handle::NONE, [4.0, 9.0]).unwrap();
    m.set_placement(Handle::NONE, Placement([10.0; 9])).unwrap();
    let mut output = Placement([42.0; 9]);
    m.get_placement(Handle::NONE, &mut output).unwrap();
    assert_eq!(output, Placement([42.0; 9]));
    m.remove(Handle::NONE).unwrap();
    m.set_shown(Handle::NONE, true).unwrap();
    m.set_alpha(Handle::NONE, 0.5).unwrap();
    m.set_color(Handle::NONE, [1.0; 3]).unwrap();
    m.set_draw_state(Handle::NONE, 4).unwrap();
    m.enable_self_lighting(Handle::NONE, 1).unwrap();
    m.set_texture_scale(Handle::NONE, 16.0, 0.0).unwrap();
    assert_eq!(m.get_field(Handle::NONE), Ok(None));
    assert_eq!(m.alpha(Handle::NONE), Ok(0.0));
    assert_eq!(m.color(Handle::NONE), Ok(None));
    assert_eq!(m.draw_state(Handle::NONE), Ok(None));
    assert_eq!(m.shown(Handle::NONE), Ok(None));
    assert_eq!(m.snapshot(h).unwrap(), before);
    assert!(m.take_changed_rectangles().is_empty());
}

#[test]
fn conformance_group_set_shown_hides_all_eight_and_invalidates() {
    let mut m = manager(false);
    let (g, parts) = group(&mut m, flag::SHOWN | flag::PART);
    m.set_shown(g, false).unwrap();
    for h in parts {
        let state = m.snapshot(h).unwrap().unwrap();
        assert_eq!(state.flags, flag::PART);
        assert_eq!(m.shown(h), Ok(Some(false)));
    }
    assert_eq!(m.take_changed_rectangles(), vec![rect([1, 1], [3, 3]); 8]);
    m.set_shown(g, false).unwrap();
    assert!(m.take_changed_rectangles().is_empty());
}

#[test]
fn conformance_shadow_quality_zero_returns_none_before_factory_lookup() {
    let v = view(0);
    let factory = v.factory.as_ref().unwrap().clone();
    let mut m = OverlayManager::new();
    m.init(v, false, false).unwrap();
    assert_eq!(m.check_shadow_add(true), Ok(Handle::NONE));
    assert_eq!(
        m.check_shadow_add(false),
        Err(Error::Gap(ContractGap::ShadowAndModelConstructionOv2))
    );
    assert!(factory.calls.lock().unwrap().is_empty());
    assert!(m.overlays.iter().next().is_none());
}

#[test]
fn conformance_texture_scale_sets_flag_and_semantic_coefficients() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(flag::SHOWN));
    // Â§9 vector plus the offset/second-axis flip from Â§3 slot21.
    for (size, offset, expected_scale, expected_offset) in [
        (16.0, 0.0, [0.0625, -0.0625], 1.0),
        (8.0, 4.0, [0.125, -0.125], 1.5),
        (-4.0, 2.0, [-0.25, 0.25], 0.5),
    ] {
        m.set_texture_scale(h, size, offset).unwrap();
        let state = m.snapshot(h).unwrap().unwrap();
        assert_eq!(state.flags, flag::SHOWN | flag::TEXTURE_MATRIX);
        assert_eq!(
            state.texture_scale,
            Some(TextureScale {
                scale: expected_scale,
                second_axis_offset: expected_offset
            })
        );
    }
    assert_eq!(m.take_changed_rectangles().len(), 3);
}

#[test]
fn conformance_dynamic_snapshot_draws_nothing_without_changing_stats() {
    let mut m = manager(true);
    seed(&mut m, overlay(flag::SHOWN | flag::PART | flag::SNAPSHOT));
    let frame = FrameOptions {
        snapshot: true,
        ..FRAME
    };
    assert_eq!(m.select_frame(frame, &[]).unwrap().decals, Vec::<u32>::new());
    assert!(m.select_frame(frame, &[]).unwrap().skipped_snapshot);
    m.draw(frame, &[]).unwrap();
    assert_eq!(m.stats_string(), "0 decals, 0 shadows");
}

#[test]
fn conformance_stats_format_only_not_gpu_draw_count_coverage() {
    let mut m = manager(false);
    // Â§9.6's output format is defined; no assertion that the missing renderer
    // produced these counters. OV8 asks when/how they are incremented/reset.
    m.decals_drawn = 3;
    m.shadows_drawn = 2;
    assert_eq!(m.stats_string(), "3 decals, 2 shadows");
}

#[test]
fn conformance_slot20_corrected_or_only_low_bit_no_redraw() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(0));
    // Spec's 0,1,0 => 0,0x40,0x40, extended to distinguish low bit from nonzero.
    for (on, expected) in [(0, 0), (2, 0), (1, 0x40), (0, 0x40), (2, 0x40), (3, 0x40)] {
        m.enable_self_lighting(h, on).unwrap();
        assert_eq!(m.snapshot(h).unwrap().unwrap().flags, expected);
        assert!(m.take_changed_rectangles().is_empty());
    }
}

#[test]
fn conformance_slot20_indirect_group_visits_every_part_and_preserves_other_flags() {
    let mut m = manager(false);
    let base = flag::PART | flag::SHOWN | flag::SNAPSHOT;
    let (g, parts) = group(&mut m, base);
    for (on, expected) in [(0, base), (1, base | 0x40), (0, base | 0x40)] {
        m.enable_self_lighting(g, on).unwrap();
        for h in parts {
            assert_eq!(m.snapshot(h).unwrap().unwrap().flags, expected);
        }
        assert!(m.take_changed_rectangles().is_empty());
    }
    m.enable_self_lighting(Handle::NONE, 1).unwrap();
    for h in parts {
        assert_eq!(m.snapshot(h).unwrap().unwrap().flags, base | 0x40);
    }
}

#[test]
fn slot20_indirect_single_resolves_handle_record_without_touching_other_overlay() {
    let mut m = manager(false);
    let other = seed(&mut m, overlay(0));
    let h = seed(&mut m, overlay(0));
    let alias = indirect(&mut m, Target::Overlay(h.index()));
    m.enable_self_lighting(alias, 1).unwrap();
    assert_eq!(m.snapshot(h).unwrap().unwrap().flags, 0x40);
    assert_eq!(m.snapshot(other).unwrap().unwrap().flags, 0);
    assert!(m.take_changed_rectangles().is_empty());
}

#[test]
fn group_color_draw_state_alpha_endpoints_and_scale_fan_out() {
    let mut m = manager(false);
    let (g, parts) = group(&mut m, 0);
    m.set_color(g, [0.0, 0.25, 0.5]).unwrap();
    m.set_draw_state(g, 4).unwrap();
    m.set_alpha(g, 0.0).unwrap();
    m.set_alpha(g, 1.0).unwrap();
    m.set_texture_scale(g, 16.0, 0.0).unwrap();
    for h in parts {
        assert_eq!(m.color(h), Ok(Some([0.0, 0.25, 0.5])));
        assert_eq!(m.draw_state(h), Ok(Some(4)));
        assert_eq!(m.alpha(h), Ok(1.0));
        assert_eq!(
            m.snapshot(h).unwrap().unwrap().texture_scale.unwrap().scale,
            [0.0625, -0.0625]
        );
    }
    assert_eq!(m.take_changed_rectangles().len(), 40);
}

#[test]
fn static_substep_gap_is_atomic_but_dynamic_alpha_does_not_need_step() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(0));
    let before = m.snapshot(h).unwrap();
    assert_eq!(m.set_alpha(h, 0.5), Err(Error::Gap(ContractGap::StaticAlphaStepOv6)));
    assert_eq!(m.snapshot(h).unwrap(), before);
    assert!(m.take_changed_rectangles().is_empty());
    let mut d = manager(true);
    let h = seed(&mut d, overlay(0));
    d.set_alpha(h, 0.7499).unwrap();
    assert_eq!(d.alpha(h), Ok(0.7499));
}

#[test]
fn placement_getter_copies_nine_floats_and_missing_geometry_never_mutates() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(0));
    let alias = indirect(&mut m, Target::Overlay(h.index()));
    let mut out = Placement([0.0; 9]);
    m.get_placement(alias, &mut out).unwrap();
    assert_eq!(out.0, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    assert_eq!(
        m.set_position(alias, [16.0, 32.0]),
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    );
    assert_eq!(
        m.set_placement(alias, Placement([8.0; 9])),
        Err(Error::Gap(ContractGap::HandlePlacementOv5))
    );
    let saved = out;
    m.get_placement(alias, &mut out).unwrap();
    assert_eq!(out, saved);
    assert_eq!(
        m.get_placement(h, &mut out),
        Err(Error::Gap(ContractGap::HandlePlacementOv5))
    );
}

#[test]
fn removal_releases_only_owned_texture_and_source_references_and_cell_membership() {
    let mut m = manager(false);
    let texture = Arc::new(Texture);
    let source = Arc::new(Texture);
    let mut o = overlay(flag::SHOWN | flag::PART);
    o.binding = Some(texture.clone());
    o.source = Some(source.clone());
    let h = seed(&mut m, o);
    assert_eq!(Arc::strong_count(&texture), 2);
    assert_eq!(Arc::strong_count(&source), 2);
    m.remove(h).unwrap();
    assert_eq!(Arc::strong_count(&texture), 1);
    assert_eq!(Arc::strong_count(&source), 1);
    assert!(m.cell_index.as_ref().unwrap().is_empty());
    assert_eq!(m.snapshot(h), Err(Error::InvalidHandle));
    assert_eq!(m.remove(h), Err(Error::InvalidHandle));
    assert_eq!(seed(&mut m, overlay(0)), h);
}

#[test]
fn group_removal_cleans_eight_parts_group_and_indirect_record_then_reuses_slots() {
    let mut m = manager(false);
    let (g, parts) = group(&mut m, 0);
    m.remove(g).unwrap();
    assert!(m.overlays.iter().next().is_none());
    assert!(m.handles.iter().next().is_none());
    assert!(m.groups.iter().next().is_none());
    assert!(m.cell_index.as_ref().unwrap().is_empty());
    assert_eq!(m.take_changed_rectangles().len(), 8);
    assert_eq!(seed(&mut m, overlay(0)), parts[7]);
    m.shutdown();
    assert!(m.overlays.iter().next().is_none());
}

#[test]
fn malformed_group_is_rejected_before_any_part_changes() {
    let mut m = manager(false);
    let h = seed(&mut m, overlay(0));
    let index = m
        .groups
        .insert([Some(h.index()), Some(999), None, None, None, None, None, None]);
    let g = indirect(&mut m, Target::Group(index));
    assert_eq!(m.enable_self_lighting(g, 1), Err(Error::InvalidHandle));
    assert_eq!(m.remove(g), Err(Error::InvalidHandle));
    assert_eq!(m.snapshot(h).unwrap().unwrap().flags, 0);
    assert!(m.take_changed_rectangles().is_empty());
}

#[test]
fn lifecycle_holds_acquired_references_and_unsubscribes_once_before_release() {
    let v = view(12);
    let terrain = v.terrain.as_ref().unwrap().clone();
    let factory = v.factory.as_ref().unwrap().clone();
    assert_eq!(Arc::strong_count(&v), 1);
    assert_eq!(Arc::strong_count(&terrain), 2);
    let mut m = OverlayManager::new();
    m.init(v.clone(), false, true).unwrap();
    assert_eq!(Arc::strong_count(&v), 2);
    assert_eq!(Arc::strong_count(&terrain), 3);
    assert_eq!(Arc::strong_count(&factory), 3);
    assert_eq!(
        m.geometry(),
        Some(Geometry {
            cells: [8, 4],
            cell_scale: 0.0625,
            metres: [128.0, 64.0]
        })
    );
    assert_eq!(m.shadow_settings(), [12, 4]);
    assert!(!m.is_dynamic());
    assert!(m.is_water());
    assert_eq!(*v.messages.lock().unwrap(), [(true, CHANGE_MESSAGE)]);
    assert_eq!(m.init(v.clone(), true, false), Err(Error::AlreadyInitialized));
    assert!(m.is_water());
    m.shutdown();
    assert_eq!(Arc::strong_count(&v), 1);
    assert_eq!(Arc::strong_count(&terrain), 2);
    assert_eq!(Arc::strong_count(&factory), 2);
    m.shutdown();
    drop(m);
    assert_eq!(
        *v.messages.lock().unwrap(),
        [(true, CHANGE_MESSAGE), (false, CHANGE_MESSAGE)]
    );
}

#[test]
fn drop_unsubscribes_and_shutdown_supports_clean_reinitialization() {
    let v = view(3);
    {
        let mut m = OverlayManager::new();
        m.init(v.clone(), true, false).unwrap();
        assert!(m.cell_index.is_none());
        seed(&mut m, overlay(0));
        m.shutdown();
        m.init(v.clone(), false, true).unwrap();
        assert!(m.overlays.iter().next().is_none());
        assert!(m.cell_index.as_ref().unwrap().is_empty());
        assert_eq!(seed(&mut m, overlay(0)), Handle(0));
    }
    assert_eq!(
        *v.messages.lock().unwrap(),
        [
            (true, CHANGE_MESSAGE),
            (false, CHANGE_MESSAGE),
            (true, CHANGE_MESSAGE),
            (false, CHANGE_MESSAGE),
        ]
    );
}

#[test]
fn initialization_failure_rolls_back_without_subscribing_or_retaining_resources() {
    for (terrain, factory, expected) in [
        (None, None, Error::MissingTerrain),
        (
            Some(Arc::new(Terrain {
                cells: [8, 4],
                width: 16.0,
            })),
            None,
            Error::MissingFactory,
        ),
        (
            Some(Arc::new(Terrain {
                cells: [8, 4],
                width: 0.0,
            })),
            Some(Arc::new(Factory {
                calls: Mutex::new(Vec::new()),
                texture: None,
            })),
            Error::InvalidTerrain,
        ),
    ] {
        let v = Arc::new(View {
            terrain,
            factory,
            quality: 3,
            debug: 4,
            messages: Mutex::new(Vec::new()),
        });
        let mut m = OverlayManager::new();
        assert_eq!(m.init(v.clone(), false, false), Err(expected));
        assert_eq!(Arc::strong_count(&v), 1);
        if let Some(t) = &v.terrain {
            assert_eq!(Arc::strong_count(t), 1);
        }
        assert!(!m.is_initialized());
        assert_eq!(m.geometry(), None);
        assert!(v.messages.lock().unwrap().is_empty());
    }
}

#[test]
fn factory_slots_and_null_binding_are_literal_but_placement_is_explicitly_incomplete() {
    let v = view(3);
    let factory = v.factory.as_ref().unwrap().clone();
    let texture = factory.texture.as_ref().unwrap().clone();
    let mut m = OverlayManager::new();
    let placement = Placement([0.0; 9]);
    assert_eq!(m.add_decal_binding(None, placement, 0, 0), Err(Error::NotInitialized));
    m.init(v, false, false).unwrap();
    assert_eq!(m.add_decal_binding(None, placement, 0, 0), Ok(Handle::NONE));
    for lookup in [BindingLookup::Slot6, BindingLookup::Slot5] {
        assert_eq!(
            m.add_decal_key(7, placement, 0, lookup),
            Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
        );
    }
    assert_eq!(
        *factory.calls.lock().unwrap(),
        [(7, BindingLookup::Slot6), (7, BindingLookup::Slot5)]
    );
    assert_eq!(Arc::strong_count(&texture), 2);
    assert_eq!(
        m.add_decal_binding(Some(texture.clone()), placement, 0, 0),
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    );
    assert_eq!(
        m.add_decal_source(texture.clone(), placement, 0, 0),
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    );
    assert_eq!(Arc::strong_count(&texture), 2);
    assert!(m.overlays.iter().next().is_none());
    assert_eq!(
        m.add_round_decal(7, placement),
        Err(Error::Gap(ContractGap::RoundPartitionOv9))
    );
    assert_eq!(m.add_city_decal(7), Ok(Handle(0)));
    assert_eq!(m.on_message(CHANGE_MESSAGE), Ok(()));
}

#[test]
fn no_binding_from_factory_returns_none_and_retains_no_new_resource() {
    let v = Arc::new(View {
        terrain: Some(Arc::new(Terrain {
            cells: [8, 4],
            width: 16.0,
        })),
        factory: Some(Arc::new(Factory {
            calls: Mutex::new(Vec::new()),
            texture: None,
        })),
        quality: 3,
        debug: 4,
        messages: Mutex::new(Vec::new()),
    });
    let mut m = OverlayManager::new();
    m.init(v, false, false).unwrap();
    for lookup in [BindingLookup::Slot5, BindingLookup::Slot6] {
        assert_eq!(m.add_decal_key(7, Placement([0.0; 9]), 0, lookup), Ok(Handle::NONE));
    }
}

#[test]
fn static_visible_cells_deduplicate_overlays_and_apply_shown_part_snapshot_flags() {
    let mut m = manager(false);
    let a = seed(&mut m, overlay(flag::SHOWN | flag::PART));
    let b = seed(&mut m, overlay(flag::SHOWN | flag::PART | flag::SNAPSHOT));
    seed(&mut m, overlay(flag::SHOWN));
    seed(&mut m, overlay(flag::PART));
    let visible = [[1, 1], [2, 2], [1, 1], [8, 4]];
    assert_eq!(m.select_frame(FRAME, &visible).unwrap().decals, [a.index(), b.index()]);
    assert!(m.select_frame(FRAME, &[[0, 0]]).unwrap().decals.is_empty());
    let snapshot = FrameOptions {
        snapshot: true,
        ..FRAME
    };
    assert_eq!(m.select_frame(snapshot, &visible).unwrap().decals, [b.index()]);
    m.remove(a).unwrap();
    assert_eq!(m.select_frame(FRAME, &visible).unwrap().decals, [b.index()]);
    assert_eq!(m.draw(FRAME, &visible), Err(Error::Gap(ContractGap::DrawingDetailsOv8)));
    assert_eq!(m.stats_string(), "0 decals, 0 shadows");
}

#[test]
fn dynamic_selection_visits_all_and_respects_decal_toggle_and_shadow_quality_path() {
    let mut m = manager(true);
    seed(&mut m, overlay(flag::SHOWN | flag::PART));
    assert_eq!(m.select_frame(FRAME, &[]).unwrap().decals, [0]);
    assert!(m
        .select_frame(FrameOptions { decals: false, ..FRAME }, &[])
        .unwrap()
        .decals
        .is_empty());
    for (quality, stage, rough) in [(0, true, true), (2, true, true), (3, true, false), (4, false, true)] {
        m.shadow_quality = quality;
        assert_eq!(
            m.select_frame(
                FrameOptions {
                    use_second_stage: stage,
                    ..FRAME
                },
                &[]
            )
            .unwrap()
            .rough_shadows,
            rough
        );
    }
}

#[test]
fn draw_state_table_covers_all_five_public_states() {
    use render::{BlendFactor::*, DepthFunction::*};
    for state in DRAW_STATES {
        assert!(state.color_write && state.depth_test && state.blending);
        assert_eq!(state.textures, 1);
    }
    assert_eq!(DRAW_STATES.map(|s| s.depth_write), [false, false, false, false, true]);
    assert_eq!(
        DRAW_STATES.map(|s| s.depth_function),
        [LessOrEqual, LessOrEqual, LessOrEqual, Greater, Less]
    );
    assert_eq!(
        DRAW_STATES.map(|s| s.blend),
        [
            [SourceAlpha, OneMinusSourceAlpha],
            [SourceAlpha, One],
            [Zero, SourceColor],
            [SourceAlpha, OneMinusSourceAlpha],
            [SourceAlpha, OneMinusSourceAlpha],
        ]
    );
    assert_eq!(
        DRAW_STATES.map(|s| s.stage_zero_mode),
        [Some(1), Some(1), Some(1), Some(1), None]
    );
    assert_eq!(
        DRAW_STATES.map(|s| s.alpha_greater_than),
        [None, None, None, None, Some(0.5)]
    );
}

#[test]
fn rectangle_clipping_and_cell_coverage_are_owned_by_core() {
    for (input, expected) in [
        (rect([1, 1], [3, 3]), rect([1, 1], [3, 3])),
        (rect([7, 3], [12, 9]), rect([7, 3], [8, 4])),
        (rect([10, 8], [12, 9]), rect([8, 4], [8, 4])),
        (rect([3, 2], [1, 1]), rect([3, 2], [3, 2])),
    ] {
        let clipped = input.clipped([8, 4]);
        assert_eq!(clipped, expected);
        for cell in clipped.cells() {
            assert!(clipped.contains(cell));
            assert!(cell[0] < 8 && cell[1] < 4);
        }
    }
    assert_eq!(
        rect([1, 1], [3, 3]).cells().collect::<Vec<_>>(),
        [[1, 1], [2, 1], [1, 2], [2, 2]]
    );
    assert!(!rect([1, 1], [3, 3]).contains([3, 2]));
    assert!(rect([1, 1], [1, 3]).is_empty());
}

#[test]
fn core_getters_are_send_sync_and_work_before_init_without_refcell() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<OverlayManager>();
    let m = Arc::new(OverlayManager::new());
    assert_eq!(m.geometry(), None);
    assert_eq!(m.alpha(Handle::NONE), Ok(0.0));
    let reader = std::thread::spawn(move || {
        assert!(!m.is_initialized());
        assert_eq!(m.stats_string(), "0 decals, 0 shadows");
    });
    reader.join().unwrap();
}
