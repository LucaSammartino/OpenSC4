// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal public-contract vectors plus explicitly marked AC5–AC8 choice tests.

use super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Acquire(&'static str, u64),
    Release(&'static str, u64),
    Occupant(u64),
    Power(u64),
    Rect,
    Detail,
    Add(u64),
    Remove(u64, u32),
    Flag(u64, bool),
}

struct Lease {
    kind: &'static str,
    id: u64,
    events: Arc<Mutex<Vec<Event>>>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.events.lock().unwrap().push(Event::Release(self.kind, self.id));
    }
}

struct Fixture {
    events: Arc<Mutex<Vec<Event>>>,
    occupants: BTreeMap<OccupantId, Occupant>,
    powered: BTreeSet<OccupantId>,
    ticks: BTreeSet<u64>,
    city_x: u32,
    zoom: u32,
    rect: WorldRect,
    detail: u8,
    fail_services: bool,
    fail_subscription: Option<u32>,
    missing_target: bool,
    fail_add: bool,
    fail_remove: bool,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            events: Arc::default(),
            occupants: BTreeMap::new(),
            powered: BTreeSet::new(),
            ticks: BTreeSet::new(),
            city_x: 64,
            zoom: 4,
            rect: WorldRect {
                min_x: 0.0,
                min_z: 0.0,
                max_x: 512.0,
                max_z: 512.0,
            },
            detail: 2,
            fail_services: false,
            fail_subscription: None,
            missing_target: false,
            fail_add: false,
            fail_remove: false,
        }
    }
}

impl Fixture {
    fn event(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }

    fn lease(&self, kind: &'static str, id: u64) -> Lease {
        self.event(Event::Acquire(kind, id));
        Lease {
            kind,
            id,
            events: self.events.clone(),
        }
    }

    fn insert(&mut self, id: u64, x: f64, z: f64, flags: u32, properties: &[u32]) {
        self.occupants.insert(
            OccupantId(id),
            Occupant {
                model_classes: vec![0xEA12_3BE1],
                exemplar_properties: properties.iter().copied().collect(),
                flags,
                x_metres: x,
                z_metres: z,
            },
        );
        if flags & 0x20 != 0 {
            self.ticks.insert(id);
        }
    }
}

impl Host for Fixture {
    type Services = Lease;
    type Subscription = Lease;
    type Target = Lease;

    fn acquire_services(&mut self) -> Result<Lease, Error> {
        if self.fail_services {
            return Err(Error::ForeignUnavailable);
        }
        Ok(self.lease("services", 0))
    }
    fn subscribe(&mut self, id: u32) -> Result<Lease, Error> {
        if self.fail_subscription == Some(id) {
            return Err(Error::ForeignUnavailable);
        }
        Ok(self.lease("subscription", u64::from(id)))
    }
    fn city_cells_x(&mut self) -> Result<u32, Error> {
        Ok(self.city_x)
    }
    fn view_zoom(&mut self) -> Result<u32, Error> {
        Ok(self.zoom)
    }
    fn visible_world_rect(&mut self) -> Result<WorldRect, Error> {
        self.event(Event::Rect);
        Ok(self.rect)
    }
    fn animation_detail_byte(&mut self) -> Result<u8, Error> {
        self.event(Event::Detail);
        Ok(self.detail)
    }
    fn occupant(&mut self, id: OccupantId) -> Result<Occupant, Error> {
        self.event(Event::Occupant(id.0));
        self.occupants.get(&id).cloned().ok_or(Error::ForeignUnavailable)
    }
    fn powered(&mut self, id: OccupantId) -> Result<bool, Error> {
        self.event(Event::Power(id.0));
        Ok(self.powered.contains(&id))
    }
    fn message_target(&mut self, id: OccupantId) -> Result<Option<Lease>, Error> {
        Ok((!self.missing_target).then(|| self.lease("target", id.0)))
    }
    fn tick_add(&mut self, target: &Lease) -> Result<(), Error> {
        self.event(Event::Add(target.id));
        if self.fail_add {
            return Err(Error::ForeignUnavailable);
        }
        self.ticks.insert(target.id);
        Ok(())
    }
    fn tick_remove(&mut self, target: &Lease, flags: u32) -> Result<(), Error> {
        self.event(Event::Remove(target.id, flags));
        if self.fail_remove {
            return Err(Error::ForeignUnavailable);
        }
        self.ticks.remove(&target.id);
        Ok(())
    }
    fn set_animating_flag(&mut self, id: OccupantId, on: bool) {
        self.event(Event::Flag(id.0, on));
        let flags = &mut self.occupants.get_mut(&id).unwrap().flags;
        if on {
            *flags |= 0x20;
        } else {
            *flags &= !0x20;
        }
    }
}

fn owner(mut fixture: Fixture) -> AnimationCoordinator<Fixture> {
    if fixture.occupants.is_empty() {
        fixture.insert(1, 20.0, 20.0, 4, &[]);
    }
    let mut owner = AnimationCoordinator::new(fixture);
    owner.init().unwrap();
    clear(&owner);
    owner
}

fn clear(owner: &AnimationCoordinator<Fixture>) {
    owner.host().events.lock().unwrap().clear();
}

fn events(owner: &AnimationCoordinator<Fixture>) -> Vec<Event> {
    owner.host().events.lock().unwrap().clone()
}

fn operations(owner: &AnimationCoordinator<Fixture>) -> Vec<Event> {
    events(owner)
        .into_iter()
        .filter(|e| matches!(e, Event::Add(_) | Event::Remove(_, _) | Event::Flag(_, _)))
        .collect()
}

#[test]
fn constructor_and_square_city_defaults() {
    let mut owner = AnimationCoordinator::new(Fixture {
        city_x: 80,
        zoom: 1,
        ..Fixture::default()
    });
    assert!(!owner.initialized());
    assert_eq!((owner.zoom(), owner.minimum_zoom()), (4, 2));
    assert!(owner.enabled());
    assert_eq!((owner.registered_count(), owner.blocks_per_side()), (0, 0));
    assert_eq!(owner.init(), Ok(true));
    assert_eq!((owner.zoom(), owner.blocks_per_side()), (1, 5));
    let before = events(&owner);
    assert_eq!(owner.init(), Ok(false));
    assert_eq!(events(&owner), before);
    // All five raw IDs, including the uninterpreted one, subscribed exactly once.
    assert_eq!(
        &before[..6],
        &[
            Event::Acquire("services", 0),
            Event::Acquire("subscription", 0x8A07_AE00),
            Event::Acquire("subscription", 0x6A5F_D3FD),
            Event::Acquire("subscription", 0x231B_BF91),
            Event::Acquire("subscription", 0xA6B7_9612),
            Event::Acquire("subscription", 0xA6B7_9602),
        ]
    );
}

#[test]
fn independent_filter_vectors() {
    // Input model classes, properties, flags -> animatable, shown-animatable.
    let vectors: &[(&[u32], &[u32], u32, bool, bool)] = &[
        (&[], &[], 4, false, false),
        (&[0xEA12_3BE1], &[], 0, true, false),
        (&[0xAA12_3BF9], &[], 4, true, true),
        (&[9, 0xEA12_3BE1], &[], 0x24, true, true),
        (&[0xEA12_3BE1], &[0x4A70_D491], 4, false, false),
        (&[0xAA12_3BF9], &[0xEA53_93ED, 0x8A5F_88E3], 4, true, true),
        (&[0xEA12_3BE0], &[], 4, false, false),
    ];
    for &(models, props, flags, animate, shown) in vectors {
        let occupant = Occupant {
            model_classes: models.to_vec(),
            exemplar_properties: props.iter().copied().collect(),
            flags,
            x_metres: 0.0,
            z_metres: 0.0,
        };
        assert_eq!((occupant.animatable(), occupant.shown_animatable()), (animate, shown));
    }
}

#[test]
fn block_position_vectors_and_clamping() {
    let owner = owner(Fixture::default());
    // §6.1's (300,700)->(1,2); AC5 boundary and negative choices are separate rows.
    for (x, z, expected) in [
        (300.0, 700.0, Block { x: 1, z: 2 }),
        (0.0, 0.0, Block { x: 0, z: 0 }),
        (255.99, 256.0, Block { x: 0, z: 1 }),
        (-1.0, -700.0, Block { x: 0, z: 0 }),
        (99999.0, 1024.0, Block { x: 3, z: 3 }),
    ] {
        let occupant = Occupant {
            x_metres: x,
            z_metres: z,
            ..owner.host().occupants[&OccupantId(1)].clone()
        };
        assert_eq!(owner.occupant_block(&occupant), Ok(expected));
    }
}

#[test]
fn world_overlap_boundary_vectors_ac5() {
    for (world, expected) in [
        (
            WorldRect {
                min_x: 1.0,
                min_z: 300.0,
                max_x: 2.0,
                max_z: 301.0,
            },
            BlockRect {
                min_x: 0,
                min_z: 1,
                max_x: 1,
                max_z: 2,
            },
        ),
        (
            WorldRect {
                min_x: 0.0,
                min_z: 0.0,
                max_x: 256.0,
                max_z: 256.0,
            },
            BlockRect {
                min_x: 0,
                min_z: 0,
                max_x: 1,
                max_z: 1,
            },
        ),
        (
            WorldRect {
                min_x: -500.0,
                min_z: -500.0,
                max_x: 2000.0,
                max_z: 2000.0,
            },
            BlockRect {
                min_x: 0,
                min_z: 0,
                max_x: 4,
                max_z: 4,
            },
        ),
        (
            WorldRect {
                min_x: 9.0,
                min_z: 0.0,
                max_x: 8.0,
                max_z: 1.0,
            },
            BlockRect::default(),
        ),
    ] {
        assert_eq!(world.blocks(4), Ok(expected));
    }
    assert_eq!(
        WorldRect {
            min_x: f64::NAN,
            ..WorldRect::default()
        }
        .blocks(4),
        Err(Error::InvalidGeometry)
    );
}

#[test]
fn zoom_gating_literal_vectors() {
    for (zoom, animates) in [(0, false), (1, false), (2, true), (4, true), (5, true)] {
        let mut owner = owner(Fixture {
            zoom,
            ..Fixture::default()
        });
        owner.set_occupant_animation(OccupantId(1), true).unwrap();
        assert_eq!(owner.host().ticks.contains(&1), animates, "zoom {zoom}");
        assert_eq!(
            owner.host().occupants[&OccupantId(1)].flags,
            if animates { 0x24 } else { 4 }
        );
        assert_eq!(owner.registered_count(), 1);
    }
}

#[test]
fn offscreen_occupant_in_visible_block_animates() {
    let mut fixture = Fixture {
        rect: WorldRect {
            min_x: 1.0,
            min_z: 1.0,
            max_x: 2.0,
            max_z: 2.0,
        },
        ..Fixture::default()
    };
    fixture.insert(1, 250.0, 250.0, 4, &[]);
    fixture.insert(2, 256.0, 0.0, 4, &[]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    owner.set_occupant_animation(OccupantId(2), true).unwrap();
    assert_eq!(owner.host().ticks, BTreeSet::from([1]));
    assert_eq!(owner.registered_count(), 2);
}

#[test]
fn activation_and_removal_order_and_reference_lease() {
    let mut owner = owner(Fixture::default());
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    assert_eq!(
        events(&owner),
        vec![
            Event::Occupant(1),
            Event::Acquire("target", 1),
            Event::Add(1),
            Event::Flag(1, true),
            Event::Release("target", 1)
        ]
    );
    clear(&owner);
    owner.set_occupant_animation(OccupantId(1), false).unwrap();
    assert_eq!(
        events(&owner),
        vec![
            Event::Occupant(1),
            Event::Acquire("target", 1),
            Event::Remove(1, 0xC0),
            Event::Flag(1, false),
            Event::Release("target", 1)
        ]
    );
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 4);
    assert_eq!(owner.registered_count(), 0);
}

#[test]
fn duplicate_set_and_remove_leave_counts_exact() {
    let mut owner = owner(Fixture::default());
    for _ in 0..3 {
        owner.set_occupant_animation(OccupantId(1), true).unwrap();
    }
    assert_eq!(owner.registered_count(), 1);
    assert_eq!(owner.block_occupant_count(Block { x: 0, z: 0 }), 1);
    assert_eq!(operations(&owner), vec![Event::Add(1), Event::Flag(1, true)]);
    clear(&owner);
    for _ in 0..3 {
        owner.set_occupant_animation(OccupantId(1), false).unwrap();
    }
    assert_eq!(owner.registered_count(), 0);
    assert_eq!(operations(&owner), vec![Event::Remove(1, 0xC0), Event::Flag(1, false)]);
}

#[test]
fn no_flag_transition_means_no_target_query() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 0xA4, &[]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    assert_eq!(events(&owner), vec![Event::Occupant(1)]);
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 0xA4);
}

#[test]
fn excluded_model_does_not_register_or_clear_flags() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 0x24, &[0x4A70_D491]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    owner.set_occupant_animation(OccupantId(1), false).unwrap();
    assert_eq!(owner.registered_count(), 0);
    assert!(operations(&owner).is_empty());
}

#[test]
fn shown_filter_does_not_add_an_unpublished_registration_gate() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 0, &[]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 0x20);
}

#[test]
fn power_change_only_rechecks_changed_blocks() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 4, &[0xEA53_93ED]);
    fixture.insert(2, 300.0, 20.0, 4, &[0xEA53_93ED]);
    fixture.powered.extend([OccupantId(1), OccupantId(2)]);
    let mut owner = owner(fixture);
    for id in [1, 2] {
        owner.set_occupant_animation(OccupantId(id), true).unwrap();
    }
    owner.host_mut().powered.clear();
    clear(&owner);
    owner
        .do_message(Message::PowerChanged(BlockRect {
            min_x: 0,
            min_z: 0,
            max_x: 1,
            max_z: 1,
        }))
        .unwrap();
    assert_eq!(operations(&owner), vec![Event::Remove(1, 0xC0), Event::Flag(1, false)]);
    assert!(!events(&owner).contains(&Event::Power(2)));
    assert_eq!(owner.host().ticks, BTreeSet::from([2]));
    owner.host_mut().powered.insert(OccupantId(1));
    owner
        .do_message(Message::PowerChanged(BlockRect {
            min_x: 0,
            min_z: 0,
            max_x: 1,
            max_z: 1,
        }))
        .unwrap();
    assert_eq!(owner.host().ticks, BTreeSet::from([1, 2]));
}

#[test]
fn power_is_only_queried_when_property_present_and_other_gates_pass() {
    let mut owner = owner(Fixture::default());
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    assert!(!events(&owner).iter().any(|e| matches!(e, Event::Power(_))));
    owner
        .host_mut()
        .occupants
        .get_mut(&OccupantId(1))
        .unwrap()
        .exemplar_properties
        .insert(0xEA53_93ED);
    clear(&owner);
    owner
        .do_message(Message::ZoomRotation {
            zoom: 1,
            old_rotation: 0,
            new_rotation: 0,
        })
        .unwrap();
    assert!(!events(&owner).contains(&Event::Power(1)));
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 4);
}

#[test]
fn budget_is_explicitly_unresolved_without_a_foreign_coordinator_decision() {
    for initial_flags in [4, 0x24] {
        let mut fixture = Fixture::default();
        fixture.insert(1, 20.0, 20.0, initial_flags, &[0x8A5F_88E3]);
        let mut owner = owner(fixture);
        assert_eq!(
            owner.set_occupant_animation(OccupantId(1), true),
            Err(Error::UnspecifiedBudgetTest(OccupantId(1)))
        );
        assert_eq!(owner.registered_count(), 1); // AC7 choice: registration committed first.
        assert_eq!(owner.host().occupants[&OccupantId(1)].flags, initial_flags);
        assert!(operations(&owner).is_empty());
        assert_eq!(
            owner.do_message(Message::BudgetChanged(OccupantId(1))),
            Err(Error::UnspecifiedBudgetTest(OccupantId(1)))
        );
        // An explicit off bypasses the unknown budget predicate and can stop.
        owner.set_occupant_animation(OccupantId(1), false).unwrap();
        assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 4);
    }
}

#[test]
fn unpowered_budget_occupant_can_be_stopped_without_guessing_budget() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 0x24, &[0xEA53_93ED, 0x8A5F_88E3]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    assert_eq!(operations(&owner), vec![Event::Remove(1, 0xC0), Event::Flag(1, false)]);
}

#[test]
fn opaque_message_refreshes_all_without_assigning_byte_semantics() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 4, &[0xEA53_93ED]);
    let mut owner = owner(fixture);
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    owner.host_mut().powered.insert(OccupantId(1));
    owner.host_mut().detail = 0;
    clear(&owner);
    owner.do_message(Message::OpaqueRefresh).unwrap();
    assert_eq!(operations(&owner), vec![Event::Add(1), Event::Flag(1, true)]);
    assert!(!events(&owner).contains(&Event::Detail));
}

#[test]
fn enabling_byte_threshold_vectors_and_refresh_on_change_only() {
    for (byte, enabled) in [(0, false), (1, false), (2, true), (3, true), (255, true)] {
        let mut owner = owner(Fixture::default());
        owner.set_occupant_animation(OccupantId(1), true).unwrap();
        owner.host_mut().detail = byte;
        clear(&owner);
        owner.refresh_animation_enabled().unwrap();
        assert_eq!(owner.enabled(), enabled);
        assert_eq!(owner.host().ticks.contains(&1), enabled);
        clear(&owner);
        owner.refresh_animation_enabled().unwrap();
        assert_eq!(events(&owner), vec![Event::Detail]);
        if !enabled {
            owner.host_mut().detail = 2;
            owner.refresh_animation_enabled().unwrap();
            assert_eq!(operations(&owner), vec![Event::Add(1), Event::Flag(1, true)]);
        }
    }
}

#[test]
fn empty_registration_skips_global_and_power_updates() {
    let mut owner = owner(Fixture::default());
    owner.refresh_animation_enabled().unwrap();
    owner
        .do_message(Message::PowerChanged(BlockRect {
            min_x: 0,
            min_z: 0,
            max_x: 4,
            max_z: 4,
        }))
        .unwrap();
    assert!(events(&owner).is_empty());
    assert!(owner.enabled());
}

#[test]
fn scroll_leaving_then_entering_with_no_overlap_callbacks_ac7() {
    let mut fixture = Fixture::default();
    fixture.insert(1, 20.0, 20.0, 4, &[]);
    fixture.insert(2, 300.0, 20.0, 4, &[]);
    fixture.insert(3, 600.0, 20.0, 4, &[]);
    let mut owner = owner(fixture);
    for id in [1, 2, 3] {
        owner.set_occupant_animation(OccupantId(id), true).unwrap();
    }
    owner.host_mut().rect = WorldRect {
        min_x: 256.0,
        min_z: 0.0,
        max_x: 768.0,
        max_z: 512.0,
    };
    clear(&owner);
    owner.do_message(Message::Translation).unwrap();
    assert_eq!(
        operations(&owner),
        vec![
            Event::Remove(1, 0xC0),
            Event::Flag(1, false),
            Event::Add(3),
            Event::Flag(3, true)
        ]
    );
    assert!(!events(&owner).contains(&Event::Occupant(2)));
    assert_eq!(owner.host().ticks, BTreeSet::from([2, 3]));
    clear(&owner);
    owner.do_message(Message::Translation).unwrap();
    assert_eq!(events(&owner), vec![Event::Rect]);
}

#[test]
fn zoom_rotation_update_trigger_vectors() {
    for (old_zoom, zoom, old_rotation, new_rotation, refresh) in [
        (1, 1, 0, 1, false),
        (1, 0, 0, 1, false),
        (1, 2, 0, 0, true),
        (2, 1, 0, 0, true),
        (2, 4, 0, 0, false),
        (4, 4, 0, 1, true),
        (2, 3, 0, 1, true),
        (2, 2, 1, 1, false),
    ] {
        let mut owner = owner(Fixture {
            zoom: old_zoom,
            ..Fixture::default()
        });
        owner.set_occupant_animation(OccupantId(1), true).unwrap();
        clear(&owner);
        owner
            .do_message(Message::ZoomRotation {
                zoom,
                old_rotation,
                new_rotation,
            })
            .unwrap();
        assert_eq!(owner.zoom(), zoom);
        assert_eq!(events(&owner).contains(&Event::Rect), refresh);
        assert_eq!(events(&owner).contains(&Event::Occupant(1)), refresh);
        assert_eq!(owner.host().ticks.contains(&1), zoom >= 2);
    }
}

#[test]
fn zoom_is_stored_without_registered_occupants() {
    let mut owner = owner(Fixture::default());
    owner
        .do_message(Message::ZoomRotation {
            zoom: 1,
            old_rotation: 0,
            new_rotation: 1,
        })
        .unwrap();
    assert_eq!(owner.zoom(), 1);
    assert!(events(&owner).is_empty());
}

#[test]
fn unknown_and_unregistered_budget_targets_do_not_register_ac8() {
    let mut owner = owner(Fixture::default());
    assert_eq!(owner.do_message(Message::Other(123)), Ok(false));
    assert!(events(&owner).is_empty());
    owner.do_message(Message::BudgetChanged(OccupantId(1))).unwrap();
    assert_eq!(owner.registered_count(), 0);
    assert!(operations(&owner).is_empty());
}

#[test]
fn target_and_tick_failure_preserve_flags_and_release_only_owned_lease_ac7() {
    for (missing_target, fail_add, error) in [
        (true, false, Error::MissingMessageTarget),
        (false, true, Error::ForeignUnavailable),
    ] {
        let mut owner = owner(Fixture {
            missing_target,
            fail_add,
            ..Fixture::default()
        });
        assert_eq!(owner.set_occupant_animation(OccupantId(1), true), Err(error));
        assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 4);
        assert_eq!(owner.registered_count(), 1);
        assert!(!events(&owner).contains(&Event::Flag(1, true)));
        let releases = events(&owner)
            .iter()
            .filter(|e| **e == Event::Release("target", 1))
            .count();
        assert_eq!(releases, usize::from(!missing_target));
    }
    let mut owner = owner(Fixture::default());
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    owner.host_mut().fail_remove = true;
    clear(&owner);
    assert_eq!(
        owner.set_occupant_animation(OccupantId(1), false),
        Err(Error::ForeignUnavailable)
    );
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 0x24);
    assert_eq!(owner.registered_count(), 0);
    assert_eq!(
        events(&owner),
        vec![
            Event::Occupant(1),
            Event::Acquire("target", 1),
            Event::Remove(1, 0xC0),
            Event::Release("target", 1)
        ]
    );
}

#[test]
fn refresh_aborts_at_first_error_in_sorted_identity_order_ac7() {
    let mut fixture = Fixture::default();
    fixture.insert(3, 20.0, 20.0, 4, &[]);
    fixture.insert(1, 20.0, 20.0, 4, &[]);
    let mut owner = owner(fixture);
    for id in [3, 1] {
        owner.set_occupant_animation(OccupantId(id), true).unwrap();
    }
    owner.host_mut().fail_remove = true;
    owner.host_mut().rect = WorldRect {
        min_x: 768.0,
        min_z: 768.0,
        max_x: 1024.0,
        max_z: 1024.0,
    };
    clear(&owner);
    assert_eq!(owner.do_message(Message::Translation), Err(Error::ForeignUnavailable));
    assert_eq!(operations(&owner), vec![Event::Remove(1, 0xC0)]);
    assert!(!events(&owner).contains(&Event::Occupant(3)));
}

#[test]
fn failed_init_rolls_back_only_acquired_services_and_subscriptions_ac7() {
    let mut owner = AnimationCoordinator::new(Fixture {
        fail_subscription: Some(0x6A5F_D3FD),
        ..Fixture::default()
    });
    assert_eq!(owner.init(), Err(Error::ForeignUnavailable));
    assert!(!owner.initialized());
    assert_eq!(
        events(&owner),
        vec![
            Event::Acquire("services", 0),
            Event::Acquire("subscription", 0x8A07_AE00),
            Event::Release("subscription", 0x8A07_AE00),
            Event::Release("services", 0),
        ]
    );
    owner.host_mut().fail_subscription = None;
    assert_eq!(owner.init(), Ok(true));
    let mut failed = AnimationCoordinator::new(Fixture {
        fail_services: true,
        ..Fixture::default()
    });
    assert_eq!(failed.init(), Err(Error::ForeignUnavailable));
    assert!(events(&failed).is_empty());
}

#[test]
fn shutdown_unsubscribes_releases_and_empties_without_invented_tick_removal_ac6() {
    let mut owner = owner(Fixture::default());
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    clear(&owner);
    owner.shutdown();
    assert_eq!(
        events(&owner),
        vec![
            Event::Release("subscription", 0x8A07_AE00),
            Event::Release("subscription", 0x6A5F_D3FD),
            Event::Release("subscription", 0x231B_BF91),
            Event::Release("subscription", 0xA6B7_9612),
            Event::Release("subscription", 0xA6B7_9602),
            Event::Release("services", 0),
        ]
    );
    assert_eq!((owner.registered_count(), owner.blocks_per_side()), (0, 0));
    assert_eq!(owner.host().occupants[&OccupantId(1)].flags, 0x24);
    clear(&owner);
    owner.shutdown();
    assert!(events(&owner).is_empty());
    assert_eq!(owner.do_message(Message::OpaqueRefresh), Err(Error::NotInitialized));
    assert_eq!(owner.init(), Ok(true));
}

#[test]
fn drop_releases_owned_lease_once_and_never_releases_borrowed_occupant_ac6() {
    let mut owner = owner(Fixture::default());
    owner.set_occupant_animation(OccupantId(1), true).unwrap();
    let log = owner.host().events.clone();
    clear(&owner);
    drop(owner);
    let log = log.lock().unwrap();
    assert_eq!(log.len(), 6);
    assert_eq!(log.last(), Some(&Event::Release("services", 0)));
    assert!(log
        .iter()
        .all(|e| matches!(e, Event::Release("services" | "subscription", _))));
}

#[test]
fn invalid_state_geometry_and_missing_occupant_have_no_silent_defaults() {
    let mut uninitialized = AnimationCoordinator::new(Fixture::default());
    assert_eq!(
        uninitialized.set_occupant_animation(OccupantId(1), true),
        Err(Error::NotInitialized)
    );
    let mut owner = owner(Fixture {
        city_x: 15,
        ..Fixture::default()
    });
    assert_eq!(owner.set_occupant_animation(OccupantId(1), true), Err(Error::EmptyGrid));
    assert_eq!(owner.registered_count(), 0);
    assert_eq!(
        owner.set_occupant_animation(OccupantId(999), true),
        Err(Error::ForeignUnavailable)
    );
    let mut normal = super::tests::owner(Fixture::default());
    normal.host_mut().occupants.get_mut(&OccupantId(1)).unwrap().x_metres = f64::INFINITY;
    assert_eq!(
        normal.set_occupant_animation(OccupantId(1), true),
        Err(Error::InvalidGeometry)
    );
    assert_eq!(normal.registered_count(), 0);
}

#[test]
fn getter_state_can_be_shared_immutably_between_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<AnimationCoordinator<Fixture>>();
    let owner = Arc::new(owner(Fixture::default()));
    let other = owner.clone();
    let thread = std::thread::spawn(move || (other.zoom(), other.enabled(), other.registered_count()));
    assert_eq!((owner.zoom(), owner.enabled(), owner.registered_count()), (4, true, 0));
    assert_eq!(thread.join().unwrap(), (4, true, 0));
}
