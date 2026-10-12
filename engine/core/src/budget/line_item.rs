// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4LineItem` (§3.3, §8, §11.3).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::{msg, BudgetHost, Department, CLSID_LINE_ITEM};

/// Line types (§3.3).
pub const TYPE_EXPENSE: u32 = 0;
pub const TYPE_INCOME: u32 = 1;
/// Record version (§11.3).
pub const LINE_ITEM_RECORD_VERSION: u16 = 4;

#[derive(Clone, Debug)]
struct LineData {
    id: u32,
    name: Vec<u8>,
    name_group: u32,
    name_instance: u32,
    name_needs_resolve: bool,
    secondary_info: i64,
    full: i64,
    current: i64,
    income: i64,
    display_flags: u32,
    is_local: bool,
    item_type: u32,
}

impl Default for LineData {
    fn default() -> Self {
        LineData {
            id: 0,
            name: Vec::new(),
            name_group: 0,
            name_instance: 0,
            name_needs_resolve: false,
            secondary_info: 0,
            full: 0,
            current: 0,
            income: 0,
            display_flags: 5, // ShowLineItem | bit 4
            is_local: false,
            item_type: TYPE_EXPENSE,
        }
    }
}

/// A line item. Shared (`Rc`); its owner is a weak back-reference.
pub struct LineItem {
    host: Rc<dyn BudgetHost>,
    token: Cell<u64>,
    d: RefCell<LineData>,
    owner: RefCell<Weak<Department>>,
}

impl LineItem {
    /// A fresh line item (as created by its class factory). `token` is the
    /// host identity of the object fronting it.
    pub fn new(host: Rc<dyn BudgetHost>, token: u64) -> Rc<LineItem> {
        Rc::new(LineItem {
            host,
            token: Cell::new(token),
            d: RefCell::new(LineData::default()),
            owner: RefCell::new(Weak::new()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn owner(&self) -> Option<Rc<Department>> {
        self.owner.borrow().upgrade()
    }

    pub(crate) fn set_owner(&self, owner: Option<&Rc<Department>>) {
        *self.owner.borrow_mut() = owner.map(Rc::downgrade).unwrap_or_default();
    }

    pub(crate) fn init(&self, id: u32, is_local: bool) {
        let mut d = self.d.borrow_mut();
        d.id = id;
        d.is_local = is_local;
    }

    fn notify_owner(&self, msg_type: u32) {
        if let Some(o) = self.owner() {
            o.notify(msg_type, 0);
        }
    }

    // ---- cISC4LineItem (SDK order) ----

    pub fn id(&self) -> u32 {
        self.d.borrow().id
    }

    pub fn item_type(&self) -> u32 {
        self.d.borrow().item_type
    }

    pub fn set_type(&self, t: u32) -> bool {
        self.d.borrow_mut().item_type = t;
        true
    }

    /// `GetName`: resolves a name loaded from a save first (as `SetName`).
    pub fn name(&self) -> Vec<u8> {
        let (resolve, g, i) = {
            let d = self.d.borrow();
            (d.name_needs_resolve, d.name_group, d.name_instance)
        };
        if resolve {
            self.set_name(g, i);
        }
        self.d.borrow().name.clone()
    }

    /// `SetName(group, instance)`: `group == 0` → the display name of building
    /// type `instance`; otherwise the LTEXT string.
    pub fn set_name(&self, group: u32, instance: u32) -> bool {
        let name = if group == 0 {
            self.host.building_type_name(instance)
        } else {
            self.host.ltext(group, instance)
        };
        let mut d = self.d.borrow_mut();
        d.name_group = group;
        d.name_instance = instance;
        d.name = name;
        d.name_needs_resolve = false;
        true
    }

    pub fn name_ids(&self) -> (u32, u32) {
        let d = self.d.borrow();
        (d.name_group, d.name_instance)
    }

    pub fn secondary_info(&self) -> i64 {
        self.d.borrow().secondary_info
    }

    pub fn set_secondary_info(&self, v: i64) -> bool {
        self.d.borrow_mut().secondary_info = v;
        true
    }

    pub fn full_expenses(&self) -> i64 {
        self.d.borrow().full
    }

    pub fn current_expenses(&self) -> i64 {
        self.d.borrow().current
    }

    pub(crate) fn set_current(&self, v: i64) {
        self.d.borrow_mut().current = v;
    }

    /// `SetFullExpenses(v)` (§8): `full := v`; a non-local line's current
    /// becomes `trunc(v × owner.p)` (no rounding, no clamp, Income lines too);
    /// then the owner posts IdealMonthlyFundingChanged.
    pub fn set_full_expenses(&self, v: i64) -> bool {
        let owner = self.owner();
        {
            let mut d = self.d.borrow_mut();
            d.full = v;
            if !d.is_local {
                // [choice] single precision, like the rest of the percentage math;
                // without an owner the current amount is left unchanged.
                if let Some(o) = &owner {
                    d.current = (v as f32 * o.funding_percentage()) as i64;
                }
            }
        }
        self.notify_owner(msg::IDEAL_MONTHLY_FUNDING_CHANGED);
        true
    }

    pub fn add_to_full_expenses(&self, v: i64) -> bool {
        let full = self.full_expenses();
        self.set_full_expenses(full.wrapping_add(v))
    }

    pub fn income(&self) -> i64 {
        self.d.borrow().income
    }

    /// `SetIncome(v)`: `income := max(0, v)`.
    pub fn set_income(&self, v: i64) -> bool {
        self.d.borrow_mut().income = v.max(0);
        self.notify_owner(msg::IDEAL_MONTHLY_FUNDING_CHANGED);
        true
    }

    /// `AddToIncome(v)`: no clamp.
    pub fn add_to_income(&self, v: i64) -> bool {
        {
            let mut d = self.d.borrow_mut();
            d.income = d.income.wrapping_add(v);
        }
        self.notify_owner(msg::IDEAL_MONTHLY_FUNDING_CHANGED);
        true
    }

    pub fn display_flags(&self) -> u32 {
        self.d.borrow().display_flags
    }

    pub fn set_display_flag(&self, flag: u32, on: bool) -> bool {
        let mut d = self.d.borrow_mut();
        if on {
            d.display_flags |= flag;
        } else {
            d.display_flags &= !flag;
        }
        true
    }

    pub fn is_local(&self) -> bool {
        self.d.borrow().is_local
    }
}

impl SaveObject for LineItem {
    fn class_id(&self) -> u32 {
        CLSID_LINE_ITEM
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        let d = self.d.borrow().clone();
        let owner = match self.owner() {
            Some(o) => RefValue::Object(o as Rc<dyn SaveObject>),
            None => RefValue::Null,
        };
        out.put_u16(LINE_ITEM_RECORD_VERSION);
        out.put_str(&d.name);
        out.put_i64(d.secondary_info);
        out.put_i64(d.full);
        out.put_i64(d.current);
        out.put_i64(d.income);
        out.put_u32(d.id);
        out.put_u32(d.display_flags);
        out.put_bool(d.is_local);
        out.put_u16(d.item_type as u16);
        out.put_ref(&owner);
        out.put_u32(d.name_instance);
        out.put_u32(d.name_group);
        out.error() == 0
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        let r = (|| -> Option<(LineData, RefValue)> {
            if inp.get_u16()? != LINE_ITEM_RECORD_VERSION {
                return None; // §11.3: exactly version 4
            }
            let mut d = LineData {
                name: inp.get_str()?,
                ..Default::default()
            };
            d.secondary_info = inp.get_i64()?;
            d.full = inp.get_i64()?;
            d.current = inp.get_i64()?;
            d.income = inp.get_i64()?;
            d.id = inp.get_u32()?;
            d.display_flags = inp.get_u32()?;
            d.is_local = inp.get_bool()?;
            d.item_type = u32::from(inp.get_u16()?);
            let owner = inp.get_ref()?;
            d.name_instance = inp.get_u32()?;
            d.name_group = inp.get_u32()?;
            // §11.3: re-resolve on the next GetName. [choice] Not when both
            // LTEXT IDs are 0: there is nothing to resolve; keep the stored name.
            d.name_needs_resolve = d.name_group != 0 || d.name_instance != 0;
            Some((d, owner))
        })();
        let Some((d, owner)) = r else { return false };
        *self.d.borrow_mut() = d;
        match &owner {
            RefValue::Object(_) => self.set_owner(owner.object::<Department>().as_ref()),
            RefValue::Host { token, .. } => self.host.release(*token), // not a department: drop it
            RefValue::Null => self.set_owner(None),
        }
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
