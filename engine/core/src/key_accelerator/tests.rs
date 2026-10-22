// SPDX-License-Identifier: GPL-3.0-or-later
// Expected fixtures written before the owner implementation, from widget-toolkit §13.
use super::*;
use std::sync::{Arc, Mutex};

const BOUND: Message = Message {
    type_id: 0x7654_3210,
    data: [0x1122_3344, 0x89ab_cdef, 0xfedc_ba98],
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Service(u32),
    AddRef(u32),
    Release(u32),
    Send(u32, Message),
}

struct Host {
    server: Option<u32>,
    events: Arc<Mutex<Vec<Event>>>,
}

impl MessageHost for Host {
    type Server = u32;
    fn find_service(&mut self, service: u32) -> Option<u32> {
        self.events.lock().unwrap().push(Event::Service(service));
        self.server
    }
    fn add_ref(&mut self, server: u32) {
        self.events.lock().unwrap().push(Event::AddRef(server));
    }
    fn release(&mut self, server: u32) {
        self.events.lock().unwrap().push(Event::Release(server));
    }
    fn message_send(&mut self, server: u32, message: Message) {
        self.events.lock().unwrap().push(Event::Send(server, message));
    }
}

fn owner(server: Option<u32>) -> (KeyAccelerator<Host>, Arc<Mutex<Vec<Event>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let host = Host { server, events: events.clone() };
    // Explicit caller-supplied live count, not a claim about the native constructor.
    (KeyAccelerator::new(host, std::num::NonZeroU32::new(1).unwrap()), events)
}

#[test]
fn independent_key_and_modifier_vectors() {
    // (stored code, vk, modifiers, down, hit). Expected codes are literals,
    // never calculated by a copy of the implementation's packing expression.
    let vectors = [
        (0x0000_004e, 0x4e, 0, true, true),
        (0x0000_004e, 0x4f, 0, true, false),
        (0x0000_004e, 0x4e, 0, false, false),
        (0x1000_004e, 0x4e, 0, false, true),
        (0x1000_004e, 0x4e, 0, true, false),
        (0x0000_004e, 0x4e, 0x10, true, true),
        (0x0000_004e, 0x4e, 0x20, true, true),
        (0x0000_004e, 0x4e, 0x40, true, true),
        (0x0000_004e, 0x4e, 0x70, true, true),
        (0x0001_004e, 0x4e, 1, true, true),
        (0x0002_004e, 0x4e, 2, true, true),
        (0x0004_004e, 0x4e, 4, true, true),
        (0x0008_004e, 0x4e, 8, true, true),
        (0x0080_004e, 0x4e, 0x80, true, true),
        (0x0000_004e, 0x4e, 8, true, false),
        (0x0100_004e, 0x4e, 0x100, true, true),
        (0x8000_004e, 0x4e, 0x8000, true, true),
        (0x1000_004e, 0x4e, 0x1000, true, true),
        (0x1000_004e, 0x4e, 0x1000, false, true),
        (0x0000_004e, 0x4e, 0x10000, true, true),
        (0xff8f_004e, 0x4e, 0xffff_fffe, true, false),
        (0xff8e_004e, 0x4e, 0xffff_fffe, true, true),
        (0xff8f_004e, 0x4e, 0xffff_ffff, true, false),
        (0x0000_0010, 0x10, 0, true, false),
        (0x0000_0011, 0x11, 0, true, false),
        (0x1000_0012, 0x12, 0, false, false),
        (0x0000_0000, 0, 0, true, true),
        (0x0000_ffff, 0xffff, 0, true, true),
    ];
    for (code, vk, modifiers, down, hit) in vectors {
        let (mut accelerator, _) = owner(None);
        assert!(accelerator.register(code, BOUND));
        assert_eq!(accelerator.test_key(vk, modifiers, down), hit.then_some(BOUND),
            "code={code:08x} vk={vk:x} modifiers={modifiers:x} down={down}");
        assert_eq!(accelerator.test_key_type(vk, modifiers, down), hit.then_some(BOUND.type_id));
    }
}

#[test]
fn presence_masks_only_stored_code_and_only_published_bits() {
    let (mut accelerator, _) = owner(None);
    accelerator.register(0x100f_004e, BOUND);
    let vectors = [
        (0x100f_004e, true, true, true),
        (0x1008_004e, false, true, true),
        (0x000f_004e, true, false, true),
        (0x0008_004e, false, false, true),
        (0x0000_004e, false, false, false),
        (0x100f_004e, false, true, false),
        (0x100f_004e, true, false, false),
        (0x000f_004e, false, false, false),
        (0x0008_004f, false, false, false),
    ];
    for (probe, mods, up, expected) in vectors {
        assert_eq!(accelerator.is_present(probe, mods, up), expected,
            "probe={probe:08x} mods={mods} up={up}");
    }
}

#[test]
fn insertion_duplicates_removal_and_reinsertion_preserve_full_record() {
    let (mut accelerator, _) = owner(None);
    assert!(accelerator.register(0x4e, BOUND));
    assert!(accelerator.register_type(0x4e, 99));
    assert_eq!(accelerator.test_key(0x4e, 0, true), Some(BOUND));
    assert!(!accelerator.unregister(0x1000_004e));
    assert!(accelerator.unregister(0x4e));
    assert!(!accelerator.unregister(0x4e));
    assert_eq!(accelerator.test_key(0x4e, 0, true), None);
    assert!(accelerator.register_type(0x4e, 99));
    assert_eq!(accelerator.test_key(0x4e, 0, true), Some(Message { type_id: 99, data: [0; 3] }));
}

#[test]
fn enumeration_unsigned_order_capacity_and_total() {
    let (mut accelerator, _) = owner(None);
    for code in [u32::MAX, 0x8000_0000, 0x1000_004e, 0, 0x4e] {
        accelerator.register(code, BOUND);
    }
    accelerator.register_type(1, 99);
    for capacity in 0..=6 {
        let mut output = [0xdead_beef; 6];
        let result = accelerator.keys_for_type(BOUND.type_id, &mut output[..capacity]);
        assert_eq!(result, KeyCounts { written: capacity.min(5), total: 5 });
        let expected = [0, 0x4e, 0x1000_004e, 0x8000_0000, u32::MAX];
        assert_eq!(&output[..result.written], &expected[..result.written]);
        assert!(output[result.written..].iter().all(|&v| v == 0xdead_beef));
    }
    let mut output = [0xdead_beef; 2];
    assert_eq!(accelerator.keys_for_type(0, &mut output), KeyCounts { written: 0, total: 0 });
    assert_eq!(output, [0xdead_beef; 2]);
    accelerator.unregister(0);
    assert_eq!(accelerator.keys_for_type(BOUND.type_id, &mut output), KeyCounts { written: 2, total: 4 });
    assert_eq!(output, [0x4e, 0x1000_004e]);
}

#[test]
fn keyboard_message_types_and_all_four_dwords() {
    let (mut accelerator, _) = owner(None);
    accelerator.register(0x4e, BOUND);
    accelerator.register_type(0x1000_004e, 44);
    for (type_id, expected) in [(0, None), (4, None), (5, Some(BOUND)),
        (6, Some(Message { type_id: 44, data: [0; 3] })), (7, None), (u32::MAX, None)] {
        let input = Message { type_id, data: [0x4e, 0x70, 0xffff_ffff] };
        assert_eq!(accelerator.test_message(input), expected);
        assert_eq!(accelerator.test_message_type(input), expected.map(|m| m.type_id));
    }
    assert_eq!(std::mem::size_of::<Message>(), 16);
    assert_eq!(std::mem::align_of::<Message>(), 4);
}

#[test]
fn uninitialized_is_not_disabled_and_dispatch_is_synchronous() {
    let (mut accelerator, events) = owner(Some(7));
    accelerator.register(0x4e, BOUND);
    let input = Message { type_id: 5, data: [0x4e, 0, 0] };
    assert!(accelerator.process(input));
    assert!(events.lock().unwrap().is_empty());
    assert!(accelerator.init());
    assert!(accelerator.init());
    assert!(accelerator.process(input));
    assert_eq!(*events.lock().unwrap(), vec![Event::Service(0xc108_5722), Event::AddRef(7), Event::Send(7, BOUND)]);
    assert!(!accelerator.process(Message { type_id: 4, ..input }));
    assert!(!accelerator.process(Message { data: [0x4e, u32::MAX, 0], ..input }));
    assert_eq!(events.lock().unwrap().len(), 3);
    assert!(accelerator.shutdown());
    assert!(accelerator.shutdown());
    assert!(accelerator.process(input));
    assert_eq!(accelerator.test_message(input), Some(BOUND));
    assert_eq!(*events.lock().unwrap(), vec![Event::Service(0xc108_5722), Event::AddRef(7), Event::Send(7, BOUND), Event::Release(7)]);
}

#[test]
fn reinitialize_and_final_release_cleanup_once() {
    let (mut accelerator, events) = owner(Some(7));
    accelerator.register(0x4e, BOUND);
    assert_eq!(accelerator.add_ref(), Some(2));
    accelerator.init();
    assert_eq!(accelerator.release(), Some(1));
    assert!(accelerator.is_initialized());
    accelerator.shutdown();
    accelerator.init();
    assert_eq!(accelerator.test_key(0x4e, 0, true), Some(BOUND));
    assert_eq!(accelerator.release(), Some(0));
    assert!(accelerator.is_destroyed());
    assert!(!accelerator.is_initialized());
    assert_eq!(accelerator.reference_count(), 0);
    assert_eq!(accelerator.test_key(0x4e, 0, true), None);
    assert_eq!(accelerator.add_ref(), None);
    assert_eq!(accelerator.release(), None);
    drop(accelerator);
    assert_eq!(*events.lock().unwrap(), vec![Event::Service(0xc108_5722), Event::AddRef(7), Event::Release(7), Event::Service(0xc108_5722), Event::AddRef(7), Event::Release(7)]);
}

#[test]
fn rust_drop_releases_only_reference_it_acquired() {
    for initialized in [false, true] {
        let (mut accelerator, events) = owner(Some(7));
        accelerator.register(0x4e, BOUND);
        if initialized { accelerator.init(); }
        drop(accelerator);
        let expected = if initialized {
            vec![Event::Service(0xc108_5722), Event::AddRef(7), Event::Release(7)]
        } else { vec![] };
        assert_eq!(*events.lock().unwrap(), expected);
    }
}

#[test]
fn published_interface_membership_only() {
    assert!(KeyAccelerator::<Host>::supports_interface(1));
    assert!(KeyAccelerator::<Host>::supports_interface(0xe2c1_b3c4));
    assert!(!KeyAccelerator::<Host>::supports_interface(0));
    assert!(!KeyAccelerator::<Host>::supports_interface(0x42e9_67be));
    assert!(!KeyAccelerator::<Host>::supports_interface(u32::MAX));
}
