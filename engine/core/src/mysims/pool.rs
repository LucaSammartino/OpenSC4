// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimPool`, the Sim-template pool of the pick-sim UI (mysims.md
//! §1.1: "imports Sim templates (names, portraits) for the pick-sim UI;
//! **app/UI**, out of scope"). The spec's one line and the class's vtable
//! (`docs/abi/vtables.tsv` `00a85b14`: QI, AddRef, Release, destructor,
//! RemoveRef, `ToChar`, `GetMySimTemplates`, `Import`,
//! `FindTheSimsInstallation`) name no behaviour the simulation reads, so
//! this module holds the state a UI lane needs: the imported templates in
//! list order, with the import and installation look-up behind a host.
//!
//! Not implemented (out of scope, and a question): how the templates are
//! read from the installation, their record layout, and the portraits.
//! See MS8 in `docs/specs/questions/mysims.md`.

use std::cell::RefCell;
use std::rc::Rc;

/// One Sim template as the pool holds it: the pick-sim UI's entry (name,
/// portrait path and id, the bio). The record layout is not specified
/// (§1.1), so the fields follow the agent's `MoveIn` arguments (§4.1),
/// which is what the UI eventually passes on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MySimTemplate {
    pub name: Vec<u8>,
    pub is_male: bool,
    pub sign: u8,
    pub bio: Vec<u8>,
    pub bitmap_id: u32,
    pub bitmap_path: Vec<u8>,
}

/// What the pool reads from the app: the installed Sim templates
/// (`FindTheSimsInstallation`, `Import`). Callbacks must not call back into
/// the pool.
pub trait PoolHost {
    /// The Sims installation directory is available (`FindTheSimsInstallation`).
    fn installation_found(&self) -> bool;
    /// Reads the installed templates, in file order.
    fn read_templates(&self) -> Vec<MySimTemplate>;
}

/// The pool.
pub struct MySimPool {
    host: Rc<dyn PoolHost>,
    templates: RefCell<Vec<MySimTemplate>>,
}

impl MySimPool {
    pub fn new(host: Rc<dyn PoolHost>) -> Rc<MySimPool> {
        Rc::new(MySimPool {
            host,
            templates: RefCell::new(Vec::new()),
        })
    }

    /// `GetMySimTemplates` (vtable `+0x18`): the templates, in list order.
    pub fn templates(&self) -> Vec<MySimTemplate> {
        self.templates.borrow().clone()
    }

    /// `Import` (vtable `+0x1C`): reads the installation's templates into
    /// the pool. True when the installation was found; the pool then holds
    /// every template.
    pub fn import(&self) -> bool {
        if !self.host.installation_found() {
            return false;
        }
        *self.templates.borrow_mut() = self.host.read_templates();
        true
    }

    /// `FindTheSimsInstallation` (vtable `+0x20`): whether the installation
    /// is available.
    pub fn find_the_sims_installation(&self) -> bool {
        self.host.installation_found()
    }

    /// A template by index (the pick-sim UI's current row).
    pub fn template(&self, index: usize) -> Option<MySimTemplate> {
        self.templates.borrow().get(index).cloned()
    }
}
