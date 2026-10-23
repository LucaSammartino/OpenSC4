// SPDX-License-Identifier: GPL-3.0-or-later
//! View-owned dialog result handling (§6h; modal-dialogs §4.1).
use super::{ActionCall, ActionHost, View3DWin};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstablishData {
    pub difficulty: u32,
    pub city: String,
    pub mayor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogCall {
    Establish {
        group: u32,
        instance: u32,
        centre_horizontal: bool,
        free_height_divisor: u32,
        city_edit: u32,
        mayor_edit: u32,
        data: EstablishData,
    },
    Difficulty(u32),
    CityName(String),
    CityNamePreference(String),
    NameChanged,
    MayorPreference(String),
    SavePreferences,
    EstablishCity,
    HiddenPauseSimulatorClock,
    Bankruptcy {
        group: u32,
        instance: u32,
        id: u32,
    },
    HiddenResumeSimulatorClock,
}

pub trait DialogHost {
    fn establish_preferences(&self) -> EstablishData;
    /// The window system runs the modal dialog. Only result 0 supplies accepted
    /// edit values; allocation failure/cancel returns None. Calls other than
    /// Establish ignore the reply. Bankruptcy ignores even an accepted reply.
    fn dialog(&mut self, call: DialogCall) -> Option<EstablishData>;
    fn budget_dialog_enabled(&self) -> bool;
}

impl View3DWin {
    pub fn do_establish_city_dialog(&mut self, host: &mut impl DialogHost) -> bool {
        let data = host.establish_preferences();
        let Some(data) = host.dialog(DialogCall::Establish {
            group: 0x96A0_06B0,
            instance: 0x6A41_4973,
            centre_horizontal: true,
            free_height_divisor: 3,
            city_edit: 0xAA41_49BF,
            mayor_edit: 0xAA41_49D2,
            data,
        }) else {
            return false;
        };
        // Literal character limit from §6h, not a UTF-8 byte truncation.
        let city: String = data.city.chars().take(511).collect();
        for call in [
            DialogCall::Difficulty(data.difficulty),
            DialogCall::CityName(city.clone()),
            DialogCall::CityNamePreference(city),
            DialogCall::NameChanged,
            DialogCall::MayorPreference(data.mayor),
            DialogCall::SavePreferences,
            DialogCall::EstablishCity,
        ] {
            host.dialog(call);
        }
        true
    }

    pub fn do_budget_failure(&mut self, host: &mut (impl DialogHost + ActionHost)) {
        if host.budget_dialog_enabled() {
            host.dialog(DialogCall::HiddenPauseSimulatorClock);
            host.dialog(DialogCall::Bankruptcy {
                group: 0x96A0_06B0,
                instance: 0x0A5C_F71D,
                id: 0x2A5C_FB2C,
            });
            host.dialog(DialogCall::HiddenResumeSimulatorClock);
        }
        // Gate a suppresses only the dialog. Never save this bankrupt city.
        host.action(ActionCall::Region { prompt: false });
    }
}
