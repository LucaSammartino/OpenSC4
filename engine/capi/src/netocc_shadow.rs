// SPDX-License-Identifier: GPL-3.0-or-later
//! Private record-only verification transport; no public DLL ABI exports.
//! References are capture object numbers, never native pointers or owned refs.
use fc4_core::automata::OccupantBase;
use fc4_core::network_occupants::{
    connection::*, identity::NetOccClass, power_line::*, power_pole::*, pylon::*, record::*, underground::*,
};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue, ResKey};

struct Numbers;
impl RefResolver for Numbers {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}
struct Output(ByteWriter);
macro_rules! put {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self, v: $ty) -> bool {
            self.0.$name(v)
        }
    };
}
impl OutStream for Output {
    put!(put_u8, u8);
    put!(put_u16, u16);
    put!(put_u32, u32);
    put!(put_u64, u64);
    put!(put_f32, f32);
    put!(put_f64, f64);
    put!(put_bytes, &[u8]);
    put!(put_str, &[u8]);
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            RefValue::Object(_) => {
                self.0.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
    fn set_error(&mut self) {
        self.0.set_error();
    }
}
// Numeric capture identities have no interface-view displacement or ownership.
struct Host;
impl References for Host {
    fn add_ref(&mut self, _: &RefValue) {}
    fn release(&mut self, _: &RefValue) {}
}
impl ConnectionRecordHost for Host {
    fn adjust_view(&mut self, r: RefValue, _: i32) -> RefValue {
        r
    }
}
impl PoleStaticHost for Host {
    fn acquire_resource_manager(&mut self) -> RefValue {
        RefValue::Null
    }
    fn acquire_texture_manager(&mut self, _: u32, _: u32) -> RefValue {
        RefValue::Null
    }
    fn register_shutdown(&mut self) {}
    fn control_exemplar(&mut self, _: ResKey) -> bool {
        false
    }
    fn float_property(&self, _: u32) -> Option<f32> {
        None
    }
    fn uint_property(&self, _: u32) -> Option<u32> {
        None
    }
    fn width_property(&self, _: u32) -> Option<[f32; 5]> {
        None
    }
    fn sint64_property(&self, _: u32) -> Option<i64> {
        None
    }
    fn pack(&mut self, _: RenderStateArgs) -> u64 {
        0
    }
}
impl PoleRecordHost for Host {
    fn exemplar_model_instance(&mut self, base: &OccupantBase) -> u32 {
        // network-occupants §4.6 selects by the exemplar resource key, not
        // the model property 0x27812821. The captured base retains that key.
        base.holder.parent_key().map_or(0, |key| key.instance)
    }
}

fn roundtrip(class: NetOccClass, input: &mut dyn InStream, out: &mut dyn OutStream) -> Option<bool> {
    use NetOccClass::*;
    let mut host = Host;
    Some(match class {
        Network | PreBuilt | Bridge | Tunnel => write_network_record(&read_network_record(input)?, out),
        Underground => write_underground_record(&read_underground_record(input)?, out),
        Pipe => {
            let (r, w) = read_pipe_record(input)?;
            write_pipe_record(&r, w, out)
        }
        Subway => write_subway_record(&read_subway_record(input)?, out),
        PowerLine => write_power_line_record(&read_power_line_record(input)?, out),
        Pylon => {
            let mut r = PylonRecord::new([0; 3]);
            r.read_into(input, &mut host) && r.write(out)
        }
        PowerPole => {
            let mut r = fc4_core::network_occupants::power_pole::PowerPole::new(PoleFields::default());
            let mut tables = AttachmentTables::default();
            let mut statics = PoleStatics::new(0, 0);
            r.read_into(&mut tables, &mut statics, input, &mut host).ok()? && r.write(&tables, out).ok()?
        }
        UndergroundConnection | PipeConnection | SubwayConnection => {
            let mut r = ConnectionRecord {
                base: OccupantBase::default(),
                saved_68: 0,
                walls: Default::default(),
                cell: (0, 0),
                heights: [0.0; 4],
                saved_170: 0.0,
                saved_178: 0,
                model_state: false,
                attached: RefValue::Null,
                linked: RefValue::Null,
                saved_240: 0,
                position: [0.0; 3],
                loaded: false,
            };
            r.read_into(input, &mut host) && r.write(out, &mut host)
        }
    })
}

// Independent raw-reference normalization. Never serialize the decoded network
// record to manufacture expected bytes: that would conceal codec divergences.
fn normalize(class: NetOccClass, bytes: &[u8]) -> Option<(Vec<u8>, u32)> {
    use NetOccClass::*;
    let mut numbers = Numbers;
    let mut r = ByteReader::with_resolver(bytes, &mut numbers);
    r.get_u16()?;
    match class {
        Pipe | Subway => {
            r.get_u16()?;
            r.get_u16()?;
        }
        Underground => {
            r.get_u16()?;
        }
        _ => {}
    }
    OccupantBase::read(&mut r)?;
    let mut fixups = 0;
    let mut expected = bytes.to_vec();
    let transform = match class {
        c if c.is_network_piece() => Some(r.position()),
        PowerPole => Some(r.position()),
        Pylon => {
            if !r.skip(7) {
                return None;
            }
            r.get_ref()?;
            Some(r.position())
        }
        _ => None,
    };
    let mut erase = None;
    if let Some(at) = transform {
        let mask = r.get_u8()?;
        if mask & 4 != 0 && !r.skip(36) {
            return None;
        }
        if mask & 1 != 0 && !r.skip(12) {
            return None;
        }
        if mask & 2 != 0 {
            let scale_at = r.position();
            let scale = r.get_f32()?;
            // NO45 permits this exact canonicalization only. Other mask
            // changes remain visible as mismatches.
            if mask == 7 && scale == 1.0 {
                expected[at] = 5;
                erase = Some(scale_at);
                fixups += 1;
            }
        }
    }
    if class.is_network_piece() {
        if !r.skip(96 + 10) || r.get_u16()? != 2 {
            return None;
        }
        let count = usize::from(r.get_u8()?);
        if !r.skip(5 + count * 5) {
            return None;
        }
        let triangles = r.get_u32()?;
        for _ in 0..triangles {
            r.get_u32()?;
            for _ in 0..3 {
                if !r.skip(23) {
                    return None;
                }
                let at = r.position();
                if r.get_u8()? != 255 {
                    expected[at] = 255;
                    fixups += 1;
                }
            }
        }
        if !r.skip(24) {
            return None;
        }
        let at = r.position();
        let flags = r.get_u32()?;
        if flags & 0xE000 != 0 {
            expected[at..at + 4].copy_from_slice(&(flags & !0xE000).to_le_bytes());
            fixups += 1;
        }
    }
    if let Some(at) = erase {
        expected.drain(at..at + 4);
    }
    Some((expected, fixups))
}

#[repr(C)]
#[derive(Default)]
struct ResultRow {
    status: u32,
    fixups: u32,
    first: u32,
    expected_len: u32,
    actual_len: u32,
    expected: u32,
    actual: u32,
}

fn compare(class: NetOccClass, bytes: &[u8]) -> ResultRow {
    let mut row = ResultRow::default();
    let Some((expected, fixups)) = normalize(class, bytes) else {
        row.status = 2;
        return row;
    };
    row.fixups = fixups;
    let mut numbers = Numbers;
    let mut input = ByteReader::with_resolver(bytes, &mut numbers);
    let mut output = Output(ByteWriter::new());
    if roundtrip(class, &mut input, &mut output) != Some(true)
        || input.remaining() != 0
        || input.error() != 0
        || output.error() != 0
    {
        row.status = 3;
        row.first = input.position() as u32;
        row.actual_len = output.0.len() as u32;
        row.expected_len = expected.len() as u32;
        return row;
    }
    let actual = output.0.bytes();
    row.expected_len = expected.len() as u32;
    row.actual_len = actual.len() as u32;
    if expected != actual {
        row.status = 1;
        let at = expected
            .iter()
            .zip(actual)
            .position(|(a, b)| a != b)
            .unwrap_or(expected.len().min(actual.len()));
        row.first = at as u32;
        row.expected = expected.get(at).map_or(256, |v| u32::from(*v));
        row.actual = actual.get(at).map_or(256, |v| u32::from(*v));
    }
    row
}

// SAFETY: the synchronous private plugin caller supplies a valid captured byte
// buffer and writable result. No pointer/reference crosses the call's lifetime.
#[no_mangle]
unsafe extern "C" fn fc4_netocc_shadow_compare(
    class_id: u32,
    data: *const u8,
    len: usize,
    result: *mut ResultRow,
) -> bool {
    let Some(class) = NetOccClass::from_class_id(class_id) else {
        return false;
    };
    if result.is_null() || data.is_null() || len == 0 || len > 64 * 1024 * 1024 {
        return false;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    unsafe {
        *result = compare(class, bytes);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pole_attachment_selection_uses_exemplar_key_not_model_property() {
        use fc4_core::property::PropertyHolder;
        use fc4_core::variant::{mem, Elem, Variant};
        use std::rc::Rc;

        for subtype in 1..=4 {
            let mut pole = PowerPole::new(PoleFields::default());
            pole.base.holder.parent = Some((
                ResKey {
                    type_id: 0x6534_284A,
                    group: 1,
                    instance: 0x1234_0000 | (subtype << 8),
                },
                Rc::new(PropertyHolder::new()),
            ));
            pole.base.holder.own.add(
                0x2781_2821,
                Variant::Array {
                    elem: mem::UINT32,
                    items: vec![Elem::U32(0x5AD0_E817), Elem::U32(0), Elem::U32(0x0AD3_0000)],
                },
            );
            pole.select_connection_point_table(subtype as u8);
            let mut out = ByteWriter::new();
            assert_eq!(pole.write(&AttachmentTables::default(), &mut out), Ok(true));
            assert_eq!(compare(NetOccClass::PowerPole, out.bytes()).status, 0);
        }
        // A missing key must not silently select a default attachment table.
        assert_eq!(Host.exemplar_model_instance(&OccupantBase::default()), 0);
    }

    #[test]
    fn exact_normalization_and_unrelated_bytes_stay_visible() {
        let mut record = NetworkBaseRecord {
            network_flags: 0x0600_E000,
            triangles: vec![Triangle::default()],
            ..NetworkBaseRecord::default()
        };
        record.transform.mask = 7;
        let mut out = ByteWriter::new();
        assert!(write_network_record(&record, &mut out));
        let row = compare(NetOccClass::Network, out.bytes());
        assert_eq!((row.status, row.fixups), (0, 5));
        let (expected, _) = normalize(NetOccClass::Network, out.bytes()).unwrap();
        let mut changed = expected.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert_ne!(expected, changed);
        assert_eq!(compare(NetOccClass::Network, &changed).status, 0);
        // The mutation is retained, not normalized away.
        assert_eq!(normalize(NetOccClass::Network, &changed).unwrap().0, changed);
    }
    #[test]
    fn nested_records_and_truncation() {
        let record = UndergroundRecord {
            base: NetworkBaseRecord {
                network_flags: 0x8000,
                ..NetworkBaseRecord::default()
            },
            ..UndergroundRecord::default()
        };
        let mut out = ByteWriter::new();
        assert!(write_pipe_record(&record, 0x87654321, &mut out));
        assert_eq!(
            (
                compare(NetOccClass::Pipe, out.bytes()).status,
                compare(NetOccClass::Pipe, out.bytes()).fixups
            ),
            (0, 1)
        );
        for end in 0..out.len() {
            assert_ne!(compare(NetOccClass::Pipe, &out.bytes()[..end]).status, 0);
        }
    }
}
