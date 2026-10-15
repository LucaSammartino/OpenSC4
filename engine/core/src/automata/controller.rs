// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Controllers (§3.5): the controller base record (version 5), the generator
//! (`0x4A1628C5`) and the attractor (`0x6A1628BA`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::codec::{
    get_curve, get_ref_checked, get_refs, get_time, get_vec3, put_curve, put_refs, put_time, put_vec3,
    ref_lists_equal, refs_equal, ControllerTime, Vec3,
};
use super::script::{ControllerTemplate, TemplateKind};
use super::{
    CLSID_ATTRACTOR, CLSID_GENERATOR, IID_AUTOMATA_CONTROLLER, IID_AUTOMATON, IID_OCCUPANT, IID_SERIALIZABLE,
    IID_UNKNOWN,
};

/// The controller base record's version (§3.5; Read ignores it).
pub const CONTROLLER_BASE_VERSION: u16 = 5;
/// The generator's own version (§3.5; Read requires it).
pub const GENERATOR_VERSION: u16 = 4;
/// The attractor's `u16` after the base (§3.5; read and ignored).
pub const ATTRACTOR_TAG: u16 = 1;
/// Controller flag 8 (§3.2, controller slot 12 `CheckFlag(8)`): a transient
/// controller, never saved; references to it are written as null.
pub const FLAG_TRANSIENT: u32 = 8;
/// The flags of a controller made from a template that names none
/// *[choice]*: 7, the value of almost every saved controller (§3.5).
pub const DEFAULT_FLAGS: u32 = 7;

/// The controller base record (§3.5, version 5).
#[derive(Clone, Debug)]
pub struct ControllerBase {
    /// The version as read; Read ignores it and it is written back unchanged
    /// *[choice]* so the record re-encodes byte-identically (5 for a new
    /// controller).
    pub version: u16,
    /// The occupant the controller is attached to (read with `0x26BB767E`).
    pub occupant: RefValue,
    /// The template name; empty when the controller has no template.
    pub template_name: Vec<u8>,
    pub position: Vec3,
    /// Radius and a second extent *[inferred]*.
    pub radius: f32,
    pub extent: f32,
    /// 0 in the whole corpus.
    pub reserved: u32,
    pub flags: u32,
    /// The automata it currently controls (read with `0xA9B40F05`).
    pub automata: Vec<RefValue>,
}

impl Default for ControllerBase {
    fn default() -> ControllerBase {
        ControllerBase {
            version: CONTROLLER_BASE_VERSION,
            occupant: RefValue::Null,
            template_name: Vec::new(),
            position: [0.0; 3],
            radius: 0.0,
            extent: 0.0,
            reserved: 0,
            flags: 0,
            automata: Vec::new(),
        }
    }
}

impl PartialEq for ControllerBase {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && refs_equal(&self.occupant, &o.occupant)
            && self.template_name == o.template_name
            && self.position == o.position
            && self.radius == o.radius
            && self.extent == o.extent
            && self.reserved == o.reserved
            && self.flags == o.flags
            && ref_lists_equal(&self.automata, &o.automata)
    }
}

impl ControllerBase {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        out.put_ref(&self.occupant);
        out.put_str(&self.template_name);
        put_vec3(out, &self.position);
        out.put_f32(self.radius);
        out.put_f32(self.extent);
        out.put_u32(self.reserved);
        out.put_u32(self.flags);
        put_refs(out, &self.automata);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<ControllerBase> {
        Some(ControllerBase {
            version: inp.get_u16()?,
            occupant: get_ref_checked(inp, IID_OCCUPANT)?,
            template_name: inp.get_str()?,
            position: get_vec3(inp)?,
            radius: inp.get_f32()?,
            extent: inp.get_f32()?,
            reserved: inp.get_u32()?,
            flags: inp.get_u32()?,
            automata: get_refs(inp, IID_AUTOMATON)?,
        })
    }
}

/// What follows (generator) or precedes (attractor) the base record.
#[derive(Clone, Debug, PartialEq)]
pub enum ControllerKind {
    /// `0x4A1628C5`: `u16` 4, the base record, `time` (period), `time`
    /// (second period), `u32` (a day stamp *[inferred]*), `u32` (0–3).
    Generator {
        period: ControllerTime,
        second_period: ControllerTime,
        day_stamp: u32,
        counter: u32,
    },
    /// `0x6A1628BA`: the base record, `u16` (read and ignored; kept as read
    /// *[choice]*, 1 for a new attractor), the attraction curve.
    Attractor { tag: u16, curve: Vec<(f32, f32)> },
}

/// A whole controller record.
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerRecord {
    pub base: ControllerBase,
    pub kind: ControllerKind,
}

impl ControllerRecord {
    /// A new record of `class` (generator or attractor); `None` for any other
    /// class.
    pub fn for_class(class: u32) -> Option<ControllerRecord> {
        let kind = match class {
            CLSID_GENERATOR => ControllerKind::Generator {
                period: ControllerTime::default(),
                second_period: ControllerTime::default(),
                day_stamp: 0,
                counter: 0,
            },
            CLSID_ATTRACTOR => ControllerKind::Attractor {
                tag: ATTRACTOR_TAG,
                curve: Vec::new(),
            },
            _ => return None,
        };
        Some(ControllerRecord {
            base: ControllerBase::default(),
            kind,
        })
    }

    pub fn class_id(&self) -> u32 {
        match self.kind {
            ControllerKind::Generator { .. } => CLSID_GENERATOR,
            ControllerKind::Attractor { .. } => CLSID_ATTRACTOR,
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        match &self.kind {
            ControllerKind::Generator {
                period,
                second_period,
                day_stamp,
                counter,
            } => {
                out.put_u16(GENERATOR_VERSION);
                self.base.write(out);
                put_time(out, period);
                put_time(out, second_period);
                out.put_u32(*day_stamp);
                out.put_u32(*counter);
            }
            ControllerKind::Attractor { tag, curve } => {
                self.base.write(out);
                out.put_u16(*tag);
                put_curve(out, curve);
            }
        }
        out.error() == 0
    }

    /// Reads a record of the same class as `self` into a new record. `None`
    /// on a stream error or (generator) a version other than 4.
    pub fn read_as(&self, inp: &mut dyn InStream) -> Option<ControllerRecord> {
        match self.kind {
            ControllerKind::Generator { .. } => {
                if inp.get_u16()? != GENERATOR_VERSION {
                    return None;
                }
                let base = ControllerBase::read(inp)?;
                Some(ControllerRecord {
                    base,
                    kind: ControllerKind::Generator {
                        period: get_time(inp)?,
                        second_period: get_time(inp)?,
                        day_stamp: inp.get_u32()?,
                        counter: inp.get_u32()?,
                    },
                })
            }
            ControllerKind::Attractor { .. } => {
                let base = ControllerBase::read(inp)?;
                Some(ControllerRecord {
                    base,
                    kind: ControllerKind::Attractor {
                        tag: inp.get_u16()?,
                        curve: get_curve(inp)?,
                    },
                })
            }
        }
    }
}

/// A controller object (`cISC4AutomataController`, §3). Its saved state is a
/// [`ControllerRecord`]; the template binding, `Init`, the controller grid and
/// `Shutdown` of §3.4 are the unsaved state. What a controller does to the
/// automata near it is animation and is not modelled.
pub struct Controller {
    rec: RefCell<ControllerRecord>,
    /// Slot 15 `SetTemplate` happened (at creation from a template, or at
    /// `PostCityInit` after a load, §3.4).
    template_bound: Cell<bool>,
    initialized: Cell<bool>,
    in_grid: Cell<bool>,
    shut_down: Cell<bool>,
    token: Cell<u64>,
}

impl Controller {
    /// A new controller of `class` (generator or attractor), unbound; `None`
    /// for any other class.
    pub fn for_class(class: u32) -> Option<Rc<Controller>> {
        ControllerRecord::for_class(class).map(Controller::from_record)
    }

    pub fn from_record(rec: ControllerRecord) -> Rc<Controller> {
        Rc::new(Controller {
            rec: RefCell::new(rec),
            template_bound: Cell::new(false),
            initialized: Cell::new(false),
            in_grid: Cell::new(false),
            shut_down: Cell::new(false),
            token: Cell::new(0),
        })
    }

    /// A controller made from `template` for `occupant` (slot 6
    /// `AddControllerFromTemplate`): the template's class, name and flags,
    /// bound, `Init`ed and in the grid. The template's other contents
    /// (spawn rules, curves) are script data the spec does not give; the
    /// curve and the periods are copied when the template has them
    /// *[choice]*.
    pub fn from_template(template: &ControllerTemplate, occupant: RefValue) -> Rc<Controller> {
        let kind = match template.kind {
            TemplateKind::Generator => ControllerKind::Generator {
                period: template.period,
                second_period: template.second_period,
                day_stamp: 0,
                counter: 0,
            },
            TemplateKind::Attractor => ControllerKind::Attractor {
                tag: ATTRACTOR_TAG,
                curve: template.curve.clone(),
            },
        };
        let c = Controller::from_record(ControllerRecord {
            base: ControllerBase {
                occupant,
                template_name: template.name.clone().into_bytes(),
                flags: template.flags,
                ..ControllerBase::default()
            },
            kind,
        });
        c.template_bound.set(true);
        c.initialized.set(true);
        c.in_grid.set(true);
        c
    }

    pub fn record(&self) -> ControllerRecord {
        self.rec.borrow().clone()
    }

    pub fn set_record(&self, rec: ControllerRecord) {
        *self.rec.borrow_mut() = rec;
    }

    pub fn with_record<R>(&self, f: impl FnOnce(&mut ControllerRecord) -> R) -> R {
        f(&mut self.rec.borrow_mut())
    }

    pub fn flags(&self) -> u32 {
        self.rec.borrow().base.flags
    }

    pub fn set_flags(&self, flags: u32) {
        self.rec.borrow_mut().base.flags = flags;
    }

    /// Slot 12 `CheckFlag(flag)`.
    pub fn check_flag(&self, flag: u32) -> bool {
        self.flags() & flag != 0
    }

    /// Flag 8 is set (§3.2).
    pub fn is_transient(&self) -> bool {
        self.check_flag(FLAG_TRANSIENT)
    }

    pub fn template_name(&self) -> String {
        String::from_utf8_lossy(&self.rec.borrow().base.template_name).into_owned()
    }

    pub fn occupant(&self) -> RefValue {
        self.rec.borrow().base.occupant.clone()
    }

    pub fn template_bound(&self) -> bool {
        self.template_bound.get()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    pub fn in_grid(&self) -> bool {
        self.in_grid.get()
    }

    pub fn is_shut_down(&self) -> bool {
        self.shut_down.get()
    }

    /// Slot 15 `SetTemplate`: the name is the template's.
    pub fn set_template(&self, template: &ControllerTemplate) {
        self.rec.borrow_mut().base.template_name = template.name.clone().into_bytes();
        self.template_bound.set(true);
    }

    /// Slot 3 `Init`.
    pub fn init(&self) -> bool {
        self.initialized.set(true);
        self.shut_down.set(false);
        true
    }

    /// Added to the controller grid (§3.4 step 3).
    pub fn add_to_grid(&self) {
        self.in_grid.set(true);
    }

    /// Slot 4 `Shutdown`: out of the grid, the automata let go.
    pub fn shutdown(&self) -> bool {
        self.in_grid.set(false);
        self.initialized.set(false);
        self.shut_down.set(true);
        true
    }

    pub fn set_host_token(&self, token: u64) {
        self.token.set(token);
    }

    /// `QueryInterface`: the controller interface, `cIGZSerializable` and
    /// `cIGZUnknown`.
    pub fn answers_iid(&self, iid: u32) -> bool {
        matches!(iid, IID_UNKNOWN | IID_AUTOMATA_CONTROLLER | IID_SERIALIZABLE)
    }
}

impl SaveObject for Controller {
    fn class_id(&self) -> u32 {
        self.rec.borrow().class_id()
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        self.rec.borrow().write(out)
    }

    /// Reads a record of this controller's class. On failure nothing changes
    /// *[choice]*.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let parsed = self.rec.borrow().read_as(inp);
        match parsed {
            Some(r) => {
                *self.rec.borrow_mut() = r;
                self.template_bound.set(false);
                inp.error() == 0
            }
            None => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

/// Whether a referenced controller has flag 8: decided for core controllers;
/// a host controller asks `host_flags` (the host's `CheckFlag(8)`).
pub fn ref_is_transient(r: &RefValue, host_flags: &dyn Fn(u64) -> u32) -> bool {
    match r {
        RefValue::Null => false,
        RefValue::Object(o) => o
            .clone()
            .as_any_rc()
            .downcast::<Controller>()
            .map(|c| c.is_transient())
            .unwrap_or(false),
        RefValue::Host { token, .. } => host_flags(*token) & FLAG_TRANSIENT != 0,
    }
}

/// A controller reference as written by automata and states (§6.1 field 5,
/// §6.2): null when the controller has flag 8.
pub fn saved_controller_ref(r: &RefValue, host_flags: &dyn Fn(u64) -> u32) -> RefValue {
    if ref_is_transient(r, host_flags) {
        RefValue::Null
    } else {
        r.clone()
    }
}
