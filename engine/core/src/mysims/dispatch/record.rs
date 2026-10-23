// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch and cLocation payloads, mysims.md §6.2.1. No city sizing on Read.
use super::*;
use crate::stream::{InStream, OutStream, RefValue};

/// Adapter for opaque foreign identities. Native AddRef/Release and the owner's
/// serializable sub-object adjustment belong to this adapter, never to core IDs.
pub trait DispatchRecordRefs {
    fn reference(&self, token: u64) -> RefValue;
    fn token(&self, reference: RefValue) -> Option<u64>;
}

fn put_ref(out: &mut dyn OutStream, refs: &dyn DispatchRecordRefs, token: Option<u64>) {
    out.put_ref(&token.map_or(RefValue::Null, |t| refs.reference(t)));
}

fn get_ref(inp: &mut dyn InStream, refs: &dyn DispatchRecordRefs, iid: u32) -> Option<Option<u64>> {
    Some(refs.token(inp.get_ref_as(iid)?))
}

impl DispatchLocation {
    pub fn write(&self, out: &mut dyn OutStream, refs: &dyn DispatchRecordRefs) -> bool {
        put_ref(out, refs, self.occupant);
        out.put_f32(self.world.0);
        out.put_f32(self.world.1);
        out.put_i32(self.cell.0);
        out.put_i32(self.cell.1);
        out.put_i32(self.neighbor_city.0);
        out.put_i32(self.neighbor_city.1);
        out.put_bool(self.in_city);
        out.put_bool(self.by_commute);
        out.put_u32(self.reserved.0);
        out.put_u32(self.reserved.1);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream, refs: &dyn DispatchRecordRefs) -> Option<Self> {
        Some(Self {
            occupant: get_ref(inp, refs, 0x26BB767E)?,
            world: (inp.get_f32()?, inp.get_f32()?),
            cell: (inp.get_i32()?, inp.get_i32()?),
            neighbor_city: (inp.get_i32()?, inp.get_i32()?),
            in_city: inp.get_bool()?,
            by_commute: inp.get_bool()?,
            reserved: (inp.get_u32()?, inp.get_u32()?),
        })
    }
}

impl DispatchState {
    pub fn write(&self, out: &mut dyn OutStream, refs: &dyn DispatchRecordRefs) -> bool {
        out.put_u32(1);
        out.put_bool(self.auxiliary_flag);
        for word in [
            self.tuning.approach_distance as u32,
            self.tuning.proximity as u32,
            self.tuning.idle_minimum as u32,
            self.tuning.location_minimum as u32,
            self.tuning.switch_minimum as u32,
            self.opaque_28,
            self.path_indices.0,
            self.path_indices.1,
            self.change_counter,
            self.previous_change_counter,
            self.tracking_value,
            self.search.state as u32,
            self.search.prev_state as u32,
            self.search.flags,
            self.minutes as u32,
            self.travel_type,
            self.wait_timestamp as u32,
            self.neighbor_retries,
        ] {
            out.put_u32(word);
        }
        for location in [
            self.current,
            self.search.home,
            self.destination,
            self.search.commute_destination,
        ] {
            location.write(out, refs);
        }
        put_ref(out, refs, self.auxiliary_occupant);
        out.put_u32(self.auxiliary_value);
        out.put_str(&self.auxiliary_string);
        put_ref(out, refs, self.automata_occupant);
        put_ref(out, refs, self.owner);
        put_ref(out, refs, self.simulation_service);
        out.put_u32(self.search.route.len() as u32);
        for route in &self.search.route {
            out.put_u8(route.x);
            out.put_u8(route.z);
            out.put_u8(route.travel_type);
        }
        out.put_u32(self.models.len() as u32);
        for (&travel, &(key, offset)) in &self.models {
            out.put_i32(travel);
            out.put_res_key(key);
            out.put_i32(offset);
        }
        out.put_i32(self.route_marker);
        out.put_u32(self.queue.len() as u32);
        for location in &self.queue {
            location.write(out, refs);
        }
        out.error() == 0
    }

    /// Consume any version, as the native reader does. Counts enumerate entries;
    /// no allocation is derived from a city or opaque container capacity.
    pub fn read(inp: &mut dyn InStream, refs: &dyn DispatchRecordRefs) -> Option<Self> {
        inp.get_u32()?;
        let mut st = Self {
            auxiliary_flag: inp.get_bool()?,
            ..Self::default()
        };
        st.tuning = DispatchTuning {
            approach_distance: inp.get_i32()?,
            proximity: inp.get_i32()?,
            idle_minimum: inp.get_i32()?,
            location_minimum: inp.get_i32()?,
            switch_minimum: inp.get_i32()?,
        };
        st.opaque_28 = inp.get_u32()?;
        st.path_indices = (inp.get_u32()?, inp.get_u32()?);
        st.change_counter = inp.get_u32()?;
        st.previous_change_counter = inp.get_u32()?;
        st.tracking_value = inp.get_u32()?;
        st.search.state = inp.get_i32()?;
        st.search.prev_state = inp.get_i32()?;
        st.search.flags = inp.get_u32()?;
        st.minutes = inp.get_i32()?;
        st.travel_type = inp.get_u32()?;
        st.wait_timestamp = inp.get_i32()?;
        st.neighbor_retries = inp.get_u32()?;
        st.current = DispatchLocation::read(inp, refs)?;
        st.search.home = DispatchLocation::read(inp, refs)?;
        st.destination = DispatchLocation::read(inp, refs)?;
        st.search.commute_destination = DispatchLocation::read(inp, refs)?;
        st.auxiliary_occupant = get_ref(inp, refs, 0x26BB767E)?;
        st.auxiliary_value = inp.get_u32()?;
        st.auxiliary_string = inp.get_str()?;
        st.automata_occupant = get_ref(inp, refs, 0x26BB767E)?;
        st.owner = get_ref(inp, refs, 0x8BC01FFE)?;
        st.simulation_service = get_ref(inp, refs, 0x681BD53A)?;
        for _ in 0..inp.get_u32()? {
            st.search.route.push_back(RouteData {
                x: inp.get_u8()?,
                z: inp.get_u8()?,
                travel_type: inp.get_u8()?,
            });
        }
        for _ in 0..inp.get_u32()? {
            let travel = inp.get_i32()?;
            let key = inp.get_res_key()?;
            let offset = inp.get_i32()?;
            st.models.insert(travel, (key, offset));
        }
        st.route_marker = inp.get_i32()?;
        for _ in 0..inp.get_u32()? {
            st.queue.push_back(DispatchLocation::read(inp, refs)?);
        }
        if inp.error() == 0 {
            Some(st)
        } else {
            None
        }
    }
}

impl MySimDispatch {
    pub fn write_record(&self, sim: i32, out: &mut dyn OutStream, refs: &dyn DispatchRecordRefs) -> bool {
        self.get_state(sim).write(out, refs)
    }

    pub fn read_record(
        &self,
        sim: i32,
        inp: &mut dyn InStream,
        refs: &dyn DispatchRecordRefs,
        services: &dyn TripServices,
    ) -> bool {
        let restored = DispatchState::read(inp, refs);
        let success = restored.is_some();
        self.edit(sim, |st| {
            if let Some(restored) = restored {
                *st = restored;
            }
            st.read_pending = true;
            st.read_succeeded = success;
        });
        services.register_deferred_setup(sim, true);
        success
    }

    /// Message 0x26D31EC1: init first, unregister, then replay restored values.
    /// Replay is not a new state/flag mutation and does not increment counters.
    pub fn deferred_setup_message(&self, sim: i32, services: &dyn TripServices) -> bool {
        let st = self.get_state(sim);
        if !st.read_pending || !st.read_succeeded {
            return false;
        }
        if !self.init(sim) {
            return false;
        }
        self.edit(sim, |st| st.read_pending = false);
        services.register_deferred_setup(sim, false);
        let st = self.get_state(sim);
        self.host
            .notify(sim, super::super::dispatch_note::TRIP_STATE_A, 0, st.search.flags);
        self.host.notify(
            sim,
            super::super::dispatch_note::TRIP_STATE_B,
            (-2i32) as u32,
            self.get_state(sim).search.state as u32,
        );
        true
    }
}
