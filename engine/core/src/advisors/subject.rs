// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AdviceSubject` (§2.5, §12.4) and its record (§11.4).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::system::Shared;
use super::{clsid, iid, SubjectInit, SUBJECT_RECORD_VERSION};

/// The subject's fields (§2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SubjectState {
    /// (x, z) in city cells.
    pub location: Option<(f32, f32)>,
    /// The object (an `IGZUnknown`, held).
    pub object: Option<u64>,
    pub data1: u32,
    pub data2: u32,
}

/// One advice subject: a small refcounted value.
pub struct AdviceSubject {
    shared: Rc<Shared>,
    serial: u32,
    token: Cell<u64>,
    s: RefCell<SubjectState>,
}

impl AdviceSubject {
    pub(crate) fn new(shared: &Rc<Shared>) -> Rc<AdviceSubject> {
        let s = Rc::new(AdviceSubject {
            shared: shared.clone(),
            serial: shared.next_serial(),
            token: Cell::new(0),
            s: RefCell::new(SubjectState::default()),
        });
        s.token.set(shared.created(s.clone()));
        s
    }

    /// A subject filled from `init` (the location unclamped, the object
    /// held).
    pub(crate) fn from_init(shared: &Rc<Shared>, init: &SubjectInit) -> Rc<AdviceSubject> {
        let s = AdviceSubject::new(shared);
        if let Some((x, z)) = init.location {
            s.set_location(x, z);
        }
        if init.object != 0 {
            s.set_object(Some(init.object));
        }
        s.set_data1(init.data1);
        s.set_data2(init.data2);
        s
    }

    pub fn serial(&self) -> u32 {
        self.serial
    }

    /// The shim's identity of the object (written for references).
    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn state(&self) -> SubjectState {
        *self.s.borrow()
    }

    /// `QueryInterface`: `cISC4AdviceSubject`, `cIGZSerializable`, `1`.
    pub fn query_interface(iid_: u32) -> bool {
        matches!(iid_, iid::UNKNOWN | iid::SUBJECT | iid::SERIALIZABLE)
    }

    /// Slot 3 `SetLocation(x, z)` (§2.5): stores the pair, then clamps it to
    /// `[0, cellCountX] × [0, cellCountZ]` *[compat]*. True.
    pub fn set_location(&self, x: f32, z: f32) -> bool {
        let (cx, cz) = self.shared.host.cell_counts();
        let clamp = |v: f32, hi: f32| {
            let v = if v < 0.0 { 0.0 } else { v };
            if v > hi {
                hi
            } else {
                v
            }
        };
        self.s.borrow_mut().location = Some((clamp(x, cx as f32), clamp(z, cz as f32)));
        true
    }

    /// Slots 4 / 5: 0 without a location.
    pub fn location_x(&self) -> f32 {
        self.s.borrow().location.map_or(0.0, |l| l.0)
    }
    pub fn location_z(&self) -> f32 {
        self.s.borrow().location.map_or(0.0, |l| l.1)
    }
    /// Slot 6.
    pub fn has_location(&self) -> bool {
        self.s.borrow().location.is_some()
    }

    /// Slot 7 `SetObject` (AddRef; the previous object is released). True.
    pub fn set_object(&self, object: Option<u64>) -> bool {
        let h = &self.shared.host;
        if let Some(o) = object {
            h.add_ref(o);
        }
        let old = std::mem::replace(&mut self.s.borrow_mut().object, object);
        if let Some(o) = old {
            h.release(o);
        }
        true
    }

    /// Slot 8 `GetObjectA` (not AddRef'd).
    pub fn object(&self) -> Option<u64> {
        self.s.borrow().object
    }

    pub fn data1(&self) -> u32 {
        self.s.borrow().data1
    }
    /// Slot 10: a replacement returns true (§12.4).
    pub fn set_data1(&self, v: u32) -> bool {
        self.s.borrow_mut().data1 = v;
        true
    }
    pub fn data2(&self) -> u32 {
        self.s.borrow().data2
    }
    pub fn set_data2(&self, v: u32) -> bool {
        self.s.borrow_mut().data2 = v;
        true
    }

    /// Slot 13 `Clone`: a new subject with the same fields (the object held
    /// again).
    pub fn clone_subject(&self) -> Rc<AdviceSubject> {
        let st = self.state();
        let c = AdviceSubject::new(&self.shared);
        if let Some(o) = st.object {
            self.shared.host.add_ref(o);
        }
        *c.s.borrow_mut() = st;
        c
    }
}

impl std::fmt::Debug for AdviceSubject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdviceSubject#{} {:?}", self.serial, self.state())
    }
}

impl Drop for AdviceSubject {
    fn drop(&mut self) {
        if let Some(o) = self.s.get_mut().object.take() {
            self.shared.host.release(o);
        }
        let token = self.token.get();
        if token != 0 {
            self.shared.host.object_dropped(token);
        }
    }
}

impl SaveObject for AdviceSubject {
    fn class_id(&self) -> u32 {
        clsid::SUBJECT
    }

    /// §11.4: `u16 4`, `u8 hasLocation`, `[f32 x, f32 z]`, `u32 data1`,
    /// `u32 data2`, `ref object`.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let st = self.state();
        out.put_u16(SUBJECT_RECORD_VERSION);
        out.put_bool(st.location.is_some());
        if let Some((x, z)) = st.location {
            out.put_f32(x);
            out.put_f32(z);
        }
        out.put_u32(st.data1);
        out.put_u32(st.data2);
        let r = st.object.map_or(RefValue::Null, |t| self.shared.host.object_ref(t));
        out.put_ref(&r);
        out.error() == 0
    }

    /// §11.4: any version other than 4 → false. The fields are stored as
    /// read (no clamp) [choice]; the object is read with IID 1.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let h = &self.shared.host;
        let Some(v) = inp.get_u16() else { return false };
        if v != SUBJECT_RECORD_VERSION {
            return false;
        }
        let Some(has) = inp.get_bool() else { return false };
        let location = if has {
            let (Some(x), Some(z)) = (inp.get_f32(), inp.get_f32()) else {
                return false;
            };
            Some((x, z))
        } else {
            None
        };
        let (Some(d1), Some(d2)) = (inp.get_u32(), inp.get_u32()) else {
            return false;
        };
        let Some(r) = inp.get_ref_as(iid::UNKNOWN) else {
            return false;
        };
        let object = if r.is_null() {
            None
        } else {
            h.resolve_ref(&r, iid::UNKNOWN)
        };
        let old = {
            let mut s = self.s.borrow_mut();
            let old = s.object.take();
            *s = SubjectState {
                location,
                object,
                data1: d1,
                data2: d2,
            };
            old
        };
        if let Some(o) = old {
            h.release(o);
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
