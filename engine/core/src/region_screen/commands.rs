// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl RegionScreen {
    pub fn button(
        &mut self,
        region: &mut Region,
        view: &mut RegionView,
        host: &mut impl Host,
        id: u32,
        notification: u32,
        value: bool,
    ) -> Result<bool> {
        if id == 0xea5a_96e6 && notification == 0x2872_59f7 {
            view.enable_grid(host, value);
            return Ok(true);
        }
        if notification != 0x2872_59f6 {
            return Ok(false);
        }
        let dialog = match id {
            0x4a56_0000 => return self.play_selected(region, view, host),
            0x4a56_0001 => {
                host.restore_signposts();
                Some(Dialog::DeleteCity)
            }
            // SPEC-GAP: QR2/QR4 leave post-import ownership/refresh unresolved.
            0x4a56_0002 => return Err(Unsupported::Open("QR2/QR4")),
            0x4a56_0003 => {
                if let Some(id) = view.selected {
                    host.reset_snapshots(id);
                    return Err(Unsupported::Open("RGV5"));
                }
                return Ok(false);
            }
            0x2a5b_0000 => Some(Dialog::NewRegion),
            0x2a5b_0001 => Some(Dialog::LoadRegion),
            0x2a5b_0002 => Some(Dialog::DeleteRegion),
            0x098f_4f6c => Some(Dialog::VideoPreferences),
            0x0a55_10a9 => Some(Dialog::GamePreferences),
            0x8a1d_a655 => Some(Dialog::SnapshotTaker),
            0xca1d_a670 => Some(Dialog::SnapshotAlbum),
            0x26c1_0a3e => {
                self.save_region_position(region, host);
                host.request_quit();
                return Ok(true);
            }
            0xaba2_90e1 | 0xcba2_90ec | 0xaba2_90f6 => {
                // SPEC-GAP: §5 lists three map modes without numbers; use table order 0/1/2.
                let mode = match id {
                    0xaba2_90e1 => 0,
                    0xcba2_90ec => 1,
                    _ => 2,
                };
                view.set_map_mode(host, mode);
                return Ok(true);
            }
            0x09eb_f2bd | 0x09eb_f2c3 | 0x09eb_f2c8 | 0x4a77_9a09 | 0x4a77_9a1a | 0x4bb9_2c1f | 0xa98f_4f88
            | 0xca5c_fee2 | 0xebb9_1356 => return Err(Unsupported::Open("QR3")),
            _ => return Ok(false),
        };
        if let Some(dialog) = dialog {
            host.show_dialog(dialog, view.selected);
        }
        Ok(true)
    }

    pub fn process_command(
        &mut self,
        region: &mut Region,
        view: &mut RegionView,
        host: &mut impl Host,
        id: u32,
        arguments: &[&str],
    ) -> Result<bool> {
        match id {
            0x6a93_5cd8 => self.velocity.0 = -self.scroll_speed,
            0x6a93_5ce0 => self.velocity.0 = self.scroll_speed,
            0x6a93_5cdd => self.velocity.1 = -self.scroll_speed,
            0x6a93_5ce2 => self.velocity.1 = self.scroll_speed,
            0x2a94_826e | 0x2a94_8275 => self.velocity.0 = 0.0,
            0x2a94_826b | 0x2a94_8272 => self.velocity.1 = 0.0,
            0x6a93_5cf1 => {
                self.held = None;
                view.clear_selection(host)?;
            }
            0x6a97_57c2 => return Err(Unsupported::Open("QR3")),
            0x6aa9_fe51 => {
                self.preference_f0a = match arguments.first() {
                    None => !self.preference_f0a,
                    Some(word) => word
                        .parse::<i32>()
                        .map(|n| n != 0)
                        .map_err(|_| Unsupported::Precondition("preference byte"))?,
                };
            }
            0x6a93_5e3c => {
                self.save_region_position(region, host);
                host.save_preferences();
            }
            0x0bb3_c277 => return Ok(arguments.first().is_some_and(|name| self.load_region(host, name))),
            0x0bb2_747d => {
                let [name, size, rest @ ..] = arguments else {
                    return Err(Unsupported::Precondition("LaunchCity arguments"));
                };
                let size = match *size {
                    "small" => 0,
                    "medium" => 1,
                    "large" => 2,
                    "any" => 3,
                    word => word.parse().map_err(|_| Unsupported::Precondition("LaunchCity size"))?,
                };
                return self.launch_city(
                    region,
                    host,
                    (!name.is_empty()).then_some(*name),
                    size,
                    rest.first() == Some(&"empty"),
                );
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn message(
        &mut self,
        view: &mut RegionView,
        host: &mut impl Host,
        message: u32,
        a: i32,
        b: i32,
    ) -> Result<bool> {
        match message {
            0x8a4b_ac53 => {
                view.scrolling = true;
                host.redraw();
            }
            0x8a4b_ac5b => view.scrolling = false,
            0xaaa1_cdf2 => self.scroll(view, host, a, b),
            0x1b if a == 5 || a == 6 => {
                if let Some(command) = host.accelerator_command(message, a as u32) {
                    host.post_command(command);
                }
            }
            0x231b_bf91 => host.preference_visibility(false),
            0x2b96_b3ea => {
                host.post_camera_view_init();
                host.preference_visibility(true);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
