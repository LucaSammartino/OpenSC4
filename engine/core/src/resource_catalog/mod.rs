// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable effects from docs/specs/resource-catalog.md §§2–6.
//!
//! No native pointer/COM ABI is provided. `RawKey` borrows must remain valid for
//! the entire catalog lifetime (RC1), including after enumeration completes.
//! Hosts supply only raw enumeration, names, row permutations and widget reads.
//! Invalid safe-Rust indices/permutations are checked extensions, not native
//! false return values. RC1–RC5 remain open; no Drop performs foreign cleanup.

pub mod key_input;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Key {
    pub t: u32,
    pub g: u32,
    pub i: u32,
}

impl Key {
    pub const fn new(t: u32, g: u32, i: u32) -> Self {
        Self { t, g, i }
    }
    pub const fn gti(self) -> [u32; 3] {
        [self.g, self.t, self.i]
    }
}

/// Identity is opaque; the backing object is borrowed, never AddRef'd/Released.
#[derive(Debug)]
pub struct RawKey {
    pub identity: u64,
    pub words: Key,
}

pub type Handle = u64;

/// Each nonnull preview represents one descriptor-held reference, even aliases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub t: u32,
    pub g: u32,
    pub name: String,
    pub preview: Option<Handle>,
}

#[derive(Clone, Debug)]
pub struct Row<'a> {
    pub data: &'a RawKey,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// RC3: no invented descriptor/map default or registration repair.
    RegistrationOrEmptyAdmission,
    /// RC4: resource creation is an unknown owned algorithm, not a host callback.
    AddResource,
    UnknownMethod,
    /// Safe Rust extension; does not describe native invalid-storage behavior.
    CheckedIndex,
    CheckedPermutation,
    InvalidEnumerationEntry,
    /// Uninitialized controls have no native false/default contract.
    CheckedControls,
}

/// Intent trace only. Handles do not promise native interface ownership or ABI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    ClearRows,
    /// Column-0 text followed by raw row-data storage; widget returns ignored.
    Append {
        row: usize,
        text: String,
        identity: u64,
    },
    Order([u32; 4]),
    SelectionRange {
        start: usize,
        count: usize,
        flags: u32,
    },
    Select(usize),
    PreviewClear(Handle),
    Hide(Handle),
    Show(Handle),
    PreviewKey(Handle, Key),
    FocusList(Handle),
    PreviewReadOnly(Handle, u8),
    AttachPreview(Handle),
    AppendDescriptor(String, i32),
    SelectDescriptor(usize),
    ReleaseSelector,
    ModalClose(u32),
    RemoveFromParent,
    AssignInputVariant,
    ReadVariantData,
    WriteVariant([u32; 3]),
    AssignOutputVariant,
    WriteQiPointer(Handle),
    AddRef(Handle),
    HideOwner,
    Window158Zero,
    Release(Handle),
    DestroyDescriptorName,
    ClearCurrentPreview,
    BaseShutdown,
    InstallCatalogTables,
    DestroyVectorMap,
    DestroyBaseWindow,
    InstallPickerTables,
    DestroyVariant,
    HeapFree,
    ConfigureBase,
    CreateBaseChild,
    ConfigureAddChild(Handle),
}

pub trait Effects {
    fn emit(&mut self, effect: Effect);
}

/// Count is queried before the first index and again after *each* iteration.
/// `entry` must point into stable backing storage with lifetime 'a (RC1).
pub trait RowPrimitives<'a>: Effects {
    fn enumerate(&mut self, resource_type: u32) -> bool;
    fn enumeration_count(&mut self) -> usize;
    fn entry(&mut self, index: usize) -> Option<&'a RawKey>;
    fn name(&mut self, key: &'a RawKey) -> Option<String>;
    /// Foreign slot 20C: return original row indices in displayed order (RC2).
    /// No stable, numeric or locale ordering is assumed by the owner.
    fn post_order(&mut self, rows: &[Row<'a>]) -> Vec<usize>;
    /// Coordinate-call success is intentionally not consumed (§3).
    fn selected_coordinates(&mut self) -> (i32, i32);
    fn row_data(&mut self, coordinates: (i32, i32)) -> Option<&'a RawKey>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rebuild {
    Retained,
    EnumerationFailedAfterClear,
    Ordered(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorReply {
    MissingChild,
    QiFailed,
    VisibleIndex(usize),
}

#[derive(Debug)]
pub struct Catalog<'a> {
    pub descriptors: Vec<Descriptor>,
    pub rows: Vec<Row<'a>>,
    pub visible_to_original: Vec<usize>,
    pub descriptor_index: u32,
    pub current_preview: Option<Handle>,
    pub list: Option<Handle>,
    pub type_filter: u32,
    pub group_filter: u32,
    pub read_only: u8,
    pub byte_dd: u8,
}

impl<'a> Catalog<'a> {
    /// Supplied descriptors must already be registered, valid and nonempty.
    /// No Ini parsing, fallback descriptor or creation/registration is inferred.
    pub fn new(descriptors: Vec<Descriptor>, list: Option<Handle>) -> Result<Self, Boundary> {
        if descriptors.is_empty() {
            return Err(Boundary::RegistrationOrEmptyAdmission);
        }
        Ok(Self {
            descriptors,
            rows: Vec::new(),
            visible_to_original: Vec::new(),
            descriptor_index: u32::MAX,
            current_preview: None,
            list,
            type_filter: 0,
            group_filter: 0,
            read_only: 0,
            byte_dd: 1,
        })
    }

    // Literal setters: no rebuild or speculative later use of these words.
    pub fn set_type_filter(&mut self, word: u32) {
        self.type_filter = word;
    }
    pub fn set_group_filter(&mut self, word: u32) {
        self.group_filter = word;
    }
    pub fn set_read_only(&mut self, byte: u8) {
        self.read_only = byte;
    }
    pub fn set_byte_dd(&mut self, byte: u8) {
        self.byte_dd = byte;
    }

    /// Known Init descriptor loop only; not the whole native Init body.
    pub fn init_descriptor_map(&mut self, host: &mut impl Effects) -> Result<(), Boundary> {
        let admitted: Vec<_> = self
            .descriptors
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                (self.type_filter == 0 || self.type_filter == d.t)
                    && (self.group_filter == 0 || self.group_filter == d.g)
            })
            .map(|(i, _)| i)
            .collect();
        if admitted.is_empty() {
            return Err(Boundary::RegistrationOrEmptyAdmission);
        }
        // Known Init mapping assumes valid registered preview interfaces (RC3).
        if admitted
            .iter()
            .any(|&i| self.descriptors[i].preview.is_none())
        {
            return Err(Boundary::RegistrationOrEmptyAdmission);
        }
        for &i in &admitted {
            let d = &self.descriptors[i];
            if let Some(p) = d.preview {
                host.emit(Effect::PreviewReadOnly(p, self.read_only));
                host.emit(Effect::AttachPreview(p));
                host.emit(Effect::Hide(p));
            }
            host.emit(Effect::AppendDescriptor(d.name.clone(), -2));
        }
        self.descriptor_index = admitted[0] as u32;
        self.visible_to_original = admitted;
        host.emit(Effect::SelectDescriptor(0));
        Ok(())
    }

    pub fn update_key_list(
        &mut self,
        requested: Key,
        host: &mut impl RowPrimitives<'a>,
    ) -> Result<Rebuild, Boundary> {
        let Some(d) = self.descriptors.get(self.descriptor_index as usize) else {
            return Ok(Rebuild::Retained);
        };
        if self.list.is_none() {
            return Ok(Rebuild::Retained);
        }
        let (t, g) = (d.t, d.g);
        host.emit(Effect::ClearRows);
        self.rows.clear();
        if !host.enumerate(t) {
            return Ok(Rebuild::EnumerationFailedAfterClear);
        }
        let mut i = 0;
        while i < host.enumeration_count() {
            let key = host.entry(i).ok_or(Boundary::InvalidEnumerationEntry)?;
            // Literal exact group; no returned-type check and no deduplication.
            if key.words.g == g {
                let text = host
                    .name(key)
                    .unwrap_or_else(|| format!("0x{:08X}", key.words.i));
                host.emit(Effect::Append {
                    row: self.rows.len(),
                    text: text.clone(),
                    identity: key.identity,
                });
                self.rows.push(Row { data: key, text });
            }
            i += 1;
        }
        host.emit(Effect::Order([0, 0, 1, 0]));
        let permutation = host.post_order(&self.rows);
        // Checked extension: never call a malformed permutation native failure.
        let mut seen = vec![false; self.rows.len()];
        if permutation.len() != seen.len() {
            return Err(Boundary::CheckedPermutation);
        }
        for &j in &permutation {
            let slot = seen.get_mut(j).ok_or(Boundary::CheckedPermutation)?;
            if *slot {
                return Err(Boundary::CheckedPermutation);
            }
            *slot = true;
        }
        self.rows = permutation.iter().map(|&j| self.rows[j].clone()).collect();
        if !self.rows.is_empty() {
            host.emit(Effect::SelectionRange {
                start: 0,
                count: self.rows.len(),
                flags: 15,
            });
            let mut chosen = self.rows[0].data.words;
            for (row, data) in self.rows.iter().enumerate() {
                if row == 0 || data.data.words == requested {
                    host.emit(Effect::Select(row));
                    chosen = data.data.words;
                }
            }
            self.set_view_key(chosen, host)?;
        }
        Ok(Rebuild::Ordered(self.rows.len()))
    }

    pub fn get_selected_key(output: &mut Key, host: &mut impl RowPrimitives<'a>) -> bool {
        let coordinates = host.selected_coordinates();
        if let Some(raw) = host.row_data(coordinates) {
            *output = raw.words;
            true
        } else {
            false
        }
    }

    pub fn refresh(
        &mut self,
        host: &mut impl RowPrimitives<'a>,
    ) -> Result<Option<Rebuild>, Boundary> {
        let mut key = Key::default();
        if Self::get_selected_key(&mut key, host) {
            self.update_key_list(key, host).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn set_view_key(&mut self, key: Key, host: &mut impl Effects) -> Result<(), Boundary> {
        let list = self.list.ok_or(Boundary::CheckedControls)?;
        let next = self
            .descriptors
            .get(self.descriptor_index as usize)
            .ok_or(Boundary::CheckedIndex)?
            .preview;
        if next != self.current_preview {
            if let Some(old) = self.current_preview {
                host.emit(Effect::PreviewClear(old));
                host.emit(Effect::Hide(old));
            }
            self.current_preview = next;
            if let Some(new) = next {
                host.emit(Effect::Show(new));
            }
        }
        if let Some(p) = self.current_preview {
            host.emit(Effect::PreviewKey(p, key));
        }
        host.emit(Effect::FocusList(list));
        Ok(())
    }

    /// Primitive selector QI reply, not a host-supplied final descriptor choice.
    pub fn selector_command(
        &mut self,
        command: u32,
        control: u32,
        reply: SelectorReply,
        host: &mut impl RowPrimitives<'a>,
    ) -> Result<bool, Boundary> {
        if command != 0xE8711255 || control != 0x04C9303E {
            return Ok(false);
        }
        match reply {
            SelectorReply::MissingChild => return Ok(true),
            SelectorReply::QiFailed => {}
            SelectorReply::VisibleIndex(i) => {
                let mapped = self.visible_to_original.get(i).copied();
                // QI success always releases, even if safe bounds refuse lookup.
                host.emit(Effect::ReleaseSelector);
                self.descriptor_index = mapped.ok_or(Boundary::CheckedIndex)? as u32;
            }
        }
        self.update_key_list(Key::default(), host)?;
        if let Some(list) = self.list {
            host.emit(Effect::FocusList(list));
        }
        Ok(false)
    }

    pub fn kill_self(&self, modal_flag_1000: bool, result: u32, host: &mut impl Effects) {
        host.emit(if modal_flag_1000 {
            Effect::ModalClose(result)
        } else {
            Effect::RemoveFromParent
        });
    }

    /// The add-resource AL is published, but execution remains an explicit RC4 boundary.
    pub fn button(&self, id: u32, modal: bool, host: &mut impl Effects) -> Result<bool, Boundary> {
        match id {
            0x24C93018 => {
                if let Some(p) = self.current_preview {
                    host.emit(Effect::PreviewClear(p));
                }
                self.kill_self(modal, 1, host);
                Ok(false)
            }
            0x24C93019 => Err(Boundary::AddResource),
            0x24C9301A => {
                self.kill_self(modal, 0, host);
                Ok(false)
            }
            0x24C9301B => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn shutdown(&mut self, host: &mut impl Effects) {
        host.emit(Effect::HideOwner);
        host.emit(Effect::Window158Zero);
        self.destroy_descriptors(host);
        self.current_preview = None;
        host.emit(Effect::ClearCurrentPreview);
        if let Some(list) = self.list.take() {
            host.emit(Effect::Release(list));
        }
        host.emit(Effect::BaseShutdown);
    }

    fn destroy_descriptors(&mut self, host: &mut impl Effects) {
        for d in self.descriptors.drain(..) {
            if let Some(p) = d.preview {
                host.emit(Effect::Release(p));
            }
            host.emit(Effect::DestroyDescriptorName);
        }
    }

    /// Explicit scoped effects; does not silently call Shutdown or act as Drop.
    pub fn destroy(&mut self, delete_flag: u32, host: &mut impl Effects) {
        host.emit(Effect::InstallCatalogTables);
        self.destroy_descriptors(host);
        host.emit(Effect::DestroyVectorMap);
        host.emit(Effect::DestroyBaseWindow);
        if delete_flag & 1 != 0 {
            host.emit(Effect::HeapFree);
        }
    }
}

/// Valid variant storage is a caller precondition (RC4), represented by an array.
/// This owns only the raw permutation, never variant validation/conversion.
#[derive(Debug)]
pub struct Picker {
    pub raw: [u32; 3],
}

impl Picker {
    pub fn input<'a>(&mut self, raw: [u32; 3], catalog: &mut Catalog<'a>, host: &mut impl Effects) {
        host.emit(Effect::AssignInputVariant);
        self.raw = raw;
        catalog.set_read_only(0);
        catalog.set_byte_dd(0);
        host.emit(Effect::ReadVariantData);
        catalog.set_type_filter(self.raw[1]);
        catalog.set_group_filter(self.raw[0]);
    }

    pub fn output<'a>(&mut self, host: &mut impl RowPrimitives<'a>) -> [u32; 3] {
        let mut selected = Key::default();
        let _ = Catalog::get_selected_key(&mut selected, host);
        self.raw = selected.gti();
        host.emit(Effect::WriteVariant(self.raw));
        host.emit(Effect::AssignOutputVariant);
        self.raw
    }

    /// Consumer rule only; output() must still run on cancel.
    pub fn accepted_key(raw: [u32; 3], modal_result: u32) -> Option<Key> {
        (modal_result == 0).then(|| Key::new(raw[1], raw[0], raw[2]))
    }

    pub fn destroy<'a>(
        &mut self,
        catalog: &mut Catalog<'a>,
        delete_flag: u32,
        host: &mut impl Effects,
    ) {
        host.emit(Effect::InstallPickerTables);
        host.emit(Effect::DestroyVariant);
        catalog.destroy(0, host);
        if delete_flag & 1 != 0 {
            host.emit(Effect::HeapFree);
        }
    }
}

/// QI recognized-branch intent only; native adjusting thunks/foreign QI withheld.
pub fn catalog_qi(iid: u32, pointer: Handle, host: &mut impl Effects) -> Result<bool, Boundary> {
    if iid != 0x7A368EE3 {
        return Err(Boundary::UnknownMethod);
    }
    host.emit(Effect::WriteQiPointer(pointer));
    host.emit(Effect::AddRef(pointer));
    Ok(true)
}

pub fn picker_qi(iid: u32, pointer: Handle, host: &mut impl Effects) -> Result<bool, Boundary> {
    if iid != 0x06C5FF7B {
        return Err(Boundary::UnknownMethod);
    }
    host.emit(Effect::AddRef(pointer));
    host.emit(Effect::WriteQiPointer(pointer));
    Ok(true)
}

pub fn picker_word_output(output: &mut u32) {
    *output = u32::MAX;
}

/// Base Init's known control-flow only; no widget geometry or construction ABI.
pub fn base_init(
    initialized: bool,
    created_child: Option<Handle>,
    host: &mut impl Effects,
) -> bool {
    if initialized {
        return true;
    }
    host.emit(Effect::ConfigureBase);
    host.emit(Effect::CreateBaseChild);
    if let Some(child) = created_child {
        host.emit(Effect::ConfigureAddChild(child));
        host.emit(Effect::Release(child));
    }
    true
}

#[cfg(test)]
mod tests;
