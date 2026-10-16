// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Component 40, the automata script system (§2): the controller templates
//! and occupant groups that the Lua automata scripts define (rebuilt at
//! `Init`, never saved), and its 2-byte record.

use crate::stream::{InStream, OutStream};

use super::codec::ControllerTime;
use super::controller::DEFAULT_FLAGS;

/// The record's only version (§2: Read fails on anything else).
pub const SCRIPT_SYSTEM_VERSION: u16 = 1;

/// Which controller class a template makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateKind {
    Generator,
    Attractor,
}

/// A controller template (script slot 16 `CreateControllerTemplate`). Only
/// the name matters to the saved state and the hooks; the rest is what a new
/// controller copies *[choice]* (the scripts' contents are not specified).
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerTemplate {
    pub name: String,
    pub kind: TemplateKind,
    pub flags: u32,
    pub period: ControllerTime,
    pub second_period: ControllerTime,
    pub curve: Vec<(f32, f32)>,
}

impl ControllerTemplate {
    /// A template with the default flags (7) and no curve or periods.
    pub fn new(name: &str, kind: TemplateKind) -> ControllerTemplate {
        ControllerTemplate {
            name: name.to_owned(),
            kind,
            flags: DEFAULT_FLAGS,
            period: ControllerTime::default(),
            second_period: ControllerTime::default(),
            curve: Vec::new(),
        }
    }
}

/// An occupant group (script slots 11 / 12 / 13): an ID, a name and the
/// templates it adds to an occupant, in list order.
#[derive(Clone, Debug, PartialEq)]
pub struct OccupantGroup {
    pub id: u32,
    pub name: String,
    pub templates: Vec<String>,
}

/// What the controller manager asks the script system (§3.4, §7.1): script
/// slots 11, 12 and 17.
pub trait ScriptSource {
    /// Slot 17 `GetControllerTemplate(name)`.
    fn controller_template(&self, name: &str) -> Option<ControllerTemplate>;
    /// Slot 11 `GetOccupantGroup(ID)`.
    fn occupant_group_by_id(&self, id: u32) -> Option<OccupantGroup>;
    /// Slot 12 `GetOccupantGroup(name)`.
    fn occupant_group_by_name(&self, name: &str) -> Option<OccupantGroup>;
}

/// The script system (`0x4A1B664E`, city slot 48).
#[derive(Clone, Debug, Default)]
pub struct ScriptSystem {
    templates: Vec<ControllerTemplate>,
    groups: Vec<OccupantGroup>,
    initialized: bool,
}

impl ScriptSystem {
    pub fn new() -> ScriptSystem {
        ScriptSystem::default()
    }

    /// Slot 3 `Init`: the scripts run and define the templates and groups
    /// (the host adds them with the two calls below).
    pub fn init(&mut self) -> bool {
        self.initialized = true;
        true
    }

    /// Slot 4 `Shutdown`: the script data goes.
    pub fn shutdown(&mut self) -> bool {
        self.templates.clear();
        self.groups.clear();
        self.initialized = false;
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Slot 16 `CreateControllerTemplate`: a template of the same name is
    /// replaced *[choice]*.
    pub fn define_template(&mut self, t: ControllerTemplate) {
        match self.templates.iter_mut().find(|x| x.name == t.name) {
            Some(x) => *x = t,
            None => self.templates.push(t),
        }
    }

    /// Slot 13 `CreateOccupantGroup`: a group with the same ID is replaced
    /// *[choice]*.
    pub fn define_occupant_group(&mut self, g: OccupantGroup) {
        match self.groups.iter_mut().find(|x| x.id == g.id) {
            Some(x) => *x = g,
            None => self.groups.push(g),
        }
    }

    pub fn templates(&self) -> &[ControllerTemplate] {
        &self.templates
    }

    pub fn groups(&self) -> &[OccupantGroup] {
        &self.groups
    }

    /// §2: `u16` 1. Returns `GetError() == 0`.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(SCRIPT_SYSTEM_VERSION);
        out.error() == 0
    }

    /// §2: reads the `u16`; fails when it is not 1.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        matches!(inp.get_u16(), Some(SCRIPT_SYSTEM_VERSION)) && inp.error() == 0
    }
}

impl ScriptSource for ScriptSystem {
    fn controller_template(&self, name: &str) -> Option<ControllerTemplate> {
        self.templates.iter().find(|t| t.name == name).cloned()
    }

    fn occupant_group_by_id(&self, id: u32) -> Option<OccupantGroup> {
        self.groups.iter().find(|g| g.id == id).cloned()
    }

    fn occupant_group_by_name(&self, name: &str) -> Option<OccupantGroup> {
        self.groups.iter().find(|g| g.name == name).cloned()
    }
}
