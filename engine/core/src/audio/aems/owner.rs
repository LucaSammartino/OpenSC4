// SPDX-License-Identifier: GPL-3.0-or-later
//! AEMS service owner, audio §10.7.1. Backend operations are single IO calls.
use super::{AemsEventSystem, AemsSoundBank, BankFile};
use crate::curve::ResponseCurve;
use crate::x87::ftol32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    GetOptions,
    SetOptions,
    Init,
    EffectsInit,
    Enter,
    Leave,
    RemoveSystem,
    RestoreSystem,
    RegisterTarget,
    UnregisterTarget,
    Subscribe,
    Unsubscribe,
    SystemResource,
    Stop(usize, i32),
    Start(usize),
    Parameter(usize, i32),
    MasterVolume(i32),
    RegisterBank,
    RemoveBank(u32),
    AddSystem,
}

pub trait Host {
    fn backend(&mut self, call: Backend) -> i32;
    fn install_callback(&mut self, callback: fn() -> i32);
    fn init_backend(&mut self, workspace: &mut [u8], constant: u32) -> i32;
    fn init_effects(&mut self, arguments: (u32, i32, u32, u32, u32)) -> i32;
    fn register_bank(&mut self, image: &[u8], zero1: u32, zero2: u32, callback: fn(u64) -> u64) -> i32;
    fn add_system(&mut self, image: &[u8]) -> i32;
    fn system_file(&mut self) -> Option<Box<dyn BankFile>>;
    fn preferences(&mut self) -> (bool, i32);
    fn tuning(&mut self) -> Option<Tuning>;
    fn has_sound_system(&mut self) -> bool;
    fn has_messages(&mut self) -> bool;
    fn service_query(&mut self, iid: u32) -> Option<u64>;
    fn service_add_ref(&mut self) -> u32;
    fn service_release(&mut self) -> u32;
}

#[derive(Default)]
pub struct Tuning {
    pub gain: Option<f32>,
    pub effects: Option<f32>,
    pub selector: Option<u32>,
    pub curve: Option<ResponseCurve>,
    pub speeds: Option<Vec<f32>>,
}

#[derive(Default, Debug)]
pub struct Process {
    pub initialized: bool,
    pub parameter: f32,
}

#[derive(Debug)]
pub struct AemsOwner {
    pub system: AemsEventSystem,
    pub handles: [u32; 21],
    pub workspace: Vec<u8>,
    pub image: AemsSoundBank,
    pub master: f32,
    pub speed: f32,
    pub speeds: Vec<f32>,
    pub curve: ResponseCurve,
    pub bank_handles: Vec<u32>,
}

impl Default for AemsOwner {
    fn default() -> Self {
        Self {
            system: AemsEventSystem::default(),
            handles: [0; 21],
            workspace: Vec::new(),
            image: AemsSoundBank::default(),
            master: 1.0,
            speed: 1.0,
            speeds: Vec::new(),
            curve: ResponseCurve::from_points(&[(1.0, 1.0)]),
            bank_handles: Vec::new(),
        }
    }
}

impl AemsOwner {
    pub fn add_ref(&mut self, host: &mut impl Host) -> u32 {
        host.service_add_ref()
    }
    pub fn release(&mut self, host: &mut impl Host) -> u32 {
        host.service_release()
    }
    pub fn query(&mut self, iid: u32, host: &mut impl Host) -> Option<u64> {
        let view = match iid {
            0xBB52_3BB2 => Some(0),
            0x8C1F_AEB7 => Some(4),
            0x4522_94AA => Some(8),
            _ => None,
        };
        if view.is_some() {
            host.service_add_ref();
            view
        } else {
            host.service_query(iid)
        }
    }

    pub fn read_tuning(&mut self, host: &mut impl Host) -> bool {
        let Some(t) = host.tuning() else { return false };
        if let Some(v) = t.gain {
            self.system.scale_110 = v;
        }
        if let Some(v) = t.effects {
            self.system.scale_114 = v;
        }
        if let Some(v) = t.selector {
            self.system.value_118 = v;
        }
        self.curve = t.curve.unwrap_or_else(|| ResponseCurve::from_points(&[(1.0, 1.0)]));
        self.speeds = t.speeds.unwrap_or_else(|| vec![0.0, 0.0, 0.0, 0.5, 1.0]);
        true
    }

    pub fn init(&mut self, process: &mut Process, host: &mut impl Host) -> bool {
        if process.initialized {
            return true;
        }
        process.initialized = true;
        self.handles.fill(0);
        for index in 0..3 {
            self.parameter(index, process.parameter, host);
        }
        let (enabled, volume) = host.preferences();
        self.master = if enabled {
            (f64::from(volume) / 1024.0) as f32
        } else {
            0.0
        };
        self.read_tuning(host);
        self.workspace.resize(200000, 0);
        if host.backend(Backend::GetOptions) < 0 {
            self.shutdown(process, host);
            return false;
        }
        host.install_callback(|| 0);
        if host.backend(Backend::SetOptions) < 0
            || host.init_backend(&mut self.workspace, 0x72B00) < 0
            || host.init_effects(self.effects_arguments()) < 0
        {
            self.shutdown(process, host);
            return false;
        }
        self.update_volume(host);
        if host.has_sound_system() {
            host.backend(Backend::RegisterTarget);
        }
        self.load_system_resource(host);
        if host.has_messages() {
            host.backend(Backend::Subscribe);
        }
        true
    }

    pub fn effects_arguments(&self) -> (u32, i32, u32, u32, u32) {
        (0, ftol32(f64::from(self.system.scale_114)), self.system.value_118, 0, 0)
    }

    pub fn shutdown(&mut self, process: &mut Process, host: &mut impl Host) -> bool {
        if !process.initialized {
            return true;
        }
        process.initialized = false;
        self.stop_all(host);
        self.handles.fill(0);
        self.remove_all_banks(host);
        host.backend(Backend::Enter);
        host.backend(Backend::RemoveSystem);
        host.backend(Backend::Leave);
        host.backend(Backend::RestoreSystem);
        self.workspace.clear();
        if host.has_sound_system() {
            host.backend(Backend::UnregisterTarget);
        }
        if host.has_messages() {
            host.backend(Backend::Unsubscribe);
        }
        true
    }

    pub fn stop_all(&mut self, host: &mut impl Host) {
        for index in (0..15).chain([17, 18, 20, 19]) {
            self.end(index, host);
        }
    }
    pub fn start(&mut self, index: usize, host: &mut impl Host) -> bool {
        if self.handles[index] != 0 {
            return false;
        }
        host.backend(Backend::Enter);
        let value = host.backend(Backend::Start(index)).wrapping_add(1);
        host.backend(Backend::Leave);
        self.handles[index] = if value > 0 { value as u32 } else { 0 };
        true
    }
    pub fn end(&mut self, index: usize, host: &mut impl Host) -> bool {
        if self.handles[index] == 0 {
            return false;
        }
        host.backend(Backend::Enter);
        host.backend(Backend::Stop(index, self.handles[index].wrapping_sub(1) as i32));
        host.backend(Backend::Leave);
        self.handles[index] = 0;
        true
    }
    pub fn parameter(&mut self, index: usize, value: f32, host: &mut impl Host) -> bool {
        // NaN follows conversion rather than choosing a finite clamp endpoint.
        let value = value.clamp(0.0, 1.0);
        host.backend(Backend::Enter);
        let result = host.backend(Backend::Parameter(
            index,
            ftol32(f64::from(value) * f64::from(127.0f32)),
        ));
        host.backend(Backend::Leave);
        result >= 0
    }
    fn master_volume(&self, argument: f64, host: &mut impl Host) {
        let value = self.curve.eval(argument as f32);
        host.backend(Backend::MasterVolume(ftol32(f64::from(value) * f64::from(127.0f32))));
    }
    pub fn update_volume(&self, host: &mut impl Host) {
        let argument = ((f64::from(self.speed) * f64::from(self.master)) * f64::from(self.system.scale_110)) as f32;
        self.master_volume(f64::from(argument), host);
    }
    pub fn do_event(&mut self, event: u32, volume: u32, host: &mut impl Host) -> bool {
        if event == 0x2A5B_69C0 {
            let master = f64::from(volume) / 1024.0;
            self.master = master as f32;
            self.master_volume(
                (f64::from(self.speed) * f64::from(self.system.scale_110)) * master,
                host,
            );
        }
        false
    }
    pub fn do_message(&mut self, message: u32, speed: usize, host: &mut impl Host) -> bool {
        if message != 0xA6B7_9602 {
            return false;
        }
        self.speed = self.speeds[speed];
        self.master_volume(
            (f64::from(self.master) * f64::from(self.system.scale_110)) * f64::from(self.speed),
            host,
        );
        true
    }
    pub fn add_bank(&mut self, bank: Option<AemsSoundBank>, host: &mut impl Host) -> bool {
        let Some(bank) = bank else { return false };
        if host.register_bank(&bank.image, 0, 0, |p| p) != 0 {
            return false;
        }
        self.system.banks.push(bank);
        self.bank_handles.push(0);
        true
    }
    pub fn remove_bank(&mut self, instance: u32, host: &mut impl Host) -> bool {
        let Some(index) = self
            .system
            .banks
            .iter()
            .position(|b| b.cached_tgi.map_or(0, |t| t.instance) == instance)
        else {
            return false;
        };
        host.backend(Backend::RemoveBank(self.bank_handles.remove(index)));
        self.system.banks.remove(index);
        true
    }
    pub fn remove_all_banks(&mut self, host: &mut impl Host) {
        while !self.system.banks.is_empty() {
            host.backend(Backend::RemoveBank(self.bank_handles.remove(0)));
            self.system.banks.remove(0);
        }
    }
    pub fn load_system_file(&mut self, file: &mut (impl BankFile + ?Sized), host: &mut impl Host) -> bool {
        if !self.image.read_file(file) {
            return false;
        }
        host.backend(Backend::Enter);
        host.backend(Backend::RemoveSystem);
        let result = host.add_system(&self.image.image) == 0;
        host.backend(Backend::Leave);
        result
    }
    pub fn load_system_resource(&mut self, host: &mut impl Host) -> bool {
        let Some(mut file) = host.system_file() else {
            return false;
        };
        let loaded = self.load_system_file(file.as_mut(), host);
        let closed = file.close();
        loaded && closed
    }
    pub fn load_bank_from_resource(
        &mut self,
        instance: u32,
        resources: &mut impl super::BankResources,
        host: &mut impl Host,
    ) -> bool {
        let mut bank = AemsSoundBank::default();
        if !bank.read_resource(AemsEventSystem::bank_tgi(instance), resources) {
            return false;
        }
        self.add_bank(Some(bank), host)
    }
    pub fn unload_sound_bank(&mut self, instance: u32, host: &mut impl Host) -> bool {
        self.stop_all(host);
        self.handles.fill(0);
        self.remove_bank(instance, host)
    }
    pub fn load_sound_bank(
        &mut self,
        instance: u32,
        resources: &mut impl super::BankResources,
        host: &mut impl Host,
    ) -> bool {
        self.stop_all(host);
        self.load_bank_from_resource(instance, resources, host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Trace {
        calls: Vec<Backend>,
        fail: Option<Backend>,
        start: i32,
        missing_tuning: bool,
    }
    impl Host for Trace {
        fn backend(&mut self, call: Backend) -> i32 {
            self.calls.push(call);
            if self.fail == Some(call) {
                -1
            } else if matches!(call, Backend::Start(_)) {
                self.start
            } else {
                0
            }
        }
        fn install_callback(&mut self, callback: fn() -> i32) {
            assert_eq!(callback(), 0);
        }
        fn init_backend(&mut self, workspace: &mut [u8], constant: u32) -> i32 {
            assert_eq!((workspace.len(), constant), (200000, 0x72B00));
            self.backend(Backend::Init)
        }
        fn init_effects(&mut self, args: (u32, i32, u32, u32, u32)) -> i32 {
            assert_eq!(args, (0, 1, 37, 0, 0));
            self.backend(Backend::EffectsInit)
        }
        fn register_bank(&mut self, _: &[u8], z1: u32, z2: u32, callback: fn(u64) -> u64) -> i32 {
            assert_eq!((z1, z2, callback(9)), (0, 0, 9));
            self.backend(Backend::RegisterBank)
        }
        fn add_system(&mut self, _: &[u8]) -> i32 {
            self.backend(Backend::AddSystem)
        }
        fn system_file(&mut self) -> Option<Box<dyn BankFile>> {
            self.backend(Backend::SystemResource);
            None
        }
        fn preferences(&mut self) -> (bool, i32) {
            (true, 1024)
        }
        fn tuning(&mut self) -> Option<Tuning> {
            (!self.missing_tuning).then(Tuning::default)
        }
        fn has_sound_system(&mut self) -> bool {
            true
        }
        fn has_messages(&mut self) -> bool {
            true
        }
        fn service_query(&mut self, _: u32) -> Option<u64> {
            None
        }
        fn service_add_ref(&mut self) -> u32 {
            1
        }
        fn service_release(&mut self) -> u32 {
            0
        }
    }
    #[test]
    fn au48_failed_tuning_preserves_values_success_reseeds_speed_and_failed_read_skips_backend() {
        struct File;
        impl BankFile for File {
            fn size(&mut self) -> usize {
                8
            }
            fn read(&mut self, _: &mut [u8]) -> bool {
                false
            }
            fn close(&mut self) -> bool {
                true
            }
            fn oversized_read(&mut self, _: &mut [u8], _: usize) -> bool {
                panic!("unexpected oversized request")
            }
        }
        let mut a = AemsOwner::default();
        let mut h = Trace {
            missing_tuning: true,
            ..Trace::default()
        };
        a.system.scale_110 = 3.0;
        a.speeds = vec![8.0];
        assert!(!a.read_tuning(&mut h));
        assert_eq!(a.system.scale_110, 3.0);
        assert_eq!(a.speeds, [8.0]);
        h.missing_tuning = false;
        assert!(a.read_tuning(&mut h));
        assert_eq!(a.system.scale_110, 3.0);
        assert_eq!(a.speeds, [0.0, 0.0, 0.0, 0.5, 1.0]);
        assert!(!a.load_system_file(&mut File, &mut h));
        assert!(!h.calls.contains(&Backend::RemoveSystem));
        assert!(!h.calls.contains(&Backend::AddSystem));
    }
    #[test]
    fn au48_latch_negative_shutdown_retry_and_stop_indices() {
        let mut a = AemsOwner::default();
        let mut p = Process::default();
        let mut h = Trace {
            fail: Some(Backend::GetOptions),
            ..Trace::default()
        };
        assert!(!a.init(&mut p, &mut h));
        assert!(!p.initialized);
        assert!(h.calls.contains(&Backend::RestoreSystem));
        h.fail = Some(Backend::SystemResource);
        assert!(a.init(&mut p, &mut h));
        let count = h.calls.len();
        assert!(a.init(&mut p, &mut h));
        assert_eq!(h.calls.len(), count);
        a.handles.fill(1);
        h.calls.clear();
        a.stop_all(&mut h);
        let indices: Vec<_> = h
            .calls
            .iter()
            .filter_map(|c| if let Backend::Stop(i, _) = c { Some(*i) } else { None })
            .collect();
        assert_eq!(indices, (0..15).chain([17, 18, 20, 19]).collect::<Vec<_>>());
        assert_eq!(a.handles[15..17], [1, 1]);
    }
    #[test]
    fn au48_start_parameter_event_and_bank_result() {
        let mut a = AemsOwner::default();
        let mut h = Trace {
            start: -1,
            ..Trace::default()
        };
        assert!(a.start(0, &mut h));
        assert_eq!(a.handles[0], 0);
        h.start = 4;
        assert!(a.start(0, &mut h));
        assert_eq!(a.handles[0], 5);
        assert!(!a.start(0, &mut h));
        a.parameter(0, 0.5, &mut h);
        a.parameter(1, 2.0, &mut h);
        assert!(h.calls.contains(&Backend::Parameter(0, 63)));
        assert!(h.calls.contains(&Backend::Parameter(1, 127)));
        assert!(!a.do_event(0x2A5B_69C0, 512, &mut h));
        assert_eq!(a.master, 0.5);
        h.fail = Some(Backend::RegisterBank);
        assert!(!a.add_bank(Some(AemsSoundBank::default()), &mut h));
        assert!(a.system.banks.is_empty());
        h.fail = None;
        assert!(a.add_bank(Some(AemsSoundBank::default()), &mut h));
        assert!(a.remove_bank(0, &mut h));
    }
}
