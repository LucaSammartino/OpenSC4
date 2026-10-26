// SPDX-License-Identifier: GPL-3.0-or-later
//! Primitive recording widget host. Only foreign return values enter the core;
//! all requested mutations, identities and ordering are produced by the core.
use super::super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub op: u32,
    pub object: Handle,
    pub slot: u32,
    pub arguments: Vec<Value>,
}

pub struct RecordingHost<F> {
    pub calls: Vec<Call>,
    outcome: F,
}

impl<F: FnMut(&Call) -> Value> RecordingHost<F> {
    pub fn new(outcome: F) -> Self {
        Self {
            calls: Vec::new(),
            outcome,
        }
    }

    fn call(&mut self, op: u32, object: Handle, slot: u32, arguments: Vec<Value>) -> Value {
        let call = Call {
            op,
            object,
            slot,
            arguments,
        };
        let value = (self.outcome)(&call);
        self.calls.push(call);
        value
    }
}

fn raw(v: Value) -> u32 {
    match v {
        Value::U32(n) => n,
        _ => 0,
    }
}
fn identity(v: Value) -> Handle {
    match v {
        Value::Handle(h) => h,
        _ => 0,
    }
}
fn optional(v: Value) -> Option<Handle> {
    let h = identity(v);
    (h != 0).then_some(h)
}

impl<F: FnMut(&Call) -> Value> Foreign for RecordingHost<F> {
    fn create(&mut self, class: u32, iid: u32) -> Option<Handle> {
        optional(self.call(1, 0, class, vec![Value::U32(iid)]))
    }
    fn query(&mut self, h: Handle, iid: u32) -> Option<Handle> {
        optional(self.call(2, h, iid, vec![]))
    }
    fn add_ref(&mut self, h: Handle) {
        self.call(3, h, 0, vec![]);
    }
    fn release(&mut self, h: Handle) -> u32 {
        raw(self.call(4, h, 0, vec![]))
    }
    fn vocabulary_add(&mut self, h: Handle, word: &str, id: u32) {
        self.call(5, h, id, vec![Value::Text(word.into())]);
    }
    fn read(&mut self, h: Handle, field: Read) -> Value {
        let (slot, args) = match field {
            Read::Id => (0xfc, vec![]),
            Read::Area => (0xbc, vec![]),
            Read::Fill => (0x1a0, vec![]),
            Read::Caption => (0x124, vec![]),
            Read::Flag(mask) => (0x10c, vec![Value::U32(mask)]),
            Read::Param(key) => (0x1c4, vec![Value::U32(key)]),
            Read::Parent => (0x30, vec![]),
            Read::LegacyControl => (0xc, vec![]),
        };
        self.call(6, h, slot, args)
    }
    fn write(&mut self, h: Handle, operation: Write) {
        let (slot, args) = match operation {
            Write::Id(id) => (0x100, vec![Value::U32(id)]),
            Write::Area(r) => (0xd8, vec![Value::Rect(r)]),
            Write::Move(x, y) => (0xe0, vec![Value::I32(x), Value::I32(y)]),
            Write::Size(w, h) => (0xd4, vec![Value::I32(w), Value::I32(h)]),
            Write::Fill(c) => (0x1ac, vec![Value::Colour(c)]),
            Write::Caption(s) => (0x128, vec![Value::Text(s)]),
            Write::Flag(mask, on) => (0x110, vec![Value::U32(mask), Value::U32(u32::from(on))]),
            Write::Param(key, value) => (0x1c8, vec![Value::U32(key), value]),
            Write::Init => (0x10, vec![]),
            Write::Shutdown => (0x14, vec![]),
            Write::ChildAdd(h) => (0x38, vec![Value::Handle(h)]),
            Write::ChildRemove(h) => (0x3c, vec![Value::Handle(h)]),
            Write::Front => (0x5c, vec![]),
        };
        self.call(7, h, slot, args);
    }
    fn widget(&mut self, h: Handle, slot: u32, args: Vec<Value>) -> Value {
        self.call(8, h, slot, args)
    }
    fn font_id(&mut self, name: &str) -> u32 {
        raw(self.call(9, 0, 0, vec![Value::Text(name.into())]))
    }
    fn font(&mut self, id: u32) -> Handle {
        identity(self.call(10, 0, id, vec![]))
    }
    fn colour(&mut self, rgb: [u8; 3]) -> u32 {
        raw(self.call(11, 0, 0, vec![Value::Colour(rgb)]))
    }
    fn colour_four(&mut self, rgb: [u8; 3], fourth: u32) -> u32 {
        raw(self.call(12, 0, fourth, vec![Value::Colour(rgb)]))
    }
    fn grid_colour(&mut self, h: Handle, rgb: [u8; 3]) -> u32 {
        raw(self.call(13, h, 0, vec![Value::Colour(rgb)]))
    }
    fn load_image(&mut self, group: u32, instance: u32) -> Option<Handle> {
        optional(self.call(14, 0, group, vec![Value::U32(instance)]))
    }
    fn load_string(&mut self, group: u32, instance: u32) -> Option<String> {
        match self.call(15, 0, group, vec![Value::U32(instance)]) {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
    fn image_rect(&mut self, h: Handle) -> Rect {
        match self.call(16, h, 0, vec![]) {
            Value::Rect(r) => r,
            _ => [0; 4],
        }
    }
    fn notify_cell(&mut self, h: Handle, col: i32, row: i32, descriptor: CellDescriptor) {
        self.call(
            17,
            h,
            0,
            vec![Value::I32(col), Value::I32(row), Value::Cell(descriptor)],
        );
    }
    fn notify_axis(&mut self, h: Handle, kind: Notifier, values: [i32; 3]) {
        self.call(
            18,
            h,
            0,
            vec![
                Value::Notifier(kind),
                Value::Words(values.map(|v| v as u32).to_vec()),
            ],
        );
    }
    fn winmgr_slot3(&mut self) -> Handle {
        identity(self.call(19, 0, 0xc, vec![]))
    }
    fn new_string(&mut self) -> Handle {
        identity(self.call(20, 0, 0, vec![]))
    }
    fn append_string(&mut self, h: Handle, text: &str) {
        self.call(21, h, 0, vec![Value::Text(text.into())]);
    }
    fn new_default_procedure(&mut self) -> Option<Handle> {
        optional(self.call(22, 0, 0, vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_control::observation::{replay_build, OwnerState};

    #[test]
    fn rejected_and_custom_builds_record_factory_geometry_flags_and_attach_order() {
        let before = OwnerState {
            refs: 2,
            control: Some(91),
            parent: Some(9),
            procedure: None,
            own: false,
        };
        let mut host = RecordingHost::new(|c: &Call| match c.op {
            1 => Value::Handle(42),
            4 => Value::U32(1),
            _ => Value::None,
        });
        assert_eq!(
            replay_build(before, Properties(&[]), &mut host),
            Ok((before, false))
        );
        assert!(host.calls.is_empty());
        let props = [
            Property {
                key: 0xffffffff,
                value: Some(Variant::Scalar(Elem::U32(0x478d1e6f))),
            },
            Property {
                key: 0x101,
                value: Some(Variant::string(b"(1,2,30,40)")),
            },
        ];
        let (after, value) = replay_build(before, Properties(&props), &mut host).unwrap();
        assert!(!value);
        assert_eq!(
            after,
            OwnerState {
                control: Some(42),
                parent: None,
                own: true,
                ..before
            }
        );
        assert_eq!(
            host.calls[0],
            Call {
                op: 1,
                object: 0,
                slot: 0x478d1e6f,
                arguments: vec![Value::U32(WIN_IID)]
            }
        );
        let slots: Vec<_> = host
            .calls
            .iter()
            .filter(|c| c.op == 7)
            .map(|c| c.slot)
            .collect();
        assert!(slots.ends_with(&[0x10, 0x38, 0x5c]));
        assert!(host
            .calls
            .iter()
            .any(|c| c.arguments == vec![Value::Rect([1, 2, 30, 40])]));
        assert_eq!(host.calls.iter().filter(|c| c.slot == 0x110).count(), 7);
    }
}
