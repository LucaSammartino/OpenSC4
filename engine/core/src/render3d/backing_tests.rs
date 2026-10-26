// SPDX-License-Identifier: GPL-3.0-or-later
use super::{tests::Host, *};
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
enum BufferEvent {
    Create(u32),
    Free(Handle),
    Viewport,
    Copy(CopyOperation, Handle, BufferCopy),
}

struct BufferHost {
    service: bool,
    created: [Option<Handle>; 2],
    results: VecDeque<bool>,
    events: Vec<BufferEvent>,
}

impl Default for BufferHost {
    fn default() -> Self {
        Self {
            service: true,
            created: [Some(Handle(10)), Some(Handle(11))],
            results: VecDeque::new(),
            events: Vec::new(),
        }
    }
}

impl BackingServices for BufferHost {
    fn backing_service_present(&self) -> bool {
        self.service
    }
    fn create_backing_buffer(&mut self, kind: u32) -> Option<Handle> {
        self.events.push(BufferEvent::Create(kind));
        self.created[kind as usize]
    }
    fn free_backing_buffer(&mut self, buffer: Handle) {
        self.events.push(BufferEvent::Free(buffer));
    }
    fn driver_viewport_size(&mut self) -> [i32; 2] {
        self.events.push(BufferEvent::Viewport);
        [800, 600]
    }
    fn copy_backing_buffer(&mut self, operation: CopyOperation, buffer: Handle, copy: BufferCopy) -> bool {
        self.events.push(BufferEvent::Copy(operation, buffer, copy));
        self.results.pop_front().unwrap_or(true)
    }
}

fn two_copies(operation: CopyOperation, copy: BufferCopy) -> [BufferEvent; 2] {
    [
        BufferEvent::Copy(operation, Handle(10), copy),
        BufferEvent::Copy(operation, Handle(11), copy),
    ]
}

#[test]
fn create_probes_save_restore_in_buffer_order_and_reuses_existing_buffers() {
    let mut h = BufferHost::default();
    let mut globals = BackingStore::default();
    assert!(globals.create(&mut h));
    let copy = BufferCopy {
        source: [0; 2],
        size: [800, 600],
        destination: [0; 2],
    };
    assert_eq!(
        h.events,
        [
            BufferEvent::Create(0),
            BufferEvent::Create(1),
            BufferEvent::Viewport,
            BufferEvent::Copy(CopyOperation::Save, Handle(10), copy),
            BufferEvent::Copy(CopyOperation::Restore, Handle(10), copy),
            BufferEvent::Copy(CopyOperation::Save, Handle(11), copy),
            BufferEvent::Copy(CopyOperation::Restore, Handle(11), copy),
        ]
    );
    h.events.clear();
    assert!(globals.create(&mut h));
    assert_eq!(h.events[0], BufferEvent::Viewport);
    assert_eq!(h.events.len(), 5);
}

#[test]
fn missing_buffer_and_each_probe_failure_free_all_created_buffers() {
    for missing in 0..2 {
        let mut h = BufferHost::default();
        h.created[missing] = None;
        let mut globals = BackingStore::default();
        assert!(!globals.create(&mut h));
        assert_eq!(globals.buffers(), [None; 2]);
        assert_eq!(
            h.events,
            [
                BufferEvent::Create(0),
                BufferEvent::Create(1),
                BufferEvent::Free(Handle(if missing == 0 { 11 } else { 10 }))
            ]
        );
    }
    for failed_probe in 0..4 {
        let mut h = BufferHost::default();
        for _ in 0..failed_probe {
            h.results.push_back(true);
        }
        h.results.push_back(false);
        let mut globals = BackingStore::default();
        assert!(!globals.create(&mut h));
        assert_eq!(globals.buffers(), [None; 2]);
        assert_eq!(
            h.events
                .iter()
                .filter(|event| matches!(event, BufferEvent::Copy(..)))
                .count(),
            failed_probe + 1
        );
        assert_eq!(
            &h.events[h.events.len() - 2..],
            &[BufferEvent::Free(Handle(10)), BufferEvent::Free(Handle(11))]
        );
    }
}

#[test]
fn save_b1_both_buffers_ignore_copy_failures() {
    let mut h = BufferHost::default();
    let mut globals = BackingStore::default();
    assert!(globals.create(&mut h));
    h.events.clear();
    h.results.extend([false, false]);
    let mut r = Render3d::new(&mut Host::default());
    r.set_viewport_size(800, 600);
    assert!(r.has_backing_store(&globals));
    assert!(r.save_backing_store(&globals, &mut h, 0, 0, 800, 600));
    assert_eq!(
        h.events,
        two_copies(
            CopyOperation::Save,
            BufferCopy {
                source: [0; 2],
                size: [800, 600],
                destination: [0; 2]
            }
        )
    );
}

#[test]
fn partial_save_b2_and_missing_service_or_buffers_reject() {
    let mut h = BufferHost::default();
    let mut globals = BackingStore::default();
    assert!(globals.create(&mut h));
    h.events.clear();
    let mut r = Render3d::new(&mut Host::default());
    r.set_viewport_size(800, 600);
    r.state.allows_partial_copy = false;
    assert!(!r.save_backing_store(&globals, &mut h, 10, 10, 50, 50));
    assert!(h.events.is_empty());
    assert!(r.save_backing_store(&globals, &mut h, 0, 0, 800, 600));
    h.events.clear();
    h.service = false;
    assert!(!r.save_backing_store(&globals, &mut h, 0, 0, 800, 600));
    assert!(!r.restore_backing_store(&globals, &mut h));
    h.service = true;
    let empty = BackingStore::default();
    assert!(!r.save_backing_store(&empty, &mut h, 0, 0, 800, 600));
    assert!(!r.restore_backing_store(&empty, &mut h));
    assert!(h.events.is_empty());
}

#[test]
fn restore_b3_and_save_b4_shake_coordinates() {
    let mut h = BufferHost::default();
    let mut globals = BackingStore::default();
    assert!(globals.create(&mut h));
    h.events.clear();
    let mut r = Render3d::new(&mut Host::default());
    r.set_viewport_size(800, 600);
    r.state.shake_offsets = [3, -2];
    assert!(r.restore_backing_store(&globals, &mut h));
    assert_eq!(
        h.events,
        two_copies(
            CopyOperation::Restore,
            BufferCopy {
                source: [0, 2],
                size: [797, 598],
                destination: [3, 0]
            }
        )
    );
    h.events.clear();
    r.state.shake_offsets = [5, 0];
    assert!(r.save_backing_store(&globals, &mut h, 2, 0, 10, 600));
    assert_eq!(
        h.events,
        two_copies(
            CopyOperation::Save,
            BufferCopy {
                source: [0, 0],
                size: [5, 600],
                destination: [7, 0]
            }
        )
    );
}

#[test]
fn save_shake_clips_high_edges_and_empty_clipped_region_succeeds_before_service_check() {
    let mut h = BufferHost::default();
    let mut globals = BackingStore::default();
    assert!(globals.create(&mut h));
    h.events.clear();
    let mut r = Render3d::new(&mut Host::default());
    r.set_viewport_size(800, 600);
    r.state.shake_offsets = [-5, -2];
    assert!(r.save_backing_store(&globals, &mut h, 790, 590, 10, 10));
    assert_eq!(
        h.events,
        two_copies(
            CopyOperation::Save,
            BufferCopy {
                source: [795, 592],
                size: [5, 8],
                destination: [790, 590]
            }
        )
    );
    h.events.clear();
    h.service = false;
    assert!(r.save_backing_store(&BackingStore::default(), &mut h, 799, 0, 10, 10));
    assert!(h.events.is_empty());
}

#[test]
fn destroy_service_guard_and_global_enable_switch() {
    let mut h = BufferHost {
        service: false,
        ..BufferHost::default()
    };
    let mut globals = BackingStore::default();
    assert!(!globals.create(&mut h));
    assert!(!globals.destroy(&mut h));
    assert!(h.events.is_empty());
    let mut r = Render3d::new(&mut Host::default());
    r.set_backing_store_enabled(&mut globals, true);
    assert!(!r.backing_store_enabled(&globals));
    h.service = true;
    assert!(globals.create(&mut h));
    r.set_backing_store_enabled(&mut globals, true);
    let other = Render3d::new(&mut Host::default());
    assert!(other.backing_store_enabled(&globals));
    h.events.clear();
    h.service = false;
    assert!(!globals.destroy(&mut h));
    assert_eq!(globals.buffers(), [Some(Handle(10)), Some(Handle(11))]);
    h.service = true;
    assert!(globals.destroy(&mut h));
    assert_eq!(h.events, [BufferEvent::Free(Handle(10)), BufferEvent::Free(Handle(11))]);
    assert_eq!(globals.buffers(), [None; 2]);
    assert!(other.backing_store_enabled(&globals)); // Destroy does not change the switch.
}

#[test]
fn reevaluation_updates_static_bits_and_marks_empty_quadrants_too() {
    let mut r = Render3d::new(&mut Host::default());
    let mut grid = QuadGrid::new(128.0, 32.0);
    for (model, flags) in [(1, 0x840), (2, 0x948)] {
        let mut e = GridElement::new(Handle(model));
        e.flags = flags;
        assert!(grid.add_element(e, ContainerRect::new(1.0, 1.0, 2.0, 2.0), 4));
    }
    r.grid = Some(grid);
    r.state.draw_zoom = 4;
    r.reevaluate_backing_store_object_flags(BackingLimits {
        terrain: 5,
        buildings_props: 5,
        flora: 5,
    });
    let grid = r.grid.as_ref().unwrap();
    assert_eq!(grid.element(Handle(1)).unwrap().flags, 0x848);
    assert_eq!(grid.element(Handle(2)).unwrap().flags, 0x940);
    assert_eq!(grid.quadrant(0, 0).0, 5);
    assert_eq!(grid.quadrant(3, 3), (1, &[][..]));
    assert!(!r.state.full_redraw);
    assert!(r.dirty_queue().nodes().is_empty());
}
