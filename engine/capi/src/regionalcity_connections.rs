// SPDX-License-Identifier: GPL-3.0-or-later
//! Interface callbacks for the RegionalCity connection algorithm.
use super::{City, Connection, ConnectionCallbacks, Owner};
use fc4_core::regional_city::{ConnectionHost, IncomingCity, PendingChange};
use std::ffi::c_void;

type EmitAdjacent = unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32, *mut c_void);
type EmitOccupant = unsafe extern "C" fn(*mut c_void, u64);

#[repr(C)]
pub struct Host {
    context: *mut c_void,
    connections: ConnectionCallbacks,
    drive_side: bool,
    other_established: unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool,
    adjacent: unsafe extern "C" fn(*mut c_void, *mut c_void, EmitAdjacent),
    points_here: unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool,
    network_exists: unsafe extern "C" fn(*mut c_void, u32) -> bool,
    find: unsafe extern "C" fn(*mut c_void, u32, i32, i32) -> bool,
    change: unsafe extern "C" fn(*mut c_void, u32, i32, i32, bool, bool),
    present: unsafe extern "C" fn(*mut c_void, i32, i32, u32) -> bool,
    cells: unsafe extern "C" fn(*mut c_void, u32, *const [i32; 2], u32, u32),
    segment: unsafe extern "C" fn(*mut c_void, u32, i32, i32, i32, i32),
    occupants: unsafe extern "C" fn(*mut c_void, u32, i32, i32, *mut c_void, EmitOccupant),
    word: unsafe extern "C" fn(*mut c_void, u64) -> u32,
    set_word: unsafe extern "C" fn(*mut c_void, u64, u32),
    post: unsafe extern "C" fn(*mut c_void, u32),
}
struct Incoming<'a> {
    calls: &'a ConnectionCallbacks,
    cities: Vec<IncomingCity<Connection>>,
}
unsafe extern "C" fn incoming(raw: *mut c_void, x: i32, z: i32, sx: i32, sz: i32, object: *mut c_void) {
    let out = &mut *raw.cast::<Incoming<'_>>();
    if let Some(c) = Connection::acquire(object, *out.calls) {
        out.cities.push(IncomingCity {
            position: (x, z),
            size: (sx, sz),
            connections: vec![c],
        });
    }
}
unsafe extern "C" fn occupant(raw: *mut c_void, token: u64) {
    (*raw.cast::<Vec<u64>>()).push(token);
}
impl ConnectionHost<Connection> for Host {
    fn drive_side(&self) -> bool {
        self.drive_side
    }
    fn other_city_established(&mut self, c: &Connection) -> bool {
        unsafe { (self.other_established)(self.context, c.raw()) }
    }
    fn adjacent_connections(&mut self) -> Vec<IncomingCity<Connection>> {
        let mut out = Incoming {
            calls: &self.connections,
            cities: vec![],
        };
        unsafe { (self.adjacent)(self.context, (&mut out as *mut Incoming<'_>).cast(), incoming) };
        out.cities
    }
    fn points_to_this_city(&mut self, c: &Connection) -> bool {
        unsafe { (self.points_here)(self.context, c.raw()) }
    }
    fn network_exists(&mut self, n: u32) -> bool {
        unsafe { (self.network_exists)(self.context, n) }
    }
    fn find_connection(&mut self, c: PendingChange) -> bool {
        unsafe { (self.find)(self.context, c.conn_type, c.x, c.z) }
    }
    fn create_connection(&mut self, c: PendingChange, direction: bool) {
        unsafe { (self.change)(self.context, c.conn_type, c.x, c.z, true, direction) }
    }
    fn delete_connection(&mut self, c: PendingChange) {
        unsafe { (self.change)(self.context, c.conn_type, c.x, c.z, false, false) }
    }
    fn traffic_present(&mut self, (x, z): (i32, i32)) -> bool {
        unsafe { (self.present)(self.context, x, z, 0) }
    }
    fn power_present(&mut self, (x, z): (i32, i32)) -> bool {
        unsafe { (self.present)(self.context, x, z, 1) }
    }
    fn pole_present(&mut self, (x, z): (i32, i32)) -> bool {
        unsafe { (self.present)(self.context, x, z, 2) }
    }
    fn demolish(&mut self, cells: &[(i32, i32)], layer: u32) {
        let cells: Vec<_> = cells.iter().map(|&(x, z)| [x, z]).collect();
        unsafe { (self.cells)(self.context, 0, cells.as_ptr(), cells.len() as u32, layer) }
    }
    fn dezone(&mut self, cells: &[(i32, i32)]) {
        let cells: Vec<_> = cells.iter().map(|&(x, z)| [x, z]).collect();
        unsafe { (self.cells)(self.context, 1, cells.as_ptr(), cells.len() as u32, 0) }
    }
    fn place_segment(&mut self, n: u32, a: (i32, i32), b: (i32, i32)) {
        unsafe { (self.segment)(self.context, n, a.0, a.1, b.0, b.1) }
    }
    fn occupants(&mut self, n: u32, (x, z): (i32, i32)) -> Vec<u64> {
        let mut out = vec![];
        unsafe { (self.occupants)(self.context, n, x, z, (&mut out as *mut Vec<u64>).cast(), occupant) };
        out
    }
    fn connection_word(&mut self, o: u64) -> u32 {
        unsafe { (self.word)(self.context, o) }
    }
    fn set_connection_word(&mut self, o: u64, w: u32) {
        unsafe { (self.set_word)(self.context, o, w) }
    }
    fn post(&mut self, m: u32) {
        unsafe { (self.post)(self.context, m) }
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_reconcile(
    owner: *mut Owner,
    established: bool,
    forced: bool,
    host: *mut Host,
) -> bool {
    let (Some(owner), Some(host)) = (owner.as_ref(), host.as_mut()) else {
        return false;
    };
    let mut operation: City = match owner.0.lock() {
        Ok(c) => c.clone(),
        Err(_) => return false,
    };
    operation.reconcile_connections(established, forced, host);
    let Ok(mut city) = owner.0.lock() else { return false };
    city.pending = operation.pending;
    city.moved = operation.moved;
    true
}
