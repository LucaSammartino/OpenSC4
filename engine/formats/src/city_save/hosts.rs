// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline hosts: record Read/Write do not need a running city or clock.
use fc4_core::clock24::{ClockHost, ClockTuning};
use fc4_core::history::HistoryHost;

pub(super) struct Offline;
impl HistoryHost for Offline {
    fn sim_date(&self) -> u32 {
        0
    }
    fn now_ms(&self) -> u32 {
        0
    }
    fn subscribe(&self, _: u32) {}
    fn unsubscribe(&self, _: u32) {}
    fn add_agent(&self, _: u32, _: &str, _: u32) {}
    fn remove_agent(&self) {}
    fn send(&self, _: u32, _: u32, _: u32) {}
}
impl ClockHost for Offline {
    fn now_micros(&self) -> u64 {
        0
    }
    fn post(&self, _: u32, _: u32, _: u32) {}
    fn add_system_service(&self) {}
    fn remove_system_service(&self) {}
    fn add_to_on_idle(&self) {}
    fn remove_from_on_idle(&self) {}
    fn has_cheat_manager(&self) -> bool {
        false
    }
    fn register_cheat(&self, _: u32, _: &str) {}
    fn add_cheat_notification(&self, _: u32) {}
    fn remove_cheat_notification(&self, _: u32) {}
    fn unregister_cheat(&self, _: u32) {}
    fn tuning(&self) -> Option<ClockTuning> {
        None
    }
}
