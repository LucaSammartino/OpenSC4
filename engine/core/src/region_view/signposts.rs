// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SignpostData {
    pub rating: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Animator {
    pub start: u32,
    pub child: Option<Window>,
}

impl RegionView {
    pub fn update_signpost(&mut self, _host: &mut impl Host, _id: CityId) -> Result<()> {
        // SPEC-GAP: RGV6 leaves the child interface contract open.
        Err(Unsupported::Open("RGV6"))
    }
}
