// SPDX-License-Identifier: GPL-3.0-or-later
//! Window commands and legend ownership (map-view Ã‚Â§Ã‚Â§1, 3, 6).
use super::*;

pub const MAP_PANELS: [u32; 6] = [
    0x8a28_71b1,
    0x8a28_71b2,
    0xaa32_bcd4,
    0xaa32_bcd5,
    0x8a28_71c3,
    0x8a28_71c4,
];
pub const LEGEND_PANEL: u32 = 0x8a90_9e00;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Legend {
    pub panel: u32,
    pub string_group: u32,
    pub title: u32,
    pub rows: Vec<LegendRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegendRow {
    pub index: usize,
    pub rgb: u32,
    pub string: Option<u32>,
}

impl Legend {
    pub fn from_exemplar(raw: Option<&RawExemplar>) -> Result<Self, Unsupported> {
        let mut legend = Self {
            panel: LEGEND_PANEL,
            string_group: 0x6a23_1eaa,
            title: 0x2a5e_854f,
            rows: Vec::new(),
        };
        if let Some(raw) = raw {
            let colors = raw.u32_list(0x4a0b_47f2)?;
            let strings = raw.u32_list(0x4a0b_47f3)?;
            legend.title = raw.optional_u32(0x4a0b_47f4)?.unwrap_or(legend.title);
            for (index, &color) in colors.iter().enumerate() {
                if color == 0 {
                    continue;
                }
                // [choice, §6] Diagnose the native one-past-end string read.
                if index == strings.len() {
                    return Err(Unsupported::LegendStringRead(index));
                }
                legend.rows.push(LegendRow {
                    index,
                    rgb: color & 0x00ff_ffff,
                    string: strings.get(index).copied(),
                });
            }
        }
        Ok(legend)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowEffect {
    Lifecycle(Effect),
    ExcludeOtherButtons(u32),
    HideControl(u32),
    InvalidateControl(u32),
    LegendTitle {
        group: u32,
        string: u32,
    },
    LegendSwatch {
        control: u32,
        rgb: u32,
    },
    LegendLabel {
        control: u32,
        group: u32,
        string: u32,
    },
    HidePanel(u32),
    InitCustomView(u64),
    ChildFlag {
        child: u32,
        iid: u32,
        value: bool,
    },
    PostCommand(u32),
    /// Ã‚Â§3.2 explicitly asks for a redraw, before the unsettled large-panel work.
    RedrawMap,
}

pub trait WindowSystem {
    fn effect(&mut self, effect: WindowEffect) -> Result<(), OwnerError>;
    fn visible(&self, panel: u32) -> bool;
    fn button_down(&self, button: u32) -> bool;
    fn tutorial_allows(&mut self, command: u32, button: u32) -> Result<bool, OwnerError>;
    /// Returns a handle to a newly created owned object, never a borrowed one.
    fn create_custom_view(&mut self, class: u32, iid: u32) -> Result<Option<u64>, OwnerError>;
    /// Ordinary false is distinct from an invalid adapter/service error.
    fn primitive(&mut self, effect: WindowEffect) -> Result<bool, OwnerError> {
        self.effect(effect)?;
        Ok(true)
    }
    fn message_server_present(&self) -> bool {
        true
    }
    fn query_control(&mut self, _control: u32, _iid: u32) -> Result<bool, OwnerError> {
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    View(u32),
    Toggle(bool),
    Group(u32),
    Unknown,
}

pub fn canonical_button(button: u32) -> u32 {
    match button {
        0x5100..=0x51ff | 0x6100..=0x61ff => button - 0x100,
        _ => button,
    }
}

pub fn button_action(button: u32) -> ButtonAction {
    use ButtonAction::*;
    match canonical_button(button) {
        0x5001 => View(0),
        0x5002 => View(5),
        0x5003 => View(3),
        0x5004 => View(0x1b),
        0x5006 => View(10),
        0x5007 => View(7),
        0x5009 => View(0x20),
        0x500a => View(2),
        0x500b => View(1),
        0x500c => View(0x1e),
        0x500d => View(0x1c),
        0x500e => View(8),
        0x500f => View(0x18),
        0x5010 => View(0x19),
        0x5011 => View(0x1a),
        0x6001..=0x600c => View(
            [0x28, 0x29, 0x2a, 0x32, 0x33, 0x34, 0x35, 0x36, 0x3c, 0x3d, 0x3e, 0x3f]
                [(canonical_button(button) - 0x6001) as usize],
        ),
        0x8001 => View(9),
        0x8003..=0x800c => View(0x4c + button - 0x8003),
        0x8100 => Toggle(false),
        0x8101 => Toggle(true),
        group @ (0x5005 | 0x5008 | 0x8002) => Group(group),
        _ => Unknown,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelRequest {
    Show,
    Hide,
    Large,
    Small,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelBoundary {
    pub request: PanelRequest,
    pub panels: [u32; 6],
    pub boundary: Unsupported,
}

impl MapView {
    /// Ã‚Â§1/QV3 gives panel identities, but not the visibility matrix. No foreign
    /// callback is asked to invent that owner policy.
    pub fn panel_request(&self, request: PanelRequest) -> PanelBoundary {
        PanelBoundary {
            request,
            panels: MAP_PANELS,
            boundary: Unsupported::PanelsQV3,
        }
    }

    pub fn show_with(
        &mut self,
        request: PanelRequest,
        resources: &mut impl Resources,
        windows: &mut impl WindowSystem,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<PanelBoundary, OwnerError> {
        // Ã‚Â§1 requires legend redraw and ResetDataView on showing; actual panel
        // visibility and its ordering relative to these calls remain QV3.
        if matches!(request, PanelRequest::Show | PanelRequest::Large) {
            self.display_legend(self.current_view.unwrap_or(0), resources, windows)?;
            self.reset_with(resources, windows, renderer, simulators)?;
        }
        Ok(self.panel_request(request))
    }

    pub fn display_legend(
        &self,
        view: u32,
        resources: &mut impl Resources,
        windows: &mut impl WindowSystem,
    ) -> Result<(), OwnerError> {
        for base in [0x8a90_9e00, 0x8a90_9e10] {
            for index in 0..9 {
                windows.primitive(WindowEffect::HideControl(base + index))?;
            }
        }
        windows.primitive(WindowEffect::InvalidateControl(0x8a90_9e00))?;
        let raw = if view == 0 {
            None
        } else {
            match resources.exemplar(exemplar_key(view)?)? {
                Some(raw) => Some(raw),
                None => return Ok(()), // Rows stay hidden, old title untouched.
            }
        };
        let title = match &raw {
            Some(raw) => raw.optional_u32(0x4a0b_47f4)?.unwrap_or(0x2a5e_854f),
            None => 0x2a5e_854f,
        };
        if windows.query_control(0x8a90_9d6a, 0x212c_dc1f)? {
            windows.primitive(WindowEffect::LegendTitle {
                group: 0x6a23_1eaa,
                string: title,
            })?;
        }
        if view == 0 {
            windows.primitive(WindowEffect::HidePanel(0x8a28_71c4))?;
        } else {
            let raw = raw.as_ref().expect("loaded nonzero legend");
            let strings = raw.u32_list(0x4a0b_47f3)?;
            for (index, &color) in raw.u32_list(0x4a0b_47f2)?.iter().enumerate() {
                if color == 0 {
                    continue;
                }
                let offset = u32::try_from(index).map_err(|_| Unsupported::LegendLayout)?;
                let swatch = 0x8a90_9e10u32.checked_add(offset).ok_or(Unsupported::LegendLayout)?;
                if windows.query_control(swatch, 0xc2af_a76f)? {
                    windows.primitive(WindowEffect::LegendSwatch {
                        control: swatch,
                        rgb: color & 0x00ff_ffff,
                    })?;
                }
                // [choice, §6] Keep earlier work, including this swatch, before
                // diagnosing the native unchecked label read.
                if index == strings.len() {
                    return Err(Unsupported::LegendStringRead(index).into());
                }
                if let Some(&string) = strings.get(index) {
                    let control = 0x8a90_9e00u32.checked_add(offset).ok_or(Unsupported::LegendLayout)?;
                    if windows.query_control(control, 0x212c_dc1f)? {
                        windows.primitive(WindowEffect::LegendLabel {
                            control,
                            group: 0x6a23_1eaa,
                            string,
                        })?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Standalone non-button messages. Tick delivery goes through tick_with;
    /// button notifications go through button_with (tutorial command 287259F6).
    pub fn window_message(
        &mut self,
        message: u32,
        value: Option<u32>,
        windows: &mut impl WindowSystem,
    ) -> Result<Option<PanelBoundary>, OwnerError> {
        match message {
            0xca28_71f0 | 0xca28_71f1 => {
                windows.effect(WindowEffect::RedrawMap)?;
                Ok(Some(self.panel_request(PanelRequest::Large)))
            }
            0x8a28_71d5 => {
                windows.effect(WindowEffect::PostCommand(0x6a93_5cf1))?;
                Ok(None)
            }
            0xa32c_acd4 | 0x0a32_cac3 => Ok(Some(self.panel_request(PanelRequest::Small))),
            _ => {
                for effect in self.notify(message, value)? {
                    windows.effect(WindowEffect::Lifecycle(effect))?;
                }
                Ok(None)
            }
        }
    }

    pub fn button_with(
        &mut self,
        button: u32,
        resources: &mut impl Resources,
        windows: &mut impl WindowSystem,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<Option<PanelBoundary>, OwnerError> {
        if !windows.tutorial_allows(0x2872_59f6, button)? {
            return Ok(None);
        }
        let view = match button_action(button) {
            ButtonAction::Unknown => return Ok(None),
            ButtonAction::Group(0x5008) if windows.button_down(0x8001) => 9,
            ButtonAction::Group(group) => return Err(Unsupported::GroupButtonQV3(group).into()),
            ButtonAction::Toggle(value) => {
                self.two_way_toggle = Some(value);
                self.reset_with(resources, windows, renderer, simulators)?;
                return Ok(None);
            }
            ButtonAction::View(view) => view,
        };
        windows.effect(WindowEffect::ExcludeOtherButtons(button))?;
        let result = self.select_with(view, resources, windows, renderer, simulators);
        if !matches!(result, Err(OwnerError::MissingExemplar)) {
            result?;
        }
        self.display_legend(view, resources, windows)?;
        let large = windows
            .visible(0xaa32_bcd5)
            .then(|| self.panel_request(PanelRequest::Large));
        self.mode_button = Some(button);
        Ok(large)
    }

    pub fn two_way_toggle(&self) -> Option<bool> {
        self.two_way_toggle
    }

    pub fn reset_with(
        &mut self,
        resources: &mut impl Resources,
        windows: &mut impl WindowSystem,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<(), OwnerError> {
        let Some(button) = self.mode_button else {
            return Err(Unsupported::GroupButtonQV3(0).into());
        };
        let view = match button_action(button) {
            ButtonAction::View(view) => view,
            ButtonAction::Group(0x5008) if windows.button_down(0x8001) => 9,
            _ => return Err(Unsupported::GroupButtonQV3(button).into()),
        };
        // §3.3: keep the same-view guard; false does not gate legend refresh.
        let result = self.select_with(view, resources, windows, renderer, simulators);
        if !matches!(result, Err(OwnerError::MissingExemplar)) {
            result?;
        }
        if windows.visible(0x8a28_71c3) {
            self.display_legend(view, resources, windows)?;
        }
        Ok(())
    }
}
