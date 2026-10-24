// SPDX-License-Identifier: GPL-3.0-or-later
//! QV3: child flags and independent requested-view memories.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewMemories {
    pub desirability: u32,
    pub traffic: u32,
    pub mode: u32,
}

impl MapView {
    pub(super) fn control_flag(windows: &mut impl WindowSystem, control: u32, flag: u8, value: bool) -> Result<(), OwnerError> {
        if windows.query_control(control, 0x22ba_0121)? {
            windows.primitive(WindowEffect::ControlFlag { control, flag, value })?;
        }
        Ok(())
    }

    pub(super) fn volume_enabled(windows: &mut impl WindowSystem, enabled: bool) -> Result<(), OwnerError> {
        for control in 0x8003..=0x800c { Self::control_flag(windows, control, 2, enabled)?; }
        Ok(())
    }

    pub(super) fn exclude_button(windows: &mut impl WindowSystem, button: u32) -> Result<(), OwnerError> {
        let (first, last) = match button {
            0x5001..=0x5011 => (0x5001, 0x5011),
            0x6001..=0x600c => (0x6001, 0x600c),
            0x8001..=0x8002 => (0x8001, 0x8002),
            0x8003..=0x800c => (0x8003, 0x800c),
            0x8100..=0x8101 => (0x8100, 0x8101),
            _ => return Ok(()),
        };
        for control in first..=last {
            if windows.query_control(control, 0x8810)? {
                windows.primitive(WindowEffect::SelectButton { control, selected: control == button })?;
            }
        }
        Ok(())
    }

    pub fn layout_with(&self, request: PanelRequest, windows: &mut impl WindowSystem) -> Result<(), OwnerError> {
        let (small, desire, traffic) = (request == PanelRequest::Small, request == PanelRequest::Desirability, request == PanelRequest::Traffic);
        if matches!(request, PanelRequest::Show | PanelRequest::Hide) { return Ok(()); }
        for (control, value) in [
            (0x8a28_71b1, small), (0x8a28_71b2, true),
            (0xaa32_bcd4, !small && !desire && !traffic), (0xaa32_bcd5, desire || traffic),
            (0x8a28_71c3, !small), (0x8a28_71c4, !small && !desire && !traffic && self.legend_content),
        ] { Self::control_flag(windows, control, 1, value)?; }
        if desire || traffic { Self::control_flag(windows, 0xcb72_f247, 1, traffic)?; }
        if !small {
            for control in [0x8100, 0x8101] { Self::control_flag(windows, control, 1, traffic)?; }
        }
        Ok(())
    }

    pub(super) fn show_panels(&mut self, request: PanelRequest, resources: &mut impl Resources, windows: &mut impl WindowSystem, renderer: &mut impl Renderer, simulators: &mut impl Simulators) -> Result<(), OwnerError> {
        match request {
            PanelRequest::Show => {
                if !windows.visible(0x8a28_71c3) { Self::control_flag(windows, 0x8a28_71b1, 1, true)?; }
                Self::control_flag(windows, 0x8a28_71b2, 1, true)?;
                for control in windows.message_controls() {
                    if windows.query_control(control, 0x22ba_0121)? { windows.primitive(WindowEffect::MessageTarget(control))?; }
                }
                self.display_legend(0, resources, windows)?;
                if self.memories.desirability == 99 {
                    for control in [0x5001, 0x6001, 0x8001, 0x8100, 0x8003] {
                        if windows.query_control(control, 0x8810)? {
                            windows.primitive(WindowEffect::SelectButton { control, selected: true })?;
                        }
                    }
                    Self::volume_enabled(windows, false)?;
                    self.memories.desirability = 0x28;
                }
                self.map_enabled = true;
                self.reset_with(resources, windows, renderer, simulators)?;
            }
            PanelRequest::Hide => {
                if !windows.query_control(0x8a28_71b1, 0x22ba_0121)? { return Ok(()); }
                for control in [0x4200, 0x8a28_71b1, 0x8a28_71b2, 0xaa32_bcd4, 0xaa32_bcd5, 0x8a28_71c3, 0x8a28_71c4] {
                    Self::control_flag(windows, control, 1, false)?;
                }
                self.map_enabled = false;
                self.reset_with(resources, windows, renderer, simulators)?;
            }
            _ => self.layout_with(request, windows)?,
        }
        Ok(())
    }

    pub fn memories(&self) -> ViewMemories { self.memories }

    /// Full message entry point; returns whether the owner consumes the message.
    pub fn message_with(&mut self, message: u32, value: Option<u32>, resources: &mut impl Resources, windows: &mut impl WindowSystem, renderer: &mut impl Renderer, simulators: &mut impl Simulators) -> Result<bool, OwnerError> {
        match message {
            0xca28_71f0 | 0xca28_71f1 => {
                self.poll_highlights(renderer, simulators)?;
                self.draw_with(renderer, simulators)?;
                self.layout_with(PanelRequest::Large, windows)?;
                return Ok(true);
            }
            0xa32c_acd4 | 0x0a32_cac3 | 0xea8b_80a7 | 0xea8b_80a8 => self.layout_with(PanelRequest::Small, windows)?,
            0x4a32_ca92 | 0xea8b_80a6 => {
                let (layout, view) = if windows.query_control(0x5005, 0x8810)? && windows.button_down(0x5005) {
                    (PanelRequest::Desirability, self.memories.desirability)
                } else if self.source == 76 {
                    (PanelRequest::Traffic, self.memories.traffic)
                } else if self.source == 9 {
                    (PanelRequest::Traffic, 9)
                } else { (PanelRequest::Large, self.memories.mode) };
                self.layout_with(layout, windows)?;
                let result = self.select_with(view, resources, windows, renderer, simulators);
                if !matches!(result, Err(OwnerError::MissingExemplar)) { result?; }
                self.display_legend(view, resources, windows)?;
            }
            0xea28_71d9 | 0xea28_71e9 => self.show_panels(PanelRequest::Hide, resources, windows, renderer, simulators)?,
            _ => { self.window_message(message, value, windows)?; }
        }
        Ok(false)
    }
    pub fn map_enabled(&self) -> bool { self.map_enabled }
    /// Raw saved/queue state consumed by ResetDataView, independent of D/T/M.
    pub fn set_reset_requests(&mut self, saved: u32, queued: Vec<u32>) {
        self.saved_view = saved;
        self.queued_views = queued;
    }
}
