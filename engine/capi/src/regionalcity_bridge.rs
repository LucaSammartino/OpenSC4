// SPDX-License-Identifier: GPL-3.0-or-later
//! Private interface-only bridges for RegionalCity's lists, deals and city cache.
use super::{City, Connection, ConnectionCallbacks, Deal, DealCallbacks, Owner};
use fc4_core::neighbors::CityId;
use fc4_core::regional_city::{CacheHost, DealRegion, Preferences};
use std::{collections::BTreeMap, ffi::c_void};

unsafe fn slice<'a, T>(p: *const T, count: u32) -> Option<&'a [T]> {
    if count == 0 {
        Some(&[])
    } else if p.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(p, count as usize))
    }
}

/// Emit borrowed primary interfaces. The receiver acquires references it keeps.
/// Clone under the lock, emit after unlocking: a list allocator/retain callback
/// may call one of the owner's getters without deadlocking.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_list(
    owner: *const Owner,
    kind: u32,
    context: *mut c_void,
    emit: unsafe extern "C" fn(*mut c_void, *mut c_void),
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    match kind {
        0 => {
            let list = match owner.0.lock() {
                Ok(c) => c.connections.clone(),
                Err(_) => return false,
            };
            for c in list {
                emit(context, c.raw());
            }
        }
        1 => {
            let list = match owner.0.lock() {
                Ok(c) => c.local_deals.clone(),
                Err(_) => return false,
            };
            for d in list {
                emit(context, d.raw());
            }
        }
        _ => return false,
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_find(owner: *const Owner, kind: u32, x: i32, z: i32) -> *mut c_void {
    let Some(owner) = owner.as_ref() else {
        return std::ptr::null_mut();
    };
    let Ok(c) = owner.0.lock() else {
        return std::ptr::null_mut();
    };
    c.find_connection(kind, x, z)
        .map_or(std::ptr::null_mut(), Connection::raw)
}

#[repr(C)]
pub struct RegionCalls {
    context: *mut c_void,
    this_city: CityId,
    current_city: CityId,
    city_at: unsafe extern "C" fn(*mut c_void, i32, i32) -> CityId,
}
impl DealRegion for RegionCalls {
    fn current_city(&self) -> Option<CityId> {
        (self.current_city != 0).then_some(self.current_city)
    }
    fn city_at(&self, (x, z): (i32, i32)) -> Option<CityId> {
        let id = unsafe { (self.city_at)(self.context, x, z) };
        (id != 0).then_some(id)
    }
}

/// 0 UpdateLocalDeals; 1 SetLocalDeals; 2 UpdateImportExport.
/// `objects` are borrowed, including when they alias the owner's live list.
/// Invalid input is rejected before clearing the old local deals.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_deal_action(
    owner: *mut Owner,
    action: u32,
    region: *const RegionCalls,
    objects: *const *mut c_void,
    count: u32,
    calls: *const DealCallbacks,
) -> bool {
    let (Some(owner), Some(region)) = (owner.as_ref(), region.as_ref()) else {
        return false;
    };
    let mut incoming = Vec::new();
    if action == 1 {
        let (Some(objects), Some(calls)) = (slice(objects, count), calls.as_ref()) else {
            return false;
        };
        let Some(list) = objects
            .iter()
            .map(|&p| Deal::acquire(p, *calls))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        incoming = list;
    }
    let Ok(mut c) = owner.0.lock() else { return false };
    match action {
        0 => c.update_local_deals(region.this_city, region),
        1 => c.set_local_deals(&mut incoming, region.this_city, region),
        2 => {
            c.update_import_export(region.this_city, region);
            true
        }
        _ => false,
    }
}

/// PostLoad/PostCityShutdown list return, PreSave growth reset, Shutdown lists.
/// Message routing/subscription is the COM object's responsibility.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_event(owner: *mut Owner, event: u32) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    let Ok(mut c) = owner.0.lock() else { return false };
    match event {
        0 => c.return_local_deals(),
        1 => c.reset_growth(),
        2 => {
            c.connections.clear();
            c.other_deals.clear();
            c.local_deals.clear();
        }
        _ => return false,
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_change_connection(
    owner: *mut Owner,
    object: *mut c_void,
    calls: *const ConnectionCallbacks,
    source_x: i32,
    source_z: i32,
    source_sx: i32,
    source_sz: i32,
    add: bool,
) -> bool {
    let (Some(owner), Some(calls)) = (owner.as_ref(), calls.as_ref()) else {
        return false;
    };
    let Some(connection) = Connection::acquire(object, *calls) else {
        return false;
    };
    let Ok(mut c) = owner.0.lock() else { return false };
    c.change_symmetric_connection(&connection, (source_x, source_z), (source_sx, source_sz), u8::from(add));
    true
}

#[repr(C)]
pub struct Population {
    kind: u32,
    count: i32,
    jobs: [f32; 4],
}
#[repr(C)]
pub struct Point {
    kind: u32,
    position: [f32; 3],
    livable: bool,
}
/// Host gathers through interfaces before entering this call. In particular,
/// UpdateCommodityStats can re-enter the COM setters and runs outside the lock.
/// Text and live connections have already been copied through their transports.
#[repr(C)]
pub struct Cache {
    difficulty: u8,
    serial: u32,
    established: bool,
    mayor_rating: i8,
    tax: [f32; 12],
    tutorial: i32,
    residential: bool,
    population: i32,
    workforce: f32,
    populations: *const Population,
    population_count: u32,
    points: *const Point,
    point_count: u32,
}
struct Snapshot<'a> {
    cache: &'a Cache,
    base: City,
    populations: &'a [Population],
    points: &'a [Point],
}
impl CacheHost<Connection, Deal> for Snapshot<'_> {
    fn mayor_name(&mut self) -> Vec<u8> {
        self.base.mayor_name.clone()
    }
    fn city_name(&mut self) -> Vec<u8> {
        self.base.city_name.clone()
    }
    fn difficulty(&mut self) -> u8 {
        self.cache.difficulty
    }
    fn file_path(&mut self) -> Vec<u8> {
        self.base.file_path.clone()
    }
    fn serial(&mut self) -> u32 {
        self.cache.serial
    }
    fn established(&mut self) -> bool {
        self.cache.established
    }
    fn mayor_rating(&mut self) -> i8 {
        self.cache.mayor_rating
    }
    fn tax_rate(&mut self, i: usize) -> f32 {
        self.cache.tax[i]
    }
    fn connections(&mut self) -> Vec<Connection> {
        self.base.connections.clone()
    }
    fn update_commodity_stats(&mut self, _: &mut City) {} // Host ran the re-entrant interface call.
    fn tutorial_id(&mut self) -> Option<i32> {
        (self.cache.tutorial != -1).then_some(self.cache.tutorial)
    }
    fn residential(&mut self) -> Option<(i32, f32)> {
        self.cache
            .residential
            .then_some((self.cache.population, self.cache.workforce))
    }
    fn population_summary(&mut self) -> BTreeMap<u32, i32> {
        self.populations.iter().map(|p| (p.kind, p.count)).collect()
    }
    fn jobs_per_unit(&mut self, kind: u32) -> [f32; 4] {
        self.populations
            .iter()
            .find(|p| p.kind == kind)
            .map_or([0.0; 4], |p| p.jobs)
    }
    fn lots(&mut self, kind: u32) -> Vec<(bool, [f32; 3])> {
        self.points
            .iter()
            .filter(|p| p.kind == kind)
            .map(|p| (p.livable, p.position))
            .collect()
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_cache(owner: *mut Owner, cache: *const Cache) -> bool {
    let (Some(owner), Some(cache)) = (owner.as_ref(), cache.as_ref()) else {
        return false;
    };
    let (Some(populations), Some(points)) = (
        slice(cache.populations, cache.population_count),
        slice(cache.points, cache.point_count),
    ) else {
        return false;
    };
    let Ok(mut c) = owner.0.lock() else { return false };
    let mut snapshot = Snapshot {
        cache,
        base: c.clone(),
        populations,
        points,
    };
    c.update_city_cache(&mut snapshot);
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_preferences(
    owner: *mut Owner,
    city: *const u8,
    city_len: u32,
    mayor: *const u8,
    mayor_len: u32,
    difficulty: u8,
) -> bool {
    let (Some(owner), Some(city), Some(mayor)) = (owner.as_ref(), slice(city, city_len), slice(mayor, mayor_len))
    else {
        return false;
    };
    let Ok(mut c) = owner.0.lock() else { return false };
    c.update_preferences(&Preferences {
        city_name: city.to_vec(),
        mayor_name: mayor.to_vec(),
        difficulty,
    });
    true
}

/// Returns false for an unknown type, preserving the caller's output vector.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_points(
    owner: *const Owner,
    kind: u32,
    context: *mut c_void,
    emit: unsafe extern "C" fn(*mut c_void, *const [f32; 3], u32),
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    let points = match owner.0.lock() {
        Ok(c) => c.points.get(&kind).cloned(),
        Err(_) => return false,
    };
    let Some(points) = points else { return false };
    emit(context, points.as_ptr(), points.len() as u32);
    true
}

#[cfg(test)]
#[path = "regionalcity_bridge_tests.rs"]
mod tests;
