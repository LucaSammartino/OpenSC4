// SPDX-License-Identifier: GPL-3.0-or-later
//! cISC4App slots 19/20, application.md §13.3. Raw bytes are not booleans.

pub const SETTINGS: [(u32, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (6, 7),
    (4, 5),
    (5, 6),
    (8, 9),
    (7, 8),
    (9, 10),
    (10, 12),
    (15, 28),
];

pub trait VideoHost {
    fn set_setting(&mut self, id: u32, value: u32);
    fn setting(&mut self, id: u32) -> u32;
    fn post_video_changed(&mut self, kind: u32);
}

impl super::Application {
    pub fn apply_video_preferences(&self, record: &[u8; 29], host: &mut impl VideoHost) {
        for (id, offset) in SETTINGS {
            host.set_setting(id, u32::from(record[offset]));
        }
        if self.message_server2_available {
            host.post_video_changed(0xC9F7_75BB);
        }
    }

    pub fn get_auto_video_preferences(&self, record: &mut [u8; 29], host: &mut impl VideoHost) {
        for (id, offset) in SETTINGS {
            record[offset] = host.setting(id) as u8;
            if id == 10 {
                record[11] = 1;
            }
        }
        if let Some((w, h)) = match host.setting(11) {
            0 => Some((800u32, 600u32)),
            1 => Some((1024, 768)),
            2 => Some((1280, 1024)),
            3 => Some((1600, 1200)),
            _ => None,
        } {
            record[16..20].copy_from_slice(&w.to_le_bytes());
            record[20..24].copy_from_slice(&h.to_le_bytes());
        }
        if let Some(depth) = match host.setting(12) {
            0 => Some(16u32),
            1 => Some(32),
            _ => None,
        } {
            record[24..28].copy_from_slice(&depth.to_le_bytes());
        }
    }
}
