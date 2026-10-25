// SPDX-License-Identifier: GPL-3.0-or-later
//! Construction/destruction and interface forwarding contracts (watercraft §3.1).
//!
//! Interface pointers are modelled as `(object, offset)` pairs. Every adjustor
//! subtracts its offset and forwards to the complete-object method unchanged.

use crate::stream::{InStream, OutStream};

use super::{BoatId, ContractGap, Fleet, Result, Watercraft, WatercraftRecord, CLASS_ID, RECORD_VERSION};

/// Offsets of the embedded interfaces that have QueryInterface adjustors
/// (`0x0056DC70`, `0x0056DC80`, `0x0056DC90`, `0x0056DCD0`, `0x0056DCE0`).
pub const QUERY_INTERFACE_OFFSETS: [u32; 5] = [4, 8, 12, 32, 56];
/// Offset of the serialization/lifecycle interface (Write/Read/Init/Shutdown adjustors).
pub const LIFECYCLE_INTERFACE: u32 = 4;
/// Offset of the interface whose slot is the deleting-destructor adjustor (`0x0056DCA0`).
pub const DELETING_INTERFACE: u32 = 12;

/// A pointer to the complete object (offset 0) or one of its embedded interfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterfacePointer {
    pub object: BoatId,
    pub offset: u32,
}

impl InterfacePointer {
    pub fn complete(object: BoatId) -> Self {
        Self { object, offset: 0 }
    }
}

/// Inherited automaton interface entry points the owner forwards to.
pub trait AutomatonInterfaces {
    /// Owner slot 1.
    fn add_ref(&mut self, object: BoatId) -> u32;
    /// Automaton QueryInterface (`0x00505D80`), including its output/failure rules.
    fn query_interface(&mut self, object: BoatId, iid: u32, out: &mut Option<InterfacePointer>) -> bool;
}

/// One step of complete destruction (`0x0056DD20`), in execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// Effect reference by index into `EFFECTS` (+0xDB0 is 0, +0xDC4 is 5).
    Effect(usize),
    NeighborRouteStorage,
    NeighborOccupant,
    TravelStorage,
    Tug,
    Towed,
    /// Route storage of the current station (`None`) or station A/B (`Some(0/1)`).
    RouteStorage(Option<usize>),
    Station(Option<usize>),
    AutomatonBase,
}

impl Watercraft {
    /// GetGZCLSID (`0x0056DC60`).
    pub fn gzclsid(&self) -> u32 {
        CLASS_ID
    }

    /// QueryInterface (`0x00566F30`) on the complete object `object`.
    pub fn query_interface(
        &self,
        object: BoatId,
        iid: u32,
        out: &mut Option<InterfacePointer>,
        base: &mut dyn AutomatonInterfaces,
    ) -> bool {
        if iid == CLASS_ID {
            *out = Some(InterfacePointer::complete(object));
            base.add_ref(object);
            return true;
        }
        base.query_interface(object, iid, out)
    }

    /// Write (`0x00566EE0`): the version-write status is not checked; the
    /// base Write result is returned.
    pub fn write_record(&self, stream: &mut dyn OutStream) -> bool {
        let _ = stream.put_u16(RECORD_VERSION);
        self.record.base.write(stream)
    }

    /// Read (`0x00566F00`): only version 2 calls base Read, whose result is returned.
    /// A failed base read leaves the existing record untouched.
    pub fn read_record(&mut self, stream: &mut dyn InStream) -> bool {
        match WatercraftRecord::read(stream) {
            Some(record) => {
                self.record.base = record.base;
                true
            }
            None => false,
        }
    }

    /// Complete destruction (`0x0056DD20`). Null references/storage are
    /// skipped. No Shutdown, effect stop or berth return happens here.
    pub fn destroy(&mut self) -> Vec<Release> {
        let mut log = Vec::new();
        for index in (0..self.effects.len()).rev() {
            if self.effects[index].take().is_some() {
                log.push(Release::Effect(index));
            }
        }
        if let Some(neighbor) = self.neighbor.as_mut() {
            neighbor.cells = std::sync::Arc::from(Vec::new());
            log.push(Release::NeighborRouteStorage);
            if neighbor.occupant.take().is_some() {
                log.push(Release::NeighborOccupant);
            }
        }
        self.neighbor = None;
        // The constructor always allocates travel storage (§3.1).
        self.travel = Vec::new();
        log.push(Release::TravelStorage);
        if self.tug.take().is_some() {
            log.push(Release::Tug);
        }
        if self.towed.take().is_some() {
            log.push(Release::Towed);
        }
        if self.current_station.take().is_some() {
            log.push(Release::RouteStorage(None));
            log.push(Release::Station(None));
        }
        for index in [1, 0] {
            if self.stations[index].take().is_some() {
                log.push(Release::RouteStorage(Some(index)));
                log.push(Release::Station(Some(index)));
            }
        }
        log.push(Release::AutomatonBase);
        self.destroyed = true;
        log
    }
}

impl Fleet {
    fn resolve(&self, pointer: InterfacePointer, allowed: &[u32]) -> Result<BoatId> {
        if !allowed.contains(&pointer.offset) {
            return Err(ContractGap::UnknownInterface);
        }
        self.boat(pointer.object)?;
        Ok(pointer.object)
    }

    /// QueryInterface from the complete object or any adjusted interface.
    pub fn query_interface(
        &self,
        pointer: InterfacePointer,
        iid: u32,
        out: &mut Option<InterfacePointer>,
        base: &mut dyn AutomatonInterfaces,
    ) -> Result<bool> {
        let mut allowed = vec![0];
        allowed.extend(QUERY_INTERFACE_OFFSETS);
        let id = self.resolve(pointer, &allowed)?;
        Ok(self.boat(id)?.query_interface(id, iid, out, base))
    }

    /// Write through the +4 adjustor (`0x0056DCB0`) or the complete object.
    pub fn write_through(&self, pointer: InterfacePointer, stream: &mut dyn OutStream) -> Result<bool> {
        let id = self.resolve(pointer, &[0, LIFECYCLE_INTERFACE])?;
        Ok(self.boat(id)?.write_record(stream))
    }

    /// Read through the +4 adjustor (`0x0056DCC0`) or the complete object.
    pub fn read_through(&mut self, pointer: InterfacePointer, stream: &mut dyn InStream) -> Result<bool> {
        let id = self.resolve(pointer, &[0, LIFECYCLE_INTERFACE])?;
        Ok(self.boat_mut(id)?.read_record(stream))
    }

    /// Resolve the +4 Init/Shutdown adjustors (`0x0056F7B0`, `0x0056DCF0`).
    pub fn lifecycle_target(&self, pointer: InterfacePointer) -> Result<BoatId> {
        self.resolve(pointer, &[0, LIFECYCLE_INTERFACE])
    }

    /// Deleting destructor (`0x0056DD00`, or `0x0056DCA0` from +12): destroy,
    /// free storage only when bit 0 of `flags` is set, return the original
    /// complete-object pointer.
    pub fn delete(&mut self, pointer: InterfacePointer, flags: u32) -> Result<(InterfacePointer, Vec<Release>)> {
        let id = self.resolve(pointer, &[0, DELETING_INTERFACE])?;
        let log = self.boat_mut(id)?.destroy();
        if flags & 1 != 0 {
            self.boats.remove(&id);
            self.order.retain(|other| *other != id);
        }
        Ok((InterfacePointer::complete(id), log))
    }
}
