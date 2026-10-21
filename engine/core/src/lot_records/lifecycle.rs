//! Lot life cycle after a load and business-unit membership
//! (`docs/specs/lot-records.md` §3.5, §3.6, §4; `lot-developer.md` §5.7 for
//! the `cISC4BusinessUnit` `Add` / `Remove` slots).
//!
//! The lot does not reach other subsystems itself: terrain, the lot file's
//! random generator (`0x00B49120`) and the lot configuration manager
//! (`0x00B43D18`) are behind [`LotHost`], supplied by the owner (the zone
//! developer's `Init` / `PostCityInit` walk, §3.5).

use std::rc::Rc;

use super::lot::{lot_flag, BusinessUnit, Lot};
use crate::stream::{RefValue, SaveObject};

// ---- lot properties observed in saves (§3.6) ------------------------------------------------------

/// uint16[12] trip statistics.
pub const PROP_TRIP_STATISTICS: u32 = 0x8A0B_4100;
/// No-power counter.
pub const PROP_NO_POWER_COUNTER: u32 = 0xE999_B11F;
/// Traffic random value.
pub const PROP_TRAFFIC_RANDOM: u32 = 0x4A61_1773;
/// Traffic strategy bucket.
pub const PROP_TRAFFIC_STRATEGY_BUCKET: u32 = 0x691C_863C;
/// No-water counter.
pub const PROP_NO_WATER_COUNTER: u32 = 0x8999_B20B;
/// Low-desirability counter.
pub const PROP_LOW_DESIRABILITY_COUNTER: u32 = 0xC999_E206;
/// Network link IDs.
pub const PROP_NETWORK_LINK_IDS: u32 = 0x2A4F_9C74;
/// Travel counter.
pub const PROP_TRAVEL_COUNTER: u32 = 0x8999_B10B;
/// Current zot.
pub const PROP_CURRENT_ZOT: u32 = 0xC999_C45E;
/// First and last locally funded line item (`0xEA54D283…0xEA54D286`).
pub const PROP_LOCAL_FUNDING_FIRST: u32 = 0xEA54_D283;
pub const PROP_LOCAL_FUNDING_LAST: u32 = 0xEA54_D286;

/// The strategy byte of a newly constructed lot (§3.5 step 2: "constructor value").
pub const STRATEGY_UNSET: u8 = 0xFF;

/// What a lot needs from the rest of the city during `Init` / `PostCityInit`.
pub trait LotHost {
    /// `terrain.GetAltitude(lot.GetCenter())` for a lot with this cell rect
    /// (x1, z1, x2, z2 inclusive) and the slope `SetAltitude` stores in both
    /// slope fields with it (§3.5 step 1).
    // Spec silent (LR1): neither the `GetCenter` point nor the slope
    // `SetAltitude` computes is given, so both come from the host.
    fn center_altitude_and_slope(&mut self, rect: [u8; 4]) -> (f32, f32);
    /// `RandomUint32Uniform(n)` on the lot file's `cRZRandomSuper` `0x00B49120`.
    fn random_uint32_uniform(&mut self, n: u32) -> u32;
    /// Lot configuration manager slot 8 `GetLotConfiguration(id)`: true when
    /// the configuration exists in the loaded data.
    fn has_lot_configuration(&mut self, id: u32) -> bool;
    /// Lock terrain cell (x, z) flat at `altitude` in the terrain view.
    fn lock_cell(&mut self, x: u8, z: u8, altitude: f32);
    /// Unlock terrain cell (x, z).
    fn unlock_cell(&mut self, x: u8, z: u8);
}

/// `cISC4Lot::Init` (`0x006BF490`, slot 5, §3.5). Returns true.
pub fn lot_init(lot: &Lot, host: &mut dyn LotHost) -> bool {
    let mut r = lot.rec.borrow_mut();
    // Step 1: only a lot that was neither read nor initialised.
    if !lot.initialised.get() {
        r.travel_jobs = [0.0; 3];
        let (alt, slope) = host.center_altitude_and_slope(r.rect);
        // `SetAltitude` "also sets both slopes" (§3.5); LR1.
        r.altitude = alt;
        r.slope_at_altitude = slope;
        r.slope = slope;
        lot.initialised.set(true);
    }
    // Step 2: one draw for a lot still holding the constructor value.
    if r.strategy == STRATEGY_UNSET {
        r.strategy = host.random_uint32_uniform(100) as u8;
    }
    // Step 3: release the configuration, look the stored ID up, then clear it.
    lot.configuration.set(None);
    if r.configuration_id != 0 && host.has_lot_configuration(r.configuration_id) {
        lot.configuration.set(Some(r.configuration_id));
    }
    r.configuration_id = 0;
    true
}

/// `cISC4Lot::SetFlags` (`0x006BF220`, slot 13): store the flags; on a change
/// of bit 0 lock (set) or unlock (clear) every cell of the rect (§3.5).
pub fn lot_set_flags(lot: &Lot, flags: u32, host: &mut dyn LotHost) {
    let mut r = lot.rec.borrow_mut();
    let old = r.flags;
    // The record keeps a byte (§3.2 field 3); the slot takes a u32.
    r.flags = flags as u8;
    if (old ^ r.flags) & lot_flag::TERRAIN_LOCKED == 0 {
        return;
    }
    let [x1, z1, x2, z2] = r.rect;
    let lock = r.flags & lot_flag::TERRAIN_LOCKED != 0;
    // Spec silent on the visiting order (LR2): x outer, z inner.
    for x in x1..=x2 {
        for z in z1..=z2 {
            if lock {
                host.lock_cell(x, z, r.altitude);
            } else {
                host.unlock_cell(x, z);
            }
        }
    }
}

/// `cISC4Lot::PostCityInit` (`0x006BF200`, slot 8): `f := flags; flags := 0;
/// SetFlags(f)`, so a loaded lot with bit 0 re-locks its terrain (§3.5, T14).
pub fn lot_post_city_init(lot: &Lot, host: &mut dyn LotHost) {
    let f = {
        let mut r = lot.rec.borrow_mut();
        std::mem::replace(&mut r.flags, 0)
    };
    lot_set_flags(lot, u32::from(f), host);
}

/// `cISC4Lot::Shutdown` (`0x006C3060`, slot 7, §3.5). Returns true.
pub fn lot_shutdown(lot: &Rc<Lot>) -> bool {
    if !lot.initialised.get() {
        return true;
    }
    lot.initialised.set(false);
    let unit = {
        let mut r = lot.rec.borrow_mut();
        r.populations.clear();
        r.travel_populations.clear();
        r.travel_designate = RefValue::Null;
        r.business_unit.clone()
    };
    lot.configuration.set(None);
    if let Some(u) = unit.object::<BusinessUnit>() {
        business_unit_remove(&u, lot);
    }
    // "and drop the unit": also done by `Remove` through `SetUnit(0)`; a unit
    // the core does not own (host reference) is only dropped.
    lot.rec.borrow_mut().business_unit = RefValue::Null;
    true
}

fn same_lot(r: &RefValue, lot: &Rc<Lot>) -> bool {
    match r {
        RefValue::Object(o) => std::ptr::eq(Rc::as_ptr(o) as *const u8, Rc::as_ptr(lot) as *const u8),
        _ => false,
    }
}

/// `cISC4BusinessUnit::Add(lot)` (slot 3, `0x006C0940`): append the lot unless
/// it is already a member, then `lot.SetUnit(this)` in both cases.
pub fn business_unit_add(unit: &Rc<BusinessUnit>, lot: &Rc<Lot>) {
    {
        let mut u = unit.rec.borrow_mut();
        if !u.lots.iter().any(|m| same_lot(m, lot)) {
            u.lots.push(RefValue::Object(lot.clone() as Rc<dyn SaveObject>));
        }
    }
    lot.rec.borrow_mut().business_unit = RefValue::Object(unit.clone() as Rc<dyn SaveObject>);
}

/// `cISC4BusinessUnit::Remove(lot)` (slot 4, `0x006C08B0`): unlink the lot,
/// then `lot.SetUnit(0)` and `lot.SetTravelDesignate(0)`.
///
/// The return value (AL) is not given by the spec (LR3): true when the lot was
/// a member. The two setters run whether or not it was (literal order).
pub fn business_unit_remove(unit: &Rc<BusinessUnit>, lot: &Rc<Lot>) -> bool {
    let found = {
        let mut u = unit.rec.borrow_mut();
        match u.lots.iter().position(|m| same_lot(m, lot)) {
            Some(i) => {
                u.lots.remove(i);
                true
            }
            None => false,
        }
    };
    let mut r = lot.rec.borrow_mut();
    r.business_unit = RefValue::Null;
    r.travel_designate = RefValue::Null;
    found
}

/// `cISC4BusinessUnit::GetMembers()` (slot 5): the member list in order.
pub fn business_unit_members(unit: &BusinessUnit) -> Vec<RefValue> {
    unit.rec.borrow().lots.clone()
}
