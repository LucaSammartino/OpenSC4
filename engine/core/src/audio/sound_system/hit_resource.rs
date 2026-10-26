// SPDX-License-Identifier: GPL-3.0-or-later
//! §5.2 legacy framing. Generic property serialization remains foreign IO.
use super::hit::HitPropertySetTrackResource;
use super::hit_config::{split_token, unsigned_number};

pub const LEGACY_FIELDS: [u32; 27] = [
    0xFFAE_C905,
    0xFF5F_15E1,
    0xFF4D_CD0D,
    0xFF05_0A94,
    0xFFB0_333D,
    0xFF70_6616,
    0xFFCC_156F,
    0xFFCC_3B29,
    0xFF89_1EAA,
    0xFFAE_9DDB,
    0xFFF7_B8DF,
    0xFF90_AAC8,
    0xFF3C_2160,
    0xFFD7_EE4D,
    0xFF59_FDD8,
    0xFF2D_7CD1,
    0xFF02_46FE,
    0xFFFF_4E79,
    0xFF59_5FFD,
    0xFF8D_CFD8,
    0xFF69_510B,
    0xFF09_9DB2,
    0xFF53_DBF3,
    0xFF3E_C880,
    0xFF47_F685,
    0xFFCA_A344,
    0xFF30_3B1B,
];

impl HitPropertySetTrackResource {
    pub fn legacy_text(&self) -> String {
        let mut text = String::from("TKDT,2,,");
        for (index, id) in LEGACY_FIELDS.into_iter().enumerate() {
            if let Some(value) = self.properties.get(&id) {
                if matches!(index + 1, 1 | 2 | 9 | 13..=16) {
                    text.push_str(&format!("0x{:08x}", *value as u32));
                } else {
                    text.push_str(&value.to_string());
                }
            }
            text.push(',');
        }
        text.push_str("ETKD");
        text
    }
    pub fn write_legacy(&self, mut write: impl FnMut(&[u8]) -> bool) -> bool {
        write(self.legacy_text().as_bytes());
        true
    }
    /// Enclosing Read installs the instance even on subsequent failure.
    pub fn read_legacy_record(&mut self, record: &[u8]) -> bool {
        self.properties.insert(0xFF6F_9546, self.key.instance as i32);
        let Some(first) = record.get(..4) else {
            return false;
        };
        let mut header = u32::from_le_bytes(first.try_into().expect("four bytes"));
        let mut offset = 4;
        if header as usize == record.len() - 4 {
            let Some(second) = record.get(4..8) else {
                return false;
            };
            header = u32::from_le_bytes(second.try_into().expect("four bytes"));
            offset += 4;
        }
        if header == 0x544B_4432 {
            offset += 8;
        }
        // Invalid original reads are not emulated; retain the §5.2 observation boundary.
        let Some(buffer) = record.get(offset..) else {
            return false;
        };
        let mut text = String::from_utf8_lossy(buffer).into_owned();
        split_token(&mut text, ',');
        let version = unsigned_number(&split_token(&mut text, ','));
        split_token(&mut text, ',');
        let mut spatial = false;
        for (index, id) in LEGACY_FIELDS.into_iter().enumerate() {
            let field = index + 1;
            if (version == 0 && field == 6) || (version < 2 && field >= 22) {
                continue;
            }
            let token = split_token(&mut text, ',');
            let value = unsigned_number(&token);
            if field == 1 && !token.is_empty() && value == self.key.instance {
                return false;
            }
            if field == 7 {
                spatial = !token.is_empty() && value != 0;
            }
            if field == 9 {
                self.properties.insert(
                    id,
                    if !token.is_empty() && value != 0 {
                        value as i32
                    } else if spatial {
                        0x7DCD_ED3D
                    } else {
                        0x7DCD_ED2D
                    },
                );
            } else if !token.is_empty() {
                self.properties.insert(id, value as i32);
            }
        }
        true
    }
}
