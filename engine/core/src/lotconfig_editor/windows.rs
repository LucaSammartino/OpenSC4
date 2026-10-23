// SPDX-License-Identifier: GPL-3.0-or-later
//! Window/service boundaries from §§19–22. Handles denote acquired references,
//! never borrowed controls. No COM layout or generic window implementation.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub const POST_INIT: u32 = 0x0920B687;
pub const WINDOW_COMMAND: u32 = 0x287259F6;
pub const CHOOSER_GRID: u32 = 0x091C91BA;
pub const CONFIRM_DELETE: u32 = 0x5301814A;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowEffect {
    PrimaryFlag(u32, bool),
    AuxiliaryFlag(u32, bool),
    BaseInit,
    Post(u32, [u32; 3]),
    GridSetup(u64, u32, u32, u32),
    ClearGrid(u64),
    Row(u64, usize, u32, String),
    Sort(u64, u32, bool, u32, u32),
    RemoveRow(u64, u32, u32, u32),
    Error(&'static str, &'static str),
    /// The spec establishes this native message, including "non-existant",
    /// but does not publish its full text (LE8). No text is invented here.
    FileUnavailableError,
    CopyController(u64, Option<u64>),
    AttachChooser(u64),
    WindowManager94(u64),
    RemoveCreated(u64),
    ShutdownEditor(u64),
    ControllerClose(u64, bool),
    BaseShutdown,
    FreeStorage(u32),
    BaseDestroy,
    FreeObject,
    NotifyClosed(u64),
    NetworkStart(u64),
    NetworkStop(u64),
    NormalCursor,
    StaticSection(&'static str),
}

/// Foreign window, manager and filesystem primitives. Editor Open/Close are
/// deliberately NOT host decisions: callers execute those core methods between
/// begin/finish operations, propagating their explicit LE4/LE5 barriers.
pub trait WindowHost: Host {
    fn window_effect(&mut self, effect: WindowEffect);
    fn base_message(&mut self, category: u32, command: u32, control: u32) -> bool;
    fn acquire_grid(&mut self, child: u32, iid: u32) -> Option<u64>;
    /// Borrowed global manager. The owner explicitly AddRefs its stored copy.
    fn acquire_manager(&mut self) -> Option<u64>;
    fn enumerate_types(&mut self, manager: u64, option: bool) -> Vec<(u32, String)>;
    fn current_cell(&mut self, grid: u64) -> Option<(u32, u32)>;
    fn cell_text(&mut self, grid: u64, row: u32, column: u32) -> String;
    fn create_editor(&mut self, key: [u32; 3], id: u32) -> Option<u64>;
    fn confirm(&mut self, title: &'static str, text: &'static str) -> u32;
    fn file_writable_and_exists(&mut self, path: &str) -> bool;
    /// Deletes the identified database item, never the entire backing file.
    fn delete_item(&mut self, identity: &Identity) -> Result<bool>;
    fn unregister_type_if_present(&mut self, selected_type: u32);
    fn search_grid(&mut self, grid: u64, search: &GridSearch) -> Option<(u32, u32)>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridSearch {
    pub text: String,
    pub length: usize,
    pub raw: [u32; 4],
    pub final_option: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChooserAction {
    Edit(bool),
    Delete,
    Close,
    PostInit,
    Handled,
    Base(bool),
}

/// A created editor's temporary reference. Finish exactly once after invoking
/// Editor::open. The stored type is the editor's identity.id AFTER Open.
#[derive(Debug)]
pub struct PendingEditor {
    pub pointer: u64,
    pub key: [u32; 3],
}

#[derive(Debug, Default)]
pub struct Chooser {
    pub initialized: bool,
    pub controller: Option<u64>,
    pub grid: Option<u64>,
    pub manager: Option<u64>,
    /// Service discovery/acquisition is explicitly LE7; only already acquired
    /// references may be transferred into this set by an adapter.
    pub services: BTreeSet<u64>,
    pub editors: BTreeMap<u64, u32>,
    pub sort_column: u32,
    pub sort_option: bool,
}

fn init_flags(h: &mut impl WindowHost) {
    h.window_effect(WindowEffect::PrimaryFlag(0x10000, true));
    for (mask, value) in [(1, true), (0x10, true), (0x100, false), (4, false)] {
        h.window_effect(WindowEffect::AuxiliaryFlag(mask, value));
    }
    h.window_effect(WindowEffect::BaseInit);
}
fn release(slot: &mut Option<u64>, h: &mut impl Host) {
    if let Some(p) = slot.take() {
        h.effect(Effect::Release(p));
    }
}
fn replace_reference(slot: &mut Option<u64>, new: Option<u64>, h: &mut impl Host) {
    // Replacement calls in Init use AddRef-before-Release, even on equality.
    if let Some(p) = new {
        h.effect(Effect::AddRef(p));
    }
    release(slot, h);
    *slot = new;
}

impl Chooser {
    pub fn init(&mut self, h: &mut impl WindowHost) -> bool {
        if !self.initialized {
            init_flags(h);
            self.initialized = true;
            h.window_effect(WindowEffect::Post(POST_INIT, [0; 3]));
        }
        true
    }
    /// Executes all published PostInit operations. Service discovery is left
    /// Unsupported at its explicit LE7 boundary, never guessed from an IID.
    pub fn post_init(&mut self, h: &mut impl WindowHost) -> Result<()> {
        if self.grid.is_none() {
            self.grid = h.acquire_grid(CHOOSER_GRID, 0xDAA6B9BE);
        }
        let grid = self.grid.ok_or(Unsupported::Dependency("LE7 required chooser grid"))?;
        h.window_effect(WindowEffect::GridSetup(grid, 2, 1, 0));
        if self.manager.is_none() {
            let manager = h.acquire_manager();
            replace_reference(&mut self.manager, manager, h);
        }
        self.rebuild(h)?;
        Err(Unsupported::Dependency("LE7 service discovery IID 0x856C8CF7"))
    }
    pub fn message(
        &mut self,
        category: u32,
        command: u32,
        control: u32,
        control_down: bool,
        shift_down: bool,
        h: &mut impl WindowHost,
    ) -> ChooserAction {
        if category == 3 {
            if command == WINDOW_COMMAND {
                match control {
                    0x091C91BB => return ChooserAction::Edit(true),
                    0x091C91BC => return ChooserAction::Edit(false),
                    0x091C91BD => return ChooserAction::Delete,
                    0x42B7C353 => return ChooserAction::Close,
                    0x0942D1CB => self.sort(0, control_down || shift_down, h),
                    0x0942D1CC => self.sort(1, control_down || shift_down, h),
                    _ => (),
                }
            }
            ChooserAction::Handled
        } else if category == POST_INIT {
            ChooserAction::PostInit
        } else {
            ChooserAction::Base(h.base_message(category, command, control))
        }
    }
    pub fn sort(&mut self, column: u32, option: bool, h: &mut impl WindowHost) {
        if let Some(grid) = self.grid {
            self.sort_column = column;
            self.sort_option = option;
            h.window_effect(WindowEffect::Sort(grid, column, option, 1, 0));
        }
    }
    pub fn rebuild(&self, h: &mut impl WindowHost) -> Result<()> {
        let grid = self.grid.ok_or(Unsupported::Dependency("LE7 required chooser grid"))?;
        h.window_effect(WindowEffect::ClearGrid(grid));
        if let Some(manager) = self.manager {
            for (row, (id, name)) in h.enumerate_types(manager, false).into_iter().enumerate() {
                h.window_effect(WindowEffect::Row(grid, row, 0, format!("0x{id:08x}")));
                h.window_effect(WindowEffect::Row(grid, row, 1, name));
            }
            h.window_effect(WindowEffect::Sort(grid, self.sort_column, self.sort_option, 1, 0));
        }
        Ok(())
    }
    pub fn selected_type(&self, h: &mut impl WindowHost) -> Result<u32> {
        let grid = self.grid.ok_or(Unsupported::Dependency("LE7 required chooser grid"))?;
        let Some((row, _column)) = h.current_cell(grid) else {
            return Ok(0);
        };
        let text = h.cell_text(grid, row, 0);
        h.parse_hex(&text)
    }
    pub fn selected_key(&self, output: &mut [u32; 3], h: &mut impl WindowHost) -> Result<bool> {
        let ty = self.selected_type(h)?;
        if ty == 0 {
            return Ok(false);
        }
        let Some(id) = h.configuration_id_for_type(ty)? else {
            return Ok(false);
        };
        *output = [0x6534284A, 0xA8FBD372, id];
        Ok(true)
    }
    pub fn is_type_being_edited(&self, ty: u32) -> bool {
        self.editors.values().any(|&id| id == ty)
    }
    pub fn begin_edit(&self, new: bool, h: &mut impl WindowHost) -> Result<Option<PendingEditor>> {
        let ty = self.selected_type(h)?;
        if ty == 0 {
            return Ok(None);
        }
        if self.is_type_being_edited(ty) {
            h.window_effect(WindowEffect::Error(
                "No can do",
                "The lot you are trying to edit is already open for editing.",
            ));
            return Ok(None);
        }
        let mut key = [0; 3];
        if !new {
            let Some(id) = h.configuration_id_for_type(ty)? else {
                return Ok(None);
            };
            key = [0x6534284A, 0xA8FBD372, id];
        }
        let Some(pointer) = h.create_editor([0, 0x96A006B0, 0x09234182], 0xC9235387) else {
            return Ok(None);
        };
        h.window_effect(WindowEffect::AttachChooser(pointer));
        h.window_effect(WindowEffect::CopyController(pointer, self.controller));
        h.window_effect(WindowEffect::WindowManager94(pointer));
        Ok(Some(PendingEditor { pointer, key }))
    }
    pub fn finish_edit(&mut self, pending: PendingEditor, opened: bool, stored_type: u32, h: &mut impl WindowHost) {
        let p = pending.pointer;
        if opened {
            if let std::collections::btree_map::Entry::Vacant(entry) = self.editors.entry(p) {
                h.effect(Effect::AddRef(p));
                entry.insert(stored_type);
            }
        } else {
            h.window_effect(WindowEffect::Error("No can do", "Unable to open the lot for editing."));
            h.window_effect(WindowEffect::RemoveCreated(p));
            h.window_effect(WindowEffect::ShutdownEditor(p));
        }
        h.effect(Effect::Release(p)); // temporary creation reference
    }
    pub fn editor_closed(&mut self, pointer: u64, h: &mut impl Host) {
        if self.editors.remove(&pointer).is_some() {
            h.effect(Effect::Release(pointer));
        }
    }
    /// Dispatch must invoke core Editor::close with the supplied flags, applying
    /// NotifyChooser callbacks to this chooser before it returns. The set is
    /// re-read, so this is not snapshot iteration.
    pub fn user_close<H: WindowHost>(
        &mut self,
        h: &mut H,
        mut dispatch: impl FnMut(&mut Self, u64, bool, bool, &mut H) -> Result<bool>,
    ) -> Result<bool> {
        while let Some((&p, _)) = self.editors.first_key_value() {
            if !dispatch(self, p, true, true, h)? {
                return Ok(false);
            }
            if self.editors.contains_key(&p) {
                return Err(Unsupported::Dependency("successful editor close must notify chooser"));
            }
        }
        if let Some(p) = self.controller {
            h.window_effect(WindowEffect::ControllerClose(p, false));
        }
        Ok(true)
    }
    pub fn shutdown<H: WindowHost>(
        &mut self,
        h: &mut H,
        mut dispatch: impl FnMut(&mut Self, u64, bool, bool, &mut H) -> Result<bool>,
    ) -> Result<()> {
        while let Some((&p, _)) = self.editors.first_key_value() {
            dispatch(self, p, false, false, h)?;
            self.editor_closed(p, h);
        }
        for p in std::mem::take(&mut self.services) {
            h.effect(Effect::Release(p));
        }
        release(&mut self.controller, h);
        release(&mut self.grid, h);
        release(&mut self.manager, h);
        self.initialized = false;
        h.window_effect(WindowEffect::BaseShutdown);
        Ok(())
    }
    pub fn destroy(&mut self, flags: u32, h: &mut impl WindowHost) {
        self.services.clear();
        self.editors.clear();
        h.window_effect(WindowEffect::FreeStorage(0x134));
        h.window_effect(WindowEffect::FreeStorage(0x140));
        release(&mut self.manager, h);
        release(&mut self.grid, h);
        release(&mut self.controller, h);
        h.window_effect(WindowEffect::BaseDestroy);
        if flags & 1 != 0 {
            h.window_effect(WindowEffect::FreeObject);
        }
    }
    pub fn remove_row(&self, id: u32, h: &mut impl WindowHost) -> Result<bool> {
        let grid = self.grid.ok_or(Unsupported::Dependency("LE7 required chooser grid"))?;
        let text = format!("0x{id:08x}");
        let search = GridSearch {
            length: text.len(),
            text,
            raw: [0, 1, 0, 0x7FFFFFFF],
            final_option: 1,
        };
        if let Some((row, _)) = h.search_grid(grid, &search) {
            h.window_effect(WindowEffect::RemoveRow(grid, row, 1, 0));
        }
        Ok(false)
    }
    pub fn delete_selected(&self, h: &mut impl WindowHost) -> Result<()> {
        let ty = self.selected_type(h)?;
        if ty == 0 {
            return Ok(());
        }
        if self.is_type_being_edited(ty) {
            h.window_effect(WindowEffect::Error(
                "No can do",
                "The lot you are trying to delete is currently open for editing.",
            ));
            return Ok(());
        }
        let Some(identity) = identity_from_type(ty, h)? else {
            return Ok(());
        };
        // Resolve creates two retained references; release them on every exit,
        // including an explicit Unsupported from the backing-store dependency.
        let result = self.delete_identity(ty, &identity, h);
        for p in identity.references.into_iter().flatten() {
            h.effect(Effect::Release(p));
        }
        result
    }
    fn delete_identity(&self, ty: u32, identity: &Identity, h: &mut impl WindowHost) -> Result<()> {
        if h.confirm("Are you sure", "Are you sure you want to delete the selected item?") != CONFIRM_DELETE {
            return Ok(());
        }
        if !h.file_writable_and_exists(&identity.path) {
            h.window_effect(WindowEffect::FileUnavailableError);
            return Ok(());
        }
        if !h.delete_item(identity)? {
            h.window_effect(WindowEffect::Error("No can do", "Unknown error trying to delete item."));
            return Ok(());
        }
        self.remove_row(identity.id, h)?;
        h.unregister_type_if_present(ty);
        Ok(())
    }
}

/// Shared process resource guard, supplied to every editor Init. Setting the
/// flag precedes all file attempts; failure cannot cause an automatic retry.
#[derive(Debug, Default)]
pub struct EditorStaticResources {
    pub attempted: bool,
}
impl EditorStaticResources {
    pub fn begin(&mut self) -> bool {
        if self.attempted {
            return false;
        }
        self.attempted = true;
        true
    }
}

/// Reference/storage side of the editor window. Portable Configuration remains
/// owned by Editor; a native adapter can additionally track its acquired COM
/// reference at slot 0x190. Raw offsets are evidence labels, not memory layout.
#[derive(Debug, Default)]
pub struct EditorWindow {
    pub initialized: bool,
    pub references: BTreeMap<u32, u64>,
    pub storage: BTreeSet<u32>,
    pub networks: [Option<u64>; 2],
    pub owned_references: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMessage {
    Close(bool, bool),
    PostInit,
    Handled,
    Base(bool),
}

impl EditorWindow {
    /// Returns the callback subobject handle after the owner's explicit AddRef;
    /// None requires generic-window QueryInterface delegation by the adapter.
    pub fn query_interface(&self, iid: u32, owner: u64, callback: u64, h: &mut impl Host) -> Option<u64> {
        if iid != 0xC6AE7085 {
            return None;
        }
        h.effect(Effect::AddRef(owner));
        Some(callback)
    }
    /// Static path/name parsing remains LE5. `begin` describes exactly the once
    /// timing and sections; the adapter must report that residual separately.
    pub fn init(
        &mut self,
        editor: &mut Editor,
        resources: &mut EditorStaticResources,
        h: &mut impl EditorWindowHost,
    ) -> Result<bool> {
        let static_result = if resources.begin() {
            let result = h.load_family_file();
            for section in ["PropFamilies", "BuildingFamilies", "FloraFamilies"] {
                h.window_effect(WindowEffect::StaticSection(section));
            }
            result
        } else {
            Ok(())
        };
        if self.initialized {
            return Ok(true);
        }
        init_flags(h);
        self.initialized = true;
        if !self.references.contains_key(&0x134) {
            if let Some(font) = h.acquire_font("Arial", 12) {
                self.references.insert(0x134, font);
            }
        }
        let manager = h.acquire_manager();
        if let Some(p) = manager {
            h.effect(Effect::AddRef(p));
        }
        self.release_field(0x160, h);
        if let Some(p) = manager {
            self.references.insert(0x160, p);
        }
        h.window_effect(WindowEffect::Post(POST_INIT, [0; 3]));
        editor.load_cursors(h)?;
        h.window_effect(WindowEffect::NormalCursor);
        static_result?;
        Ok(true)
    }
    pub fn message(&self, category: u32, command: u32, control: u32, h: &mut impl WindowHost) -> EditorMessage {
        if category == 3 {
            if command == WINDOW_COMMAND && control == 0x42B7C353 {
                EditorMessage::Close(true, true)
            } else {
                EditorMessage::Handled
            }
        } else if category == POST_INIT {
            EditorMessage::PostInit
        } else {
            EditorMessage::Base(h.base_message(category, command, control))
        }
    }
    /// Published network subset of PostInit; the central view/alpha acquisition
    /// remains explicitly Unsupported by LE5, rather than a successful stub.
    pub fn post_init_networks(&mut self, h: &mut impl EditorWindowHost) -> Result<()> {
        for (slot, ty) in [3, 0].into_iter().enumerate() {
            if self.networks[slot].is_none() {
                if let Some(p) = h.network_object(ty, true) {
                    h.effect(Effect::AddRef(p));
                    self.networks[slot] = Some(p);
                    h.window_effect(WindowEffect::NetworkStart(p));
                }
            }
        }
        Err(Unsupported::Dependency("LE5 PostInit view/alpha setup"))
    }
    fn release_field(&mut self, offset: u32, h: &mut impl Host) {
        if let Some(p) = self.references.remove(&offset) {
            h.effect(Effect::Release(p));
        }
    }
    pub fn shutdown(&mut self, editor: &mut Editor, h: &mut impl WindowHost) {
        if self.initialized {
            if let Some(&p) = self.references.get(&0x15C) {
                h.window_effect(WindowEffect::NotifyClosed(p));
            }
            for offset in [0x134, 0x138, 0x13C, 0x15C, 0x160] {
                self.release_field(offset, h);
            }
            for p in std::mem::take(&mut editor.identity.references).into_iter().flatten() {
                h.effect(Effect::Release(p));
            }
            for slot in &mut self.networks {
                if let Some(p) = slot.take() {
                    h.window_effect(WindowEffect::NetworkStop(p));
                    h.effect(Effect::Release(p));
                }
            }
            for p in self.owned_references.drain(..) {
                h.effect(Effect::Release(p));
            }
            self.initialized = false;
        }
        h.window_effect(WindowEffect::BaseShutdown);
    }
    pub fn destroy(&mut self, editor: &mut Editor, flags: u32, h: &mut impl WindowHost) {
        for offset in [0x1D0, 0x1C4, 0x1A0] {
            if self.storage.remove(&offset) {
                h.window_effect(WindowEffect::FreeStorage(offset));
            }
        }
        // Deliberately no per-element Release: that is Shutdown's protocol.
        self.owned_references.clear();
        self.release_field(0x190, h);
        editor.current = None;
        if self.storage.remove(&0x178) {
            h.window_effect(WindowEffect::FreeStorage(0x178));
        }
        for offset in [0x160, 0x15C] {
            self.release_field(offset, h);
        }
        // Cursor references live in the existing Editor, never a second copy.
        for index in [4, 5, 6, 3, 2, 1, 0] {
            release(&mut editor.cursors[index], h);
        }
        for offset in [0x13C, 0x138, 0x134] {
            self.release_field(offset, h);
        }
        h.window_effect(WindowEffect::BaseDestroy);
        if flags & 1 != 0 {
            h.window_effect(WindowEffect::FreeObject);
        }
    }
}

pub trait EditorWindowHost: WindowHost {
    /// Missing/unreadable file is an ordinary attempted load, not an error that
    /// retries Init. Path composition/callback interpretation remain LE5.
    fn load_family_file(&mut self) -> Result<()>;
    fn acquire_font(&mut self, family: &str, size: u32) -> Option<u64>;
    fn network_object(&mut self, ty: u32, option: bool) -> Option<u64>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEvent {
    LeftDown,
    RightDown,
    LeftUp,
    Move,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseRoute {
    Edit(MouseEvent, [i32; 2]),
    OutsideMenu,
    Base(MouseEvent, [i32; 2]),
}
pub trait MouseHost {
    fn child_coordinates(&mut self, point: [i32; 2]) -> [i32; 2];
    fn window_coordinates(&mut self, point: [i32; 2]) -> [i32; 2];
    fn child_size(&mut self) -> [i32; 2];
    /// Dispatches the corresponding core edit handler / foreign generic window
    /// method. Unsettled edit handlers must propagate Unsupported, not guess.
    fn dispatch_mouse(&mut self, route: MouseRoute) -> Result<bool>;
}
pub fn mouse_wrapper(event: MouseEvent, point: [i32; 2], base_e0: bool, h: &mut impl MouseHost) -> Result<bool> {
    let bypass = matches!(event, MouseEvent::LeftUp | MouseEvent::Move) && base_e0;
    if bypass {
        return h.dispatch_mouse(MouseRoute::Base(event, point));
    }
    let p = h.child_coordinates(point);
    let size = h.child_size();
    let inside = p[0] >= 0 && p[1] >= 0 && p[0] < size[0] && p[1] < size[1];
    let route = if inside {
        MouseRoute::Edit(event, p)
    } else if event == MouseEvent::RightDown {
        MouseRoute::OutsideMenu
    } else {
        MouseRoute::Base(event, h.window_coordinates(p))
    };
    h.dispatch_mouse(route)?;
    Ok(true)
}

/// Explicitly unresolved raster phases. These tokens are NOT successful owner
/// implementations. A renderer that has no corresponding published algorithm
/// must return Unsupported(LE5); recording doubles can exercise the outer shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterPhase {
    GridOutline,
    Ordinary(RecordId),
    FoundationMarkers(RecordId),
    Fences,
}
pub trait EditorRenderer {
    fn acquire(&mut self, iid: u32) -> (bool, Option<u64>);
    fn color(&mut self, rgb: [u8; 3]) -> u32;
    fn lock(&mut self, buffer: u64, mode: u32) -> bool;
    fn fill(&mut self, buffer: u64, color: u32, option: u32);
    fn unlock(&mut self, buffer: u64, mode: u32);
    fn raster(&mut self, _buffer: u64, _phase: RasterPhase) -> Result<()> {
        Err(Unsupported::Dependency("LE5 raster geometry"))
    }
    fn release_buffer(&mut self, buffer: u64);
    fn invalidate(&mut self);
}
impl Editor {
    pub fn draw_current(&self, h: &mut impl EditorRenderer) -> Result<()> {
        let (acquired, output) = h.acquire(0x20732180);
        let buffer = output.ok_or(Unsupported::Dependency("unsafe native null buffer Release"))?;
        let result = if acquired {
            self.draw_acquired(buffer, h)
        } else {
            Ok(())
        };
        h.release_buffer(buffer);
        result
    }
    fn draw_acquired(&self, buffer: u64, h: &mut impl EditorRenderer) -> Result<()> {
        let color = h.color([192; 3]);
        if h.lock(buffer, 0x8001) {
            h.fill(buffer, color, 0);
            h.unlock(buffer, 0x8001);
        }
        if h.lock(buffer, 0x8004) {
            let result = h.raster(buffer, RasterPhase::GridOutline);
            h.unlock(buffer, 0x8004);
            result?;
        }
        if let Some(config) = &self.current {
            for record in config.objects.iter().rev() {
                if record.data.kind == 3 {
                    h.color([0, 72, 72]);
                    if h.lock(buffer, 0x8400) {
                        let result = h.raster(buffer, RasterPhase::FoundationMarkers(record.id));
                        h.unlock(buffer, 0x8400);
                        result?;
                    }
                } else {
                    h.raster(buffer, RasterPhase::Ordinary(record.id))?;
                }
            }
        }
        let result = if h.lock(buffer, 0x8004) {
            h.raster(buffer, RasterPhase::Fences)
        } else {
            Ok(())
        };
        h.unlock(buffer, 0x8004); // unconditional for fence phase
        result
    }
    pub fn trigger_redraw(&self, h: &mut impl EditorRenderer) -> Result<()> {
        self.draw_current(h)?;
        h.invalidate();
        Ok(())
    }
}

/// Numeric formatting belongs to the foreign CRT. This establishes the gate,
/// format string and odd first-character replacement without inventing NaN text.
pub fn height_label(height: f32, mut crt_format: impl FnMut(&str, f64) -> String) -> Option<String> {
    if height == 0.0 {
        return None;
    }
    let mut text = crt_format("+%.1f", f64::from(height));
    if height < 0.0 && !text.is_empty() {
        text.replace_range(..1, "-");
    }
    Some(text)
}
