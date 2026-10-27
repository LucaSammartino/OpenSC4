// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4MySimInputControlChangeJob and the shared building-tool slots.
use super::{BuildingKind, BuildingTool};

#[derive(Debug)]
pub struct ChangeJob(BuildingTool);
impl Default for ChangeJob {
    fn default() -> Self {
        Self(BuildingTool::new(BuildingKind::ChangeJob))
    }
}
impl std::ops::Deref for ChangeJob {
    type Target = BuildingTool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ChangeJob {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
