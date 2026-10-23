// SPDX-License-Identifier: GPL-3.0-or-later
use super::{DateText, Error, Gap, Result, SnapshotClock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewPosition {
    pub zoom: u32,
    pub rotation: u32,
    pub x: u32,
    pub y: u32,
}

/// Typed world inputs, not a native GetGraphicInfoForCurrentCity result.
/// The ninth value is explicit because its meaning remains SN1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicInfo {
    pub user: String,
    pub city: String,
    pub mayor: String,
    pub day: i32,
    pub date: String,
    pub population: i32,
    pub funds: i64,
    pub further: i32,
    pub view: ViewPosition,
}

impl GraphicInfo {
    pub fn sample_time(&mut self, clock: &mut impl SnapshotClock, text: &mut impl DateText) {
        self.day = clock.simulation_day();
        self.date = text.format_day(self.day);
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        // SN5: portable text inputs are UTF-8. Refuse ambiguous LF-containing fields.
        let fields = [
            "SanityCheck".to_owned(),
            self.user.clone(),
            self.city.clone(),
            self.mayor.clone(),
            self.day.to_string(),
            self.date.clone(),
            self.population.to_string(),
            self.funds.to_string(),
            self.further.to_string(),
            format!(
                "{},{},{},{}",
                self.view.zoom, self.view.rotation, self.view.x, self.view.y
            ),
        ];
        if fields.iter().any(|s| s.contains('\n')) {
            return Err(Error::Gap(Gap::EmbeddedLineFeed));
        }
        let mut bytes = Vec::new();
        for field in fields {
            bytes.extend(field.bytes().map(|b| b ^ 0x7b));
            bytes.push(b'\n' ^ 0x7b);
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CityContext {
    pub name: String,
    /// An explicit supplied number; no inference about region indexing (SN1).
    pub number: u32,
    pub info: GraphicInfo,
}

/// Raw deobfuscated fields, so parsing never guesses numeric conversion or text
/// encoding. Caption assembly remains SN2; these are its five documented inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Information {
    fields: [Vec<u8>; 9],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptionFields {
    pub city: Vec<u8>,
    pub mayor: Vec<u8>,
    pub date: Vec<u8>,
    pub population: Vec<u8>,
    pub funds: Vec<u8>,
}

impl Information {
    pub const LANGUAGE_TABLE: u32 = 0x6a23_1eaa;
    pub const LANGUAGE_STRINGS: [u32; 5] = [0xea94_6efe, 0xea94_6efd, 0xea94_6efc, 0xea94_6efb, 0xea94_6efa];

    pub fn decode(obfuscated: &[u8]) -> Result<Option<Self>> {
        let plain: Vec<_> = obfuscated.iter().map(|b| b ^ 0x7b).collect();
        let mut parts = plain.split(|&b| b == b'\n');
        if parts.next() != Some(b"SanityCheck".as_slice()) {
            return Ok(None);
        }
        let mut fields: [Vec<u8>; 9] = Default::default();
        for field in &mut fields {
            *field = parts.next().ok_or(Error::Gap(Gap::MalformedInformation))?.to_vec();
        }
        // SN5: preserve fields literally; do not guess atoi/error behavior or caption layout.
        Ok(Some(Self { fields }))
    }

    pub fn fields(&self) -> &[Vec<u8>; 9] {
        &self.fields
    }
    pub fn city(&self) -> &[u8] {
        &self.fields[1]
    }
    pub fn funds(&self) -> &[u8] {
        &self.fields[6]
    }
    pub fn caption_fields(&self) -> CaptionFields {
        CaptionFields {
            city: self.fields[1].clone(),
            mayor: self.fields[2].clone(),
            date: self.fields[4].clone(),
            population: self.fields[5].clone(),
            funds: self.fields[6].clone(),
        }
    }
}
