// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled scape/service and fire-handler contracts, audio-owner-contracts §20.
//! Full groups/resources/tick are AU42; full DisasterFire ownership is AU44.

use std::collections::BTreeMap;

pub trait ScapeHost {
    fn acquire(&mut self, service: u32, iid: u32) -> Option<u64>;
    fn add_ref(&mut self, reference: u64);
    fn release(&mut self, reference: u64);
    fn framework_available(&mut self) -> bool;
    fn register_service(&mut self, service: u32, add: bool);
    fn start_timer(&mut self, clock: u32);
    fn sound_event(&mut self, system: u64, event: u32, arguments: [u32; 4]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScapeView {
    Soundscape,
    Service,
}

/// These are known local bookkeeping fields, not a guessed group resource parser.
#[derive(Clone, Debug)]
pub struct ScapeGroup {
    pub resource: u64,
    pub flags: u32,
    pub lifetime: u32,
    pub instances: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct ScapeGlobals {
    pub sound_system: Option<u64>,
}

#[derive(Debug)]
pub struct ScapeManager {
    pub refcount: u32,
    pub sound_system: Option<u64>,
    pub sound_manager: Option<u64>,
    pub initialized: bool,
    pub elapsed: u32,
    pub enabled: bool,
    pub service_id: u32,
    pub tail: [u32; 3],
    pub zero_parameters: [u32; 6],
    pub groups: BTreeMap<u32, ScapeGroup>,
}

impl Default for ScapeManager {
    fn default() -> Self {
        Self {
            refcount: 0,
            sound_system: None,
            sound_manager: None,
            initialized: false,
            elapsed: 0,
            enabled: false,
            service_id: 0x49E5_0BA1,
            tail: [0; 3],
            zero_parameters: [0; 6],
            groups: BTreeMap::new(),
        }
    }
}

impl ScapeManager {
    /// The factory's returned interface is service +4, not the soundscape base.
    pub fn factory(allocated: bool) -> Option<(Self, ScapeView)> {
        allocated.then(|| (Self::default(), ScapeView::Service))
    }

    pub fn query_interface(&mut self, iid: u32) -> Option<ScapeView> {
        let view = match iid {
            1 | 0x29E3_9DB4 => ScapeView::Soundscape,
            0x287F_B697 => ScapeView::Service,
            _ => return None,
        };
        self.refcount = self.refcount.wrapping_add(1);
        Some(view)
    }

    pub const fn service_priority(&self) -> u32 {
        0x8000_0000
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn initialize(&mut self, globals: &mut ScapeGlobals, host: &mut impl ScapeHost) -> bool {
        if self.initialized {
            return true;
        }
        if self.sound_system.is_none() {
            self.sound_system = host.acquire(0x23CE_CBA0, 0x1060_77D2);
        }
        let Some(system) = self.sound_system else {
            return false;
        };
        if self.sound_manager.is_none() {
            self.sound_manager = host.acquire(0x1AF8_5BFB, 0x3AF8_5BB7);
        }
        if self.sound_manager.is_none() {
            return false;
        }
        if globals.sound_system.is_none() {
            host.add_ref(system);
            globals.sound_system = Some(system);
        }
        if !host.framework_available() {
            return false;
        }
        host.register_service(self.service_id, true);
        host.start_timer(4);
        self.initialized = true;
        true
    }

    /// Index insertion after the unresolved resource/group creation stage.
    /// Duplicate detection itself requires no resource calls (§20.1 slot 5).
    pub fn register_group(&mut self, key: u32, group: ScapeGroup) -> bool {
        if self.groups.contains_key(&key) {
            return false;
        }
        self.groups.insert(key, group);
        true
    }

    pub fn remove_instance(
        &mut self,
        key: u32,
        instance: u32,
        globals: &ScapeGlobals,
        host: &mut impl ScapeHost,
    ) -> bool {
        let Some(group) = self.groups.get_mut(&key) else {
            return false;
        };
        let Some(index) = group.instances.iter().position(|id| *id == instance) else {
            return false;
        };
        if let Some(system) = globals.sound_system {
            host.sound_event(system, 0x6A45_1908, [instance, 0, 0, 0]);
        }
        group.instances.remove(index);
        true
    }

    pub fn remove_group(&mut self, key: u32, globals: &ScapeGlobals, host: &mut impl ScapeHost) -> bool {
        let Some(group) = self.groups.remove(&key) else {
            return false;
        };
        for instance in group.instances {
            if let Some(system) = globals.sound_system {
                host.sound_event(system, 0x6A45_1908, [instance, 0, 0, 0]);
            }
        }
        // Resource ownership/extra group allocation cleanup is AU42. The
        // resource handle above is borrowed, never spuriously released.
        true
    }

    pub fn matching_groups(&self, mask: u32) -> Vec<u32> {
        self.groups
            .iter()
            .filter_map(|(key, group)| ((group.flags & mask) == mask).then_some(*key))
            .collect()
    }

    pub fn missing_group_instance(&self, key: u32, output: &mut u32) -> Result<(), ScapeOpen> {
        if !self.groups.contains_key(&key) {
            *output = 0;
            return Err(ScapeOpen::MissingGroup);
        }
        Err(ScapeOpen::Au42)
    }

    pub fn shutdown(&mut self, globals: &mut ScapeGlobals, host: &mut impl ScapeHost) -> bool {
        if !self.initialized {
            return true;
        }
        for key in self.groups.keys().copied().collect::<Vec<_>>() {
            self.remove_group(key, globals, host);
        }
        host.register_service(self.service_id, false);
        if let Some(system) = self.sound_system.take() {
            host.release(system);
        }
        if let Some(manager) = self.sound_manager.take() {
            host.release(manager);
        }
        if let Some(system) = globals.sound_system.take() {
            host.release(system);
        }
        self.initialized = false;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScapeOpen {
    MissingGroup,
    Au42,
}

pub trait FireHost {
    fn acquire_message_server(&mut self) -> Option<u64>;
    fn acquire_scape(&mut self, service: u32, iid: u32) -> Option<u64>;
    fn subscribe(&mut self, server: u64, message: u32, add: bool);
    fn release(&mut self, reference: u64);
    fn fire_occupant(&mut self, scape: u64, sound: u32, occupant: u64);
}

/// Explicit retained state, since construction/defaults are open AU44.
#[derive(Debug)]
pub struct DisasterFireHandler {
    pub initialized: bool,
    pub server: Option<u64>,
    pub scape: Option<u64>,
}

impl DisasterFireHandler {
    pub fn init(&mut self, host: &mut impl FireHost) -> bool {
        if self.initialized {
            return true;
        }
        if self.server.is_none() {
            self.server = host.acquire_message_server();
        }
        let Some(server) = self.server else {
            return false;
        };
        for message in [0x99EF_1142, 0x99EF_1143] {
            host.subscribe(server, message, true);
        }
        self.scape = host.acquire_scape(0x49E5_0BA1, 0x29E3_9DB4);
        self.initialized = true;
        true
    }

    pub fn shutdown(&mut self, host: &mut impl FireHost) {
        if let Some(server) = self.server {
            for message in [0x99EF_1142, 0x99EF_1143, 0x26D3_1EC1] {
                host.subscribe(server, message, false);
            }
        }
        if let Some(scape) = self.scape.take() {
            host.release(scape);
        }
        self.initialized = false;
    }

    pub fn iterate_fire_occupant(&self, occupant: Option<u64>, host: &mut impl FireHost) {
        if let Some(occupant) = occupant {
            if let Some(scape) = self.scape {
                host.fire_occupant(scape, 0x49E4_CE85, occupant);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        calls: Vec<(u32, u64)>,
        available: bool,
    }
    impl ScapeHost for Host {
        fn acquire(&mut self, service: u32, _: u32) -> Option<u64> {
            self.available.then_some(service as u64)
        }
        fn add_ref(&mut self, reference: u64) {
            self.calls.push((1, reference));
        }
        fn release(&mut self, reference: u64) {
            self.calls.push((2, reference));
        }
        fn framework_available(&mut self) -> bool {
            self.available
        }
        fn register_service(&mut self, service: u32, add: bool) {
            self.calls.push((if add { 3 } else { 4 }, service as u64));
        }
        fn start_timer(&mut self, clock: u32) {
            self.calls.push((5, clock as u64));
        }
        fn sound_event(&mut self, _: u64, event: u32, args: [u32; 4]) {
            assert_eq!(&args[1..], &[0; 3]);
            self.calls.push((event, args[0] as u64));
        }
    }
    fn group(instances: Vec<u32>) -> ScapeGroup {
        ScapeGroup {
            resource: 77,
            flags: 3,
            lifetime: 10,
            instances,
        }
    }

    #[test]
    fn au9_07_08_factory_qi_duplicate_and_mask_contracts() {
        assert!(ScapeManager::factory(false).is_none());
        let (mut scape, view) = ScapeManager::factory(true).unwrap();
        assert_eq!(view, ScapeView::Service);
        assert_eq!(scape.query_interface(0x29E3_9DB4), Some(ScapeView::Soundscape));
        assert_eq!(scape.query_interface(0x287F_B697), Some(ScapeView::Service));
        assert_eq!(scape.query_interface(99), None);
        assert_eq!(scape.refcount, 2);
        assert!(scape.register_group(7, group(vec![8])));
        assert!(!scape.register_group(7, group(vec![9])));
        assert_eq!(scape.groups[&7].instances, [8]);
        assert_eq!(scape.matching_groups(2), [7]);
        assert!(scape.matching_groups(4).is_empty());
        let mut out = 9;
        assert_eq!(scape.missing_group_instance(99, &mut out), Err(ScapeOpen::MissingGroup));
        assert_eq!(out, 0);
    }

    #[test]
    fn au9_09_removal_without_sound_system_still_cleans_local_state() {
        let mut scape = ScapeManager::default();
        let mut globals = ScapeGlobals::default();
        let mut host = Host::default();
        scape.register_group(1, group(vec![11, 12]));
        assert!(scape.remove_instance(1, 11, &globals, &mut host));
        assert!(host.calls.is_empty());
        globals.sound_system = Some(1);
        assert!(scape.remove_group(1, &globals, &mut host));
        assert_eq!(host.calls, [(0x6A45_1908, 12)]);
        assert!(!scape.remove_group(1, &globals, &mut host));
    }

    #[test]
    fn scape_service_initialization_and_reference_balance() {
        let mut scape = ScapeManager::default();
        let mut globals = ScapeGlobals::default();
        let mut host = Host::default();
        assert!(!scape.initialize(&mut globals, &mut host));
        host.available = true;
        assert!(scape.initialize(&mut globals, &mut host));
        assert!(scape.initialize(&mut globals, &mut host));
        scape.shutdown(&mut globals, &mut host);
        scape.shutdown(&mut globals, &mut host);
        assert_eq!(
            host.calls,
            [
                (1, 0x23CE_CBA0),
                (3, 0x49E5_0BA1),
                (5, 4),
                (4, 0x49E5_0BA1),
                (2, 0x23CE_CBA0),
                (2, 0x1AF8_5BFB),
                (2, 0x23CE_CBA0)
            ]
        );
    }

    impl FireHost for Host {
        fn acquire_message_server(&mut self) -> Option<u64> {
            self.available.then_some(1)
        }
        fn acquire_scape(&mut self, service: u32, iid: u32) -> Option<u64> {
            assert_eq!((service, iid), (0x49E5_0BA1, 0x29E3_9DB4));
            Some(2)
        }
        fn subscribe(&mut self, _: u64, message: u32, add: bool) {
            self.calls.push((message, u64::from(add)));
        }
        fn release(&mut self, reference: u64) {
            self.calls.push((2, reference));
        }
        fn fire_occupant(&mut self, _: u64, sound: u32, occupant: u64) {
            self.calls.push((sound, occupant));
        }
    }

    #[test]
    fn fire_requires_server_and_forwards_only_nonnull_occupants() {
        let mut fire = DisasterFireHandler {
            initialized: false,
            server: None,
            scape: None,
        };
        let mut host = Host::default();
        assert!(!fire.init(&mut host));
        host.available = true;
        assert!(fire.init(&mut host));
        assert!(fire.init(&mut host));
        fire.iterate_fire_occupant(None, &mut host);
        fire.iterate_fire_occupant(Some(4), &mut host);
        fire.shutdown(&mut host);
        assert_eq!(
            host.calls,
            [
                (0x99EF_1142, 1),
                (0x99EF_1143, 1),
                (0x49E4_CE85, 4),
                (0x99EF_1142, 0),
                (0x99EF_1143, 0),
                (0x26D3_1EC1, 0),
                (2, 2)
            ]
        );
    }
}
