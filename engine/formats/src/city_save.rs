// SPDX-License-Identifier: GPL-3.0-or-later
//! Standalone city-save audit. Unknown owner payloads are retained for incremental
//! work, but never counted as handled. IDs are preserved throughout the archive.

use crate::dbpf::{Compression, DIR_TGI};
use crate::{Package, PackageBuilder, Source, Tgi};
use fc4_core::camera::{CameraControl, CAMERA_CONTROL_CLASS_ID};
use fc4_core::city_detail::DetailRecord;
use fc4_core::clock24::{Clock24, CLSID_24_HOUR_CLOCK};
use fc4_core::commercial::CommercialRecord;
use fc4_core::commercial::IndustrialSim;
use fc4_core::history::{HistoryWarehouse, CLSID_HISTORY_WAREHOUSE};
use fc4_core::save::SaveRecord;
use fc4_core::store::archive::{read_class, write_class, ClassIndex, StoredObject};
use fc4_core::store::{INDEX_TYPE, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter, SaveObject};
use fc4_core::terrain::{SurfaceWaterRecord, Terrain, TerrainRecord, TerrainViewRecord};
use std::collections::BTreeMap;
use std::rc::Rc;

mod grids;
mod hosts;
mod terrain;

enum Payload {
    Simulator(SaveRecord),
    History(Box<HistoryWarehouse>),
    Clock(Box<Clock24>),
    Camera(Box<CameraControl>),
    Serializable(Rc<dyn SaveObject>),
    Terrain(TerrainRecord),
    TerrainView(TerrainViewRecord),
    SurfaceWater(SurfaceWaterRecord),
    Commercial(CommercialRecord),
    Detail(DetailRecord),
    Unhandled(Vec<u8>),
}

impl Payload {
    fn read(class: u32, bytes: &[u8]) -> Result<Self, String> {
        let mut r = ByteReader::new(bytes);
        let (record, ok) = match class {
            0x2990_C1E5 => {
                return SaveRecord::decode(bytes)
                    .map(Self::Simulator)
                    .map_err(|e| e.to_string())
            }
            CLSID_HISTORY_WAREHOUSE => {
                let mut record = Box::new(HistoryWarehouse::new(Box::new(hosts::Offline)));
                let ok = record.read(&mut r);
                (Self::History(record), ok)
            }
            CLSID_24_HOUR_CLOCK => {
                let mut record = Box::new(Clock24::new(Box::new(hosts::Offline)));
                let ok = record.read(&mut r);
                (Self::Clock(record), ok)
            }
            CAMERA_CONTROL_CLASS_ID => {
                let mut record = Box::new(CameraControl::new());
                let ok = record.read(&mut r);
                (Self::Camera(record), ok)
            }
            0xA990_C022 => {
                let record = IndustrialSim::new(0);
                let ok = record.read(&mut r);
                (Self::Serializable(record), ok)
            }
            0xE98F_9525 => (
                Self::Terrain(TerrainRecord::read(&mut r).ok_or("terrain class Read failed")?),
                true,
            ),
            0xC9B8_4E10 => (
                Self::TerrainView(TerrainViewRecord::read(&mut r).ok_or("terrain view Read failed")?),
                true,
            ),
            0x0B8D_A7DB => (
                Self::SurfaceWater(SurfaceWaterRecord::read(&mut r).ok_or("water Read failed")?),
                true,
            ),
            0x8990_C080 => (
                Self::Commercial(CommercialRecord::read(&mut r).ok_or("commercial Read failed")?),
                true,
            ),
            0xEAD4_BA36 => (
                Self::Detail(DetailRecord::read(&mut r).ok_or("city detail Read failed")?),
                true,
            ),
            _ => match grids::create(class) {
                Some(record) => {
                    let ok = record.read(&mut r);
                    (Self::Serializable(record), ok)
                }
                None => return Ok(Self::Unhandled(bytes.to_vec())),
            },
        };
        if !ok || r.remaining() != 0 {
            return Err(format!(
                "class {class:08X}: Read failed or left {} bytes",
                r.remaining()
            ));
        }
        Ok(record)
    }

    fn write(&self) -> Result<Vec<u8>, String> {
        let mut w = ByteWriter::new();
        let ok = match self {
            Self::Simulator(record) => return Ok(record.encode()),
            Self::Unhandled(bytes) => return Ok(bytes.clone()),
            Self::History(record) => record.write(&mut w),
            Self::Clock(record) => record.write(&mut w),
            Self::Camera(record) => record.write(&mut w),
            Self::Serializable(record) => record.write(&mut w),
            Self::Terrain(record) => record.write(&mut w),
            Self::TerrainView(record) => record.write(&mut w),
            Self::SurfaceWater(record) => record.write(&mut w),
            Self::Commercial(record) => record.write(&mut w),
            Self::Detail(record) => record.write(&mut w),
        };
        if !ok {
            return Err("owner Write failed".into());
        }
        Ok(w.into_bytes())
    }

    fn handled(&self) -> bool {
        !matches!(self, Self::Unhandled(_))
    }
}

enum RecordData {
    Index(ClassIndex),
    Class(Vec<(u32, Payload)>),
    Terrain(Rc<Terrain>, terrain::Part),
    Edge(Vec<f32>),
    Unhandled(Vec<u8>),
}

struct Record {
    key: Tgi,
    compressed: bool,
    original: Vec<u8>,
    stored: Vec<u8>,
    data: RecordData,
}

impl Record {
    fn write(&self) -> Result<Vec<u8>, String> {
        match &self.data {
            RecordData::Index(index) => Ok(index.write()),
            RecordData::Class(objects) => write_class(
                &objects
                    .iter()
                    .map(|(id, p)| {
                        Ok(StoredObject {
                            id: *id,
                            payload: p.write()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            RecordData::Unhandled(bytes) => Ok(bytes.clone()),
            RecordData::Terrain(terrain, part) => Ok(part.write(terrain)),
            RecordData::Edge(values) => Ok(Terrain::write_edge_altitudes(values)),
        }
    }

    fn handled(&self) -> bool {
        match &self.data {
            RecordData::Index(_) => true,
            RecordData::Terrain(..) | RecordData::Edge(_) => true,
            RecordData::Class(objects) => objects.iter().all(|(_, p)| p.handled()),
            RecordData::Unhandled(_) => false,
        }
    }
}

/// Parsed package, class framing and owner models. No source file is modified.
pub struct CitySave {
    records: Vec<Record>,
    created: u32,
    modified: u32,
    directory: Option<Vec<u8>>,
}

impl CitySave {
    pub fn read<S: Source>(package: &Package<S>) -> Result<Self, String> {
        let mut records = Vec::new();
        let terrain = terrain::load(package)?;
        let mut directory = None;
        let mut keys = std::collections::BTreeSet::new();
        for entry in package.entries() {
            if !keys.insert(entry.tgi) {
                return Err(format!("duplicate record {}", entry.tgi));
            }
            if entry.tgi == DIR_TGI {
                directory = Some(package.read(entry).map_err(|e| e.to_string())?);
                continue; // PackageBuilder regenerates compression metadata.
            }
            let original = package.read(entry).map_err(|e| e.to_string())?;
            let key = entry.tgi;
            let data = if key == Tgi::new(INDEX_TYPE, STORE_GROUP, 0) {
                RecordData::Index(ClassIndex::read(&original)?)
            } else if key.group == STORE_GROUP && key.instance == 0 {
                let objects = read_class(&original)?;
                RecordData::Class(
                    objects
                        .into_iter()
                        .map(|o| Ok((o.id, Payload::read(key.type_id, &o.payload)?)))
                        .collect::<Result<_, String>>()?,
                )
            } else if let Some(part) = terrain::Part::from_key(key) {
                RecordData::Terrain(terrain.clone().ok_or("missing terrain")?, part)
            } else if terrain::is_edge(key) {
                RecordData::Edge(Terrain::read_edge(&original).map_err(|e| format!("terrain edge Read: {e:?}"))?)
            } else {
                RecordData::Unhandled(original.clone())
            };
            records.push(Record {
                key,
                compressed: package.is_compressed(entry),
                original,
                stored: package.read_raw(entry).map_err(|e| e.to_string())?,
                data,
            });
        }
        Ok(Self {
            records,
            created: package.header().created,
            modified: package.header().modified,
            directory,
        })
    }

    pub fn write(&self) -> Result<Vec<u8>, String> {
        let mut builder = PackageBuilder::new();
        builder.created = self.created;
        builder.modified = self.modified;
        for record in &self.records {
            let written = record.write()?;
            // Compression is a container cache only: every owner has already
            // encoded its model, and equality is checked before reusing it.
            if written == record.original {
                let size = record.compressed.then_some(written.len() as u32);
                builder.add_stored(record.key, record.stored.clone(), size);
                continue;
            }
            let compression = if record.compressed {
                Compression::Always
            } else {
                Compression::None
            };
            builder
                .add(record.key, &written, compression)
                .map_err(|e| e.to_string())?;
        }
        builder.build().map_err(|e| e.to_string())
    }

    /// Compare actual output after reopening and decompressing the new package.
    pub fn audit(&self, written: &[u8]) -> Result<Report, String> {
        let package = Package::from_bytes(written).map_err(|e| e.to_string())?;
        let mut report = Report::default();
        let expected = self.records.len() + usize::from(self.directory.is_some());
        if package.entries().len() != expected {
            return Err("output record count changed".into());
        }
        for record in &self.records {
            let bytes = package.read_tgi(&record.key).map_err(|e| e.to_string())?;
            let offset = first_difference(&record.original, &bytes);
            let outcome = if let Some(offset) = offset {
                report.differences.push(format!(
                    "{} differs at byte {offset} ({} -> {} bytes)",
                    record.key,
                    record.original.len(),
                    bytes.len()
                ));
                Outcome::Differs
            } else if record.handled() {
                Outcome::Identical
            } else {
                Outcome::NotHandled
            };
            let stats = report.types.entry(record.key.type_id).or_default();
            stats.count += 1;
            stats.bytes += record.original.len() as u64;
            match outcome {
                Outcome::Identical => {
                    stats.identical += 1;
                    stats.exact_bytes += record.original.len() as u64;
                }
                Outcome::Differs => stats.differs += 1,
                Outcome::NotHandled => stats.not_handled += 1,
            }
        }
        if let Some(original) = &self.directory {
            let bytes = package.read_tgi(&DIR_TGI).map_err(|e| e.to_string())?;
            let stats = report.types.entry(DIR_TGI.type_id).or_default();
            stats.count += 1;
            stats.bytes += original.len() as u64;
            if let Some(offset) = first_difference(original, &bytes) {
                stats.differs += 1;
                report.differences.push(format!("{DIR_TGI} differs at byte {offset}"));
            } else {
                stats.identical += 1;
                stats.exact_bytes += original.len() as u64;
            }
        }
        Ok(report)
    }
}

pub fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(a, b)| a != b)
        .or_else(|| (a.len() != b.len()).then_some(a.len().min(b.len())))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Identical,
    Differs,
    NotHandled,
}

#[derive(Default, Debug)]
pub struct TypeStats {
    pub count: u64,
    pub bytes: u64,
    pub identical: u64,
    pub exact_bytes: u64,
    pub differs: u64,
    pub not_handled: u64,
}

#[derive(Default, Debug)]
pub struct Report {
    pub types: BTreeMap<u32, TypeStats>,
    pub differences: Vec<String>,
}

impl Report {
    pub fn merge(&mut self, other: Self) {
        for (ty, s) in other.types {
            let t = self.types.entry(ty).or_default();
            t.count += s.count;
            t.bytes += s.bytes;
            t.identical += s.identical;
            t.exact_bytes += s.exact_bytes;
            t.differs += s.differs;
            t.not_handled += s.not_handled;
        }
        self.differences.extend(other.differences);
    }

    pub fn table(&self) -> String {
        let mut out = String::from(
            "| Record type | Count | Bytes | Identical | Differs | Not handled |\n|---|---:|---:|---:|---:|---:|\n",
        );
        for (ty, s) in &self.types {
            out.push_str(&format!(
                "| {ty:08X} | {} | {} | {} | {} | {} |\n",
                s.count, s.bytes, s.identical, s.differs, s.not_handled
            ));
        }
        out
    }
}
