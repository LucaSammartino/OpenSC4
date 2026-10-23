// SPDX-License-Identifier: GPL-3.0-or-later
use super::{References, Result, Token, Unsupported};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Primary,
    Secondary,
}

/// Application hooks include the existing application::Lifecycle orchestration;
/// this owner does not duplicate that application's startup/service algorithm.
pub trait Host: References {
    fn prepare_com(&mut self);
    fn register_com_service(&mut self);
    fn setup_application(&mut self);
    fn init_application(&mut self) -> bool;
    fn init_framework(&mut self) -> bool;
    fn run(&mut self) -> u32;
    fn shutdown(&mut self);
    fn post_quit(&mut self, result: u32);
    fn abort_cleanup(&mut self);
    fn terminate(&mut self, result: u32);
    fn enable(&mut self, enabled: bool);
    fn exception_dialog(&mut self, text: &str);
}

#[derive(Debug)]
pub struct Framework {
    refs: u32,
    services: BTreeMap<u32, Token>,
    ticks: Vec<Token>,
    ordered: Vec<(u32, i32, Token)>,
    pub registry_protected: bool,
    pub ticks_protected: bool,
    pub ordered_protected: bool,
    scheduling: i32,
    debug_level: u32,
    pub phase: u32,
    pub alternate_loop: bool,
    pub pending_quit: bool,
    pub process_result: u32,
    pub application: Option<Token>,
}
impl Default for Framework {
    fn default() -> Self {
        Self {
            refs: 0,
            services: BTreeMap::new(),
            ticks: Vec::new(),
            ordered: Vec::new(),
            registry_protected: false,
            ticks_protected: false,
            ordered_protected: false,
            scheduling: 4,
            debug_level: 0,
            phase: 0,
            alternate_loop: false,
            pending_quit: false,
            process_result: 0,
            application: None,
        }
    }
}
impl Framework {
    pub fn query_interface(&mut self, iid: u32, out: &mut Option<Face>) -> bool {
        let face = match iid {
            1 | 0x65 => Face::Primary,
            0x23EA70A1 => Face::Secondary,
            _ => return false,
        };
        self.add_ref();
        *out = Some(face);
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    pub fn release(&mut self) -> u32 {
        if self.refs < 2 {
            0
        } else {
            self.refs -= 1;
            self.refs
        }
    }
    pub fn ref_count(&self) -> u32 {
        self.refs
    }
    pub fn scheduling_count(&self) -> i32 {
        self.scheduling
    }
    pub fn set_scheduling_count(&mut self, count: i32) {
        self.scheduling = count.max(1);
    }
    pub fn debug_level(&self) -> u32 {
        self.debug_level
    }
    pub fn set_debug_level(&mut self, level: u32) {
        self.debug_level = level;
    }
    pub fn services(&self) -> impl Iterator<Item = (u32, Token)> + '_ {
        self.services.iter().map(|(&id, &t)| (id, t))
    }
    pub fn add_service(&mut self, host: &mut impl References, id: u32, service: Option<Token>) -> bool {
        let Some(t) = service.filter(|&t| t != 0) else {
            return false;
        };
        if self.registry_protected || self.services.contains_key(&id) {
            return false;
        }
        host.add_ref(t);
        self.services.insert(id, t);
        true
    }
    pub fn lookup(&self, host: &mut impl References, id: u32, iid: u32, out: &mut Option<Token>) -> bool {
        let Some(&t) = self.services.get(&id) else {
            return false;
        };
        let Some(found) = host.query(t, iid) else {
            return false;
        };
        *out = Some(found);
        true
    }
    pub fn remove_service(&mut self, host: &mut impl References, id: u32) -> bool {
        if self.registry_protected || self.ticks_protected || self.ordered_protected {
            return false;
        }
        let Some(t) = self.services.remove(&id) else {
            return false;
        };
        self.remove_tick(host, t);
        let ids: Vec<_> = self.ordered.iter().filter(|e| e.2 == t).map(|e| e.0).collect();
        for id in ids {
            self.remove_ordered(host, id);
        }
        host.release(t);
        true
    }
    pub fn add_tick(&mut self, host: &mut impl References, t: Token) -> bool {
        if self.ticks_protected || self.ticks.contains(&t) {
            return false;
        }
        host.add_ref(t);
        self.ticks.push(t);
        true
    }
    pub fn remove_tick(&mut self, host: &mut impl References, t: Token) -> bool {
        if self.ticks_protected {
            return false;
        }
        let Some(i) = self.ticks.iter().position(|&v| v == t) else {
            return false;
        };
        self.ticks.remove(i);
        host.release(t);
        true
    }
    pub fn add_ordered(&mut self, host: &mut impl References, id: u32, order: i32, t: Token) -> Result<bool> {
        if self.ordered_protected || self.ordered.iter().any(|e| e.0 == id) {
            return Ok(false);
        }
        if self.ordered.iter().any(|e| e.1 == order) {
            return Err(Unsupported("GZF1"));
        }
        let i = self.ordered.partition_point(|e| e.1 < order);
        host.add_ref(t);
        self.ordered.insert(i, (id, order, t));
        Ok(true)
    }
    pub fn remove_ordered(&mut self, host: &mut impl References, id: u32) -> bool {
        if self.ordered_protected {
            return false;
        }
        let Some(i) = self.ordered.iter().position(|e| e.0 == id) else {
            return false;
        };
        host.release(self.ordered.remove(i).2);
        true
    }
    pub fn init(&mut self, host: &mut impl Host) -> bool {
        self.phase = 0;
        host.prepare_com();
        host.register_com_service();
        host.setup_application();
        if !host.init_application() || !host.init_framework() {
            self.process_result = u32::MAX;
            return false;
        }
        true
    }
    pub fn main(&mut self, host: &mut impl Host) -> u32 {
        if self.init(host) {
            self.process_result = host.run();
        }
        host.shutdown();
        self.process_result
    }
    pub fn quit(&mut self, host: &mut impl Host, result: u32) {
        if self.alternate_loop {
            self.pending_quit = true;
            self.process_result = result;
        } else {
            host.post_quit(result);
        }
    }
    pub fn abort(&mut self, host: &mut impl Host, result: u32) {
        host.abort_cleanup();
        host.terminate(result);
    }
    pub fn report_exception(&self, host: &mut impl Host, text: Option<&str>) {
        host.enable(false);
        host.exception_dialog(text.unwrap_or("??? Unknown exception ???"));
        host.enable(true);
    }
    /// Unnamed callback kinds, streams and propagated debug semantics are open.
    pub fn unsupported_entry(&self) -> Unsupported {
        Unsupported("GZF1")
    }
}
