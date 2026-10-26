// SPDX-License-Identifier: GPL-3.0-or-later
//! automata.md §§6.4.1–6.4.4 (AT29–AT33), independently framed worked cases.
use fc4_core::automata::*;
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue};
use std::cell::RefCell;

fn reference(token: u64, class_id: u32) -> RefValue {
    RefValue::Host { token, class_id }
}

struct Tokens;
impl RefResolver for Tokens {
    fn resolve(&mut self, class: u32, id: u32) -> Option<RefValue> {
        Some(reference(id as u64, class))
    }
}

struct Output(ByteWriter);
impl OutStream for Output {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, v: &RefValue) -> bool {
        match v {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            _ => panic!("fixture uses host references"),
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
}
fn encode(write: impl FnOnce(&mut dyn OutStream) -> bool) -> Vec<u8> {
    let mut out = Output(ByteWriter::new());
    assert!(write(&mut out));
    out.0.into_bytes()
}

struct Input<'a> {
    inner: ByteReader<'a>,
    iids: Vec<u32>,
}
impl InStream for Input<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.inner.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.inner.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.inner.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.inner.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.inner.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.inner.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.inner.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.inner.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.inner.get_ref()
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        self.iids.push(iid);
        self.inner.get_ref_as(iid)
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}
fn input<'a>(bytes: &'a [u8], tokens: &'a mut Tokens) -> Input<'a> {
    Input {
        inner: ByteReader::with_resolver(bytes, tokens),
        iids: vec![],
    }
}

#[derive(Default)]
struct Refs(RefCell<Vec<(&'static str, u64)>>);
impl DerivedRecordRefs for Refs {
    fn add_ref(&self, r: &RefValue) {
        if let RefValue::Host { token, .. } = r {
            self.0.borrow_mut().push(("add", *token));
        }
    }
    fn release(&self, r: &RefValue) {
        if let RefValue::Host { token, .. } = r {
            self.0.borrow_mut().push(("release", *token));
        }
    }
}

fn vehicle() -> VehicleRecord {
    let mut v = VehicleRecord::default();
    v.traffic.base.controller = reference(11, CLSID_GENERATOR);
    v.traffic.base.leader = reference(12, CLSID_PEDESTRIAN);
    v.traffic.base.callbacks.push(reference(13, 0x1234));
    v.traffic.automaton = reference(14, CLSID_PEDESTRIAN);
    v.traffic.network = reference(15, 0xC9C0_5C6E);
    v.path.push((0x1234, -3, 9));
    v.v = [4.0, 5.0, 6.0];
    v.f = 7.0;
    v
}
fn prefix(bytes: &[u8], words: &[u16]) {
    let expected: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    assert_eq!(&bytes[..expected.len()], expected);
}
const INHERITED_IIDS: [u32; 5] = [0x6A18_57F8, 0xA9B4_0F05, 0x4522_94AA, 0xA9B4_0F05, 0xA821_EF94];

#[test]
fn at29_carjacked_and_limo_full_vehicle_wrappers_and_failure_boundaries() {
    let v = vehicle();
    let raw = encode(|out| v.write(out));
    let bytes = encode(|out| out.put_u16(1) && out.put_bytes(&raw) && out.put_u32(0xDEAD_BEEF));
    for _class in [CLSID_CARJACKED, CLSID_LIMO] {
        let mut r = VehicleWrapperRecord::default();
        let mut tokens = Tokens;
        let mut inp = input(&bytes, &mut tokens);
        assert!(r.read(&mut inp));
        assert_eq!(inp.inner.position(), raw.len() + 2);
        assert_eq!(inp.get_u32(), Some(0xDEAD_BEEF));
        assert_eq!(inp.iids, INHERITED_IIDS);
        let output = encode(|out| r.write(out));
        prefix(&output, &[1, 2, 2, 8, 4]);
        assert_eq!(output.len(), raw.len() + 2);
        assert_eq!(output, bytes[..bytes.len() - 4]);
    }
    for (offset, word, consumed) in [(0, 0u16, 2), (0, 2, 2), (2, 3, 4)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
        let mut inp = ByteReader::new(&bad);
        assert!(!VehicleWrapperRecord::default().read(&mut inp));
        assert_eq!(inp.position(), consumed);
    }
}

fn emergency_bytes(v: &[u8]) -> Vec<u8> {
    encode(|out| {
        out.put_u16(2);
        out.put_bytes(v);
        for x in [1.0, 2.0, 3.0] {
            out.put_f32(x);
        }
        out.put_u32(0);
        out.put_u8(2);
        out.put_u16(0x1234);
        out.put_u32(3)
    })
}

#[test]
fn at30_at33_ambulance_and_fire_truck_layout_iids_and_changed_crew() {
    let v = encode(|out| vehicle().write(out));
    let e = emergency_bytes(&v);
    assert_eq!(e.len(), v.len() + 25);
    let ambulance = encode(|out| out.put_u16(1) && out.put_bytes(&e));
    assert_eq!(ambulance.len(), v.len() + 27);
    prefix(&ambulance, &[1, 2, 2, 2, 8, 4]);
    let mut tokens = Tokens;
    let mut inp = input(&ambulance, &mut tokens);
    let mut r = AmbulanceRecord::default();
    assert!(r.read(&mut inp));
    assert_eq!(inp.inner.remaining(), 0);
    assert_eq!(&inp.iids[..5], &INHERITED_IIDS);
    assert_eq!(inp.iids[5], IID_OCCUPANT);
    assert_eq!(
        (r.emergency.v, r.emergency.b, r.emergency.w, r.emergency.u),
        ([1.0, 2.0, 3.0], 2, 0x1234, 3)
    );
    assert_eq!(encode(|out| r.write(out)), ambulance);
    for (crew, extra) in [(reference(37, 0x4A22_2E4D), 35), (RefValue::Null, 31)] {
        let bytes = encode(|out| out.put_u16(2) && out.put_bytes(&e) && out.put_ref(&crew));
        assert_eq!(bytes.len(), v.len() + extra);
        prefix(&bytes, &[2, 2, 2, 2, 8, 4]);
        let mut r = FireTruckRecord {
            crew: reference(99, 0x4A22_2E4D),
            ..FireTruckRecord::default()
        };
        let refs = Refs::default();
        let mut tokens = Tokens;
        let mut inp = input(&bytes, &mut tokens);
        assert!(r.read(&mut inp, &refs));
        assert!(same_ref(&r.crew, &crew));
        assert_eq!(inp.iids[6], IID_AUTOMATON);
        assert_eq!(inp.inner.position(), bytes.len());
        assert_eq!(encode(|out| r.write(out)), bytes);
        assert_eq!(
            *refs.0.borrow(),
            if crew.is_null() {
                vec![("release", 99)]
            } else {
                vec![("add", 37), ("release", 99)]
            }
        );
        refs.0.borrow_mut().clear();
        let mut tokens = Tokens;
        assert!(r.read(&mut input(&bytes, &mut tokens), &refs));
        assert!(
            refs.0.borrow().is_empty(),
            "identical crew has no explicit ownership operations"
        );
    }
    for (offset, word, consumed) in [(0, 2u16, 2), (2, 1, 4), (4, 3, 6)] {
        let mut bad = ambulance.clone();
        bad[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
        let mut inp = ByteReader::new(&bad);
        assert!(!AmbulanceRecord::default().read(&mut inp));
        assert_eq!(inp.position(), consumed);
    }
    let refs = Refs::default();
    let mut fire = FireTruckRecord {
        crew: reference(99, 0x4A22_2E4D),
        ..FireTruckRecord::default()
    };
    let mut inp = ByteReader::new(&[2, 0, 1, 0]);
    assert!(!fire.read(&mut inp, &refs));
    assert_eq!(inp.position(), 4);
    assert!(same_ref(&fire.crew, &reference(99, 0x4A22_2E4D)));
    assert!(refs.0.borrow().is_empty());
    // Emergency's inherited success survives a short tail, with no dispatch side effects.
    let truncated = &e[..e.len() - 1];
    let mut tokens = Tokens;
    let mut inp = input(truncated, &mut tokens);
    let mut emergency = EmergencyRecord::default();
    emergency.u = 77;
    assert!(emergency.read_into(&mut inp));
    assert_ne!(inp.error(), 0);
    assert_eq!(emergency.u, 77);
}

#[test]
fn at31_tank_signed_bytes_last_match_historical_versions_and_error_quirk() {
    let tank = TankRecord {
        vehicle: vehicle(),
        saved_indices: [7, 9],
    };
    let v = encode(|out| tank.vehicle.write(out));
    let bytes = encode(|out| tank.write_with_models(out, &[10, 20, 30, 20], [20, 30]));
    assert_eq!(bytes.len(), v.len() + 4);
    assert_eq!(&bytes[bytes.len() - 2..], &[3, 2]);
    let absent = encode(|out| tank.write_with_models(out, &[10], [20, 30]));
    assert_eq!(&absent[absent.len() - 2..], &[0xFF, 0xFF]);
    let mut models = vec![0; 258];
    models[257] = 20;
    models[128] = 30;
    let wrapped = encode(|out| tank.write_with_models(out, &models, [20, 30]));
    assert_eq!(&wrapped[wrapped.len() - 2..], &[1, 128]);
    for version in [0, 1] {
        let old = encode(|out| out.put_u16(version) && out.put_bytes(&v) && out.put_u8(0xCC));
        let mut r = tank.clone();
        let mut tokens = Tokens;
        let mut inp = input(&old, &mut tokens);
        assert!(r.read(&mut inp));
        assert_eq!(r.saved_indices, [7, 9]);
        assert_eq!(inp.inner.position(), v.len() + 2);
        assert_eq!(inp.get_u8(), Some(0xCC));
    }
    let mut inp = ByteReader::new(&[2, 0, 3, 0, 0xAA, 0xBB]);
    let mut r = tank.clone();
    assert!(r.read(&mut inp));
    assert_eq!(inp.position(), 4);
    assert_eq!(inp.get_u16(), Some(0xBBAA));
    assert_eq!(r.saved_indices, [7, 9]);
    let mut inp = ByteReader::new(&[3, 0, 3, 0]);
    assert!(!r.read(&mut inp));
    assert_eq!(inp.position(), 2);
    let mut inp = ByteReader::new(&[2, 0]);
    assert!(!r.read(&mut inp));
    assert_ne!(inp.error(), 0);
    let mut tokens = Tokens;
    let mut inp = input(&wrapped, &mut tokens);
    assert!(r.read(&mut inp));
    assert_eq!(r.saved_indices, [1, -128]);
    assert_eq!(encode(|out| r.write(out)), wrapped);
}

#[test]
fn at32_mass_transit_train_complete_nesting_append_nulls_and_tail() {
    let v = encode(|out| vehicle().write(out));
    let bytes = encode(|out| {
        for word in [1, 2, 2] {
            out.put_u16(word);
        }
        out.put_bytes(&v);
        out.put_u32(3);
        out.put_ref(&reference(21, 0x4A41_6254));
        out.put_u32(0);
        out.put_ref(&reference(22, 0x4A41_6254));
        out.put_u32(0x1122_3344);
        for x in [1.0, 2.0, 3.0] {
            out.put_f32(x);
        }
        out.put_u32(0);
        out.put_u32(0x5566_7788)
    });
    assert_eq!(bytes.len(), v.len() + 54);
    prefix(&bytes, &[1, 2, 2, 2, 2, 8, 4]);
    let mut r = MassTransitTrainRecord {
        cars: vec![reference(20, 0x4A41_6254)],
        network: reference(99, 0xC9C0_5C6E),
        ..MassTransitTrainRecord::default()
    };
    let refs = Refs::default();
    let mut tokens = Tokens;
    let mut inp = input(&bytes, &mut tokens);
    assert!(r.read(&mut inp, &refs));
    assert_eq!(inp.inner.remaining(), 0);
    assert_eq!(&inp.iids[..5], &INHERITED_IIDS);
    assert_eq!(
        &inp.iids[5..],
        &[0x4A41_6254, 0x4A41_6254, 0x4A41_6254, IID_NETWORK_OCCUPANT]
    );
    assert_eq!(r.cars.len(), 3);
    for (car, token) in r.cars.iter().zip([20, 21, 22]) {
        assert!(same_ref(car, &reference(token, 0x4A41_6254)));
    }
    assert!(r.network.is_null());
    assert_eq!(*refs.0.borrow(), vec![("release", 99)]);
    assert_eq!(
        (r.raw_2c0, r.vector_2d4, r.raw_2e4),
        (0x1122_3344, [1.0, 2.0, 3.0], 0x5566_7788)
    );
    for (offset, word) in [(0, 2u16), (2, 3), (4, 3), (6, 3)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
        let mut inp = ByteReader::new(&bad);
        let before = r.cars.len();
        assert!(!r.read(&mut inp, &refs));
        assert_eq!(inp.position(), offset + 2);
        assert_eq!(r.cars.len(), before);
    }
    let empty = MassTransitTrainRecord {
        vehicle: vehicle(),
        ..MassTransitTrainRecord::default()
    };
    assert_eq!(encode(|out| empty.write(out)).len(), v.len() + 34);
    // Tail helpers cannot replace a successful inherited result.
    let mut tokens = Tokens;
    let mut inp = input(&bytes[..bytes.len() - 1], &mut tokens);
    assert!(r.read(&mut inp, &refs));
    assert_ne!(inp.error(), 0);
}
