// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4MySimInputControlChangeHouse, mysim-tools §§1–4.
use super::{BuildingKind, BuildingTool};

#[derive(Debug)]
pub struct ChangeHouse(BuildingTool);
impl Default for ChangeHouse {
    fn default() -> Self {
        Self(BuildingTool::new(BuildingKind::ChangeHouse))
    }
}
impl std::ops::Deref for ChangeHouse {
    type Target = BuildingTool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ChangeHouse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
