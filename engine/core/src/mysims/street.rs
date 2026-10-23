// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Dedicated, source-only Street owner, from my-street-sim.md §§2–3.
//! No registration or replacement of the existing ordinary-MySim runtime alias.
//! ST2: inherited state and already-owned reference tokens are supplied, never
//! synthesized from ordinary MySim defaults. ST5: valid, returning individual
//! callbacks only; this is not a native allocator, COM object or reentry engine.

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

use crate::stream::{InStream, OutStream};

/// Opaque, nonnull foreign identity. No pointer arithmetic or implied ownership.
pub type Token = NonZeroU64;

/// Valid inherited state. A supplied automaton carries an existing owned ref.
/// No Default implementation: these defaults are not established (ST2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inherited {
    pub state: i32,
    pub automaton: Option<Token>,
    pub table: u32,
    pub script: Option<Token>,
    pub active: u8,
    pub extent: i32,
    pub timer: Token,
    pub category: i32,
}

/// Snapshot locks are released before calling any host primitive. Getters may
/// run on another thread. Only inherited primitives may replace this snapshot.
pub struct InheritedState(RwLock<Inherited>);

impl InheritedState {
    pub fn get(&self) -> Inherited {
        *self.0.read().expect("Street state lock")
    }
    pub fn set(&self, value: Inherited) {
        *self.0.write().expect("Street state lock") = value;
    }
}

/// Distinct supplied identities; not offsets into a portable allocation.
#[derive(Clone, Copy, Debug)]
pub struct Identities {
    pub complete: Token,
    pub message: Token,
    pub street: Token,
}

/// One foreign method per variant, with raw return bits (including ignored ones).
#[derive(Clone, Debug, PartialEq)]
pub enum WordCall {
    AddRef(Token),
    Release(Token),
    CreateLuaTable,
    Shutdown,
    InheritedDestroy,
    ActivateLuaContext,
    Balloons(u32),
    DayEvents,
    Month,
    RemoveTarget(Token, Token),
    AddTarget(Token, Token),
    ConfigureByte(Token, u32),
    ConfigureFlag(Token, u32, u32),
    Untrack(Token, u32, u32),
    Track(Token, Token, u32, u32, u32),
    ControlType(Token),
    ClearControl(u32),
    Select(Token, &'static str),
    PushNumber(Token, f64),
    Set(Token, i32),
    Context(Token, u32),
    Finish(Token, u32),
    IsGroup(Token, u32),
    AutomatonState(Token),
    RandomUniform(Token),
    MessageId(Token),
    MessageData(Token),
    InheritedMessage(Token),
    TickerEnabled(Token),
    Elapsed(Token),
    Restart(Token),
    ControlInit(Token, Token),
    InstallControl(Token, u32),
    GridSample(Token, i32, i32),
    LotDistress(Token),
    LotWealth(Token),
    StringType(Token, u32),
    PushObject(Token, Token, u32),
    DisasterCount,
    DisasterId(Token),
}

/// One pointer-returning foreign method per variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleCall {
    View,
    CurrentControl,
    Descriptor(Token),
    MessagePayload(Token),
    Ticker,
    CreateControl,
    TrafficGlobal,
    PollutionGlobal,
    CrimeGlobal,
    CongestionGrid(Token),
    PollutionGrid(Token, u32),
    CrimeGrid(Token),
    MayorGrid,
    LotManagerGlobal,
    Building(Token),
    AsOccupant(Token),
    Name(Token),
    CreateNameHelper(Token),
    StringTypeInterface,
    PropertyHolder(Token),
    Random,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RealCall {
    TimeRatio(Token, i32, i32, i32),
    GridSample(Token, i32, i32),
}

/// Output of the existing inherited location primitive; retained occupant is
/// distinct from the descriptor input. No ordinary commute algorithm copied.
#[derive(Clone, Copy, Debug)]
pub struct Location {
    pub retained: Option<Token>,
    pub x: f32,
    pub z: f32,
    pub cx: i32,
    pub cz: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect(pub i32, pub i32, pub i32, pub i32);

impl Rect {
    pub fn around(cx: i32, cz: i32, extent: i32) -> Self {
        let h = extent / 2;
        Self(
            cx.wrapping_sub(h),
            cz.wrapping_sub(h),
            cx.wrapping_add(h),
            cz.wrapping_add(h),
        )
    }
    pub fn contains(self, x: i32, z: i32) -> bool {
        x >= self.0 && z >= self.1 && x <= self.2 && z <= self.3
    }
}

/// Only individual primitives, never Attach/Detach/trip-summary callbacks.
/// The methods mirror actual raw AL/full-word channels, unlike MySimHost's
/// bool/aggregate script and location interfaces. Existing stream traits are
/// reused; no separate Street record codec is defined.
pub trait Host {
    fn word(&mut self, call: WordCall) -> u32;
    fn handle(&mut self, call: HandleCall) -> Option<Token>;
    fn qi(&mut self, object: Token, iid: u32, out: &mut Option<Token>) -> u32;
    fn base_init(&mut self, inherited: &InheritedState, arg: u32) -> u32;
    fn create_lua_table(&mut self, inherited: &InheritedState) -> u32;
    fn base_write(&mut self, stream: &mut dyn OutStream) -> u32;
    fn base_read(&mut self, inherited: &InheritedState, stream: &mut dyn InStream) -> u32;
    /// Stream +0x48's raw word; existing error() is a full signed code, not AL.
    fn write_status(&mut self, stream: &mut dyn OutStream) -> u32;
    fn read_status(&mut self, stream: &mut dyn InStream) -> u32;
    fn location(&mut self, occupant: Token) -> Location;
    fn quantize(&mut self, grid: Token, x: f32, z: f32, gx: &mut i32, gz: &mut i32) -> u32;
    fn real(&mut self, call: RealCall) -> f64;
    /// EQ/HQ +0x54/+0x58, with independent, initially-null grid output.
    fn education_grid(&mut self, health: bool, out: &mut Option<Token>) -> u32;
    /// One LotManager rectangle query; elements each carry one retained ref.
    /// Vec storage substitutes portable nodes/sentinel, never a native free.
    fn lots(&mut self, manager: Token, rect: Rect, out: &mut Vec<Token>) -> u32;
    fn funding(&mut self, holder: Token, purpose: u32, funding: &mut f32) -> u32;
    fn disaster(&mut self, index: i32, object: &mut Option<Token>, x: &mut i32, z: &mut i32) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InvalidOutput(&'static str),
    NumericDomain,
}

/// Explicit completion, so an unproved numeric branch cannot masquerade as a
/// complete Street update. Prefix calls and writes are never rolled back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripOutcome {
    NoScript,
    MissingGlobal,
    Complete,
}

/// An empty helper's portable data; its token is supplied fresh and valid.
/// This models the own empty-storage stores, not native allocation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyHelper {
    pub token: Token,
    pub storage: [Option<u8>; 8],
    pub trailing: u32,
}

pub struct StreetSim {
    ids: Identities,
    inherited: InheritedState,
    raw_word: u32,
    tracking: AtomicU32,
    helper: Option<EmptyHelper>,
}

fn al(word: u32) -> bool {
    word as u8 != 0
}

impl StreetSim {
    /// fresh_helper is a newly created, valid helper identity, not a borrowed
    /// automaton. Only its first storage byte is established; remaining bytes
    /// are explicitly unknown (ST2). Native allocation/free is not bound.
    pub fn new(ids: Identities, inherited: Inherited, fresh_helper: Option<Token>, host: &mut dyn Host) -> Self {
        let helper = fresh_helper.map(|token| {
            host.word(WordCall::AddRef(token));
            let mut storage = [None; 8];
            storage[0] = Some(0);
            EmptyHelper {
                token,
                storage,
                trailing: 0,
            }
        });
        Self {
            ids,
            inherited: InheritedState(RwLock::new(inherited)),
            raw_word: 0x0BFA_331E,
            tracking: AtomicU32::new(u32::MAX),
            helper,
        }
    }

    pub fn state(&self) -> Inherited {
        self.inherited.get()
    }
    pub fn tracking(&self) -> u32 {
        self.tracking.load(Ordering::Relaxed)
    }
    pub fn raw_word(&self) -> u32 {
        self.raw_word
    }
    pub fn helper(&self) -> Option<&EmptyHelper> {
        self.helper.as_ref()
    }
    pub fn class_id(&self) -> u32 {
        super::clsid::MY_STREET_SIM
    }

    pub fn init(&self, arg: u32, host: &mut dyn Host) -> u8 {
        if !al(host.base_init(&self.inherited, arg)) {
            return 0;
        }
        if self.state().table == 0 {
            host.create_lua_table(&self.inherited);
        }
        1
    }

    /// Tail-delegated channel; no detach or helper release.
    pub fn shutdown(&self, host: &mut dyn Host) -> u32 {
        host.word(WordCall::Shutdown)
    }

    /// Consume the portable owner once. Return the old identity plus the native
    /// bit-0 free decision, not a fabricated heap free or live native pointer.
    pub fn destroy(self, flags: u32, host: &mut dyn Host) -> (Token, bool) {
        if let Some(helper) = &self.helper {
            host.word(WordCall::Release(helper.token));
        }
        host.word(WordCall::InheritedDestroy);
        (self.ids.complete, flags & 1 != 0)
    }

    pub fn write(&self, stream: &mut dyn OutStream, host: &mut dyn Host) -> u32 {
        host.base_write(stream);
        u32::from(!al(host.write_status(stream)))
    }

    pub fn read(&self, stream: &mut dyn InStream, host: &mut dyn Host) -> u32 {
        host.base_read(&self.inherited, stream);
        u32::from(!al(host.read_status(stream)))
    }

    pub fn day(&self, host: &mut dyn Host) {
        if self.state().automaton.is_some() {
            host.word(WordCall::ActivateLuaContext);
            host.word(WordCall::Balloons(0));
        }
    }
    pub fn day_events(&self, host: &mut dyn Host) {
        if self.state().automaton.is_some() {
            host.word(WordCall::ActivateLuaContext);
            host.word(WordCall::DayEvents);
        }
    }
    pub fn month(&self, host: &mut dyn Host) {
        if self.state().automaton.is_some() {
            host.word(WordCall::Month);
        }
    }

    fn change(&self, f: impl FnOnce(&mut Inherited)) {
        let mut s = self.state();
        f(&mut s);
        self.inherited.set(s);
    }

    pub fn detach(&self, host: &mut dyn Host) -> Result<u8, Unsupported> {
        if let Some(a) = self.state().automaton {
            let mut q = None;
            host.qi(a, 0x8B99_EE7C, &mut q);
            host.word(WordCall::RemoveTarget(a, self.ids.message));
            if let Some(current) = self.state().automaton {
                self.change(|s| s.automaton = None);
                host.word(WordCall::Release(current));
            }
            self.change(|s| s.state = 0);
            if let Some(q) = q {
                host.word(WordCall::Release(q));
            }
        }
        if self.tracking() != u32::MAX {
            let view = host
                .handle(HandleCall::View)
                .ok_or(Unsupported::InvalidOutput("view"))?;
            host.word(WordCall::Untrack(view, self.tracking(), 0));
            self.tracking.store(u32::MAX, Ordering::Relaxed);
        }
        self.change(|s| {
            s.state = 0;
            s.active = 0;
        });
        Ok(1)
    }

    fn number(&self, host: &mut dyn Host, key: &'static str, value: i32) -> Result<(), Unsupported> {
        let l = self.state().script.ok_or(Unsupported::InvalidOutput("script"))?;
        host.word(WordCall::Select(l, key));
        host.word(WordCall::PushNumber(l, f64::from(value)));
        host.word(WordCall::Set(l, -3));
        Ok(())
    }

    pub fn attach(&self, incoming: Option<Token>, host: &mut dyn Host) -> Result<u8, Unsupported> {
        self.detach(host)?;
        let (Some(x), Some(l)) = (incoming, self.state().script) else {
            return Ok(1);
        };
        if self.state().automaton != Some(x) {
            host.word(WordCall::AddRef(x));
            let old = self.state().automaton;
            self.change(|s| s.automaton = Some(x));
            if let Some(old) = old {
                host.word(WordCall::Release(old));
            }
        }
        let a = self.state().automaton.ok_or(Unsupported::InvalidOutput("automaton"))?;
        let mut q = None;
        if al(host.qi(a, 0x8B99_EE7C, &mut q)) {
            let q = q.ok_or(Unsupported::InvalidOutput("true QI null"))?;
            host.word(WordCall::ConfigureByte(q, 255));
            host.word(WordCall::ConfigureFlag(q, 0x0200_0000, 1));
        }
        host.word(WordCall::AddTarget(a, self.ids.message));
        host.word(WordCall::Context(l, self.state().table));
        for (key, value) in [
            ("wealth", 2),
            ("trip_in_transit", 1),
            ("trip_at_location", 0),
            ("trip_at_transit_switch", 0),
            ("trip_idle", 0),
            ("trip_approaching_location", 0),
        ] {
            self.number(host, key, value)?;
        }
        let descriptor = host
            .handle(HandleCall::Descriptor(x))
            .ok_or(Unsupported::InvalidOutput("descriptor"))?;
        for key in [
            "trip_on_bus",
            "trip_on_foot",
            "trip_on_subway",
            "trip_in_car",
            "trip_on_train",
            "trip_on_elevated_train",
            "trip_on_monorail",
        ] {
            self.number(host, key, 0)?;
        }
        self.change(|s| s.category = 9);
        for (group, key, category) in [
            (0x4001, "trip_on_foot", 0),
            (0x420B, "trip_in_car", 1),
            (0x4200, "trip_on_bus", 2),
            (0x4201, "trip_on_train", 3),
            (0x4215, "trip_on_train", 3),
            (0x422C, "trip_on_elevated_train", 7),
            (0x422D, "trip_on_elevated_train", 7),
            (0x422E, "trip_on_monorail", 8),
            (0x422F, "trip_on_monorail", 8),
        ] {
            if !al(host.word(WordCall::IsGroup(descriptor, group))) {
                continue;
            }
            self.number(host, key, 1)?;
            if category == 0 {
                self.change(|s| s.state = 6);
                self.change(|s| s.category = 0);
                if host.word(WordCall::AutomatonState(a)) == 7 {
                    self.number(host, "trip_idle", 1)?;
                }
            } else {
                let random = required(host.handle(HandleCall::Random), "random")?;
                let state = if host.word(WordCall::RandomUniform(random)) < 0x7FFF_FFFF {
                    5
                } else {
                    4
                };
                if category <= 2 {
                    self.change(|s| s.state = state);
                    self.change(|s| s.category = category);
                } else {
                    self.change(|s| s.category = category);
                    self.change(|s| s.state = state);
                }
            }
            break;
        }
        self.number(host, "state", self.state().state)?;
        host.word(WordCall::Finish(l, 1));
        self.update_trip(host)?;
        let view = host
            .handle(HandleCall::View)
            .ok_or(Unsupported::InvalidOutput("view"))?;
        let a = self.state().automaton.ok_or(Unsupported::InvalidOutput("automaton"))?;
        let occupant = host
            .handle(HandleCall::Descriptor(a))
            .ok_or(Unsupported::InvalidOutput("descriptor"))?;
        let tracking = host.word(WordCall::Track(view, occupant, 3, self.raw_word, 0));
        self.tracking.store(tracking, Ordering::Relaxed);
        self.change(|s| s.active = 1);
        if let Some(q) = q {
            host.word(WordCall::Release(q));
        }
        Ok(1)
    }

    pub fn show_location(&self, host: &mut dyn Host) -> Result<(), Unsupported> {
        if let Some(saved) = self.state().automaton {
            host.word(WordCall::AddRef(saved));
            self.attach(Some(saved), host)?;
            host.word(WordCall::Release(saved));
        }
        Ok(())
    }

    pub fn hide_location(&self, host: &mut dyn Host) -> Result<(), Unsupported> {
        let Some(saved) = self.state().automaton else {
            return Ok(());
        };
        host.word(WordCall::AddRef(saved));
        self.detach(host)?;
        let p = host.handle(HandleCall::CurrentControl);
        if let Some(p) = p {
            host.word(WordCall::AddRef(p));
            if host.word(WordCall::ControlType(p)) == 0xCC23_D564 {
                host.word(WordCall::ClearControl(0));
            }
        }
        let current = self.state().automaton;
        if current != Some(saved) {
            host.word(WordCall::AddRef(saved));
            self.change(|s| s.automaton = Some(saved));
            if let Some(old) = current {
                host.word(WordCall::Release(old));
            }
        }
        if let Some(p) = p {
            host.word(WordCall::Release(p));
        }
        host.word(WordCall::Release(saved));
        Ok(())
    }

    pub fn message(&self, m: Token, host: &mut dyn Host) -> Result<u32, Unsupported> {
        match host.word(WordCall::MessageId(m)) {
            0x09D9_2FC3 => {
                if matches!(host.word(WordCall::MessageData(m)), 2 | 3 | 5) {
                    self.detach(host)?;
                }
            }
            0x4BC4_2C48 => {
                if host.handle(HandleCall::MessagePayload(m)) == Some(self.ids.street) {
                    self.update_trip(host)?;
                    host.word(WordCall::Balloons(1));
                }
            }
            0x86AD_10EE if self.state().automaton.is_some() => {
                if let Some(ticker) = host.handle(HandleCall::Ticker) {
                    if al(host.word(WordCall::TickerEnabled(ticker)))
                        && host.word(WordCall::Elapsed(self.state().timer)) > 2
                    {
                        self.update_trip(host)?;
                        host.word(WordCall::Balloons(1));
                        host.word(WordCall::Restart(self.state().timer));
                    }
                }
            }
            _ => {}
        }
        Ok(host.word(WordCall::InheritedMessage(m)))
    }

    pub fn create_tool(&self, host: &mut dyn Host) -> Result<u8, Unsupported> {
        let p = host
            .handle(HandleCall::CreateControl)
            .ok_or(Unsupported::InvalidOutput("control allocation"))?;
        host.word(WordCall::AddRef(p));
        let success = al(host.word(WordCall::ControlInit(p, self.ids.complete)));
        if success {
            host.word(WordCall::InstallControl(p, 2));
        }
        host.word(WordCall::Release(p));
        Ok(u8::from(success))
    }

    fn object_number(&self, host: &mut dyn Host, key: &'static str, object: Token) -> Result<(), Unsupported> {
        let l = self.state().script.ok_or(Unsupported::InvalidOutput("script"))?;
        host.word(WordCall::Select(l, key));
        host.word(WordCall::AddRef(object));
        let factory = host
            .handle(HandleCall::StringTypeInterface)
            .ok_or(Unsupported::InvalidOutput("string type interface"))?;
        let kind = host.word(WordCall::StringType(factory, 1));
        host.word(WordCall::PushObject(l, object, kind));
        host.word(WordCall::Set(l, -3));
        Ok(())
    }

    fn sample(&self, host: &mut dyn Host, grid: Token, location: Location) -> (i32, i32) {
        let (mut gx, mut gz) = (0, 0);
        // Valid written outputs are a precondition even when AL=0 (ST5).
        host.quantize(grid, location.x, location.z, &mut gx, &mut gz);
        (gx, gz)
    }

    fn sample_word(&self, host: &mut dyn Host, grid: Token, location: Location) -> u32 {
        let (gx, gz) = self.sample(host, grid, location);
        host.word(WordCall::GridSample(grid, gx, gz))
    }

    /// Own §3.7 algorithm. Unsupported carries the exact committed prefix; it
    /// is not an ordinary native false return, nor a partial Complete result.
    pub fn update_trip(&self, host: &mut dyn Host) -> Result<TripOutcome, Unsupported> {
        let Some(l) = self.state().script else {
            return Ok(TripOutcome::NoScript);
        };
        host.word(WordCall::Context(l, self.state().table));
        let a = self
            .state()
            .automaton
            .ok_or(Unsupported::InvalidOutput("trip automaton"))?;
        let occupant = host
            .handle(HandleCall::Descriptor(a))
            .ok_or(Unsupported::InvalidOutput("trip descriptor"))?;
        let loc = host.location(occupant);
        let rect = Rect::around(loc.cx, loc.cz, self.state().extent);
        // Ordered short circuit; earlier context/location work is preserved.
        let traffic = host.handle(HandleCall::TrafficGlobal);
        let pollution = if traffic.is_some() {
            host.handle(HandleCall::PollutionGlobal)
        } else {
            None
        };
        let crime = if pollution.is_some() {
            host.handle(HandleCall::CrimeGlobal)
        } else {
            None
        };
        let (Some(traffic), Some(pollution), Some(crime)) = (traffic, pollution, crime) else {
            if let Some(o) = loc.retained {
                host.word(WordCall::Release(o));
            }
            return Ok(TripOutcome::MissingGlobal);
        };
        let g = required(host.handle(HandleCall::CongestionGrid(traffic)), "congestion grid")?;
        let v = self.sample_word(host, g, loc) as u8;
        self.number(host, "trip_traffic_congestion", i32::from(v))?;
        let v = exact_trunc(host.real(RealCall::TimeRatio(traffic, loc.cx, loc.cz, self.state().category)))?;
        self.number(host, "trip_time_ratio", v)?;
        self.number(host, "trip_wait_length", 0)?;
        for (index, key) in [
            (0, "trip_pollution_air"),
            (1, "trip_pollution_water"),
            (2, "trip_pollution_garbage"),
        ] {
            let g = required(
                host.handle(HandleCall::PollutionGrid(pollution, index)),
                "pollution grid",
            )?;
            let value = self.sample_word(host, g, loc) as i16;
            self.number(host, key, i32::from(value))?;
        }
        let g = required(host.handle(HandleCall::CrimeGrid(crime)), "crime grid")?;
        let v = self.sample_word(host, g, loc) as u8;
        self.number(host, "trip_crime", i32::from(v))?;
        let g = required(host.handle(HandleCall::MayorGrid), "mayor grid")?;
        let v = self.sample_word(host, g, loc) as i8;
        self.number(host, "trip_mayor_rating", i32::from(v))?;
        let mut grids = [None; 2];
        for (i, key) in [(0, "trip_eq"), (1, "trip_hq")] {
            host.education_grid(i == 1, &mut grids[i]);
            let grid = required(grids[i], "EQ/HQ grid")?;
            let (gx, gz) = self.sample(host, grid, loc);
            let value = exact_trunc(host.real(RealCall::GridSample(grid, gx, gz)))?;
            self.number(host, key, value)?;
        }
        let global = required(host.handle(HandleCall::LotManagerGlobal), "lot manager global")?;
        let mut manager = None;
        host.qi(global, 0x6819_05BD, &mut manager);
        let manager = required(manager, "lot manager QI")?;
        let mut lots = Vec::new();
        host.lots(manager, rect, &mut lots);
        let mut total = 0u32;
        let mut counts = [0u32; 6];
        let mut selected = None;
        while let Some(lot) = lots.pop() {
            if let Some(building) = host.handle(HandleCall::Building(lot)) {
                total = total.wrapping_add(1);
                let distress = host.word(WordCall::LotDistress(lot)) as i32;
                if distress >= 4 {
                    counts[2] = counts[2].wrapping_add(1);
                } else if distress >= 3 {
                    counts[1] = counts[1].wrapping_add(1);
                } else if distress >= 2 {
                    counts[0] = counts[0].wrapping_add(1);
                }
                let wealth = host.word(WordCall::LotWealth(lot)) as i32;
                if wealth >= 3 {
                    counts[5] = counts[5].wrapping_add(1);
                } else if wealth >= 2 {
                    counts[4] = counts[4].wrapping_add(1);
                } else if wealth >= 1 {
                    counts[3] = counts[3].wrapping_add(1);
                }
                let u = required(host.handle(HandleCall::AsOccupant(building)), "building occupant")?;
                let landmark = al(host.word(WordCall::IsGroup(u, 0x150A)));
                if landmark && selected.is_none() {
                    host.word(WordCall::AddRef(building));
                    selected = Some(building);
                }
            }
            // Pop/unlink precedes release; portable Vec owns the node storage.
            host.word(WordCall::Release(lot));
        }
        for (count, key) in counts.into_iter().zip([
            "trip_distress1_p",
            "trip_distress2_p",
            "trip_distress3_p",
            "trip_wealth_low_p",
            "trip_wealth_med_p",
            "trip_wealth_high_p",
        ]) {
            self.number(host, key, percentage(total, count)?)?;
        }
        self.number(host, "trip_landmark_nearby", i32::from(selected.is_some()))?;
        if let Some(j) = selected {
            let name = required(host.handle(HandleCall::Name(j)), "landmark name")?;
            let helper = required(
                host.handle(HandleCall::CreateNameHelper(name)),
                "landmark string helper",
            )?;
            self.object_number(host, "trip_landmark_name", helper)?;
            let u = required(host.handle(HandleCall::AsOccupant(j)), "landmark occupant")?;
            let holder = required(host.handle(HandleCall::PropertyHolder(u)), "landmark properties")?;
            let mut funding = 0.0f32;
            host.funding(holder, 0xAA59_670C, &mut funding);
            // Exact dyadic product is required, not a generic Rust float cast.
            let value = exact_trunc(f64::from(funding) * 100.0)?;
            self.number(host, "trip_landmark_funding_p", value)?;
        } else {
            let helper = self
                .helper
                .as_ref()
                .ok_or(Unsupported::InvalidOutput("default helper"))?;
            self.object_number(host, "trip_landmark_name", helper.token)?;
            self.number(host, "trip_landmark_funding_p", 0)?;
        }
        self.scan_disasters(rect, host)?;
        host.word(WordCall::Finish(l, 1));
        if let Some(j) = selected {
            host.word(WordCall::Release(j));
        }
        // All nodes were popped; Vec's sentinel/storage is portable, not a host
        // allocation. Cleanup follows finish, then R/HQ/EQ/location order.
        drop(lots);
        host.word(WordCall::Release(manager));
        for grid in grids.into_iter().rev().flatten() {
            host.word(WordCall::Release(grid));
        }
        if let Some(o) = loc.retained {
            host.word(WordCall::Release(o));
        }
        Ok(TripOutcome::Complete)
    }

    fn scan_disasters(&self, rect: Rect, host: &mut dyn Host) -> Result<(), Unsupported> {
        let count = host.word(WordCall::DisasterCount) as i32;
        let (mut fire, mut riot, mut any) = (false, false, false);
        for index in 0..count {
            let (mut object, mut x, mut z) = (None, 0, 0);
            host.disaster(index, &mut object, &mut x, &mut z);
            if rect.contains(x, z) {
                any = true;
                if !riot {
                    riot = host.word(WordCall::DisasterId(required(object, "disaster")?)) == 0xCA38_0B06;
                }
                if !fire {
                    fire = host.word(WordCall::DisasterId(required(object, "disaster")?)) == 0x621C_DA33;
                }
            }
            if let Some(d) = object {
                host.word(WordCall::Release(d));
            }
            if fire && riot && any {
                break;
            }
        }
        for (key, value) in [
            ("trip_fire_nearby", fire),
            ("trip_riot_nearby", riot),
            ("trip_any_disaster_nearby", any),
        ] {
            self.number(host, key, i32::from(value))?;
        }
        Ok(())
    }
}

fn required(value: Option<Token>, name: &'static str) -> Result<Token, Unsupported> {
    value.ok_or(Unsupported::InvalidOutput(name))
}

/// ST3 bounded normal domain: exact binary32 inputs in signed-dword range.
/// They are also exact at x87 PC24/53/64; truncation is independent of RC.
/// Nonfinite, extended-precision-only and out-of-range values are Unsupported.
fn exact_trunc(value: f64) -> Result<i32, Unsupported> {
    if !value.is_finite() || f64::from(value as f32) != value || !(-2147483648.0..2147483648.0).contains(&value) {
        return Err(Unsupported::NumericDomain);
    }
    Ok(value.trunc() as i32)
}

/// Qualify only finite normal percentages whose truncation is invariant across
/// the native precision/rounding choices. For 1<=t<=4096 the denominator,
/// reciprocal, binary32 spill and two ordered products each have relative
/// error <=2^-23 at PC>=24, including directed rounding. The conservative
/// combined envelope 2^-19 also covers the binary64 evaluation here. If the
/// entire envelope has one truncation, that integer is proved without assuming
/// universal f64/x87 equivalence. Zero products are exact. ST3 remains open.
fn percentage(total: u32, count: u32) -> Result<i32, Unsupported> {
    if count == 0 && total <= 4096 {
        return Ok(0);
    }
    if total == 0 || total > 4096 || count > total {
        return Err(Unsupported::NumericDomain);
    }
    let epsilon = f64::from_bits(0x3F1A_36E2_EB1C_432D);
    let nominal = (f64::from(count) / (f64::from(total) + epsilon)) * 100.0;
    let low = (nominal * (1.0 - 1.0 / 524288.0)).trunc();
    let high = (nominal * (1.0 + 1.0 / 524288.0)).trunc();
    let q = (1.0 / (f64::from(total) + epsilon)) as f32;
    let product = (f64::from(count) * f64::from(q)) * f64::from(100.0f32);
    if low == high && product.trunc() == low {
        Ok(product.trunc() as i32)
    } else {
        Err(Unsupported::NumericDomain)
    }
}

#[cfg(test)]
mod tests;
