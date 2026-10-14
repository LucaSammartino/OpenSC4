// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The industrial simulator stub (§2.2, §2.3, §3, §8.2): no agents, no
//! subscriptions, no grids, no host.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, SaveObject};

use super::{
    CLSID_INDUSTRIAL_SIMULATOR, IID_INDUSTRIAL_SIMULATOR, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN,
    INDUSTRIAL_RECORD_VERSION,
};

/// Every field of §2.2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndustrialState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object (§2.3).
    pub destroyed: bool,
    /// The write-only static `0x00B490A4` (§2.2).
    pub static_flag: bool,
}

/// The industrial simulator.
pub struct IndustrialSim {
    token: Cell<u64>,
    s: RefCell<IndustrialState>,
}

impl IndustrialSim {
    /// A constructed (not initialised) simulator, reference count 0.
    pub fn new(token: u64) -> Rc<IndustrialSim> {
        Rc::new(IndustrialSim {
            token: Cell::new(token),
            s: RefCell::new(IndustrialState::default()),
        })
    }

    /// `QueryInterface` (§1): its own IID, `1`, `cIGZMessageTarget2` and
    /// `cIGZSerializable`.
    pub fn query_interface(iid: u32) -> bool {
        matches!(
            iid,
            IID_UNKNOWN | IID_INDUSTRIAL_SIMULATOR | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
        )
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn snapshot(&self) -> IndustrialState {
        *self.s.borrow()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut IndustrialState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// §2.3 *[compat]*: as the commercial simulator's `Release`.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        self.s.borrow_mut().destroyed = true;
        0
    }

    /// `Init` (§2.2): sets `initialized` and the write-only static; returns
    /// true.
    pub fn init(&self) -> bool {
        let mut s = self.s.borrow_mut();
        s.initialized = true;
        s.static_flag = true;
        true
    }

    /// `Shutdown`: returns true. [question] (IN1-adjacent) The spec gives
    /// only "AL (1)" (§9); the literal reading for a stub whose only state
    /// is `initialized` is that `Shutdown` clears it.
    pub fn shutdown(&self) -> bool {
        self.s.borrow_mut().initialized = false;
        true
    }

    /// `DoMessage` (§3): reads the type and returns true.
    pub fn do_message(&self, _msg_type: u32) -> bool {
        true
    }
}

impl SaveObject for IndustrialSim {
    fn class_id(&self) -> u32 {
        CLSID_INDUSTRIAL_SIMULATOR
    }

    /// §8.2: `01 00`.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(INDUSTRIAL_RECORD_VERSION);
        out.error() == 0
    }

    /// §8.2: false unless the version is exactly 1.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        matches!(inp.get_u16(), Some(INDUSTRIAL_RECORD_VERSION)) && inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
