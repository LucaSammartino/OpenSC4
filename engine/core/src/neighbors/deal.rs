// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbour deal (§5): a monthly contract with an adjacent city, its
//! flags (§5.2), status reports (§5.3), `Clone` (§5.5) and record (§5.6).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::simulator::NeighborsSimulator;
use super::{deal_type, flag, status, NeighborsHost, CLSID_NEIGHBOR_DEAL};
use crate::property::PropertyHolder;
use crate::stream::{InStream, OutStream, SaveObject};

/// The record version written (§5.6).
pub const DEAL_RECORD_VERSION: u16 = 5;

/// The initial value of the four dates (§5.1). *[choice]* (NB1) The
/// original default-constructs them from the wall clock (`0x0091307C`:
/// `time`, `localtime`, then `Jday`), so its value depends on the day the
/// deal is built; the core uses 0. The date survives only in fields nothing
/// reads before overwriting them.
pub const DEFAULT_DATE: u32 = 0;

/// A deal's state (§5.1), also the snapshot form of a deal the region holds
/// (`GetLocalDeals`, §14.1 step 7).
#[derive(Clone, Debug, PartialEq)]
pub struct DealData {
    pub deal_type: u32,
    /// `regionCellOtherSide`: the neighbour.
    pub other_side: (i32, i32),
    /// `regionCellCitySide`: "our" city in this deal's perspective.
    pub city_side: (i32, i32),
    pub amount: i32,
    pub inc_amount: i32,
    pub inc_amount2: i32,
    pub last_amount: i32,
    pub last_inc_amount: i32,
    pub amount_used: i32,
    pub cost: i64,
    pub start_date: u32,
    pub expiration_date: u32,
    pub current_date: u32,
    pub suspension_date: u32,
    pub flags: u32,
    pub props: PropertyHolder,
}

impl Default for DealData {
    fn default() -> DealData {
        DealData {
            deal_type: 0,
            other_side: (0, 0),
            city_side: (0, 0),
            amount: 0,
            inc_amount: 0,
            inc_amount2: 0,
            last_amount: 0,
            last_inc_amount: 0,
            amount_used: 0,
            cost: 0,
            start_date: DEFAULT_DATE,
            expiration_date: DEFAULT_DATE,
            current_date: DEFAULT_DATE,
            suspension_date: DEFAULT_DATE,
            flags: flag::INITIAL,
            props: PropertyHolder::new(),
        }
    }
}

/// A call on a foreign deal's `cISC4NeighborDeal` (§15.4 NG4): the
/// simulation drives a deal `AddDeal` kept through the deal's own methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DealCall {
    SetType(u32),
    SetOtherSide(i32, i32),
    SetCitySide(i32, i32),
    SetLastAmount(i32),
    SetLastIncAmount(i32),
    SetIncAmount(i32),
    SetIncAmount2(i32),
    SetAmountUsed(i32),
    SetStartDate(u32),
    SetExpirationDate(u32),
    SetCurrentDate(u32),
    SetSuspensionDate(u32),
    SetAmount(i32),
    UpdateCost,
    UpdateState,
    Cancel,
    Activate,
    SetFlags(u32, bool),
}

/// `cISC4NeighborDeal` (§5).
///
/// A deal is either **ours** (its state below) or **foreign** (§15.4 NG4):
/// a caller's object that `AddDeal` kept as the listed deal. A foreign deal
/// keeps no state of its own: every read goes to the host object through
/// [`NeighborsHost::foreign_deal`] and every write or behaviour through
/// [`NeighborsHost::foreign_deal_call`], so the deal's own code runs and the
/// caller's later calls act on the simulated deal. The reference `AddDeal`
/// took is released ([`NeighborsHost::release_deal`]) when the core object
/// goes, that is when no list, report or caller of the core holds it.
#[derive(Default)]
pub struct NeighborDeal {
    d: RefCell<DealData>,
    /// The simulator that created or adopted the deal: "the current city's
    /// neighbours simulator" of §5.4.
    sim: RefCell<Weak<NeighborsSimulator>>,
    token: Cell<u64>,
    /// Set for a foreign deal: the host that answers for it (held strongly,
    /// so the release at the end always reaches it).
    foreign: RefCell<Option<Rc<dyn NeighborsHost>>>,
}

impl std::fmt::Debug for NeighborDeal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NeighborDeal")
            .field("data", &*self.d.borrow())
            .field("token", &self.token.get())
            .field("foreign", &self.is_foreign())
            .finish()
    }
}

impl Drop for NeighborDeal {
    fn drop(&mut self) {
        if let Some(h) = self.foreign.get_mut().take() {
            h.release_deal(self.token.get());
        }
    }
}

/// `trunc_s64` of a double: truncation toward zero; out-of-range values give
/// the x87 integer indefinite `0x8000000000000000`.
pub(crate) fn trunc_s64(v: f64) -> i64 {
    if v.is_finite() && v > -9.223_372_036_854_776e18 && v < 9.223_372_036_854_776e18 {
        v.trunc() as i64
    } else {
        i64::MIN
    }
}

impl NeighborDeal {
    /// A new deal: Proposed + Silent, everything else 0 (§5.1).
    pub fn new() -> Rc<NeighborDeal> {
        Rc::new(NeighborDeal::default())
    }

    /// A deal holding `data`, linked to no simulator.
    pub fn from_data(data: DealData) -> Rc<NeighborDeal> {
        let d = NeighborDeal::new();
        *d.d.borrow_mut() = data;
        d
    }

    /// A foreign deal (§15.4 NG4): the host object `token`, read and driven
    /// through `host`. The core object holds the one reference the caller's
    /// `AddDeal` took, released through [`NeighborsHost::release_deal`] when
    /// the core object goes.
    pub fn foreign(token: u64, host: &Rc<dyn NeighborsHost>) -> Rc<NeighborDeal> {
        let d = NeighborDeal::new();
        d.token.set(token);
        *d.foreign.borrow_mut() = Some(host.clone());
        d
    }

    /// A host object, not one of ours (NG4).
    pub fn is_foreign(&self) -> bool {
        self.foreign.borrow().is_some()
    }

    fn foreign_host(&self) -> Option<Rc<dyn NeighborsHost>> {
        self.foreign.borrow().clone()
    }

    /// A call on a foreign deal; `None` for ours.
    fn call(&self, c: DealCall) -> Option<bool> {
        let h = self.foreign_host()?;
        Some(h.foreign_deal_call(self.token.get(), c))
    }

    /// The live state of a foreign deal; `None` for ours, or when the host
    /// cannot read it (the stored fields, all initial, answer then).
    fn live(&self) -> Option<DealData> {
        let h = self.foreign_host()?;
        h.foreign_deal(self.token.get())
    }

    /// One field: live for a foreign deal, stored for ours.
    fn get<R>(&self, f: impl FnOnce(&DealData) -> R) -> R {
        match self.live() {
            Some(d) => f(&d),
            None => f(&self.d.borrow()),
        }
    }

    pub(crate) fn attach(&self, sim: &Rc<NeighborsSimulator>) {
        *self.sim.borrow_mut() = Rc::downgrade(sim);
    }

    fn simulator(&self) -> Option<Rc<NeighborsSimulator>> {
        self.sim.borrow().upgrade()
    }

    /// A deal of ours: the token of its host object (a foreign deal keeps
    /// the token it was made with).
    pub fn set_token(&self, token: u64) {
        if !self.is_foreign() {
            self.token.set(token);
        }
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    /// A copy of the whole state (a foreign deal's read through its
    /// interface, with an empty property holder).
    pub fn data(&self) -> DealData {
        self.get(DealData::clone)
    }

    /// Replaces the whole state (test and load helper; no reports; the
    /// stored state only).
    pub fn set_data(&self, data: DealData) {
        *self.d.borrow_mut() = data;
    }

    // ---- plain accessors (§5.4); a foreign deal's through its interface ----

    pub fn deal_type(&self) -> u32 {
        self.get(|d| d.deal_type)
    }
    pub fn set_type(&self, t: u32) {
        if self.call(DealCall::SetType(t)).is_none() {
            self.d.borrow_mut().deal_type = t;
        }
    }
    pub fn other_side(&self) -> (i32, i32) {
        self.get(|d| d.other_side)
    }
    pub fn set_other_side(&self, x: i32, z: i32) {
        if self.call(DealCall::SetOtherSide(x, z)).is_none() {
            self.d.borrow_mut().other_side = (x, z);
        }
    }
    pub fn city_side(&self) -> (i32, i32) {
        self.get(|d| d.city_side)
    }
    pub fn set_city_side(&self, x: i32, z: i32) {
        if self.call(DealCall::SetCitySide(x, z)).is_none() {
            self.d.borrow_mut().city_side = (x, z);
        }
    }
    pub fn amount(&self) -> i32 {
        self.get(|d| d.amount)
    }
    pub fn last_amount(&self) -> i32 {
        self.get(|d| d.last_amount)
    }
    pub fn set_last_amount(&self, v: i32) {
        if self.call(DealCall::SetLastAmount(v)).is_none() {
            self.d.borrow_mut().last_amount = v;
        }
    }
    pub fn last_inc_amount(&self) -> i32 {
        self.get(|d| d.last_inc_amount)
    }
    pub fn set_last_inc_amount(&self, v: i32) {
        if self.call(DealCall::SetLastIncAmount(v)).is_none() {
            self.d.borrow_mut().last_inc_amount = v;
        }
    }
    pub fn inc_amount(&self) -> i32 {
        self.get(|d| d.inc_amount)
    }
    pub fn set_inc_amount(&self, v: i32) {
        if self.call(DealCall::SetIncAmount(v)).is_none() {
            self.d.borrow_mut().inc_amount = v;
        }
    }
    pub fn inc_amount2(&self) -> i32 {
        self.get(|d| d.inc_amount2)
    }
    pub fn set_inc_amount2(&self, v: i32) {
        if self.call(DealCall::SetIncAmount2(v)).is_none() {
            self.d.borrow_mut().inc_amount2 = v;
        }
    }
    pub fn amount_used(&self) -> i32 {
        self.get(|d| d.amount_used)
    }
    pub fn set_amount_used(&self, v: i32) {
        if self.call(DealCall::SetAmountUsed(v)).is_none() {
            self.d.borrow_mut().amount_used = v;
        }
    }
    pub fn cost(&self) -> i64 {
        self.get(|d| d.cost)
    }
    pub fn start_date(&self) -> u32 {
        self.get(|d| d.start_date)
    }
    pub fn set_start_date(&self, v: u32) {
        if self.call(DealCall::SetStartDate(v)).is_none() {
            self.d.borrow_mut().start_date = v;
        }
    }
    pub fn expiration_date(&self) -> u32 {
        self.get(|d| d.expiration_date)
    }
    pub fn set_expiration_date(&self, v: u32) {
        if self.call(DealCall::SetExpirationDate(v)).is_none() {
            self.d.borrow_mut().expiration_date = v;
        }
    }
    pub fn current_date(&self) -> u32 {
        self.get(|d| d.current_date)
    }
    pub fn set_current_date(&self, v: u32) {
        if self.call(DealCall::SetCurrentDate(v)).is_none() {
            self.d.borrow_mut().current_date = v;
        }
    }
    pub fn suspension_date(&self) -> u32 {
        self.get(|d| d.suspension_date)
    }
    pub fn set_suspension_date(&self, v: u32) {
        if self.call(DealCall::SetSuspensionDate(v)).is_none() {
            self.d.borrow_mut().suspension_date = v;
        }
    }
    pub fn flags(&self) -> u32 {
        self.get(|d| d.flags)
    }
    /// `CheckFlagsAnd(f)`: all bits of `f` set.
    pub fn check_flags_and(&self, f: u32) -> bool {
        self.flags() & f == f
    }
    /// `CheckFlagsOr(f)`: any bit of `f` set.
    pub fn check_flags_or(&self, f: u32) -> bool {
        self.flags() & f != 0
    }
    pub fn is_silent(&self) -> bool {
        self.check_flags_or(flag::SILENT)
    }
    /// `AsISCPropertyHolder()`.
    pub fn properties(&self) -> PropertyHolder {
        self.d.borrow().props.clone()
    }

    // ---- status reports (§5.3) ----

    /// Posts a status report unless the deal is Silent.
    pub(crate) fn report(self: &Rc<Self>, s: u32) {
        if self.is_silent() {
            return;
        }
        if let Some(sim) = self.simulator() {
            sim.host().post_status(Some(self), s);
        }
    }

    /// Sends a status report (synchronously) unless the deal is Silent.
    fn report_sent(self: &Rc<Self>, sim: &NeighborsSimulator, s: u32) {
        if !self.is_silent() {
            sim.host().send_status(self, s);
        }
    }

    // ---- methods with behaviour (§5.4) ----

    /// `SetAmount(a)`: only if the current city exists, has a neighbours
    /// simulator, and neither Grace nor Probation is set: `amount := a`,
    /// report status 0 (**sent**), `UpdateCost()`. Otherwise ignored
    /// *[compat]*.
    pub fn set_amount(self: &Rc<Self>, a: i32) {
        if self.call(DealCall::SetAmount(a)).is_some() {
            return;
        }
        let Some(sim) = self.simulator() else { return };
        if sim.host().current_city().is_none() {
            return;
        }
        if self.check_flags_or(flag::GRACE | flag::PROBATION) {
            return;
        }
        self.d.borrow_mut().amount = a;
        self.report_sent(&sim, status::CHANGED);
        self.update_cost();
    }

    /// `UpdateCost()`: unless Grace or Probation, `cost :=
    /// trunc_s64(GetDealCost(type, amount, other-side city))` through the
    /// current city's simulator, 1 when that is 0; report status 0 (posted).
    /// *[choice]* (NB5) Without a simulator nothing changes and nothing is
    /// reported; the original (`0x006CF9C0`) checks neither the global city
    /// nor its simulator and crashes.
    pub fn update_cost(self: &Rc<Self>) {
        if self.call(DealCall::UpdateCost).is_some() {
            return;
        }
        if self.check_flags_or(flag::GRACE | flag::PROBATION) {
            return;
        }
        let Some(sim) = self.simulator() else { return };
        let other = self.other_city_with(&sim);
        let (t, amount) = (self.deal_type(), self.amount());
        let mut cost = trunc_s64(sim.get_deal_cost(t, f64::from(amount), other));
        if cost == 0 {
            cost = 1;
        }
        self.d.borrow_mut().cost = cost;
        self.report(status::CHANGED);
    }

    /// `SetFlags(f, on)` (§5.4): when setting, a Grace deal drops `0x38` from
    /// `f`; when the flags changed and the deal is not Silent (after the
    /// change), report status 0 (posted). Returns true.
    pub fn set_flags(self: &Rc<Self>, f: u32, on: bool) -> bool {
        if let Some(r) = self.call(DealCall::SetFlags(f, on)) {
            return r;
        }
        let changed = {
            let mut d = self.d.borrow_mut();
            let before = d.flags;
            if on {
                let mut f = f;
                if before & flag::GRACE != 0 {
                    f &= !flag::FAILURES;
                }
                d.flags |= f;
            } else {
                d.flags &= !f;
            }
            d.flags != before
        };
        if changed {
            self.report(status::CHANGED);
        }
        true
    }

    /// `Cancel()`: Cancelled on, Active off, `SetAmount(0)`.
    pub fn cancel(self: &Rc<Self>) {
        if self.call(DealCall::Cancel).is_some() {
            return;
        }
        self.set_flags(flag::CANCELLED, true);
        self.set_flags(flag::ACTIVE, false);
        self.set_amount(0);
    }

    /// `Activate()` (§5.4, NB2, `0x006CE640`): if Proposed and none of
    /// `0x38`, three calls through [`Self::set_flags`]: `(Active, true)`,
    /// `(Proposed, false)`, `(Cancelled, false)`, in that order, then true;
    /// else false. Each call that changes the flags of a non-Silent deal
    /// posts a status 0 report, so a Proposed, non-Cancelled, non-Silent deal
    /// posts two. Cancelled is not in `0x38`: a cancelled proposal is
    /// activated too.
    pub fn activate(self: &Rc<Self>) -> bool {
        if let Some(r) = self.call(DealCall::Activate) {
            return r;
        }
        let flags = self.flags();
        if flags & flag::PROPOSED == 0 || flags & flag::FAILURES != 0 {
            return false;
        }
        self.set_flags(flag::ACTIVE, true);
        self.set_flags(flag::PROPOSED, false);
        self.set_flags(flag::CANCELLED, false);
        true
    }

    /// `Copy(src)` (§5.4): type, flags, amount, amountUsed, cost, incAmount,
    /// lastAmount, both region cells and the four dates (into the stored
    /// state: the core copies only into new deals of its own).
    pub fn copy_from(&self, src: &DealData) {
        let mut d = self.d.borrow_mut();
        d.deal_type = src.deal_type;
        d.flags = src.flags;
        d.amount = src.amount;
        d.amount_used = src.amount_used;
        d.cost = src.cost;
        d.inc_amount = src.inc_amount;
        d.last_amount = src.last_amount;
        d.other_side = src.other_side;
        d.city_side = src.city_side;
        d.start_date = src.start_date;
        d.expiration_date = src.expiration_date;
        d.current_date = src.current_date;
        d.suspension_date = src.suspension_date;
    }

    /// `GetCityOnOtherSideOfConnection()`: the regional city at the other-side
    /// region cell, or null.
    pub fn other_city(&self) -> Option<super::CityId> {
        let sim = self.simulator()?;
        self.other_city_with(&sim)
    }

    fn other_city_with(&self, sim: &NeighborsSimulator) -> Option<super::CityId> {
        let (x, z) = self.other_side();
        sim.host().city_at(x, z)
    }

    /// `UpdateState()` (§7.3 step 1); a foreign deal's own (NG4).
    pub fn update_state(self: &Rc<Self>) {
        if self.call(DealCall::UpdateState).is_some() {
            return;
        }
        if let Some(sim) = self.simulator() {
            sim.update_deal_state(self);
        }
    }

    /// `Clone(&out, silent)` (§5.5), through this deal's simulator. Without
    /// one the clone is a plain copy with Silent set as asked.
    pub fn clone_deal(self: &Rc<Self>, silent: bool) -> Rc<NeighborDeal> {
        match self.simulator() {
            Some(sim) => sim.clone_deal_data(&self.data(), silent),
            None => {
                let e = NeighborDeal::new();
                e.copy_from(&self.data());
                e.set_flags(flag::SILENT, silent);
                e
            }
        }
    }

    // ---- persistence (§5.6) ----

    /// §5.6, version 5.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let d = self.d.borrow().clone();
        out.put_u16(DEAL_RECORD_VERSION);
        out.put_u32(d.deal_type);
        out.put_i32(d.other_side.0);
        out.put_i32(d.other_side.1);
        out.put_i32(d.amount);
        out.put_i32(d.amount_used);
        out.put_i64(d.cost);
        out.put_i32(d.flags as i32);
        out.put_i32(d.city_side.0);
        out.put_i32(d.city_side.1);
        out.put_i32(d.inc_amount);
        out.put_i32(d.last_amount);
        // [quirk] amountUsed is written twice.
        out.put_i32(d.amount_used);
        out.put_u32(d.current_date);
        out.put_u32(d.start_date);
        out.put_u32(d.expiration_date);
        let ok = d.props.write(out);
        out.put_u32(d.suspension_date);
        ok && out.error() == 0
    }

    /// §5.6: any version; the suspension date only when `version > 4`; the
    /// second `amountUsed` is the one kept. Returns `!stream.GetError()`
    /// *[inferred]*.
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        let mut d = self.d.borrow().clone();
        let version = inp.get_u16().unwrap_or(0);
        d.deal_type = inp.get_u32().unwrap_or(0);
        d.other_side = (inp.get_i32().unwrap_or(0), inp.get_i32().unwrap_or(0));
        d.amount = inp.get_i32().unwrap_or(0);
        d.amount_used = inp.get_i32().unwrap_or(0);
        d.cost = inp.get_i64().unwrap_or(0);
        d.flags = inp.get_i32().unwrap_or(0) as u32;
        d.city_side = (inp.get_i32().unwrap_or(0), inp.get_i32().unwrap_or(0));
        d.inc_amount = inp.get_i32().unwrap_or(0);
        d.last_amount = inp.get_i32().unwrap_or(0);
        d.amount_used = inp.get_i32().unwrap_or(0);
        d.current_date = inp.get_u32().unwrap_or(0);
        d.start_date = inp.get_u32().unwrap_or(0);
        d.expiration_date = inp.get_u32().unwrap_or(0);
        let mut props = PropertyHolder::new();
        props.read(inp);
        d.props = props;
        if version > 4 {
            d.suspension_date = inp.get_u32().unwrap_or(0);
        }
        *self.d.borrow_mut() = d;
        inp.error() == 0
    }

    /// The deal's commodity (§3).
    pub fn commodity(&self) -> Option<i32> {
        deal_type::commodity(self.deal_type())
    }
}

impl SaveObject for NeighborDeal {
    fn class_id(&self) -> u32 {
        CLSID_NEIGHBOR_DEAL
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        NeighborDeal::write(self, out)
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        NeighborDeal::read(self, inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
