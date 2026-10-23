// SPDX-License-Identifier: GPL-3.0-or-later
//! Window lifetime. All service calls are synchronous; returned tokens carry
//! one owned reference. No foreign object is implemented by this module.
use super::{InputHost, View3DWin};
use std::collections::BTreeMap;

pub const CHEATS: [(u32, &str); 13] = [
    (0x8973_A69C, "GOL"),
    (0xCA7B_C5B5, "SizeOf"),
    (0xAA7A_2746, "TastyZots"),
    (0x6A7A_276E, "Zoneria"),
    (0x8AB7_30AE, "TerrainQuery"),
    (0x88F3_D4BB, "RenderProp"),
    (0x88F3_D4BC, "RP"),
    (0x6976_AFA0, "Recorder"),
    (0x4A97_2F37, "WatchMeMove"),
    (0x4A82_253E, "You don't deserve it"),
    (0x4AC0_96C6, "LotPlop"),
    (0x4AC0_96CA, "BuildingPlop"),
    (0x0BFD_8064, "toll"),
];

pub const NOTIFICATIONS: [u32; 33] = [
    0x26D3_1EC0, 0x26D3_1EC1, 0x26D3_1EC2, 0x26D3_1EC4, 0x26D3_1EC5,
    0x26C6_3341, 0x26C6_3343, 0x26C6_3344, 0x0AB9_9380, 0x0772_FAD4,
    0x08DA_C3E7, 0x08DA_C3E8, 0x06CD_B65B, 0x4477_8231, 0x4477_8233,
    0x6695_6814, 0x6695_6816, 0x4A7F_B7E9, 0x4A7F_B7E2, 0xAA7F_B7E0,
    0xABBB_CA64, 0xA6B7_9602, 0xA6B7_9612, 0xA6B7_9621, 0xCA1B_1E64,
    0xAA1B_2002, 0xAA5C_F1E3, 0xAC08_3D7D, 0x4A44_667F,
    // Subscription is a set: registration order has no specified effect (§2).
    0x6A93_5C34, 0x6A93_5CA4, 0x6A93_5CA8, 0x6A93_5CAB,
];

#[derive(Debug, Default)]
pub struct Lifecycle {
    pub initialized: bool,
    pub city_initializing: bool,
    pub active: bool,
    pub progress: Option<u32>,
    /// Absolute field offset -> owned opaque service reference.
    pub objects: BTreeMap<u32, u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LifecycleCall {
    ScriptCreate,
    Subscribe,
    RegisterCommands,
    CheatListener(bool),
    Cheat(u32, &'static str, bool),
    BaseInit,
    Name(&'static str),
    WindowClass(u32),
    FitParent,
    VisibleEnabled,
    InitCoverage,
    StartTimer,
    LoadKeyMap { type_id: u32, group: u32, instance: u32, iid: u32 },
    Create { class: u32, iid: u32 },
    FillKeyMap { resource: u32, accelerator: u32 },
    CreateProgress,
    InitTail,
    Release(u32),
    Stop(u32),
    ProgressShutdown(u32),
    UnregisterCommands,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LifecycleReply {
    pub success: bool,
    pub object: Option<u32>,
}

pub trait LifecycleHost: InputHost {
    fn lifecycle(&mut self, call: LifecycleCall) -> LifecycleReply;
    fn cheat_manager_exists(&self) -> bool;
    fn development_cheats(&self) -> bool;
    fn loading_city(&self) -> bool;
}

impl View3DWin {
    /// Foreign base-window construction precedes this factory. `new` is the
    /// token-only compatibility constructor; adapters use this final hook.
    pub fn constructed(this: u32, win: super::WinToken, host: &mut impl LifecycleHost) -> Self {
        let view = Self::new(this, win);
        host.lifecycle(LifecycleCall::ScriptCreate);
        view
    }

    pub fn lifecycle_state(&self) -> &Lifecycle {
        &self.owner.lifecycle
    }

    /// Init's base result is retained, but a false result does not skip the
    /// subsequent setup (§3). Repeated successful Init has no side effects.
    pub fn init(&mut self, host: &mut impl LifecycleHost) -> bool {
        if self.owner.lifecycle.initialized {
            return true;
        }
        host.lifecycle(LifecycleCall::Subscribe);
        // VW9: exact ordered 104-record registry is not specified. This service
        // boundary must not manufacture records from guessed command IDs.
        host.lifecycle(LifecycleCall::RegisterCommands);
        if host.cheat_manager_exists() {
            host.lifecycle(LifecycleCall::CheatListener(true));
            for (index, &(id, name)) in CHEATS.iter().enumerate() {
                if index < 10 || host.development_cheats() {
                    host.lifecycle(LifecycleCall::Cheat(id, name, true));
                }
            }
        }
        let result = host.lifecycle(LifecycleCall::BaseInit).success;
        self.owner.lifecycle.initialized = result;
        host.lifecycle(LifecycleCall::Name("cSC4View3DWin"));
        host.lifecycle(LifecycleCall::WindowClass(0x9A47_B417));
        host.lifecycle(LifecycleCall::FitParent);
        host.lifecycle(LifecycleCall::VisibleEnabled);
        host.set_cursor(0xC16C_4E1B);
        host.lifecycle(LifecycleCall::InitCoverage);
        host.lifecycle(LifecycleCall::StartTimer);
        let resource = host.lifecycle(LifecycleCall::LoadKeyMap {
            type_id: 0xA2E3_D533, group: 0x6A23_1EAA, instance: 0x6A93_62EF, iid: 0x42E3_EA4B,
        }).object;
        if let Some(resource) = resource {
            if let Some(accelerator) = host.lifecycle(LifecycleCall::Create {
                class: 0x42E9_67BE, iid: 0xE2C1_B3C4,
            }).object {
                self.owner.lifecycle.objects.insert(0x120, accelerator);
                host.lifecycle(LifecycleCall::FillKeyMap { resource, accelerator });
            }
            host.lifecycle(LifecycleCall::Release(resource));
        }
        self.owner.lifecycle.active = true;
        if host.loading_city() {
            self.owner.lifecycle.progress = host.lifecycle(LifecycleCall::CreateProgress).object;
        }
        host.lifecycle(LifecycleCall::InitTail);
        result
    }

    /// Helper teardown is split from GUI teardown so shutdown preserves §4's
    /// ordering. Tokens here were created/acquired by this owner, never borrowed.
    pub fn shutdown_helpers(&mut self, host: &mut impl LifecycleHost) {
        if host.cheat_manager_exists() {
            host.lifecycle(LifecycleCall::CheatListener(false));
            for &(id, name) in &CHEATS {
                host.lifecycle(LifecycleCall::Cheat(id, name, false));
            }
        }
        self.remove_all_controls(host, false);
        self.remove_all_persistent_controls(host);
        for (offset, token) in std::mem::take(&mut self.owner.lifecycle.objects) {
            if matches!(offset, 0x184 | 0x188 | 0x410) {
                // VW8: unidentified helper. Preserve the specified stop/release.
                host.lifecycle(LifecycleCall::Stop(token));
            }
            host.lifecycle(LifecycleCall::Release(token));
        }
    }

    pub fn shutdown_tail(&mut self, host: &mut impl LifecycleHost) {
        if let Some(progress) = self.owner.lifecycle.progress.take() {
            host.lifecycle(LifecycleCall::ProgressShutdown(progress));
            host.lifecycle(LifecycleCall::Release(progress));
        }
        host.lifecycle(LifecycleCall::UnregisterCommands);
        self.owner.lifecycle.initialized = false;
        self.owner.lifecycle.active = false;
    }
}
