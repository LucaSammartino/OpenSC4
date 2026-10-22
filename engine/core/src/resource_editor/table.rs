// SPDX-License-Identifier: GPL-3.0-or-later
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

use super::{standard_flags, EditorError, TableDefaults, BUTTON_MESSAGE};

pub const TABLE_SAVE: u32 = 0xDA0A_DBAA;
pub const TABLE_ADD: u32 = 0xDA0A_DBA8;
pub const TABLE_EDIT: u32 = 0xDA0A_DBAD;
pub const TABLE_DELETE: u32 = 0xDA0A_DBA9;

pub fn key_words(key: ResKey) -> [u32; 3] {
    [key.group, key.type_id, key.instance]
}
pub fn words_key(words: [u32; 3]) -> ResKey {
    ResKey {
        type_id: words[1],
        group: words[0],
        instance: words[2],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPickerRequest {
    pub initial: [u32; 3],
    pub width: i32,
    pub height: i32,
    pub centered_on_main_window: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPickerResult {
    /// Zero means accept; every other native result cancels.
    pub modal_result: i32,
    pub words: [u32; 3],
}

/// Only primitive services: the model owns selection, swapping, dirty state,
/// row construction and saved-value persistence. No catalog methods or disk
/// serialization are invented (Save Table only changes the held Variant).
pub trait ResourceTableHost {
    fn pick_key(&mut self, request: KeyPickerRequest) -> KeyPickerResult;
    fn property_name(&mut self, property: u32, value: &Variant) -> String;
    fn property_value_text(&mut self, property: u32, value: &Variant) -> String;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceTableRow {
    pub debug_name: String,
    pub resource_key: String,
}

pub struct ResourceKeyTableEditor {
    pub property: u32,
    pub keys: Vec<ResKey>,
    pub held: Variant,
    pub defaults: TableDefaults,
    pub dirty: bool,
    pub save_enabled: bool,
    pub read_only: bool,
    pub rows: Vec<ResourceTableRow>,
    pub flags: u32,
    pub background_fill: bool,
    pub buttons: Vec<u32>,
    initialized: bool,
}

impl ResourceKeyTableEditor {
    /// Defaults must be supplied explicitly; constructor defaults were not
    /// published. `property` is the caller's choice of row-name ID (RE8).
    pub fn new(property: u32, defaults: TableDefaults) -> Self {
        Self {
            property,
            keys: Vec::new(),
            held: Variant::Empty,
            defaults,
            dirty: false,
            save_enabled: false,
            read_only: false,
            rows: Vec::new(),
            flags: 0,
            background_fill: false,
            buttons: Vec::new(),
            initialized: false,
        }
    }

    pub fn set_value(&mut self, value: &Variant) -> Result<(), EditorError> {
        let keys = load_keys(value)?;
        self.held = value.clone();
        self.keys = keys;
        Ok(())
    }

    pub fn init(&mut self, host: &mut impl ResourceTableHost) {
        if self.initialized {
            return;
        }
        self.flags = standard_flags(self.flags);
        self.background_fill = true;
        if !self.read_only {
            self.buttons = vec![TABLE_SAVE, TABLE_ADD, TABLE_EDIT, TABLE_DELETE];
        }
        self.refresh(host);
        self.set_dirty(false);
        self.initialized = true;
    }

    pub fn refresh(&mut self, host: &mut impl ResourceTableHost) {
        self.rows = self
            .keys
            .iter()
            .map(|&key| {
                let value = key_variant(key);
                ResourceTableRow {
                    debug_name: format!("{} (L)", host.property_name(self.property, &value)),
                    resource_key: host.property_value_text(self.property, &value),
                }
            })
            .collect();
    }

    pub fn add_key(&mut self, host: &mut impl ResourceTableHost) {
        if let Some(key) = pick_key(self.defaults.key, host) {
            self.keys.push(key);
            self.set_dirty(true);
            self.refresh(host);
        }
    }

    pub fn edit_key(&mut self, selection: Option<usize>, host: &mut impl ResourceTableHost) {
        let Some(index) = selection.filter(|&r| r < self.keys.len()) else {
            return;
        };
        if let Some(key) = pick_key(self.keys[index], host) {
            if key != self.keys[index] {
                self.keys[index] = key;
                self.set_dirty(true);
                self.refresh(host);
            }
        }
    }

    pub fn delete_key(&mut self, selection: Option<usize>, host: &mut impl ResourceTableHost) {
        if let Some(index) = selection {
            if index < self.keys.len() {
                self.keys.remove(index);
            }
            // §12: even an out-of-range nonnegative selection marks dirty.
            self.set_dirty(true);
            self.refresh(host);
        }
    }

    pub fn save_table(&mut self, host: &mut impl ResourceTableHost) {
        self.held = self.table_value();
        self.set_dirty(false);
        self.refresh(host);
    }

    pub fn revert(&mut self) -> Result<(), EditorError> {
        self.keys = load_keys(&self.held)?;
        self.set_dirty(false);
        // The published Revert doesn't call Refresh; leave rows alone (RE8).
        Ok(())
    }

    pub fn read(&mut self) -> Variant {
        self.held = self.table_value();
        // GetValue updates the saved value even without Save Table; dirty state
        // is not cleared by that method in §12.
        self.held.clone()
    }

    pub fn command(
        &mut self,
        message: u32,
        button: u32,
        selection: Option<usize>,
        host: &mut impl ResourceTableHost,
    ) -> bool {
        if message != BUTTON_MESSAGE {
            return false;
        }
        // Read-only controls button creation, not an unpublished handler guard.
        match button {
            TABLE_SAVE => self.save_table(host),
            TABLE_ADD => self.add_key(host),
            TABLE_EDIT => self.edit_key(selection, host),
            TABLE_DELETE => self.delete_key(selection, host),
            _ => return false,
        }
        true
    }

    fn table_value(&self) -> Variant {
        Variant::array(
            mem::UINT32,
            self.keys
                .iter()
                .flat_map(|&key| key_words(key).map(Elem::U32))
                .collect(),
        )
    }

    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
        self.save_enabled = dirty;
    }
}

fn load_keys(value: &Variant) -> Result<Vec<ResKey>, EditorError> {
    let Variant::Array {
        elem: mem::UINT32,
        items,
    } = value
    else {
        return Err(EditorError::UnsupportedType(value.mem_type()));
    };
    if items.len() % 3 != 0 {
        return Err(EditorError::OutOfBoundsNativeRead);
    }
    items
        .chunks_exact(3)
        .map(|words| {
            let [Elem::U32(group), Elem::U32(type_id), Elem::U32(instance)] = words else {
                return Err(EditorError::InvalidValue);
            };
            Ok(ResKey {
                type_id: *type_id,
                group: *group,
                instance: *instance,
            })
        })
        .collect()
}

fn key_variant(key: ResKey) -> Variant {
    Variant::array(mem::UINT32, key_words(key).map(Elem::U32).into())
}

fn pick_key(initial: ResKey, host: &mut impl ResourceTableHost) -> Option<ResKey> {
    let result = host.pick_key(KeyPickerRequest {
        initial: key_words(initial),
        width: 575,
        height: 475,
        centered_on_main_window: true,
    });
    (result.modal_result == 0).then(|| words_key(result.words))
}
