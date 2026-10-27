// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4MySimInputControlMoveIn, holding the MySim interface rather than object.
use super::{BuildingKind, BuildingTool};

#[derive(Debug)]
pub struct MoveIn(BuildingTool);
impl Default for MoveIn {
    fn default() -> Self {
        Self(BuildingTool::new(BuildingKind::MoveIn))
    }
}
impl std::ops::Deref for MoveIn {
    type Target = BuildingTool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for MoveIn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
