// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn reference(token: u64) -> RefValue {
    RefValue::Host {
        token,
        class_id: 0x09C0_5C6A,
    }
}

struct Host {
    cell: (i32, i32),
    water: bool,
    log: Vec<String>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            cell: (12, 3),
            water: false,
            log: vec![],
        }
    }
}
impl References for Host {
    fn add_ref(&mut self, value: &RefValue) {
        self.log.push(format!("add:{value:?}"));
    }
    fn release(&mut self, value: &RefValue) {
        self.log.push(format!("release:{value:?}"));
    }
}
impl PoleLinkHost for Host {
    fn pole_cell(&mut self, _: &RefValue) -> (i32, i32) {
        self.log.push("cell".into());
        self.cell
    }
    fn attachment_points(&mut self, _: &RefValue, orientation: u32) -> Result<[[f32; 3]; 4], PoleUnsupported> {
        self.log.push(format!("points:{orientation}"));
        let mut points = AttachmentTables::default().0[0][orientation as usize];
        for point in &mut points {
            point[0] += self.cell.0 as f32 * 16.0;
            point[2] += self.cell.1 as f32 * 16.0;
        }
        Ok(points)
    }
    fn cell_size(&mut self) -> [f32; 2] {
        [16.0; 2]
    }
    fn is_water(&mut self, cell: (i32, i32)) -> bool {
        self.log.push(format!("water:{cell:?}"));
        self.water
    }
    fn post_message(&mut self, message: u32, data: u32) {
        assert_eq!((message, data), (0x99EF_1145, 0));
        self.log.push("post".into());
    }
}

fn pole() -> PowerPole {
    let mut pole = PowerPole::fresh();
    pole.fields.position = [32.0, 0.0, 48.0];
    pole.fields.cell = (90, 91); // The builder computes its start, but water/enable use this stored pair.
    pole.select_connection_point_table(1);
    pole
}

fn add(pole: &mut PowerPole, host: &mut Host, other: RefValue, lines: &[RefValue]) {
    pole.add_connection(
        &other,
        lines,
        &AttachmentTables::default(),
        CurveParams::default(),
        host,
    )
    .unwrap();
}

fn refresh(pole: &mut PowerPole, host: &mut Host, other: RefValue) {
    pole.rebuild_wires(&other, &AttachmentTables::default(), CurveParams::default(), host)
        .unwrap();
}

#[test]
fn owning_connection_copies_duplicate_lines_once_each_and_uses_computed_start() {
    let (mut p, mut host) = (pole(), Host::default());
    let wires = [reference(7), reference(7)];
    add(&mut p, &mut host, reference(8), &wires);
    let r = p.connection(0).unwrap();
    assert_eq!(r.flags, [0, 1]);
    assert_eq!(r.words, [12, 3]);
    assert_eq!(r.wire_points.len(), 2);
    assert!(r.wire_points.iter().zip(&wires).all(|(a, b)| same_ref(a, b)));
    assert_eq!(r.vertices[0].len(), 4);
    assert_eq!(r.vertices[0][0][0], [32.0, f32::from_bits(0x41FD999A), 52.8]);
    assert!(r.vertices[0].iter().all(|curve| curve.len() >= 5));
    assert!(r.vertices[1].is_empty());
    assert_eq!(host.log.iter().filter(|s| s.starts_with("add:")).count(), 3);
    assert_eq!(
        host.log
            .iter()
            .filter(|s| s.starts_with("water:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["water:(90, 91)", "water:(12, 3)"]
    );
    assert_eq!(host.log.last().unwrap(), "post");
    assert!(p.stale);
}

#[test]
fn nonowner_still_probes_water_but_never_reads_points_or_retains_lines() {
    let (mut p, mut host) = (
        pole(),
        Host {
            cell: (0, 0),
            ..Default::default()
        },
    );
    add(&mut p, &mut host, reference(8), &[reference(7)]);
    let r = p.connection(0).unwrap();
    assert_eq!(r.flags, [0, 0]);
    assert!(r.wire_points.is_empty() && r.vertices.iter().all(Vec::is_empty));
    assert_eq!(host.log.len(), 4); // cell, retain endpoint, two terrain probes.
    assert!(p.stale);
}

#[test]
fn duplicate_cell_and_coincident_start_are_inert_even_for_different_pointers() {
    let (mut p, mut host) = (pole(), Host::default());
    add(&mut p, &mut host, reference(8), &[]);
    for other in [reference(9), RefValue::Null] {
        host.log.clear();
        p.stale = false;
        add(&mut p, &mut host, other, &[reference(7)]);
        assert!(!p.stale);
        assert_eq!(p.connection_count(), 1);
        assert!(host.log.iter().all(|s| s == "cell"));
    }
    host.cell = (2, 3);
    host.log.clear();
    add(&mut p, &mut host, reference(10), &[]);
    assert_eq!(host.log, ["cell"]);
    assert_eq!(p.connection_count(), 1);
}

#[test]
fn enable_probes_even_with_no_match_and_uses_stored_unsigned_cell() {
    let (mut p, mut host) = (pole(), Host::default());
    p.enable_connection(&reference(8), true, &mut host);
    assert_eq!(host.log, ["cell"]);
    p.fields.connections.push(PoleConnection {
        pole: reference(8),
        flags: [1, 1],
        ..Default::default()
    });
    p.fields.connections.push(p.fields.connections[0].clone());
    host.log.clear();
    p.enable_connection(&reference(8), true, &mut host);
    assert_eq!(host.log, ["cell", "cell"]);
    assert_eq!(p.fields.connections[0].flags, [1, 0]);
    assert_eq!(p.fields.connections[1].flags, [1, 1]);
    p.fields.cell = (-1, 3);
    p.enable_connection(&reference(8), true, &mut host);
    assert_eq!(p.fields.connections[0].flags[1], 0);
    p.fields.cell = (0, 3);
    p.enable_connection(&reference(8), true, &mut host);
    assert_eq!(p.fields.connections[0].flags[1], 1);
    host.log.clear();
    p.enable_connection(&reference(8), false, &mut host);
    assert_eq!(host.log, ["cell"]);
    assert_eq!(p.fields.connections[0].flags, [1, 0]);
    assert!(!p.stale);
}

#[test]
fn refresh_appends_water_curves_preserves_lines_and_stored_destination() {
    let (mut p, mut host) = (
        pole(),
        Host {
            water: true,
            ..Default::default()
        },
    );
    add(&mut p, &mut host, reference(8), &[reference(7)]);
    let original = p.fields.connections[0].vertices[0].clone();
    assert_eq!(p.fields.connections[0].vertices[1].len(), 4);
    assert_eq!(host.log.iter().filter(|s| s.starts_with("water:")).count(), 1);
    host.cell = (18, 3);
    host.log.clear();
    refresh(&mut p, &mut host, reference(8));
    let r = &p.fields.connections[0];
    assert_ne!(r.vertices[0], original);
    assert_eq!(r.vertices[1].len(), 8);
    assert_eq!(r.words, [12, 3]);
    assert_eq!(r.flags, [0, 1]);
    assert_eq!(r.wire_points.len(), 1);
    assert!(host
        .log
        .iter()
        .all(|s| !s.starts_with("add:") && !s.starts_with("release:")));
    host.water = false;
    let coarse = p.fields.connections[0].vertices[1].clone();
    refresh(&mut p, &mut host, reference(8));
    assert_eq!(p.fields.connections[0].vertices[1], coarse);
}

#[test]
fn refresh_no_match_nonowner_or_coincident_cells_preserves_all_geometry() {
    let (mut p, mut host) = (pole(), Host::default());
    add(&mut p, &mut host, reference(8), &[]);
    let original = p.fields.connections[0].vertices.clone();
    for other in [RefValue::Null, reference(9)] {
        host.log.clear();
        refresh(&mut p, &mut host, other);
        assert!(host.log.is_empty());
    }
    host.cell = (2, 3);
    refresh(&mut p, &mut host, reference(8));
    assert_eq!(p.fields.connections[0].vertices, original);
    p.fields.connections[0].flags[1] = 0;
    host.log.clear();
    refresh(&mut p, &mut host, reference(8));
    assert!(host.log.is_empty());
}

#[test]
fn unsupported_attachment_invariant_balances_the_new_endpoint_reference() {
    let (mut p, mut host) = (PowerPole::fresh(), Host::default());
    assert_eq!(
        p.add_connection(
            &reference(8),
            &[],
            &AttachmentTables::default(),
            CurveParams::default(),
            &mut host
        ),
        Err(PoleUnsupported::AttachmentInvariant)
    );
    assert_eq!(p.connection_count(), 0);
    assert_eq!(host.log.iter().filter(|s| s.starts_with("add:")).count(), 1);
    assert_eq!(host.log.iter().filter(|s| s.starts_with("release:")).count(), 1);
}
