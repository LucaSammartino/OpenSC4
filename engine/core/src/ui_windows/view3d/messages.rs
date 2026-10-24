// SPDX-License-Identifier: GPL-3.0-or-later
//! The two message interfaces and child-window filter (§§5–6).
use super::{LifecycleHost, View3DWin};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowMessage {
    pub kind: u32,
    pub data: [u32; 3],
    pub text: String,
    /// Cheat command line already split by the foreign command-line parser.
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notification {
    pub kind: u32,
    pub data: [u32; 2],
    /// Result of querying IID 4B99446A, not a guess from message type.
    pub command_interface: bool,
    pub progress_interface: bool,
    /// Borrowed GetIGZUnknown attachment, separate from data1 (§6l).
    pub attachment: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    CityName,
    Budget,
    Clock24,
    ClockHover,
    AmbientLight,
    SimulationClock,
    Population,
    MayorRating,
    Speed,
    Alert,
    PausedOverlay,
    Mission(u32, u32),
    /// VW18: retain raw activation payload until its derived argument is known.
    MissionActivation(u32),
    Zoom,
    Background,
    CursorPosition,
    Renderer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageCall {
    Refresh(Refresh),
    PreCityInit,
    PostCityInit,
    PreCityShutdown,
    CityEstablished,
    CityObliterated,
    Load(u32),
    Save(u32),
    BudgetFailure,
    DisasterButton { window: u32, flag: u32, enabled: bool },
    Progress([u32; 2]),
    Focus,
    InvokeCatalog(u32, u32),
    Button(u32),
    Advice(u32, [u32; 3]),
    MouseExit,
}

/// These are foreign window/dispatcher/URL/tutorial services. The owner decides
/// routing, precedence, guards, deduplication and return values.
pub trait MessageHost:
    super::ActionHost + LifecycleHost + super::WidgetHost + super::HelperHost + super::ReferenceHost + super::DialogHost
{
    fn message_call(&mut self, call: MessageCall);
    /// Return true when tutorial consumes a message; it may also rewrite it.
    fn tutorial_message(&mut self, message: &mut WindowMessage) -> bool;
    fn base_message(&mut self, message: &WindowMessage) -> bool;
    fn open_url(&mut self, url: &str) -> bool;
    fn focus_loss_stops_scroll(&self) -> bool;
    fn accelerator(&mut self, token: u32, key: u32, modifiers: u32, down: bool) -> Option<u32>;
    fn post_command(&mut self, command: u32);
    fn modal_safe(&self) -> bool;
    fn city_view_hidden(&self) -> bool;
    fn paused_overlay_exists(&self) -> bool;
    fn filter_window_passthrough(&self) -> bool;
}

impl View3DWin {
    pub(super) fn refresh(&mut self, host: &mut impl MessageHost, refresh: Refresh) {
        host.message_call(MessageCall::Refresh(refresh));
        match refresh {
            Refresh::CityName => self.update_city_name(host),
            Refresh::Budget => self.update_funds(host),
            Refresh::SimulationClock => self.update_date(host),
            Refresh::Population => self.update_population(host),
            Refresh::AmbientLight => self.update_ambient_light(host),
            Refresh::Clock24 => self.update_24hour_clock(host, 24, 0),
            Refresh::ClockHover => self.update_clock_hover(host, None),
            Refresh::Zoom => self.update_zoom_buttons(host),
            Refresh::MayorRating => {
                let data = host.status_snapshot();
                self.update_mayor_rating(host, data.rating, data.rating_width);
            }
            Refresh::Speed => {
                let data = host.status_snapshot();
                self.update_speed_buttons(host, data.speed, data.paused);
            }
            Refresh::Alert => {
                let data = host.status_snapshot();
                self.update_alert_border(host, data.disaster, data.situation, data.paused);
            }
            Refresh::Background => {
                self.refresh_background(host);
            }
            // VW10/VW18: the remaining refresh boundaries lack sufficient
            // owner contracts; never infer mission data from an integer token.
            _ => {}
        }
    }

    /// cIGZMessageTarget2: command QI wins even for a notification's type.
    pub fn notification(&mut self, host: &mut impl super::CityHost, message: Notification) -> bool {
        if message.command_interface {
            self.process_command(host, message.kind, message.data);
            return true;
        }
        let [a, b] = message.data;
        match message.kind {
            0x26D3_1EC0 => self.pre_city_init(host),
            0x26D3_1EC1 => self.post_city_init(host),
            0x26D3_1EC2 => {
                host.message_call(MessageCall::PreCityShutdown);
                self.pre_city_shutdown(host);
            }
            0x26D3_1EC4 => self.city_established(host),
            0x26D3_1EC5 => self.city_obliterated(host),
            0x26C6_3341 => {
                host.message_call(MessageCall::Load(a));
                if let Some(record) = host.read_view_record(a) {
                    // Read may precede Init; no city-sized allocation or UI.
                    let _ = self.load_object_record(host, &record);
                }
            }
            0x26C6_3343 => self.remove_all_controls(host, false),
            0x26C6_3344 => {
                host.message_call(MessageCall::Save(a));
                let available = host.reference_body_available(a);
                let record = self.save_object_record(host, available);
                host.write_view_record(a, record);
            }
            0x0AB9_9380 => self.refresh(host, Refresh::CityName),
            0x0772_FAD4 | 0x08DA_C3E7 | 0x08DA_C3E8 => self.refresh(host, Refresh::Budget),
            0x06CD_B65B => {
                if self.owner.lifecycle.objects.contains_key(&0xF4) {
                    self.refresh(host, Refresh::Renderer);
                }
            }
            0x4477_8231 => self.refresh(host, Refresh::Clock24),
            0x4477_8233 => self.refresh(host, Refresh::AmbientLight),
            0x6695_6814 => {
                self.refresh(host, Refresh::SimulationClock);
                self.refresh(host, Refresh::Population);
            }
            0x6695_6816 => self.refresh(host, Refresh::MayorRating),
            0x4A7F_B7E9 => self.refresh(host, Refresh::Speed),
            0x4A7F_B7E2 | 0xAA7F_B7E0 | 0xABBB_CA64 => {
                if host.paused_overlay_exists() {
                    self.refresh(host, Refresh::PausedOverlay);
                }
                self.refresh(host, Refresh::Speed);
                self.refresh(host, Refresh::Alert);
                if message.kind == 0xABBB_CA64 {
                    self.refresh(host, Refresh::Mission(message.attachment.unwrap_or(0), a));
                }
            }
            0xA6B7_9602 | 0xA6B7_9612 => {
                if !host.city_view_hidden() {
                    if message.kind == 0xA6B7_9602 {
                        self.refresh(host, Refresh::Zoom);
                        self.refresh(host, Refresh::Background);
                    }
                    self.temporary_hidden_pause(host);
                    if !self.owner.display.texts.is_empty() {
                        self.refresh(host, Refresh::CursorPosition);
                    }
                }
            }
            0xA6B7_9621 => self.temporary_hidden_pause(host),
            0xCA1B_1E64 | 0xAA1B_2002 => {
                if message.kind == 0xCA1B_1E64 && self.secondary_kind() == 0x4992_3239 {
                    self.close_owner_secondary(host);
                }
                host.message_call(MessageCall::DisasterButton {
                    window: 0x8991_EE08,
                    flag: 2,
                    enabled: message.kind == 0xAA1B_2002,
                });
                self.refresh(host, Refresh::Alert);
            }
            0xAA5C_F1E3 => {
                if host.modal_safe() {
                    host.message_call(MessageCall::BudgetFailure);
                    self.do_budget_failure(host);
                } else if !self.owner.deferred.iter().any(|m| m.kind == message.kind) {
                    host.deferred_reference(&message, true);
                    self.owner.deferred.push(message);
                }
            }
            0xAC08_3D7D => self.refresh(host, Refresh::Mission(message.attachment.unwrap_or(0), 1)),
            0x4A44_667F if message.progress_interface && self.owner.lifecycle.progress.is_some() => {
                host.message_call(MessageCall::Progress([a, b]));
            }
            _ => {}
        }
        true
    }

    /// Compatibility entry for the same one-message paint tail (§6n).
    pub fn replay_deferred(&mut self, host: &mut impl super::CityHost) {
        self.paint_deferred(host);
    }

    pub fn window_message(&mut self, host: &mut impl MessageHost, mut message: WindowMessage) -> bool {
        if host.tutorial_message(&mut message) {
            return true;
        }
        let [a, b, c] = message.data;
        match message.kind {
            0x230E_27AC => {
                let args: Vec<_> = message.arguments.iter().map(String::as_str).collect();
                self.process_cheat(host, a, &args)
            }
            3 if a == 0x49D9_93D4 => {
                host.message_call(MessageCall::Focus);
                self.invoke_catalog_item(host, b, c, false);
                true
            }
            0x13 | 0x14 | 0x0D => {
                if message.kind != 0x0D {
                    self.owner.mouse_inside = message.kind == 0x13;
                }
                self.refresh(host, Refresh::ClockHover);
                host.base_message(&message)
            }
            0x10 if host.focus_loss_stops_scroll() => {
                self.owner.motion.keyboard = [0, 0];
                self.owner.motion.edge = [0, 0];
                let edge = host.edge_scrolling_preference();
                self.recalc_scroll_velocity(host, edge);
                self.scroll_stop();
                host.message_call(MessageCall::MouseExit);
                true
            }
            0x1B if a == 5 || a == 6 => {
                let down = a == 5;
                if self.process_key(host, b, c, down) {
                    return true;
                }
                let command = self
                    .owner
                    .lifecycle
                    .objects
                    .get(&0x120)
                    .copied()
                    .and_then(|token| host.accelerator(token, b, c, down));
                if let Some(command) = command {
                    host.post_command(command);
                    true
                } else {
                    false
                }
            }
            0xE4A8_5D91 => !host.open_url(&message.text),
            0x6A5B_DFCD | 0x6A72_0993 | 0xEA5B_DFBC => {
                host.message_call(MessageCall::Advice(message.kind, message.data));
                host.base_message(&message)
            }
            _ => host.base_message(&message),
        }
    }

    /// Child filter owns button/tab routes; normal DoMessage leaves buttons to
    /// cGZWin. Unrecognized messages are not consumed by this filter.
    pub fn filter_message(&mut self, host: &mut impl MessageHost, mut message: WindowMessage) -> bool {
        if host.tutorial_message(&mut message) {
            return true;
        }
        if host.filter_window_passthrough() {
            return false;
        }
        if message.kind == 0x12 {
            return true;
        }
        if message.kind != 3 {
            return false;
        }
        match message.data[0] {
            0x2872_59F6 => {
                self.handle_button_activated(host, message.data[1]);
                true
            }
            0x2872_59F7 => {
                let tab = message.data[1];
                if tab == 0x2988_BC85 {
                    self.panel_command(host, 0x6A93_5C34);
                    return true;
                }
                let panel = match tab {
                    0x8988_BC94 => 3,
                    0x4988_BC6A => 2,
                    0xC988_BC79 => 1,
                    _ => return false,
                };
                let on = host.tab_toggled(tab);
                if panel == 1 && !host.city_established() && !self.do_establish_city_dialog(host) {
                    host.toggle_tab(tab, false);
                    return true;
                }
                self.apply_panel_tab(host, tab, panel, on);
                true
            }
            _ => false,
        }
    }
}
