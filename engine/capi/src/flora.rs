// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flora simulator's C ABI (fc4.h "flora", docs/specs/flora.md).
//!
//! Two objects:
//!
//! - [`fc4_flora_registry`]: the process-wide flora type registry (§3.2). The
//!   host creates one per process and hands it to every simulator. It owns the
//!   core [`FloraRegistry`] and, for each static record the game can see, the
//!   game-layout block the host built for it (docs/abi/layouts.md "Static
//!   flora record"): `GetStaticFloraData` returns that block. The host keeps
//!   one reference on every block for the life of the process (layouts.md:
//!   no game Release ever sees a count of 1 or less), so blocks are never
//!   freed here either.
//! - [`fc4_flora`]: one `cISC4FloraSimulator` (the core [`FloraSim`]); the
//!   host implements [`fc4_flora_host`] (every `FloraHost` method, in the
//!   trait's order).
//!
//! Tokens are opaque host identities (fc4.h): occupants, blocks, exemplars,
//! segments, segment record streams. 0 is the null object.
//!
//! # Threads
//!
//! The game may call the interface from a second thread while the main
//! thread is inside a day or a month (HANDOFF §5.1). The core keeps its state
//! in `RefCell`s and shares the registry through an `Rc`, so:
//!
//! - every call that reaches the core (lifecycle, messages, slots 5, 6 and
//!   15-24, persistence, diagnostics) runs under the registry's process-wide
//!   re-entrant lock: one thread at a time, and the thread that holds it may
//!   re-enter (a host callback that ends in another flora call);
//! - the registry getters of slots 7-14 never take that lock: they read a
//!   snapshot (types, keys, families, blocks) behind its own `Mutex`, rebuilt
//!   after every call that can change the registry. A getter therefore never
//!   waits for a month in progress.
//!
//! Re-entrancy (one thread): a message delivered while a call is running is
//! queued and handled when the outermost call returns (as in flammability);
//! other re-entrant calls run and are counted
//! (`fc4_flora_state::reentrant_calls`).
//!
//! Counters (`fc4_flora_state`, the status line): this layer sees every host
//! call the core makes, so it counts them by phase (SimNewMonth, SimNewDay,
//! anything else): planted, spread, aged, reborn, burnt, cleared, refused.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

use fc4_core::flora::{
    msg, occupant_type, prop, CellRect, CommandParam, FloraHost, FloraOccupant, FloraRandom, FloraRecord,
    FloraRegistry, FloraRng, FloraSim, Message, OccupantFilter, SegmentId, SharedRegistry, StreamId, Vec3,
    FLORA_EXEMPLAR_TYPE,
};
use fc4_core::property::PropertyHolder;
use fc4_core::props::u32_of;
use fc4_core::stream::{RefValue, ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

// ---- ABI structs ------------------------------------------------------------------------------------

/// A static flora record's values (fc4.h `fc4_flora_record`, §3.3), in the
/// order of the game layout.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_flora_record {
    pub type_id: u32,
    pub next_type: u32,
    pub seed_type: u32,
    pub water: u8,
    pub flags: u8,
    pub reserved: [u8; 2],
    /// The 14 floats of property `0x083DC35B`, in order.
    pub floats: [f32; 14],
    pub cluster_radius: f32,
    pub cluster_height: f32,
    /// `[elevation index][moisture type]`.
    pub table: [u8; 256],
    pub unnamed: u32,
    pub family_count: u32,
    pub plop_cost: i64,
    /// `family_count` family IDs (borrowed for the call; NULL when 0).
    pub families: *const u32,
}

impl Default for fc4_flora_record {
    fn default() -> Self {
        fc4_flora_record {
            type_id: 0,
            next_type: 0,
            seed_type: 0,
            water: 0,
            flags: 0,
            reserved: [0; 2],
            floats: [0.0; 14],
            cluster_radius: 0.0,
            cluster_height: 0.0,
            table: [0; 256],
            unnamed: 0,
            family_count: 0,
            plop_cost: 0,
            families: std::ptr::null(),
        }
    }
}

impl fc4_flora_record {
    /// The values of a core record; `families` stays NULL (the host's blocks
    /// carry a null family vector).
    fn of(r: &FloraRecord) -> fc4_flora_record {
        fc4_flora_record {
            type_id: r.type_id,
            next_type: r.next_type,
            seed_type: r.seed_type.get(),
            water: u8::from(r.water),
            flags: r.flags,
            reserved: [0; 2],
            floats: r.floats(),
            cluster_radius: r.cluster_radius,
            cluster_height: r.cluster_height,
            table: r.table,
            unnamed: r.unnamed,
            family_count: r.families.len() as u32,
            plop_cost: r.plop_cost,
            families: std::ptr::null(),
        }
    }

    /// A core record with these values.
    ///
    /// SAFETY: `families` is NULL or valid for `family_count` values.
    pub(crate) unsafe fn core_record(&self) -> FloraRecord {
        let mut r = FloraRecord::new(self.type_id);
        r.next_type = self.next_type;
        r.seed_type.set(self.seed_type);
        r.water = self.water != 0;
        for (i, v) in self.floats.iter().enumerate() {
            if let Some(f) = r.float_mut(i) {
                *f = *v;
            }
        }
        r.cluster_radius = self.cluster_radius;
        r.cluster_height = self.cluster_height;
        r.table = self.table;
        r.unnamed = self.unnamed;
        r.flags = self.flags;
        r.plop_cost = self.plop_cost;
        if !self.families.is_null() && self.family_count > 0 {
            r.families = std::slice::from_raw_parts(self.families, self.family_count as usize).to_vec();
        }
        r
    }
}

/// The values of a flora occupant (fc4.h `fc4_flora_occupant`, §3.4).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_flora_occupant {
    pub flora_type: u32,
    pub birth_date: u32,
    pub last_seeding_date: u32,
    /// Signed byte (`GetState` sign-extends).
    pub state: i8,
    pub orientation: u8,
    pub reserved: [u8; 2],
}

/// One command parameter (fc4.h `fc4_flora_param`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_flora_param {
    pub as_u32: u32,
    pub as_f32: f32,
}

/// A message for `fc4_flora_do_message` (fc4.h `fc4_flora_message`, §5).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_flora_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (insert / remove occupant).
    pub occupant: u64,
    /// The cheat's command line (UTF-8, not terminated; NULL = none).
    pub text: *const u8,
    pub text_len: usize,
    /// The command parameters (PlaceFlora; NULL = none).
    pub params: *const fc4_flora_param,
    pub param_count: usize,
}

impl Default for fc4_flora_message {
    fn default() -> Self {
        fc4_flora_message {
            msg_type: 0,
            data1: 0,
            data2: 0,
            data3: 0,
            occupant: 0,
            text: std::ptr::null(),
            text_len: 0,
            params: std::ptr::null(),
            param_count: 0,
        }
    }
}

/// Diagnostics and the status line's counters (fc4.h `fc4_flora_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_flora_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub enabled: u8,
    pub propagation: u8,
    pub propagation_forced: u8,
    pub preserve: u8,
    pub registry_loaded: u8,
    pub pad: [u8; 2],
    pub size_x: f32,
    pub size_z: f32,
    pub cells_x: i32,
    pub cells_z: i32,
    pub today: u32,
    pub period: u32,
    pub blocks_per_month: u32,
    pub wild_seeds_per_month: u32,
    pub iterator_count: i32,
    pub iterator_index: i32,
    /// The burnt list's length.
    pub burnt_list: u32,
    pub burn_accumulator: f32,
    pub burnt_removal_accumulator: f32,
    pub burn_rate: f32,
    pub burnt_removal_step: f32,
    pub elevation_scale: f32,
    // ---- the registry (process-wide) ----
    pub map_len: u32,
    pub type_count: u32,
    pub family_count: u32,
    pub block_count: u32,
    // ---- counters since creation (host calls the core made) ----
    /// Flora occupants created (`create_flora_occupant` gave one) / refused
    /// by the factory (gave none).
    pub created: u32,
    pub create_failed: u32,
    /// Successful inserts outside the monthly pass (tool, lot developer,
    /// cheats) / during it (seeds and wild seeds).
    pub planted: u32,
    pub spread: u32,
    /// Inserts the occupant manager refused.
    pub refused: u32,
    /// Stage changes to maturing or mature (§6.7.2).
    pub aged: u32,
    /// Rebirths (birth := today, state 0; §6.7.2).
    pub reborn: u32,
    /// Stage changes to dying (low score: removed).
    pub dying: u32,
    /// Flora burnt by a fire (§6.9).
    pub burnt: u32,
    /// Removals during the daily pass (burnt trees cleared, §6.8) / during
    /// the monthly pass (dying, cluster merges) / anywhere else (demolition,
    /// buildings, networks, terrain).
    pub cleared: u32,
    pub removed_month: u32,
    pub removed_other: u32,
    /// `MoveOccupant` calls (terrain re-seats).
    pub moved: u32,
    /// Blocks visited by the monthly pass and the flora found in them.
    pub blocks_visited: u32,
    pub occupants_visited: u32,
    pub months: u32,
    pub days: u32,
    /// Records handed to occupants without a block (none built).
    pub records_without_block: u32,
    pub reentrant_calls: u32,
    pub queued_messages: u32,
    /// Calls that waited for another thread's call to end.
    pub lock_waits: u32,
    pub reserved: u32,
}

/// The game-layout record builder (fc4.h `fc4_flora_registry_host`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_flora_registry_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// Builds a game-layout record (docs/abi/layouts.md) with the values of
    /// `rec`, holding `exemplar` (the reference passes to the record; 0 =
    /// none), and keeps one reference on it for the life of the process.
    /// Returns its address, or 0 (the host then releases `exemplar`).
    pub create_block: Option<unsafe extern "C" fn(U, *const fc4_flora_record, u64) -> u64>,
    /// A record's seed type changed (§6.11): rewrite it in the block.
    pub set_block_seed: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// Releases an exemplar reference that no record kept.
    pub release_exemplar: Option<unsafe extern "C" fn(U, u64)>,
}

/// One flora exemplar (`flora_exemplars`): the visitor takes the NEW props
/// (or NULL) and the exemplar reference (0 = none).
pub type fc4_flora_exemplar_visitor = Option<
    unsafe extern "C" fn(
        ctx: *mut c_void,
        type_id: u32,
        group: u32,
        instance: u32,
        props: *mut fc4_props,
        exemplar: u64,
    ),
>;

/// Everything the flora simulator asks of the game (fc4.h `fc4_flora_host`;
/// one callback per `FloraHost` method, in the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_flora_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // city, clock, resources
    pub city_exists: Option<unsafe extern "C" fn(U) -> bool>,
    pub city_size: Option<unsafe extern "C" fn(U, *mut f32, *mut f32)>,
    pub city_cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub city_established: Option<unsafe extern "C" fn(U) -> bool>,
    pub sim_date_number: Option<unsafe extern "C" fn(U) -> u32>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    /// Calls `visit` for each exemplar of resource type `0x6534284A`, in the
    /// resource manager's order (a host may skip those whose ExemplarType is
    /// not `0x0F`: the core ignores them).
    pub flora_exemplars: Option<unsafe extern "C" fn(U, fc4_flora_exemplar_visitor, *mut c_void)>,
    pub obliterated_segment: Option<unsafe extern "C" fn(U) -> u64>,
    // framework
    pub register_app_shutdown_callback: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub register_command: Option<unsafe extern "C" fn(U, u32, *const u8, usize)>,
    pub unregister_command: Option<unsafe extern "C" fn(U, u32)>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const u8, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub register_cheat: Option<unsafe extern "C" fn(U, u32, *const u8, usize)>,
    pub unregister_cheat: Option<unsafe extern "C" fn(U, u32)>,
    pub post: Option<unsafe extern "C" fn(U, u32, u32)>,
    // terrain, lots, weather
    pub terrain_initialized: Option<unsafe extern "C" fn(U) -> bool>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub sea_level: Option<unsafe extern "C" fn(U) -> f32>,
    pub face_normal: Option<unsafe extern "C" fn(U, f32, f32, *mut f32)>,
    pub lot_savvy_altitude: Option<unsafe extern "C" fn(U, f32, f32, *mut bool) -> f32>,
    pub moisture_type: Option<unsafe extern "C" fn(U, f32, f32) -> u32>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub set_occupant_position: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    pub occupant_in_manager: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_occupant_in_manager: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub network_flags: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub flora_occupant: Option<unsafe extern "C" fn(U, u64, *mut fc4_flora_occupant) -> bool>,
    pub set_flora_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub set_birth_date: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub set_last_seeding_date: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub set_flora_state: Option<unsafe extern "C" fn(U, u64, i8)>,
    pub set_flora_orientation: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// `SetStaticFloraData(block)` (0 = null record).
    pub set_flora_record: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub post_occupant_change: Option<unsafe extern "C" fn(U, u64)>,
    /// A new flora occupant with one reference for the caller (0 = none).
    pub create_flora_occupant: Option<unsafe extern "C" fn(U) -> u64>,
    pub init_occupant: Option<unsafe extern "C" fn(U, u64)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    /// The occupant (as `iid`) of a reference read from a save (`token`, the
    /// reference the stream handed over, which passes to this call); 0 = none
    /// (the host then dropped it).
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // occupant manager
    pub has_occupant_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub insert_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub move_occupant: Option<unsafe extern "C" fn(U, u64)>,
    /// `occ_type` = 0 with `blocking` = the blocking filter (§6.4.1).
    pub occupants_in_bbox:
        Option<unsafe extern "C" fn(U, *const f32, *const f32, u32, bool, fc4_token_callback, *mut c_void)>,
    /// `rect` NULL = null corners (the whole city).
    pub occupants_in_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32, fc4_token_callback, *mut c_void)>,
    pub occupants_in_manager_cells: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, *mut c_void)>,
    // obliteration segment (§10.3, ABI 30)
    /// `(segment, type, group, instance, write)`: `QueryInterface` for
    /// `cISC4DBSegment`, then `OpenOStream(key, &s, true)` (`write`) or
    /// `OpenIStream(key, &s)`; the stream token, 0 = not open.
    pub segment_open: Option<unsafe extern "C" fn(U, u64, u32, u32, u32, bool) -> u64>,
    /// `CloseOStream` / `CloseIStream`, then both references released.
    pub segment_close: Option<unsafe extern "C" fn(U, u64)>,
    pub stream_put_u16: Option<unsafe extern "C" fn(U, u64, u16) -> bool>,
    pub stream_put_u32: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// False when the call fails (`*out` untouched).
    pub stream_get_u16: Option<unsafe extern "C" fn(U, u64, *mut u16) -> bool>,
    pub stream_get_u32: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    /// `GetError()` is non-zero.
    pub stream_error: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `(occupant, stream)`: the occupant's `cIGZSerializable::Write` / `Read`.
    pub write_occupant: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub read_occupant: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_flora_record>() == 360);
    assert!(std::mem::size_of::<fc4_flora_occupant>() == 16);
    assert!(std::mem::size_of::<fc4_flora_param>() == 8);
    assert!(std::mem::size_of::<fc4_flora_message>() == if cfg!(target_pointer_width = "32") { 40 } else { 56 });
    assert!(std::mem::size_of::<fc4_flora_state>() == 180);
    assert!(std::mem::size_of::<fc4_flora_registry_host>() == if cfg!(target_pointer_width = "32") { 20 } else { 40 });
    assert!(std::mem::size_of::<fc4_flora_host>() == if cfg!(target_pointer_width = "32") { 260 } else { 520 });
};

// ---- the process-wide re-entrant lock ---------------------------------------------------------------

/// A re-entrant lock: one thread at a time, and the owner may re-enter.
#[derive(Default)]
pub(crate) struct ReentrantLock {
    state: Mutex<(Option<ThreadId>, u32)>,
    cv: Condvar,
    waits: AtomicU32,
}

pub(crate) struct LockGuard<'a>(&'a ReentrantLock);

impl ReentrantLock {
    pub(crate) fn lock(&self) -> LockGuard<'_> {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut waited = false;
        loop {
            match s.0 {
                None => {
                    *s = (Some(me), 1);
                    break;
                }
                Some(owner) if owner == me => {
                    s.1 += 1;
                    break;
                }
                Some(_) => {
                    if !waited {
                        waited = true;
                        self.waits.fetch_add(1, Ordering::Relaxed);
                    }
                    s = self.cv.wait(s).unwrap_or_else(|e| e.into_inner());
                }
            }
        }
        LockGuard(self)
    }

    pub(crate) fn waits(&self) -> u32 {
        self.waits.load(Ordering::Relaxed)
    }
}

impl Drop for LockGuard<'_> {
    fn drop(&mut self) {
        let mut s = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        s.1 = s.1.saturating_sub(1);
        if s.1 == 0 {
            s.0 = None;
            self.0.cv.notify_one();
        }
    }
}

// ---- the registry -----------------------------------------------------------------------------------

/// A record the game can see and its block.
struct Block {
    rec: Rc<FloraRecord>,
    block: u64,
    /// The seed type last written into the block.
    seed: u32,
}

/// What the getters of slots 7-14 read (any thread).
#[derive(Clone, Debug, Default)]
struct Snapshot {
    loaded: bool,
    /// Every map key (ascending, key 0 included) and its block (0 = none).
    records: BTreeMap<u32, u64>,
    /// The type array (registration order).
    types: Vec<u32>,
    families: BTreeMap<u32, Vec<u32>>,
    block_count: u32,
}

/// The registry handle's contents.
///
/// Threads: every field but `lock` and `snapshot` (the `Rc` and the
/// `RefCell`s) is only touched by the thread that holds `lock`.
pub(crate) struct RegistryInner {
    reg: SharedRegistry,
    host: fc4_flora_registry_host,
    blocks: RefCell<Vec<Block>>,
    /// Block index by record address.
    index: RefCell<HashMap<usize, usize>>,
    /// Exemplar references waiting for their type's record, by type.
    stash: RefCell<BTreeMap<u32, u64>>,
    lock: ReentrantLock,
    snapshot: Mutex<Snapshot>,
}

// SAFETY: see the type's documentation: the non-thread-safe fields are only
// used under `lock`, which serialises the threads (its mutex orders the
// accesses).
unsafe impl Send for RegistryInner {}
unsafe impl Sync for RegistryInner {}

macro_rules! rcb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the registry promised valid callbacks.
        match $s.host.$f { Some(f) => unsafe { f($s.host.user $(, $arg)*) }, None => $default }
    };
}

fn rc_key(r: &Rc<FloraRecord>) -> usize {
    Rc::as_ptr(r) as usize
}

impl RegistryInner {
    fn release_exemplar(&self, token: u64) {
        if token != 0 {
            rcb!(self, release_exemplar, (), token);
        }
    }

    /// Keeps `token` for the record of `type_id` (the first one wins); a
    /// second exemplar of the same type is released.
    fn stash(&self, type_id: u32, token: u64) {
        if token == 0 {
            return;
        }
        let dup = {
            let mut s = self.stash.borrow_mut();
            if let std::collections::btree_map::Entry::Vacant(e) = s.entry(type_id) {
                e.insert(token);
                false
            } else {
                true
            }
        };
        if dup {
            self.release_exemplar(token);
        }
    }

    /// An exemplar from the load (§4.1 step 2): kept only when the core will
    /// register it (ExemplarType `0x0F`).
    fn stash_loaded(&self, h: &PropertyHolder, token: u64) {
        if u32_of(h, prop::EXEMPLAR_TYPE) == Some(FLORA_EXEMPLAR_TYPE) {
            self.stash(u32_of(h, prop::TYPE_ID).unwrap_or(0), token);
        } else {
            self.release_exemplar(token);
        }
    }

    fn block_of(&self, r: &Rc<FloraRecord>) -> Option<u64> {
        let i = *self.index.borrow().get(&rc_key(r))?;
        self.blocks.borrow().get(i).map(|b| b.block)
    }

    fn add_block(&self, rec: Rc<FloraRecord>, block: u64, seed: u32) {
        let mut blocks = self.blocks.borrow_mut();
        self.index.borrow_mut().insert(rc_key(&rec), blocks.len());
        blocks.push(Block { rec, block, seed });
    }

    /// Builds the blocks of new records, rewrites changed seed types,
    /// releases unused exemplars and refreshes the snapshot. Runs with the
    /// lock held, outside any core call's borrows.
    fn sync(&self) {
        let records = self.reg.borrow().records();
        let mut todo: Vec<(Rc<FloraRecord>, u64)> = Vec::new();
        {
            let index = self.index.borrow();
            let mut stash = self.stash.borrow_mut();
            for r in records {
                if !index.contains_key(&rc_key(&r)) {
                    let token = stash.remove(&r.type_id).unwrap_or(0);
                    todo.push((r, token));
                }
            }
        }
        for (r, token) in todo {
            let values = fc4_flora_record::of(&r);
            let block = rcb!(self, create_block, 0, &values, token);
            if block == 0 && token != 0 && self.host.create_block.is_none() {
                self.release_exemplar(token);
            }
            let seed = r.seed_type.get();
            self.add_block(r, block, seed);
        }
        let leftovers: Vec<u64> = std::mem::take(&mut *self.stash.borrow_mut()).into_values().collect();
        for t in leftovers {
            self.release_exemplar(t);
        }
        let mut seeds: Vec<(u64, u32)> = Vec::new();
        for b in self.blocks.borrow_mut().iter_mut() {
            let s = b.rec.seed_type.get();
            if s != b.seed {
                b.seed = s;
                if b.block != 0 {
                    seeds.push((b.block, s));
                }
            }
        }
        for (block, s) in seeds {
            rcb!(self, set_block_seed, (), block, s);
        }
        self.refresh_snapshot();
    }

    fn refresh_snapshot(&self) {
        let snap = {
            let reg = self.reg.borrow();
            let mut records = BTreeMap::new();
            for k in reg.keys() {
                let b = reg.get(k).and_then(|r| self.block_of(&r)).unwrap_or(0);
                records.insert(k, b);
            }
            let families = reg
                .family_ids()
                .into_iter()
                .map(|f| (f, reg.family(f).map(<[u32]>::to_vec).unwrap_or_default()))
                .collect();
            Snapshot {
                loaded: reg.is_loaded(),
                records,
                types: reg.types().to_vec(),
                families,
                block_count: self.blocks.borrow().len() as u32,
            }
        };
        *self.snapshot.lock().unwrap_or_else(|e| e.into_inner()) = snap;
    }

    fn snap<R>(&self, f: impl FnOnce(&Snapshot) -> R) -> R {
        f(&self.snapshot.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// Opaque handle (fc4.h `fc4_flora_registry`).
#[repr(C)]
pub struct fc4_flora_registry {
    _private: [u8; 0],
}

fn registry<'a>(p: *const fc4_flora_registry) -> Option<&'a Arc<RegistryInner>> {
    // SAFETY: documented precondition: NULL or a live fc4_flora_registry*.
    unsafe { (p as *const Arc<RegistryInner>).as_ref() }
}

/// Creates the process-wide registry (empty, not loaded). The host table is
/// copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_create(host: *const fc4_flora_registry_host) -> *mut fc4_flora_registry {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_flora_registry_host>() => {
            let inner = RegistryInner {
                reg: FloraRegistry::shared(),
                host: *h,
                blocks: RefCell::new(Vec::new()),
                index: RefCell::new(HashMap::new()),
                stash: RefCell::new(BTreeMap::new()),
                lock: ReentrantLock::default(),
                snapshot: Mutex::new(Snapshot::default()),
            };
            Box::into_raw(Box::new(Arc::new(inner))) as *mut fc4_flora_registry
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the registry handle (the simulators keep theirs alive). The
/// blocks are the host's: nothing is released.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_destroy(p: *mut fc4_flora_registry) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Arc<RegistryInner>);
        let _g = b.lock.lock();
        // The stash is empty outside a call; release anything left anyway.
        let left: Vec<u64> = std::mem::take(&mut *b.stash.borrow_mut()).into_values().collect();
        for t in left {
            b.release_exemplar(t);
        }
        drop(_g);
        drop(b);
    });
}

/// Slot 7 `GetStaticFloraData(type)`: the block of the type's record
/// (borrowed; 0 = unknown type, or no block). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_record(p: *const fc4_flora_registry, type_id: u32) -> u64 {
    match registry(p) {
        Some(r) => guard(0, || r.snap(|s| s.records.get(&type_id).copied().unwrap_or(0))),
        None => 0,
    }
}

/// Slot 11 `IsFloraTypeValid(type)`: in the map (true for 0 after an `Init`,
/// §3.2; false again after an exemplar-change reload until the next `Init`,
/// §5, FQ14). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_is_type_valid(p: *const fc4_flora_registry, type_id: u32) -> bool {
    match registry(p) {
        Some(r) => guard(false, || r.snap(|s| s.records.contains_key(&type_id))),
        None => false,
    }
}

/// Slot 12 `IsFloraFamilyTypeValid(id)`: a family, or a registered type. Any
/// thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_is_family_valid(p: *const fc4_flora_registry, id: u32) -> bool {
    match registry(p) {
        Some(r) => guard(false, || {
            r.snap(|s| s.families.contains_key(&id) || s.records.contains_key(&id))
        }),
        None => false,
    }
}

/// Slot 9 `GetAllFloraTypes(buffer, count)`: NULL buffer: `*count := N`,
/// returns N; else copies `min(*count, N)` types (registration order), stores
/// that in `*count` and returns it. N excludes key 0. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_types_into(
    p: *const fc4_flora_registry,
    buffer: *mut u32,
    count: *mut u32,
) -> u32 {
    let Some(r) = registry(p) else { return 0 };
    let Some(c) = count.as_mut() else { return 0 };
    guard(0, || {
        r.snap(|s| {
            let n = s.types.len() as u32;
            if buffer.is_null() {
                *c = n;
                return n;
            }
            let k = (*c).min(n);
            // SAFETY: documented: `buffer` holds `*count` values.
            std::slice::from_raw_parts_mut(buffer, k as usize).copy_from_slice(&s.types[..k as usize]);
            *c = k;
            k
        })
    })
}

/// The lists of slots 8, 10 and 13, through `cb` (called after the snapshot
/// is released, so it may call back): `which` 0 = the type array
/// (registration order), 1 = every map key (ascending, key 0 included), 2 =
/// the family IDs (ascending). Returns the number of values. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_list(
    p: *const fc4_flora_registry,
    which: u32,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let Some(r) = registry(p) else { return 0 };
    let values: Vec<u32> = guard(Vec::new(), || {
        r.snap(|s| match which {
            0 => s.types.clone(),
            1 => s.records.keys().copied().collect(),
            2 => s.families.keys().copied().collect(),
            _ => Vec::new(),
        })
    });
    if let Some(f) = cb {
        for &v in &values {
            f(ctx, u64::from(v));
        }
    }
    values.len() as u32
}

/// Slot 14's list: the types of `family` through `cb` (after the snapshot is
/// released). Returns their count (0 for an unknown family). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_family(
    p: *const fc4_flora_registry,
    family: u32,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let Some(r) = registry(p) else { return 0 };
    let values: Vec<u32> = guard(Vec::new(), || {
        r.snap(|s| s.families.get(&family).cloned().unwrap_or_default())
    });
    if let Some(f) = cb {
        for &v in &values {
            f(ctx, u64::from(v));
        }
    }
    values.len() as u32
}

/// Registry sizes (any thread): map entries, types, families, blocks; the
/// return value is "loaded" (the first `Init` of the process ran).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_registry_sizes(
    p: *const fc4_flora_registry,
    map_len: *mut u32,
    types: *mut u32,
    families: *mut u32,
    blocks: *mut u32,
) -> bool {
    let Some(r) = registry(p) else { return false };
    let s = guard(Snapshot::default(), || r.snap(Snapshot::clone));
    if let Some(v) = map_len.as_mut() {
        *v = s.records.len() as u32;
    }
    if let Some(v) = types.as_mut() {
        *v = s.types.len() as u32;
    }
    if let Some(v) = families.as_mut() {
        *v = s.families.len() as u32;
    }
    if let Some(v) = blocks.as_mut() {
        *v = s.block_count;
    }
    s.loaded
}

// ---- host adapter -----------------------------------------------------------------------------------

/// The phase of the message being handled (for the counters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Other,
    Day,
    Month,
}

#[derive(Clone, Copy, Debug, Default)]
struct Counters {
    created: u32,
    create_failed: u32,
    planted: u32,
    spread: u32,
    refused: u32,
    aged: u32,
    reborn: u32,
    dying: u32,
    burnt: u32,
    cleared: u32,
    removed_month: u32,
    removed_other: u32,
    moved: u32,
    blocks_visited: u32,
    occupants_visited: u32,
    months: u32,
    days: u32,
    records_without_block: u32,
}

struct CHost {
    h: fc4_flora_host,
    reg: Arc<RegistryInner>,
    phase: Cell<Phase>,
    /// The occupant whose birth date was set last (a rebirth sets it just
    /// before the state).
    last_birth: Cell<u64>,
    counters: RefCell<Counters>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

fn bump(c: &RefCell<Counters>, f: impl FnOnce(&mut Counters)) {
    f(&mut c.borrow_mut());
}

/// The visitor's context: the exemplars so far.
struct ExemplarAcc<'a> {
    out: Vec<(ResKey, Rc<PropertyHolder>)>,
    reg: &'a RegistryInner,
}

unsafe extern "C" fn visit_exemplar(
    ctx: *mut c_void,
    type_id: u32,
    group: u32,
    instance: u32,
    props: *mut fc4_props,
    exemplar: u64,
) {
    // No panic may unwind into the host.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(acc) = (ctx as *mut ExemplarAcc).as_mut() else {
            return;
        };
        match take_props(props) {
            Some(h) => {
                acc.reg.stash_loaded(&h, exemplar);
                acc.out.push((
                    ResKey {
                        type_id,
                        group,
                        instance,
                    },
                    h,
                ));
            }
            None => acc.reg.release_exemplar(exemplar),
        }
    }));
}

impl FloraHost for CHost {
    fn city_exists(&self) -> bool {
        cb!(self, city_exists, false)
    }
    fn city_size(&self) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        cb!(self, city_size, (), &mut x, &mut z);
        (x, z)
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, city_cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn city_established(&self) -> bool {
        cb!(self, city_established, false)
    }
    fn sim_date_number(&self) -> u32 {
        cb!(self, sim_date_number, 0)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn flora_exemplars(&self) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        let mut acc = ExemplarAcc {
            out: Vec::new(),
            reg: &self.reg,
        };
        cb!(
            self,
            flora_exemplars,
            (),
            Some(visit_exemplar),
            &mut acc as *mut ExemplarAcc as *mut c_void
        );
        acc.out
    }
    fn obliterated_segment(&self) -> Option<SegmentId> {
        // The records loaded so far get their blocks before the restore
        // creates occupants (they look their record up by type).
        self.reg.sync();
        opt(cb!(self, obliterated_segment, 0))
    }
    fn register_app_shutdown_callback(&self) {
        cb!(self, register_app_shutdown_callback, ())
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn register_command(&self, id: u32, name: &str) {
        cb!(self, register_command, (), id, name.as_ptr(), name.len())
    }
    fn unregister_command(&self, id: u32) {
        cb!(self, unregister_command, (), id)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(self, add_agent, (), level, name.as_ptr(), name.len(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn acquire_services(&self) {
        cb!(self, acquire_services, ())
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
    }
    fn register_cheat(&self, id: u32, name: &str) {
        cb!(self, register_cheat, (), id, name.as_ptr(), name.len())
    }
    fn unregister_cheat(&self, id: u32) {
        cb!(self, unregister_cheat, (), id)
    }
    fn post(&self, msg_type: u32, data2: u32) {
        if msg_type == msg::FLORA_TYPES_UPDATED {
            // After a (re)load: the new records get their blocks first.
            self.reg.sync();
        }
        cb!(self, post, (), msg_type, data2)
    }
    fn terrain_initialized(&self) -> bool {
        cb!(self, terrain_initialized, false)
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, altitude, 0.0, x, z)
    }
    fn sea_level(&self) -> f32 {
        cb!(self, sea_level, 0.0)
    }
    fn face_normal(&self, x: f32, z: f32) -> Vec3 {
        // §6.4: the output is initialised to 0.
        let mut n = [0.0f32; 3];
        cb!(self, face_normal, (), x, z, n.as_mut_ptr());
        n
    }
    fn lot_savvy_altitude(&self, x: f32, z: f32) -> (f32, bool) {
        let mut on_lot = false;
        let h = cb!(self, lot_savvy_altitude, 0.0, x, z, &mut on_lot);
        (h, on_lot)
    }
    fn moisture_type(&self, x: f32, z: f32) -> u32 {
        // [choice] No weather simulator: moisture 0 (§14 quirk 15).
        cb!(self, moisture_type, 0, x, z)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn occupant_has_property(&self, occ: u64, id: u32) -> bool {
        cb!(self, occupant_has_property, false, occ, id)
    }
    fn occupant_position(&self, occ: u64) -> Vec3 {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, p.as_mut_ptr());
        p
    }
    fn set_occupant_position(&self, occ: u64, pos: Vec3) {
        cb!(self, set_occupant_position, (), occ, pos.as_ptr())
    }
    fn occupant_bounding_box(&self, occ: u64) -> (Vec3, Vec3) {
        let (mut mn, mut mx) = ([0.0f32; 3], [0.0f32; 3]);
        cb!(self, occupant_bounding_box, (), occ, mn.as_mut_ptr(), mx.as_mut_ptr());
        (mn, mx)
    }
    fn occupant_in_manager(&self, occ: u64) -> bool {
        cb!(self, occupant_in_manager, false, occ)
    }
    fn set_occupant_in_manager(&self, occ: u64, on: bool) {
        cb!(self, set_occupant_in_manager, (), occ, on)
    }
    fn network_flags(&self, occ: u64) -> Option<u32> {
        let mut f = 0u32;
        cb!(self, network_flags, false, occ, &mut f).then_some(f)
    }
    fn flora_occupant(&self, occ: u64) -> Option<FloraOccupant> {
        let mut o = fc4_flora_occupant::default();
        cb!(self, flora_occupant, false, occ, &mut o).then_some(FloraOccupant {
            flora_type: o.flora_type,
            birth_date: o.birth_date,
            last_seeding_date: o.last_seeding_date,
            state: o.state,
            orientation: o.orientation,
        })
    }
    fn set_flora_type(&self, occ: u64, flora_type: u32) {
        cb!(self, set_flora_type, (), occ, flora_type)
    }
    fn set_birth_date(&self, occ: u64, day: u32) {
        self.last_birth.set(occ);
        cb!(self, set_birth_date, (), occ, day)
    }
    fn set_last_seeding_date(&self, occ: u64, day: u32) {
        cb!(self, set_last_seeding_date, (), occ, day)
    }
    fn set_flora_state(&self, occ: u64, state: i8) {
        let rebirth = self.last_birth.replace(0) == occ;
        bump(&self.counters, |c| match state {
            0 if rebirth => c.reborn += 1,
            1 | 2 => c.aged += 1,
            3 => c.burnt += 1,
            4 => c.dying += 1,
            _ => {}
        });
        cb!(self, set_flora_state, (), occ, state)
    }
    fn set_flora_orientation(&self, occ: u64, orientation: u32) {
        cb!(self, set_flora_orientation, (), occ, orientation)
    }
    fn set_flora_record(&self, occ: u64, record: Option<Rc<FloraRecord>>) {
        let block = match &record {
            None => 0,
            Some(r) => match self.reg.block_of(r) {
                Some(b) => b,
                None => {
                    // A record without a block yet (normally every record got
                    // one at its registration): build it now.
                    self.reg.sync();
                    self.reg.block_of(r).unwrap_or(0)
                }
            },
        };
        if record.is_some() && block == 0 {
            bump(&self.counters, |c| c.records_without_block += 1);
        }
        cb!(self, set_flora_record, (), occ, block)
    }
    fn post_occupant_change(&self, occ: u64) {
        cb!(self, post_occupant_change, (), occ)
    }
    fn create_flora_occupant(&self) -> Option<u64> {
        let o = opt(cb!(self, create_flora_occupant, 0));
        bump(&self.counters, |c| {
            if o.is_some() {
                c.created += 1;
            } else {
                c.create_failed += 1;
            }
        });
        o
    }
    fn init_occupant(&self, occ: u64) {
        cb!(self, init_occupant, (), occ)
    }
    fn add_ref(&self, occ: u64) {
        cb!(self, add_ref, (), occ)
    }
    fn release(&self, occ: u64) {
        cb!(self, release, (), occ)
    }
    fn object_ref(&self, occ: u64) -> RefValue {
        if occ == 0 {
            RefValue::Null
        } else {
            RefValue::Host {
                token: occ,
                class_id: 0,
            }
        }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => opt(cb!(self, resolve_ref, 0, token, iid)),
            _ => None,
        }
    }
    fn has_occupant_manager(&self) -> bool {
        cb!(self, has_occupant_manager, false)
    }
    fn insert_occupant(&self, occ: u64) -> bool {
        let ok = cb!(self, insert_occupant, false, occ);
        let month = self.phase.get() == Phase::Month;
        bump(&self.counters, |c| match (ok, month) {
            (true, true) => c.spread += 1,
            (true, false) => c.planted += 1,
            (false, _) => c.refused += 1,
        });
        ok
    }
    fn remove_occupant(&self, occ: u64) -> bool {
        let phase = self.phase.get();
        bump(&self.counters, |c| match phase {
            Phase::Day => c.cleared += 1,
            Phase::Month => c.removed_month += 1,
            Phase::Other => c.removed_other += 1,
        });
        cb!(self, remove_occupant, false, occ)
    }
    fn move_occupant(&self, occ: u64) {
        bump(&self.counters, |c| c.moved += 1);
        cb!(self, move_occupant, (), occ)
    }
    fn occupants_in_bbox(&self, min: Vec3, max: Vec3, filter: OccupantFilter) -> Vec<u64> {
        let (t, blocking) = match filter {
            OccupantFilter::Type(t) => (t, false),
            OccupantFilter::Blocking => (0, true),
        };
        collect_tokens(|sink, ctx| {
            cb!(
                self,
                occupants_in_bbox,
                (),
                min.as_ptr(),
                max.as_ptr(),
                t,
                blocking,
                sink,
                ctx
            )
        })
    }
    fn occupants_in_cells(&self, rect: Option<CellRect>, occ_type: u32) -> Vec<u64> {
        let r = rect.map(|(x0, z0, x1, z1)| fc4_cell_rect {
            x1: x0,
            z1: z0,
            x2: x1,
            z2: z1,
        });
        let p = r.as_ref().map_or(std::ptr::null(), |r| r as *const fc4_cell_rect);
        let v = collect_tokens(|sink, ctx| cb!(self, occupants_in_cells, (), p, occ_type, sink, ctx));
        if self.phase.get() == Phase::Month && rect.is_some() && occ_type == occupant_type::FLORA {
            let n = v.len() as u32;
            bump(&self.counters, |c| {
                c.blocks_visited += 1;
                c.occupants_visited += n;
            });
        }
        v
    }
    fn occupants_in_manager_cells(&self, occ_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, occupants_in_manager_cells, (), occ_type, sink, ctx))
    }
    fn segment_open(&self, seg: SegmentId, key: ResKey, write: bool) -> Option<StreamId> {
        opt(cb!(
            self,
            segment_open,
            0,
            seg,
            key.type_id,
            key.group,
            key.instance,
            write
        ))
    }
    fn segment_close(&self, s: StreamId) {
        cb!(self, segment_close, (), s)
    }
    fn stream_put_u16(&self, s: StreamId, v: u16) {
        cb!(self, stream_put_u16, false, s, v);
    }
    fn stream_put_u32(&self, s: StreamId, v: u32) {
        cb!(self, stream_put_u32, false, s, v);
    }
    fn stream_get_u16(&self, s: StreamId) -> Option<u16> {
        let mut v = 0u16;
        cb!(self, stream_get_u16, false, s, &mut v).then_some(v)
    }
    fn stream_get_u32(&self, s: StreamId) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, stream_get_u32, false, s, &mut v).then_some(v)
    }
    fn stream_error(&self, s: StreamId) -> bool {
        cb!(self, stream_error, false, s)
    }
    fn write_occupant(&self, occ: u64, s: StreamId) -> bool {
        cb!(self, write_occupant, false, occ, s)
    }
    fn read_occupant(&self, occ: u64, s: StreamId) -> bool {
        cb!(self, read_occupant, false, occ, s)
    }
}

// ---- simulator handles ------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_flora`).
#[repr(C)]
pub struct fc4_flora {
    _private: [u8; 0],
}

/// The handle. Every field but `reg` is only touched under `reg.lock`.
struct Handle {
    reg: Arc<RegistryInner>,
    host: Rc<CHost>,
    sim: Rc<FloraSim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<Message>>,
    reentrant: Cell<u32>,
    queued: Cell<u32>,
    /// A handled message may have reloaded the registry.
    dirty: Cell<bool>,
}

/// Decrements the call depth when the call ends (also on a panic).
struct Depth<'a>(&'a Cell<u32>);

impl Drop for Depth<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// Messages after which the registry may have changed (§5).
fn may_reload(t: u32) -> bool {
    t == msg::EXEMPLAR_CHANGED || t == msg::CHEAT
}

impl Handle {
    /// Handles one message with its phase set.
    fn handle(&self, m: &Message) -> bool {
        let phase = match m.msg_type {
            msg::SIM_NEW_MONTH => Phase::Month,
            msg::SIM_NEW_DAY => Phase::Day,
            _ => Phase::Other,
        };
        match phase {
            Phase::Month => bump(&self.host.counters, |c| c.months += 1),
            Phase::Day => bump(&self.host.counters, |c| c.days += 1),
            Phase::Other => {}
        }
        if may_reload(m.msg_type) {
            self.dirty.set(true);
        }
        let before = self.host.phase.replace(phase);
        let r = self.sim.do_message(m);
        self.host.phase.set(before);
        r
    }

    /// Runs `f` with the call depth counted (lock held); at the end of the
    /// outermost call the queued messages are handled, then the registry is
    /// synchronised when `sync` (or a handled message) asks for it.
    fn call<R>(&self, sync: bool, f: impl FnOnce(&Handle) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let _depth = Depth(&self.depth);
        let r = f(self);
        if self.depth.get() == 1 {
            loop {
                let next = {
                    let mut q = self.queue.borrow_mut();
                    if q.is_empty() {
                        None
                    } else {
                        Some(q.remove(0))
                    }
                };
                let Some(m) = next else { break };
                self.handle(&m);
            }
            if sync || self.dirty.replace(false) {
                self.reg.sync();
            }
        }
        r
    }
}

fn handle<'a>(p: *const fc4_flora) -> Option<&'a Handle> {
    // SAFETY: documented precondition: NULL or a live fc4_flora*.
    unsafe { (p as *const Handle).as_ref() }
}

/// A call into the core: the process-wide lock, the depth, the queue.
fn sim<R>(p: *const fc4_flora, fallback: R, sync: bool, f: impl FnOnce(&Handle) -> R) -> R {
    match handle(p) {
        Some(h) => guard(fallback, || {
            let _g = h.reg.lock.lock();
            h.call(sync, f)
        }),
        None => fallback,
    }
}

/// SAFETY: `p` NULL (with `n == 0`) or valid for `n` bytes.
unsafe fn text_of(p: *const u8, n: usize) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let bytes = if n == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(p, n)
    };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// SAFETY: as the fields of `fc4_flora_message` document.
unsafe fn message_of(m: &fc4_flora_message) -> Message {
    let params = if m.params.is_null() || m.param_count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(m.params, m.param_count)
            .iter()
            .map(|p| CommandParam {
                as_u32: p.as_u32,
                as_f32: p.as_f32,
            })
            .collect()
    };
    Message {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        occupant: opt(m.occupant),
        text: text_of(m.text, m.text_len),
        params,
    }
}

/// Creates the core object (constructor values, §3.1) over the process-wide
/// `registry`. The host table is copied. `token` is the host identity of the
/// simulator itself; `session_seed` is foundations §6.3's `S`: its own
/// `cRZRandom` (`Seed(0xFFFFFFFF)` in the original, §3.1, §8) takes
/// `derive(S, 0xC990C034)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_create(
    host: *const fc4_flora_host,
    registry_handle: *const fc4_flora_registry,
    token: u64,
    session_seed: u32,
) -> *mut fc4_flora {
    let Some(reg) = registry(registry_handle) else {
        return std::ptr::null_mut();
    };
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_flora_host>() => {
            let _g = reg.lock.lock();
            let chost = Rc::new(CHost {
                h: *h,
                reg: reg.clone(),
                phase: Cell::new(Phase::Other),
                last_birth: Cell::new(0),
                counters: RefCell::new(Counters::default()),
            });
            let rng: Box<dyn FloraRandom> = Box::new(FloraRng::from_session(session_seed));
            let sim = FloraSim::new(chost.clone(), reg.reg.clone(), token, rng);
            Box::into_raw(Box::new(Handle {
                reg: reg.clone(),
                host: chost,
                sim,
                depth: Cell::new(0),
                queue: RefCell::new(Vec::new()),
                reentrant: Cell::new(0),
                queued: Cell::new(0),
                dirty: Cell::new(false),
            })) as *mut fc4_flora
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. A loaded object that was never initialised
/// still holds its burnt list's references: they are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_destroy(p: *mut fc4_flora) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        let reg = b.reg.clone();
        let _g = reg.lock.lock();
        let d = b.sim.snapshot();
        if !d.initialized {
            for o in d.burnt {
                b.host.release(o);
            }
        }
        drop(b);
    });
}

/// `Init` (§4.1): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_init(p: *mut fc4_flora) -> bool {
    sim(p, false, true, |h| h.sim.init())
}

/// `Shutdown` (§4.2): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shutdown(p: *mut fc4_flora) -> bool {
    sim(p, false, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§5): always true. A message delivered during another call is
/// queued and handled when that call returns.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_do_message(p: *mut fc4_flora, m: *const fc4_flora_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = message_of(m);
    sim(p, true, false, |h| {
        if h.depth.get() > 1 {
            h.queued.set(h.queued.get().wrapping_add(1));
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.handle(&msg)
    })
}

/// Slot 5 `AddStaticFloraData(key, exemplar)` with the exemplar's properties
/// (`props`: NEW, taken; NULL = no exemplar → false) and the exemplar itself
/// (`exemplar`: one reference, taken; kept by the record's block, else
/// released). Builds a record (§3.3) and returns slot 6's result.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_add_exemplar(p: *mut fc4_flora, props: *mut fc4_props, exemplar: u64) -> bool {
    let holder = take_props(props);
    let Some(h) = handle(p) else {
        return false;
    };
    sim(p, false, true, |hd| {
        let Some(ex) = holder else {
            h.reg.release_exemplar(exemplar);
            return false;
        };
        h.reg.stash(u32_of(&ex, prop::TYPE_ID).unwrap_or(0), exemplar);
        hd.sim.add_static_flora_data_from(Some(&ex))
    })
}

/// Slot 6 `AddStaticFloraData(record)` with a record the caller built in the
/// game layout: `values` are its fields (families included), `block` its
/// address. False (no reference taken) when the type is already registered;
/// true: registered, and `GetStaticFloraData` returns `block` from now on
/// (the caller takes the registry's reference on it, §11.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_add_record(p: *mut fc4_flora, values: *const fc4_flora_record, block: u64) -> bool {
    let Some(v) = values.as_ref() else { return false };
    let rec = Rc::new(v.core_record());
    sim(p, false, true, |h| {
        if h.sim.is_flora_type_valid(rec.type_id) {
            return false;
        }
        let seed = v.seed_type;
        if !h.sim.add_static_flora_data(rec.clone()) {
            return false;
        }
        h.reg.add_block(rec, block, seed);
        true
    })
}

/// Slot 15 `LocationIsOKForNewFlora(type, x, z, exclude)` (§6.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_location_ok(p: *mut fc4_flora, type_id: u32, x: f32, z: f32, exclude: u64) -> bool {
    sim(p, false, false, |h| h.sim.location_ok(type_id, x, z, opt(exclude)))
}

/// Slot 16 `GetBestFloraTypeForLocation(x, z, &score)` (§6.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_best_type(p: *mut fc4_flora, x: f32, z: f32, score: *mut f32) -> u32 {
    let (t, s) = sim(p, (0, 0.0), false, |h| h.sim.best_type(x, z));
    if let Some(o) = score.as_mut() {
        *o = s;
    }
    t
}

/// Slot 17 `AddNewFloraOccupant(type, pos, force)` (§6.5): the inserted
/// occupant (borrowed) or 0. `pos` (three floats) is rewritten as the
/// original does (§14 quirk 4).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_add_new_occupant(
    p: *mut fc4_flora,
    type_id: u32,
    pos: *mut f32,
    force: bool,
) -> u64 {
    if pos.is_null() {
        return 0;
    }
    let mut v = [*pos, *pos.add(1), *pos.add(2)];
    let r = sim(p, 0, false, |h| {
        h.sim.add_new_flora_occupant(type_id, &mut v, force).unwrap_or(0)
    });
    for (i, x) in v.iter().enumerate() {
        *pos.add(i) = *x;
    }
    r
}

/// Slot 18 `AddNewFloraOccupant(occ)`: the occupant manager's insert.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_add_occupant(p: *mut fc4_flora, occ: u64) -> bool {
    sim(p, false, false, |h| h.sim.add_flora_occupant(occ))
}

/// Slot 19 `DemolishFloraOccupant(occ, flags)` (§6.10.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_demolish_occupant(p: *mut fc4_flora, occ: u64, flags: i32) -> bool {
    sim(p, false, false, |h| h.sim.demolish_flora_occupant(occ, flags))
}

/// Slot 20 `DemolishFloraInCellArea(rect, flags)` (§6.10.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_demolish_cells(p: *mut fc4_flora, rect: *const fc4_cell_rect, flags: i32) -> bool {
    let Some(r) = rect.as_ref() else { return false };
    let cr = (r.x1, r.z1, r.x2, r.z2);
    sim(p, false, false, |h| h.sim.demolish_in_cell_area(cr, flags))
}

/// Slot 21 `DemolishFloraInBoundingBox(min, max, flags)` (§6.10.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_demolish_bbox(
    p: *mut fc4_flora,
    min: *const f32,
    max: *const f32,
    flags: i32,
) -> bool {
    if min.is_null() || max.is_null() {
        return false;
    }
    let mn = [*min, *min.add(1), *min.add(2)];
    let mx = [*max, *max.add(1), *max.add(2)];
    sim(p, false, false, |h| h.sim.demolish_in_bounding_box(mn, mx, flags))
}

/// Slot 23 `KillFloraOccupant(occ)` (§6.9).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_kill(p: *mut fc4_flora, occ: u64) -> bool {
    sim(p, false, false, |h| h.sim.kill_flora_occupant(occ))
}

/// Slot 24 `SaveObliterated(segment)` (§10.3): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_save_obliterated(p: *mut fc4_flora, segment: u64) -> bool {
    sim(p, false, false, |h| h.sim.save_obliterated(segment))
}

/// Diagnostics and counters.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_get_state(p: *const fc4_flora, out: *mut fc4_flora_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle(p) else { return false };
    guard(false, || {
        let _g = h.reg.lock.lock();
        let d = h.sim.snapshot();
        let c = *h.host.counters.borrow();
        let (map_len, type_count, family_count, block_count, loaded) = h.reg.snap(|s| {
            (
                s.records.len() as u32,
                s.types.len() as u32,
                s.families.len() as u32,
                s.block_count,
                s.loaded,
            )
        });
        *o = fc4_flora_state {
            struct_size: std::mem::size_of::<fc4_flora_state>() as u32,
            initialized: u8::from(d.initialized),
            enabled: u8::from(d.enabled),
            propagation: u8::from(d.propagation),
            propagation_forced: u8::from(d.propagation_forced),
            preserve: u8::from(d.preserve),
            registry_loaded: u8::from(loaded),
            pad: [0; 2],
            size_x: d.size_x,
            size_z: d.size_z,
            cells_x: d.cells_x,
            cells_z: d.cells_z,
            today: d.today,
            period: d.period,
            blocks_per_month: d.blocks_per_month,
            wild_seeds_per_month: d.wild_seeds_per_month,
            iterator_count: d.iterator.table.len() as i32,
            iterator_index: d.iterator.index,
            burnt_list: d.burnt.len() as u32,
            burn_accumulator: d.burn_accumulator,
            burnt_removal_accumulator: d.burnt_removal_accumulator,
            burn_rate: d.burn_rate,
            burnt_removal_step: d.burnt_removal_step,
            elevation_scale: d.elevation_scale,
            map_len,
            type_count,
            family_count,
            block_count,
            created: c.created,
            create_failed: c.create_failed,
            planted: c.planted,
            spread: c.spread,
            refused: c.refused,
            aged: c.aged,
            reborn: c.reborn,
            dying: c.dying,
            burnt: c.burnt,
            cleared: c.cleared,
            removed_month: c.removed_month,
            removed_other: c.removed_other,
            moved: c.moved,
            blocks_visited: c.blocks_visited,
            occupants_visited: c.occupants_visited,
            months: c.months,
            days: c.days,
            records_without_block: c.records_without_block,
            reentrant_calls: h.reentrant.get(),
            queued_messages: h.queued.get(),
            lock_waits: h.reg.lock.waits(),
            reserved: 0,
        };
        true
    })
}

/// `Write` (§10.1). Burnt-list occupants go out as `fc4_ref`s with their
/// token (class 0).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_write(p: *const fc4_flora, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§10.1): false for a version other than 2 (nothing more read) or a
/// stream error. Each burnt-list reference is resolved as a `cISC4Occupant`.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_read(p: *mut fc4_flora, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    sim(p, false, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

#[cfg(test)]
mod lock_tests {
    use super::ReentrantLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn reentrant_lock_reenters_on_one_thread_and_excludes_others() {
        let l = Arc::new(ReentrantLock::default());
        let g1 = l.lock();
        let g2 = l.lock(); // same thread: re-enters
        let inside = Arc::new(AtomicBool::new(false));
        let (l2, in2) = (l.clone(), inside.clone());
        let t = std::thread::spawn(move || {
            let _g = l2.lock();
            in2.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!inside.load(Ordering::SeqCst), "the other thread waits");
        drop(g2);
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!inside.load(Ordering::SeqCst), "still held once");
        drop(g1);
        t.join().unwrap();
        assert!(inside.load(Ordering::SeqCst));
        assert_eq!(l.waits(), 1);
    }
}
