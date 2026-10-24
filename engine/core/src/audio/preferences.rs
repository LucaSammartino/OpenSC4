// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled preferences IO, audio-owner-contracts §21. Full dialog/song rules: AU43.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPreferences {
    pub sound_enabled: bool,
    pub master: u32,
    pub effects: u32,
    pub music: u32,
    pub further_enabled: bool,
    pub songs: [[u8; 30]; 2],
}

impl AudioPreferences {
    pub fn select_defaults(&mut self, lists: [&[u8]; 2]) {
        for (output, input) in self.songs.iter_mut().zip(lists) {
            let count = input.len().min(30);
            output[..count].copy_from_slice(&input[..count]);
        }
    }
}

pub trait PreferencesHost {
    /// Returns an acquired service reference.
    fn acquire_sound_system(&mut self, service: u32, iid: u32) -> Option<u64>;
    fn sound_event(&mut self, system: u64, event: u32, arguments: [u32; 4]);
    fn post(&mut self, message: u32, effects: u32, music: u32);
    fn release(&mut self, reference: u64);
    fn detach(&mut self, window: u64);
    fn control(&mut self, control: u32);
}

/// Retained window state supplied by the caller; constructor defaults are AU43.
#[derive(Debug)]
pub struct PreferencesWinProc {
    pub primary: Option<u64>,
    pub modal: Option<u64>,
    pub notify_modal: bool,
    pub preferences_reference: Option<u64>,
    pub values: AudioPreferences,
}

impl PreferencesWinProc {
    pub fn update_sound_system(&self, host: &mut impl PreferencesHost) {
        if let Some(system) = host.acquire_sound_system(0x23CE_CBA0, 0x1060_77D2) {
            host.sound_event(
                system,
                0x2A5B_69C0,
                [
                    if self.values.sound_enabled {
                        self.values.master
                    } else {
                        0
                    },
                    0,
                    0,
                    0,
                ],
            );
            host.release(system);
        }
        host.post(0xAA65_0860, self.values.effects, self.values.music);
    }

    pub fn update_preferences(&self, preferences: &mut AudioPreferences, host: &mut impl PreferencesHost) {
        preferences.clone_from(&self.values);
        self.update_sound_system(host);
    }

    pub fn end_dialog(&mut self, host: &mut impl PreferencesHost) {
        if self.notify_modal {
            host.post(0xAA65_0861, 0, 0);
        }
        if let Some(window) = self.primary.take() {
            host.detach(window);
            host.release(window);
        }
    }

    pub fn shutdown(&mut self, host: &mut impl PreferencesHost) {
        if let Some(window) = self.primary.take() {
            host.release(window);
        }
    }

    /// Only settled notification cases are executable. Type 3's filter/action
    /// matrix must be supplied by a future AU43 answer, not forwarded wholesale.
    pub fn do_message(
        &mut self,
        window: u64,
        enabled: bool,
        kind: u32,
        key: u32,
        host: &mut impl PreferencesHost,
    ) -> Result<bool, PreferencesOpen> {
        if !enabled || !(self.primary == Some(window) || self.modal == Some(window)) {
            return Ok(false);
        }
        match kind {
            5 if key == 27 => {
                host.control(0xEA55_09E3);
                Ok(false)
            }
            0x9A15_52D2 => {
                host.control(0x8A55_0C56);
                Ok(false)
            }
            3 => Err(PreferencesOpen::Au43),
            // Unspecified other kinds are not given a fabricated success value.
            _ => Err(PreferencesOpen::Au43),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferencesOpen {
    Au43,
}

/// Selector zero and nonzero have independent tuning banks (§21.3).
pub fn slider_volume(slider: u32, selector: u32, bounds: [(u32, u32); 2]) -> u32 {
    let (min, max) = bounds[usize::from(selector != 0)];
    match slider {
        0 => min,
        1000 => max,
        _ => {
            let difference = max.wrapping_sub(min);
            let scaled = f64::from(difference) * f64::from(slider) * f64::from(f32::from_bits(0xBA83_126F));
            let converted = crate::x87::ftol(scaled) as u32;
            let value = min.wrapping_sub(converted);
            if value < min {
                min
            } else if value > max {
                max
            } else {
                value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Host {
        available: bool,
        calls: Vec<(u32, [u32; 4])>,
        releases: Vec<u64>,
        controls: Vec<u32>,
    }
    impl PreferencesHost for Host {
        fn acquire_sound_system(&mut self, service: u32, iid: u32) -> Option<u64> {
            assert_eq!((service, iid), (0x23CE_CBA0, 0x1060_77D2));
            self.available.then_some(9)
        }
        fn sound_event(&mut self, _: u64, event: u32, args: [u32; 4]) {
            self.calls.push((event, args));
        }
        fn post(&mut self, message: u32, effects: u32, music: u32) {
            self.calls.push((message, [effects, music, 0, 0]));
        }
        fn release(&mut self, reference: u64) {
            self.releases.push(reference);
        }
        fn detach(&mut self, window: u64) {
            self.controls.push(window as u32);
        }
        fn control(&mut self, control: u32) {
            self.controls.push(control);
        }
    }
    fn window() -> PreferencesWinProc {
        PreferencesWinProc {
            primary: Some(1),
            modal: Some(2),
            notify_modal: true,
            preferences_reference: Some(3),
            values: AudioPreferences {
                sound_enabled: true,
                master: 800,
                effects: 500,
                music: 200,
                further_enabled: false,
                songs: [[0; 30]; 2],
            },
        }
    }

    #[test]
    fn au9_10_service_failure_still_posts_and_update_copies() {
        let mut window = window();
        let mut host = Host::default();
        window.update_sound_system(&mut host);
        assert_eq!(host.calls, [(0xAA65_0860, [500, 200, 0, 0])]);
        host.available = true;
        window.values.sound_enabled = false;
        window.values.songs[1][29] = 7;
        let mut saved = window.values.clone();
        saved.master = 1;
        window.update_preferences(&mut saved, &mut host);
        assert_eq!(saved, window.values);
        assert_eq!(host.calls[1], (0x2A5B_69C0, [0; 4]));
        assert_eq!(host.releases, [9]);
    }

    #[test]
    fn au9_11_12_slider_endpoints_offsets_and_banks() {
        for selector in [0, 1, u32::MAX] {
            assert_eq!(
                [0, 500, 1000].map(|s| slider_volume(s, selector, [(0, 1024); 2])),
                [0, 512, 1024]
            );
        }
        assert_eq!(slider_volume(500, 0, [(100, 1100), (0, 1024)]), 600);
        assert_eq!(slider_volume(500, 1, [(100, 1100), (0, 1024)]), 512);
        // Negative f32 multiplier plus truncation, not nearest rounding.
        assert_eq!(slider_volume(1, 0, [(0, 1024); 2]), 1);
    }

    #[test]
    fn au9_13_14_15_selection_bounds_escape_and_window_lifetime() {
        let mut window = window();
        window.values.select_defaults([&[1; 31], &[2; 31]]);
        assert_eq!(window.values.songs, [[1; 30], [2; 30]]);
        let mut host = Host::default();
        assert_eq!(window.do_message(1, false, 5, 27, &mut host), Ok(false));
        assert_eq!(window.do_message(9, true, 5, 27, &mut host), Ok(false));
        assert!(host.controls.is_empty());
        assert_eq!(window.do_message(2, true, 5, 27, &mut host), Ok(false));
        assert_eq!(host.controls, [0xEA55_09E3]);
        assert_eq!(window.do_message(1, true, 0x9A15_52D2, 0, &mut host), Ok(false));
        assert_eq!(window.do_message(1, true, 3, 0, &mut host), Err(PreferencesOpen::Au43));
        window.end_dialog(&mut host);
        window.shutdown(&mut host);
        assert_eq!(host.releases, [1]);
        assert_eq!(host.calls, [(0xAA65_0861, [0; 4])]);
    }
}
