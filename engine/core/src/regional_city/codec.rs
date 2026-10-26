// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ConnectionHandle, DealHandle, Lifecycle, PendingChange, RegionalCity};
use crate::neighbors::{flag, NeighborConnection, NeighborDeal};
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, ResKey};
use std::collections::BTreeMap;

pub const RECORD_KEY: ResKey = ResKey {
    type_id: 0xCA02_7EDB,
    group: 0xCA02_7EE1,
    instance: 0,
};
pub const VERSION_KEY: ResKey = ResKey {
    type_id: 0xEA3F_F4F5,
    group: 0xEA3F_F506,
    instance: 0xEA3F_F4F5,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordError {
    Truncated,
    Version,
    Limit,
    Element,
}

pub struct ReadContext<'a> {
    pub application_default_difficulty: u8,
    pub default_city_name: &'a [u8],
    pub localized_city_name: Option<&'a [u8]>,
    pub localized_mayor_name: Option<&'a [u8]>,
    pub map_tutorial: fn(i32) -> i32,
}

/// None represents an element that does not answer the serializable IID. The
/// count is still written from the original list, as required by records §2.1.
pub trait RecordElements<C = std::rc::Rc<NeighborConnection>, D = std::rc::Rc<NeighborDeal>> {
    fn connection(&mut self, c: &C, out: &mut dyn OutStream) -> Option<bool>;
    fn deal(&mut self, d: &D, out: &mut dyn OutStream) -> Option<bool>;
}
pub struct CoreElements;
/// Constructs owned elements through the host's interfaces. Failed/partial
/// decoding drops all elements already returned by this factory.
pub trait RecordReader<C, D> {
    fn connection(&mut self, inp: &mut dyn InStream) -> Option<C>;
    fn deal(&mut self, inp: &mut dyn InStream) -> Option<D>;
}
impl RecordReader<std::rc::Rc<NeighborConnection>, std::rc::Rc<NeighborDeal>> for CoreElements {
    fn connection(&mut self, inp: &mut dyn InStream) -> Option<std::rc::Rc<NeighborConnection>> {
        let c = NeighborConnection::new();
        c.read(inp).then_some(c)
    }
    fn deal(&mut self, inp: &mut dyn InStream) -> Option<std::rc::Rc<NeighborDeal>> {
        let d = NeighborDeal::new();
        d.read(inp).then_some(d)
    }
}
impl RecordElements for CoreElements {
    fn connection(&mut self, c: &std::rc::Rc<NeighborConnection>, out: &mut dyn OutStream) -> Option<bool> {
        Some(c.write(out))
    }
    fn deal(&mut self, d: &std::rc::Rc<NeighborDeal>, out: &mut dyn OutStream) -> Option<bool> {
        Some(d.write(out))
    }
}

fn count(inp: &mut dyn InStream, max: u32) -> Result<u32, RecordError> {
    let n = inp.get_u32().ok_or(RecordError::Truncated)?;
    if n > max {
        Err(RecordError::Limit)
    } else {
        Ok(n)
    }
}
fn map_read(inp: &mut dyn InStream, map: &mut BTreeMap<u32, i32>) -> Result<(), RecordError> {
    // RY7: assign by key without erasing entries absent from this stream.
    for _ in 0..count(inp, 0x2000_0000)? {
        map.insert(
            inp.get_u32().ok_or(RecordError::Truncated)?,
            inp.get_i32().ok_or(RecordError::Truncated)?,
        );
    }
    Ok(())
}
fn map_write(out: &mut dyn OutStream, map: &BTreeMap<u32, i32>) {
    out.put_u32(map.len() as u32);
    for (&t, &p) in map {
        out.put_u32(t);
        out.put_i32(p);
    }
}

impl RegionalCity {
    /// Wire-level decode, before package localization and deal claiming. Keeping
    /// this separate permits the foundations §14.20 same-input codec vector,
    /// while object Read still merges both deal lists and makes deals Silent.
    pub fn decode_record(inp: &mut dyn InStream, default_difficulty: u8) -> Result<Self, RecordError> {
        Self::decode_record_with(inp, default_difficulty, &mut CoreElements)
    }

    /// The object's Write always emits version 13, not the input version.
    pub fn write_record(&self, out: &mut dyn OutStream) -> bool {
        self.write_record_with(out, &mut CoreElements)
    }
}

impl<C: ConnectionHandle, D: DealHandle> RegionalCity<C, D> {
    pub fn decode_record_with(
        inp: &mut dyn InStream,
        default_difficulty: u8,
        elements: &mut impl RecordReader<C, D>,
    ) -> Result<Self, RecordError> {
        let mut c = Self::with_handles(default_difficulty);
        c.decode_record_into_with(inp, elements)?;
        Ok(c)
    }

    /// RY7: the receiver keeps prior entries and earlier mutations on failure.
    pub fn decode_record_into_with(
        &mut self,
        inp: &mut dyn InStream,
        elements: &mut impl RecordReader<C, D>,
    ) -> Result<(), RecordError> {
        macro_rules! get {
            ($f:ident) => {
                inp.$f().ok_or(RecordError::Truncated)?
            };
        }
        let version = get!(get_u16);
        if version >= 14 {
            return Err(RecordError::Version);
        }
        let c = self;
        c.position = (get!(get_i32), get!(get_i32));
        c.size = (get!(get_i32), get!(get_i32));
        c.population = get!(get_i32);
        c.commercial_jobs = get!(get_i32);
        c.industrial_jobs = get!(get_i32);
        c.workforce = if version < 10 {
            0.5
        } else {
            let w = get!(get_f32);
            if version == 10 {
                w * 0.01
            } else {
                w
            }
        };
        c.mayor_rating = get!(get_i8);
        c.difficulty = if version < 13 { 2 } else { get!(get_u8) };
        c.moved = get!(get_bool);
        c.serial = get!(get_u32);
        c.original_locale = (get!(get_i32), get!(get_i32));
        c.last_locale = (get!(get_i32), get!(get_i32));
        c.birth_date = get!(get_u32);
        c.established = get!(get_bool);
        c.city_name = get!(get_str);
        c.unknown_strings[0] = get!(get_str);
        c.mayor_name = get!(get_str);
        c.unknown_strings[1] = get!(get_str);
        c.utility_advisor_name = get!(get_str);
        c.description = get!(get_str);
        c.unknown_strings[2] = get!(get_str);
        c.world = [get!(get_f32), get!(get_f32), get!(get_f32)];
        c.tutorial_id = get!(get_i32);
        map_read(inp, &mut c.populations)?;
        map_read(inp, &mut c.extrapolated)?;
        map_read(inp, &mut c.potential)?;
        for i in 0..count(inp, 12)? {
            c.tax_rates[i as usize] = get!(get_f32);
        }
        for _ in 0..count(inp, 0x100000)? {
            c.connections
                .push(elements.connection(inp).ok_or(RecordError::Element)?);
        }
        for _ in 0..count(inp, 0x100000)? {
            c.pending.push(PendingChange::read(inp).ok_or(RecordError::Truncated)?);
        }
        c.budget = get!(get_f64);
        c.income = get!(get_f64);
        for _ in 0..count(inp, 0x100000)? {
            let t = get!(get_i32);
            let commodity = c.commodities.entry(t).or_default();
            commodity.produced = get!(get_f64);
            commodity.demanded = get!(get_f64);
            commodity.imported = get!(get_f64);
            commodity.exported = get!(get_f64);
            commodity.cost_per_unit = get!(get_f64);
        }
        for list in [&mut c.other_deals, &mut c.local_deals] {
            // RY7: nonzero/decrement loop, including the bits of a negative signed count.
            let n = get!(get_u32);
            for _ in 0..n {
                list.push(elements.deal(inp).ok_or(RecordError::Element)?);
            }
        }
        if version >= 12 {
            for _ in 0..get!(get_u32) {
                let t = get!(get_u32);
                let points = c.points.entry(t).or_default();
                for _ in 0..get!(get_u32) {
                    points.push([get!(get_f32), get!(get_f32), get!(get_f32)]);
                }
            }
        }
        if inp.error() != 0 {
            Err(RecordError::Truncated)
        } else {
            Ok(())
        }
    }

    pub fn write_record_with(&self, out: &mut dyn OutStream, elements: &mut impl RecordElements<C, D>) -> bool {
        out.put_u16(13);
        out.put_i32(self.position.0);
        out.put_i32(self.position.1);
        out.put_i32(self.size.0);
        out.put_i32(self.size.1);
        out.put_i32(self.population);
        out.put_i32(self.commercial_jobs);
        out.put_i32(self.industrial_jobs);
        out.put_f32(self.workforce);
        out.put_i8(self.mayor_rating);
        out.put_u8(self.difficulty);
        out.put_bool(self.moved);
        out.put_u32(self.serial);
        out.put_i32(self.original_locale.0);
        out.put_i32(self.original_locale.1);
        out.put_i32(self.last_locale.0);
        out.put_i32(self.last_locale.1);
        out.put_u32(self.birth_date);
        out.put_bool(self.established);
        for s in [
            &self.city_name,
            &self.unknown_strings[0],
            &self.mayor_name,
            &self.unknown_strings[1],
            &self.utility_advisor_name,
            &self.description,
            &self.unknown_strings[2],
        ] {
            out.put_str(s);
        }
        for &v in &self.world {
            out.put_f32(v);
        }
        out.put_i32(self.tutorial_id);
        map_write(out, &self.populations);
        map_write(out, &self.extrapolated);
        map_write(out, &self.potential);
        out.put_u32(12);
        for &r in &self.tax_rates {
            out.put_f32(r);
        }
        out.put_u32(self.connections.len() as u32);
        for c in &self.connections {
            if elements.connection(c, out) == Some(false) {
                return false;
            }
        }
        out.put_u32(self.pending.len() as u32);
        for p in &self.pending {
            p.write(out);
        }
        out.put_f64(self.budget);
        out.put_f64(self.income);
        out.put_i32(self.commodities.len() as i32);
        for (&t, c) in &self.commodities {
            out.put_i32(t);
            out.put_f64(c.produced);
            out.put_f64(c.demanded);
            out.put_f64(c.imported);
            out.put_f64(c.exported);
            out.put_f64(c.cost_per_unit);
        }
        for list in [&self.other_deals, &self.local_deals] {
            out.put_i32(list.len() as i32);
            for d in list {
                if elements.deal(d, out) == Some(false) {
                    return false;
                }
            }
        }
        out.put_u32(self.points.len() as u32);
        for (&t, points) in &self.points {
            out.put_u32(t);
            out.put_u32(points.len() as u32);
            for p in points {
                for &v in p {
                    out.put_f32(v);
                }
            }
        }
        out.error() == 0
    }
}

impl RegionalCity {
    pub fn read_record(
        inp: &mut dyn InStream,
        context: &ReadContext<'_>,
        host: &mut impl Lifecycle,
    ) -> Result<Self, RecordError> {
        Self::read_record_with(inp, context, host, &mut CoreElements)
    }
}

impl<C: ConnectionHandle, D: DealHandle> RegionalCity<C, D> {
    pub fn read_record_with(
        inp: &mut dyn InStream,
        context: &ReadContext<'_>,
        host: &mut impl Lifecycle,
        elements: &mut impl RecordReader<C, D>,
    ) -> Result<Self, RecordError> {
        let mut c = Self::with_handles(context.application_default_difficulty);
        c.read_record_into_with(inp, context, host, elements)?;
        Ok(c)
    }

    pub fn read_record_into_with(
        &mut self,
        inp: &mut dyn InStream,
        context: &ReadContext<'_>,
        host: &mut impl Lifecycle,
        elements: &mut impl RecordReader<C, D>,
    ) -> Result<(), RecordError> {
        self.decode_record_into_with(inp, elements)?;
        let c = self;
        if !c.established {
            c.difficulty = context.application_default_difficulty;
            c.city_name = context.default_city_name.to_vec();
        }
        if let Some(s) = context.localized_city_name {
            c.city_name = s.to_vec();
        }
        if let Some(s) = context.localized_mayor_name {
            c.mayor_name = s.to_vec();
        }
        c.tutorial_id = (context.map_tutorial)(c.tutorial_id);
        c.other_deals.append(&mut c.local_deals);
        for d in &c.other_deals {
            d.set_flags(flag::SILENT, true);
        }
        c.init(host);
        Ok(())
    }
}

/// Package interface, used without filesystem or game implementation assumptions.
pub trait Package {
    fn open(&mut self, path: &[u8], update: bool) -> bool;
    fn exists(&mut self, key: ResKey) -> bool;
    fn read(&mut self, key: ResKey) -> Option<Vec<u8>>;
    fn write(&mut self, key: ResKey, bytes: &[u8]) -> bool;
    fn is_open(&mut self) -> bool;
    fn close(&mut self) -> bool;
    fn shutdown(&mut self);
    fn release(&mut self);
}
pub fn read_package(
    package: &mut impl Package,
    out: &mut Option<RegionalCity>,
    context: &ReadContext<'_>,
    host: &mut impl Lifecycle,
) -> bool {
    let Some(bytes) = package.read(RECORD_KEY) else {
        return false;
    };
    let mut inp = ByteReader::new(&bytes);
    if inp.get_u16() != Some(1) {
        return false;
    }
    match RegionalCity::read_record(&mut inp, context, host) {
        Ok(mut city) => {
            city.add_ref();
            *out = Some(city);
            true
        }
        Err(_) => false,
    }
}
pub fn write_package(package: &mut impl Package, city: &RegionalCity) -> bool {
    let mut out = ByteWriter::new();
    out.put_u16(1);
    city.write_record(&mut out) && package.write(RECORD_KEY, out.bytes())
}
fn open(package: &mut impl Package, path: &[u8], update: bool) -> Option<bool> {
    if !package.open(path, update) {
        return None;
    }
    let version = match package.read(VERSION_KEY) {
        Some(b) => ByteReader::new(&b).get_u32().unwrap_or(0),
        None => {
            if package.exists(VERSION_KEY) {
                0
            } else {
                65
            }
        }
    };
    Some((62..=65).contains(&version))
}
fn finish(package: &mut impl Package) {
    // RR9: this is IsOpen, not Commit; cleanup never replaces the stream result.
    if package.is_open() {
        package.close();
        package.shutdown();
        package.release();
    }
}
pub fn read_path(
    package: &mut impl Package,
    path: &[u8],
    out: &mut Option<RegionalCity>,
    context: &ReadContext<'_>,
    host: &mut impl Lifecycle,
) -> bool {
    let Some(allowed) = open(package, path, false) else {
        return false;
    };
    let result = allowed && read_package(package, out, context, host);
    finish(package);
    result
}
pub fn write_path(package: &mut impl Package, path: &[u8], city: &RegionalCity) -> bool {
    let Some(allowed) = open(package, path, true) else {
        return false;
    };
    let result = allowed && write_package(package, city);
    finish(package);
    result
}
