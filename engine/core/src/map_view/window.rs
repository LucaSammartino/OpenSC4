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
    pub rows: Vec<(u32, u32)>,
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
            // Literal Ã‚Â§2 lists: a single title ID. QV12 asks about other
            // cardinalities/mismatched rows; never zip and silently lose rows.
            let title = raw.u32_list(0x4a0b_47f4)?;
            if colors.len() != strings.len() || title.len() != 1 {
                return Err(Unsupported::LegendShape);
            }
            legend.title = title[0];
            legend.rows = colors.iter().copied().zip(strings.iter().copied()).collect();
        }
        Ok(legend)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowEffect {
    Lifecycle(Effect),
    ExcludeOtherButtons(u32),
    Legend(Legend),
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
    fn create_custom_view(&mut self, class: u32, iid: u32) -> Result<u64, OwnerError>;
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
            self.display_legend(windows)?;
            self.reset_with(resources, windows, renderer, simulators)?;
        }
        Ok(self.panel_request(request))
    }

    pub fn display_legend(&self, windows: &mut impl WindowSystem) -> Result<(), OwnerError> {
        windows.effect(WindowEffect::Legend(Legend::from_exemplar(self.resource.as_ref())?))
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
        self.mode_button = Some(button);
        self.select_with(view, resources, windows, renderer, simulators)?;
        self.display_legend(windows)?;
        Ok(Some(self.panel_request(PanelRequest::Large)))
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
        // Literal Ã‚Â§3.3: call SetDataView again, including its same-view guard.
        // QV12 asks whether native Reset bypasses that guard.
        self.select_with(view, resources, windows, renderer, simulators)?;
        if windows.visible(0x8a28_71c3) {
            self.display_legend(windows)?;
        }
        Ok(())
    }
}
