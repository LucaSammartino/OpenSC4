// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable, deliberately incomplete lot configuration editor.
//!
//! Source: `docs/specs/lot-configuration-editor.md`, published round 3/3.
//! No window lifecycle, rendering, foreign parser or unknown geometry is
//! synthesized. See `coverage.md` for every inventory entry. Host calls are
//! primitive widgets, effects and explicitly foreign services, never editor
//! decisions. This module is not a drop-in implementation of the native window.
mod dialogs;
mod geometry;
mod workflows;
mod windows;
pub use crate::lot_config_manager::LotConfigObject;
pub use dialogs::*;
pub use geometry::*;
pub use workflows::*;
pub use windows::*;

pub type Result<T> = std::result::Result<T, Unsupported>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Dependency(&'static str),
    MissingAcceptedGrid(usize),
    BitOutsideLogicalRange(u32),
    NumericConversion,
    MissingConfiguration,
    MissingRecord,
    AllocationFault,
}

/// Portable pointer identity, independent of record contents and vector index.
pub type RecordId = u64;
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub id: RecordId,
    pub data: LotConfigObject,
    /// Foundation stack construction leaves the creation word indeterminate.
    pub creation_known: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Configuration {
    pub id: u32,
    pub name: String,
    pub size: [u8; 2],
    /// Fields +38, +30, +34 in acceptance order, not invented property names.
    pub numeric: [f32; 3],
    pub ignored_numeric: i32,
    pub minimum: u8,
    pub byte42: u8,
    pub checkboxes: u8,
    pub compatibility: [Vec<bool>; 3],
    pub objects: Vec<Record>,
    pub fences: Vec<[u8; 4]>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Lot".into(),
            size: [1, 1],
            numeric: [5.0, 90.0, 0.0],
            ignored_numeric: 0,
            minimum: 0,
            byte42: 0,
            checkboxes: 0,
            // [choice] Unknown initial bit-vector lengths are not invented.
            compatibility: Default::default(),
            objects: Vec::new(),
            fences: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identity {
    pub id: u32,
    pub key: [u32; 3],
    pub path: String,
    /// Snapshot clone does not AddRef. The editor performs explicit transfers.
    pub references: [Option<u64>; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Title,
    Draw,
    /// Pending call to the unresolved TriggerLotConfigurationRedraw helper;
    /// deliberately distinct from known direct DrawCurrent/Invalidate calls.
    RedrawRequired,
    Invalidate,
    Refresh,
    Prepare3D,
    ReleaseCapture,
    Cursor(usize),
    AddRef(u64),
    Release(u64),
    Register(u64),
    Unregister(u64),
    RemoveDialog(u64),
    ShutdownDialog(u64),
    Focus,
    Seed(u64, u32, Value),
    GridInit(u64, u32),
    GridRow(u64, u32, usize, String, Cell),
    GridSelect(u64, u32, usize),
    Sort(u32, u32, bool, u32, u32),
    Scroll(u32, u32),
    Callback(u64, bool),
    Message(String, u32),
    Instructions(&'static str),
    Saved(Identity),
    RegisteredConfiguration(u32),
    RefreshChooser,
    NotifyChooser,
    RemoveEditor,
    SharedSaveAs(bool),
    BrowserSetup(u64, [u32; 2], u32, u32, bool),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Integer(i32),
    Checked(bool),
    ClearFlag(u32),
    Selection(u32),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Text(String),
    Image(u64),
}

/// Hosts must not re-enter the editor. No RefCell is used for getter state.
#[derive(Debug)]
pub struct Editor {
    pub current: Option<Configuration>,
    pub identity: Identity,
    pub dirty: u8,
    pub selected: Option<RecordId>,
    pub selected_index: usize,
    pub drag: bool,
    pub capturing: bool,
    pub mode: u32,
    pub pending_ids: Vec<u32>,
    pub scale: f32,
    pub condition: u8,
    pub slope: u32,
    pub chooser: bool,
    pub cursors: [Option<u64>; 7],
    next_id: u64,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            current: None,
            identity: Identity::default(),
            dirty: 0,
            selected: None,
            selected_index: 0,
            drag: false,
            capturing: false,
            mode: 0,
            pending_ids: Vec::new(),
            scale: 1.0,
            condition: 0,
            slope: 0,
            chooser: false,
            cursors: [None; 7],
            next_id: 1,
        }
    }
}
impl Editor {
    pub fn config(&self) -> Result<&Configuration> {
        self.current.as_ref().ok_or(Unsupported::MissingConfiguration)
    }
    pub fn config_mut(&mut self) -> Result<&mut Configuration> {
        self.current.as_mut().ok_or(Unsupported::MissingConfiguration)
    }
    pub fn record(&self, id: RecordId) -> Option<&LotConfigObject> {
        self.current
            .as_ref()?
            .objects
            .iter()
            .find(|r| r.id == id)
            .map(|r| &r.data)
    }
    pub fn record_mut(&mut self, id: RecordId) -> Option<&mut LotConfigObject> {
        self.current
            .as_mut()?
            .objects
            .iter_mut()
            .find(|r| r.id == id)
            .map(|r| &mut r.data)
    }
    /// Inserts an already constructed record; geometry and policy stay in callers.
    pub fn insert(&mut self, data: LotConfigObject, front: bool) -> RecordId {
        let id = self.next_id;
        self.next_id += 1;
        let r = Record {
            id,
            data,
            creation_known: true,
        };
        let c = self.current.as_mut().expect("configuration required for insertion");
        if front {
            c.objects.insert(0, r);
        } else {
            c.objects.push(r);
        }
        id
    }
    pub fn set_dirty(&mut self, byte: u8, h: &mut impl Host) {
        self.dirty = byte;
        // Full SetTitle formatting is LE5: request it without inventing a name.
        h.effect(Effect::Title);
    }
    pub fn set_selection(&mut self, id: Option<RecordId>, h: &mut impl Host) {
        if id != self.selected {
            self.drag = false;
            if self.capturing {
                self.capturing = false;
                h.effect(Effect::ReleaseCapture);
            }
        }
        self.selected = id;
        self.selected_index = id
            .and_then(|id| self.current.as_ref()?.objects.iter().position(|r| r.id == id))
            .unwrap_or(0);
    }
    pub(crate) fn draw(&self, h: &mut impl Host) {
        h.effect(Effect::Draw);
        h.effect(Effect::Invalidate);
    }
    pub(crate) fn edited(&mut self, h: &mut impl Host) {
        self.set_dirty(1, h);
        self.draw(h);
    }
    pub fn mouse_up(&mut self, h: &mut impl Host) -> bool {
        if self.drag {
            self.drag = false;
            if self.capturing {
                self.capturing = false;
                h.effect(Effect::ReleaseCapture);
            }
        }
        true
    }
    pub fn cycle_condition(&mut self, h: &mut impl Host) {
        self.condition = self.condition.wrapping_add(1);
        if self.condition == 7 {
            self.condition = 0;
        }
        h.effect(Effect::Refresh);
    }
    pub fn placement_mode(&mut self, mode: u32, h: &mut impl Host) -> Result<()> {
        let cursor = match mode {
            4 => 4,
            8 => 6,
            _ => return Err(Unsupported::Dependency("LE4 placement selector")),
        };
        if self.mode != mode {
            self.mode = mode;
            h.effect(Effect::Cursor(cursor));
        }
        Ok(())
    }
    pub fn load_cursors(&mut self, h: &mut impl Host) -> Result<()> {
        let instances = [
            0x4A5CDC3F, 0x4A5CDC3A, 0x4A5CDC3B, 0x4A5CDC3C, 0x4A5CDC40, 0x4A5CDC3D, 0x4A5CDC3E,
        ];
        let mut temporary = [None; 7];
        for (i, instance) in instances.into_iter().enumerate() {
            temporary[i] = match h.create_cursor(0x46A006B0, instance) {
                Ok(p) => p,
                Err(error) => {
                    for p in temporary.into_iter().rev().flatten() {
                        h.destroy_cursor_temporary(p);
                    }
                    return Err(error);
                }
            };
        }
        for i in [0, 1, 2, 3, 6, 5, 4] {
            if self.cursors[i] != temporary[i] {
                if let Some(p) = temporary[i] {
                    h.effect(Effect::AddRef(p));
                }
                let old = std::mem::replace(&mut self.cursors[i], temporary[i]);
                if let Some(p) = old {
                    h.effect(Effect::Release(p));
                }
            }
        }
        for p in temporary.into_iter().rev().flatten() {
            h.destroy_cursor_temporary(p);
        }
        Ok(())
    }
}

pub fn drawing_style(kind: u8) -> ([u8; 3], f32) {
    (
        match kind {
            0 => [0, 0, 128],
            1 => [128, 0, 128],
            2 => [0, 128, 128],
            3 => [0, 72, 72],
            4 => [0, 128, 0],
            _ => [128; 3],
        },
        0.5,
    )
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_tests;
