// SPDX-License-Identifier: GPL-3.0-or-later
//! City-scoped construction and post-init orchestration (§6a).
use super::{GuiCall, GuiHost, LifecycleCall, MessageCall, MessageHost, Refresh, View3DWin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CityObject {
    Camera,
    CameraControl,
    Renderer,
    ImageLoader,
    AnimationManager,
    ModelMaker,
}

impl CityObject {
    pub fn class_id(self) -> u32 {
        match self {
            Self::Camera => 0xE9C6_262A,
            Self::CameraControl => 0xC9C6_28EC,
            Self::Renderer => 0xE9C6_22D8,
            Self::ImageLoader => 0xA2FF_B5D3,
            Self::AnimationManager => 0xA9C7_3857,
            Self::ModelMaker => 0xA9C7_41B3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CityCall {
    Create(CityObject),
    AcquireSimulator,
    Viewport(u32, i32, i32),
    CameraInit(u32, [f32; 3]),
    WireRenderer {
        renderer: u32,
        camera: Option<u32>,
        control: Option<u32>,
    },
    ScriptHook(u32, u32),
    ScriptInit,
    Progress,
    CitySize(u32, u32),
    EnableChild(u32, bool),
    DayNightButtons,
    BondMessage,
    Curve(u32, u32),
    DemandGraph {
        window: u32,
        developer: u32,
        colour: [u8; 3],
    },
    Gauge {
        window: u32,
        pictures: [u32; 2],
    },
    CreateSituationControl,
    ShowTip,
}

#[derive(Debug, Default)]
pub struct TunableValues {
    pub stop_step: Option<f32>,
    pub proportional_stop: Option<bool>,
    pub edge_speed: Option<f32>,
    pub keyboard_speed: Option<f32>,
    /// INI entries first, then command-line RenderProp switches, in source order.
    pub render_properties: Vec<(String, String)>,
}

/// Creation/acquisition returns an owned reference. Other calls return None.
/// Tuning uses exemplar (6534284A,690F693F,E90F6C84), properties
/// 4975DD20,4975DD25,49E5522F,A9E581FB respectively.
pub trait CityHost: MessageHost + GuiHost {
    fn city(&mut self, call: CityCall) -> Option<u32>;
    fn city_size(&self) -> u32;
    fn tunable_values(&mut self) -> TunableValues;
    fn tip_ready(&self) -> bool;
}

impl View3DWin {
    fn city_object(&mut self, host: &mut impl CityHost, offset: u32, kind: CityObject) -> Option<u32> {
        let token = host.city(CityCall::Create(kind));
        if let Some(old) = self.owner.lifecycle.objects.remove(&offset) {
            host.lifecycle(LifecycleCall::Release(old));
        }
        if let Some(token) = token {
            self.owner.lifecycle.objects.insert(offset, token);
        }
        token
    }

    pub fn pre_city_init(&mut self, host: &mut impl CityHost) {
        // VW14: null allocations skip dependent calls; successful references
        // still follow the specified construction and ownership order.
        self.owner.lifecycle.city_initializing = true;
        self.owner.motion.city_initializing = true;
        let (width, height) = host.client_size();
        let camera = self.city_object(host, 0xEC, CityObject::Camera);
        let control = self.city_object(host, 0xF0, CityObject::CameraControl);
        if let Some(control) = control {
            host.city(CityCall::Viewport(control, width, height));
            host.city(CityCall::CameraInit(control, [512.0, 0.0, 512.0]));
        }
        if self.owner.view.filter.is_none() {
            self.owner.view.filter = host.create_realistic_filter([true; 3]);
        }
        if let Some(renderer) = self.city_object(host, 0xF4, CityObject::Renderer) {
            host.city(CityCall::WireRenderer {
                renderer,
                camera,
                control,
            });
        }
        self.city_object(host, 0x150, CityObject::ImageLoader);
        host.message_call(MessageCall::Refresh(Refresh::Background));
        for hook in [0xAA76_5647, 0xAA76_563A] {
            host.city(CityCall::ScriptHook(0x6A23_1EAA, hook));
        }
        host.city(CityCall::Progress);
        self.city_object(host, 0xE8, CityObject::AnimationManager);
        self.city_object(host, 0xF8, CityObject::ModelMaker);
        if let Some(simulator) = host.city(CityCall::AcquireSimulator) {
            if let Some(old) = self.owner.lifecycle.objects.insert(0x154, simulator) {
                host.lifecycle(LifecycleCall::Release(old));
            }
        }
        host.city(CityCall::ScriptInit);
        self.read_tunable_values(host);
        host.city(CityCall::Progress);
        self.add_gui(host);
    }

    pub fn read_tunable_values(&mut self, host: &mut impl CityHost) {
        let values = host.tunable_values();
        if let Some(value) = values.stop_step {
            self.owner.motion.stop_step = value;
        }
        if let Some(value) = values.proportional_stop {
            self.owner.motion.proportional_stop = value;
        }
        if let Some(value) = values.edge_speed {
            self.owner.motion.edge_speed = value;
        }
        if let Some(value) = values.keyboard_speed {
            self.owner.motion.keyboard_speed = value;
        }
        for (name, value) in values.render_properties {
            if host.render_property(&name, &value) {
                host.post_render_property_change(0xC9F7_75BB);
            }
        }
    }

    pub fn post_city_init(&mut self, host: &mut impl CityHost) {
        host.city(CityCall::Progress);
        if let Some(&control) = self.owner.lifecycle.objects.get(&0xF0) {
            host.city(CityCall::CitySize(control, host.city_size()));
        }
        for refresh in [Refresh::Clock24, Refresh::AmbientLight] {
            host.message_call(MessageCall::Refresh(refresh));
        }
        for id in [0x4992_046A, 0xE992_0494] {
            host.city(CityCall::EnableChild(id, true));
        }
        for refresh in [Refresh::SimulationClock, Refresh::Population] {
            host.message_call(MessageCall::Refresh(refresh));
        }
        host.city(CityCall::DayNightButtons);
        host.city(CityCall::BondMessage);
        host.message_call(MessageCall::Refresh(Refresh::MayorRating));
        if host.city_established() {
            self.city_established(host);
        } else {
            self.city_obliterated(host);
        }
        host.city(CityCall::Curve(0x6A65_64F4, 0x6A65_64F6));
        for (window, developers, colour) in [
            (0x09D2_7EB0, &[0x1010, 0x1020, 0x1030][..], [0, 200, 0]),
            (0x29D2_7EC0, &[0x3110, 0x3120, 0x3320, 0x3130, 0x3330][..], [0, 0, 200]),
            (0x49D2_7ED0, &[0x4200, 0x4300, 0x4400][..], [200, 200, 0]),
        ] {
            for &developer in developers {
                host.city(CityCall::DemandGraph {
                    window,
                    developer,
                    colour,
                });
            }
        }
        for window in 0x6A5E_6EDC..=0x6A5E_6EE1 {
            host.city(CityCall::Gauge {
                window,
                pictures: [0x1401_5580, 0x1401_5584],
            });
        }
        self.owner.lifecycle.city_initializing = false;
        self.owner.motion.city_initializing = false;
        if let Some(control) = host.city(CityCall::CreateSituationControl) {
            self.add_persistent_control(host, control);
            host.control(control, super::ControlCall::Release);
        }
        host.message_call(MessageCall::Focus);
        if host.tip_ready() {
            host.city(CityCall::ShowTip);
        }
        if let Some(progress) = self.owner.lifecycle.progress.take() {
            host.lifecycle(LifecycleCall::ProgressShutdown(progress));
            host.lifecycle(LifecycleCall::Release(progress));
        }
    }

    pub fn city_established(&mut self, host: &mut impl CityHost) {
        for id in [0x9988_7766, 0x8B96_B73E, 0x4988_BC6A] {
            host.gui(GuiCall::EnableChild(id, true));
        }
        self.remove_current_control(host, false);
        host.message_call(MessageCall::Refresh(Refresh::Budget));
        host.message_call(MessageCall::Refresh(Refresh::CityName));
    }

    pub fn city_obliterated(&mut self, host: &mut impl CityHost) {
        host.gui(GuiCall::EnableChild(0x4988_BC6A, false));
        if self.owner.panels.current == Some(4) {
            self.hide_panel(host, 4);
            self.owner.panels.current = None;
        }
        host.toggle_tab(0x2988_BC85, false);
    }
}
