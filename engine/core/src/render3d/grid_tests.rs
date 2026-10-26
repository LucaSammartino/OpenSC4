// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn grid_case_13_ties_to_even_and_scale() {
    for factor in [1, 2, 4] {
        let grid = QuadGrid::for_map_scale(factor);
        assert_eq!(grid.side(), 256);
        assert_eq!(grid.cell_size(), 32.0 * factor as f32);
        assert_eq!(grid.size(), 8192.0 * factor as f32);
        assert_eq!(
            grid.cells_for_rect(ContainerRect::new(
                64.0 * factor as f32,
                96.0 * factor as f32,
                64.0 * factor as f32,
                96.0 * factor as f32
            )),
            Rect::new(2, 2, 2, 2)
        );
    }
}

#[test]
fn element_defaults_and_per_quadrant_references() {
    let mut grid = QuadGrid::for_map_scale(1);
    let e = GridElement::new(Handle(1));
    assert_eq!(e.reference_count(), 0);
    assert_eq!(
        e.bounds,
        WorldBox {
            min: [1.0; 3],
            max: [0.0; 3]
        }
    );
    assert!(grid.add_element(e, ContainerRect::new(-50.0, -50.0, 64.0, 64.0), 1));
    assert_eq!(grid.element_count(), 1);
    assert_eq!(grid.element(Handle(1)).unwrap().reference_count(), 9);
    assert_eq!(grid.quadrant(0, 0), (1, &[Handle(1)][..]));
    assert_eq!(grid.quadrant(2, 2), (1, &[Handle(1)][..]));
    let removed = grid.remove_element(Handle(1)).unwrap();
    assert_eq!(removed.reference_count(), 0);
    assert_eq!(grid.element_count(), 0);
    assert!(grid.quadrant(2, 2).1.is_empty());
}

#[test]
fn grid_swap_removal_move_and_unchanged_cell_flags() {
    let mut grid = QuadGrid::for_map_scale(1);
    let rect = ContainerRect::new(1.0, 1.0, 10.0, 10.0);
    for model in [1, 2, 3] {
        assert!(grid.add_element(GridElement::new(Handle(model)), rect, 0));
    }
    grid.remove_element(Handle(1));
    assert_eq!(grid.quadrant(0, 0).1, &[Handle(3), Handle(2)]);
    assert!(grid.move_element(Handle(2), rect, 1));
    assert_eq!(grid.quadrant(0, 0).0, 0); // Unchanged cells return before ORing flags.
    grid.set_quadrant_flags_for_element(Handle(2), 4);
    assert_eq!(grid.quadrant(0, 0).0, 4);
    assert!(grid.move_element(Handle(2), ContainerRect::new(65.0, 65.0, 70.0, 70.0), 1));
    assert_eq!(grid.quadrant(0, 0).1, &[Handle(3)]);
    assert_eq!(grid.quadrant(2, 2), (1, &[Handle(2)][..]));
    assert_eq!(grid.element(Handle(2)).unwrap().reference_count(), 1);
    assert!(!grid.move_element(Handle(2), ContainerRect::new(9000.0, 0.0, 9100.0, 1.0), 1));
    assert!(grid.element(Handle(2)).is_none());
}

#[test]
fn grid_collects_each_element_once_and_clears_temporary_marks() {
    let mut grid = QuadGrid::for_map_scale(1);
    for model in [1, 2, 3] {
        assert!(grid.add_element(
            GridElement::new(Handle(model)),
            ContainerRect::new(1.0, 1.0, 100.0, 100.0),
            1
        ));
    }
    assert!(grid.all_elements(0).is_empty());
    assert_eq!(grid.all_elements(2), [Handle(1), Handle(2)]);
    assert_eq!(grid.all_elements(10), [Handle(1), Handle(2), Handle(3)]);
    for model in [1, 2, 3] {
        assert_eq!(grid.element(Handle(model)).unwrap().flags & 0x4000, 0);
    }
    grid.remove_all_elements();
    assert_eq!(grid.element_count(), 0);
    assert_eq!(grid.quadrant(0, 0), (0, &[][..]));
}

#[test]
fn empty_grid_and_disjoint_range_reject_addition() {
    let mut grid = QuadGrid::for_map_scale(0);
    assert_eq!(grid.side(), 0);
    assert!(!grid.add_element(GridElement::new(Handle(1)), ContainerRect::new(0.0, 0.0, 1.0, 1.0), 1));
    let mut grid = QuadGrid::for_map_scale(1);
    assert!(!grid.add_element(
        GridElement::new(Handle(1)),
        ContainerRect::new(-100.0, -100.0, -50.0, -50.0),
        1
    ));
    assert_eq!(grid.element_count(), 0);
}

#[test]
fn object_flag_table_and_attribute_precedence() {
    for (p, a, expected) in [
        (0, 0, 0x800),
        (1, 0, 0x820),
        (2, 0, 0x840),
        (4, 0, 0x900),
        (8, 0, 0xA00),
        (0x10, 0, 0x400),
        (0x20, 0, 0x800),
        (0x40, 0, 0x1000),
        (0x80, 0, 0x2000),
        (2, 2, 0x900),
        (2, 4, 0x901),
        (6, 6, 0x901),
    ] {
        let mut e = GridElement::new(Handle(1));
        e.update_object_flags(p, a);
        assert_eq!(e.flags, expected, "p={p:x}, a={a:x}");
    }
    let mut e = GridElement::new(Handle(1));
    e.flags = 0xFFFF;
    e.update_object_flags(0, 0);
    assert_eq!(e.flags, 0xC81F);
}

#[test]
fn backing_building_case_15_and_category_gates() {
    let limits = BackingLimits {
        terrain: 3,
        buildings_props: 5,
        flora: 4,
    };
    let mut e = GridElement::new(Handle(1));
    e.update_object_flags(0x22, 0);
    assert!(e.should_go_into_backing_store(4, limits));
    assert!(!e.should_go_into_backing_store(5, limits));
    for (flags, zoom, expected) in [
        (0x440, 2, true),
        (0x440, 3, false),
        (0x2040, 3, true),
        (0x2040, 4, false),
        (0x1840, 4, true),
        (0x1860, 1, false),
        (0x1940, 1, false),
        (0x1800, 1, false),
    ] {
        e.flags = flags;
        assert_eq!(
            e.should_go_into_backing_store(zoom, limits),
            expected,
            "flags={flags:x}, zoom={zoom}"
        );
    }
}
