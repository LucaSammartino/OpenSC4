// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed persistence boundary: reference resolution belongs to the host stream.
use super::{Handle, ModelMaker, Port};

pub trait ReadRecord {
    fn u16(&mut self) -> u16;
    fn u32(&mut self) -> u32;
    fn i32(&mut self) -> i32;
    fn f32(&mut self) -> f32;
    /// Returns the reference acquired by the stream for this field.
    fn reference(&mut self, iid: u32) -> Option<Handle>;
    fn has_error(&self) -> bool;
}

pub trait WriteRecord {
    fn u16(&mut self, value: u16);
    fn u32(&mut self, value: u32);
    fn i32(&mut self, value: i32);
    fn f32(&mut self, value: f32);
    fn reference(&mut self, iid: u32, value: Option<Handle>);
    fn has_error(&self) -> bool;
}

const REFERENCES: [(Port, u32); 5] = [
    (Port::Renderer, 0x681B_D529),
    (Port::OccupantManager, 0xC6BB_8334),
    (Port::LotManager, 0x6819_05BD),
    (Port::Development, 0x4818_E5D2),
    (Port::Terrain, 0xC70B_D37E),
];

impl ModelMaker {
    /// §4: no terrain or city service is consulted, even before Init.
    pub fn read(&mut self, stream: &mut impl ReadRecord) -> bool {
        self.read_from_save = true;
        if stream.u16() != 2 {
            return false;
        }
        self.read_reference(stream, REFERENCES[0]);
        self.saved.zoom = stream.i32();
        self.saved.rotation = stream.i32();
        for value in &mut self.saved.cells {
            *value = stream.i32();
        }
        self.saved.city.x0 = stream.i32();
        self.saved.city.z0 = stream.i32();
        self.saved.city.x1 = stream.i32();
        self.saved.city.z1 = stream.i32();
        self.saved.word = stream.u32();
        for reference in &REFERENCES[1..] {
            self.read_reference(stream, *reference);
        }
        for value in &mut self.saved.camera {
            *value = stream.f32();
        }
        for value in &mut self.saved.previous_view {
            *value = stream.i32();
        }
        for value in &mut self.saved.target_view {
            *value = stream.i32();
        }
        !stream.has_error()
    }

    fn read_reference(&mut self, stream: &mut impl ReadRecord, (port, iid): (Port, u32)) {
        if let Some(handle) = stream.reference(iid) {
            self.ports.insert(port, handle);
        } else {
            self.ports.remove(&port);
        }
    }

    pub fn write(&self, stream: &mut impl WriteRecord) -> bool {
        stream.u16(2);
        stream.reference(REFERENCES[0].1, self.port(Port::Renderer));
        for value in [
            self.saved.zoom,
            self.saved.rotation,
            self.saved.cells[0],
            self.saved.cells[1],
            self.saved.city.x0,
            self.saved.city.z0,
            self.saved.city.x1,
            self.saved.city.z1,
        ] {
            stream.i32(value);
        }
        stream.u32(self.saved.word);
        for &(port, iid) in &REFERENCES[1..] {
            stream.reference(iid, self.port(port));
        }
        for &value in &self.saved.camera {
            stream.f32(value);
        }
        for &value in self.saved.previous_view.iter().chain(&self.saved.target_view) {
            stream.i32(value);
        }
        !stream.has_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_maker::{Rect, SavedState};
    use std::collections::VecDeque;

    #[derive(Clone, Debug, PartialEq)]
    enum Word {
        U16(u16),
        U32(u32),
        I32(i32),
        F32(f32),
        Ref(u32, Option<Handle>),
    }

    #[derive(Default)]
    struct Stream {
        words: VecDeque<Word>,
        error: bool,
    }

    impl WriteRecord for Stream {
        fn u16(&mut self, v: u16) {
            self.words.push_back(Word::U16(v));
        }
        fn u32(&mut self, v: u32) {
            self.words.push_back(Word::U32(v));
        }
        fn i32(&mut self, v: i32) {
            self.words.push_back(Word::I32(v));
        }
        fn f32(&mut self, v: f32) {
            self.words.push_back(Word::F32(v));
        }
        fn reference(&mut self, iid: u32, v: Option<Handle>) {
            self.words.push_back(Word::Ref(iid, v));
        }
        fn has_error(&self) -> bool {
            self.error
        }
    }

    impl ReadRecord for Stream {
        fn u16(&mut self) -> u16 {
            match self.words.pop_front().unwrap() {
                Word::U16(v) => v,
                _ => panic!("u16"),
            }
        }
        fn u32(&mut self) -> u32 {
            match self.words.pop_front().unwrap() {
                Word::U32(v) => v,
                _ => panic!("u32"),
            }
        }
        fn i32(&mut self) -> i32 {
            match self.words.pop_front().unwrap() {
                Word::I32(v) => v,
                _ => panic!("i32"),
            }
        }
        fn f32(&mut self) -> f32 {
            match self.words.pop_front().unwrap() {
                Word::F32(v) => v,
                _ => panic!("f32"),
            }
        }
        fn reference(&mut self, iid: u32) -> Option<Handle> {
            match self.words.pop_front().unwrap() {
                Word::Ref(expected, v) => {
                    assert_eq!(iid, expected);
                    v
                }
                _ => panic!("ref"),
            }
        }
        fn has_error(&self) -> bool {
            self.error
        }
    }

    #[test]
    fn m3_bad_version_marks_read_and_consumes_nothing_else() {
        let mut stream = Stream {
            words: [Word::U16(1), Word::U32(123)].into(),
            error: false,
        };
        let mut maker = ModelMaker::default();
        assert!(!maker.read(&mut stream));
        assert!(maker.read_from_save);
        assert_eq!(stream.words, [Word::U32(123)]);
        assert_eq!(maker.saved, SavedState::default());
    }

    #[test]
    fn m4_record_order_iids_round_trip_and_errors() {
        let mut maker = ModelMaker::default();
        maker.saved = SavedState {
            zoom: 3,
            rotation: 2,
            cells: [64, 32],
            city: Rect::new(0, 0, 63, 31),
            word: 0x1234,
            camera: [1.25, -3.0, 7.0, 8.0],
            previous_view: [4, 1],
            target_view: [3, 2],
        };
        for (index, &(port, _)) in REFERENCES.iter().enumerate() {
            maker.ports.insert(port, 100 + index as u64);
        }
        let mut stream = Stream::default();
        assert!(maker.write(&mut stream));
        assert_eq!(stream.words.len(), 23);
        assert_eq!(stream.words.front(), Some(&Word::U16(2)));
        assert_eq!(stream.words.get(10), Some(&Word::U32(0x1234)));
        let mut fresh = ModelMaker::default();
        assert!(fresh.read(&mut stream));
        assert_eq!(maker.saved, fresh.saved);
        assert_eq!(maker.ports, fresh.ports);
        assert!(!fresh.initialized);
        stream.error = true;
        assert!(!maker.write(&mut stream));
        assert!(!fresh.read(&mut stream));
    }
}
