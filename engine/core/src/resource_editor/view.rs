// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::BTreeMap;

use crate::property::PropertyHolder;
use crate::stream::{RefValue, ResKey};
use crate::variant::{mem, Elem, Variant, ARRAY};

use super::{standard_flags, Area, Ingredients, RESPONSE_CURVE_CLASS, RESOURCE_TABLE_CLASS};

pub const BUTTON_MESSAGE: u32 = 0x2872_59F6;
pub const SELECTION_MESSAGE: u32 = 0xE871_1255;
pub const PROPERTY_LIST: u32 = 0xFA12_B77F;
pub const PROPERTY_DONE: u32 = 0x24C9_3018;
pub const PROPERTY_CANCEL: u32 = 0x24C9_3019;
pub const EDIT_OKAY: u32 = 0x26C1_0A3B;
pub const EDIT_CANCEL: u32 = 0x26C1_0A3C;
pub const EDIT_ADD: u32 = 0x27C3_2AA4;
pub const EDIT_REMOVE: u32 = 0xC7C3_2AA9;

/// Opaque host handles represent editors, not owned native references. No
/// AddRef/Release or native lifecycle is inferred from this model (RE7).
pub trait EditorHost {
    type Editor: Copy;
    fn create_editor(&mut self, class: u32) -> Option<Self::Editor>;
    fn resolve_reference(&mut self, value: &RefValue) -> Option<RefValue>;
    fn table_parameter(&mut self, editor: Self::Editor, value: u32);
    fn table_default(&mut self, editor: Self::Editor, key: ResKey);
    fn initialize(&mut self, editor: Self::Editor, property: u32, value: &Variant);
    fn value(&mut self, editor: Self::Editor) -> Variant;
    fn show(&mut self, editor: Self::Editor, visible: bool);
    fn area(&mut self, editor: Self::Editor) -> Area;
    fn set_area(&mut self, editor: Self::Editor, area: Area);
    fn child_add(&mut self, editor: Self::Editor);
    fn child_remove(&mut self, editor: Self::Editor);
    /// `modal = true`: end modal loop (message 0x0F); otherwise close.
    fn end_dialog(&mut self, modal: bool, result: i32) -> bool;
}

/// A single game's property object, whose ID and value setters are distinct.
/// Source exemplars use the existing PropertyHolder/Variant types.
pub trait PropertyTarget {
    fn set_id(&mut self, id: u32);
    fn set_value(&mut self, value: Variant);
}

pub fn get_value_editor<H: EditorHost>(ingredients: &Ingredients, id: u32,
    supplied: Option<&Variant>, exemplar: Option<&Variant>, host: &mut H) -> Option<H::Editor> {
    let definition = ingredients.properties.get(&id)?;
    // RE5: §6's "otherwise" is ambiguous. Explicit dialog values must survive
    // Add/Remove (§9); absent supplied values use an exemplar object or default.
    let candidate = supplied.or(exemplar);
    let value = match candidate {
        Some(Variant::Object(reference)) => Variant::Object(host.resolve_reference(reference)?),
        Some(value) if supplied.is_some() => value.clone(),
        _ => definition.default.clone(),
    };
    let editor = host.create_editor(definition.editor_class)?;
    if matches!(definition.editor_class, RESOURCE_TABLE_CLASS | 0x281F_7625) {
        if let Some(default) = ingredients.table_defaults.get(&id) {
            host.table_parameter(editor, default.parameter);
            host.table_default(editor, default.key);
        }
    }
    host.initialize(editor, id, &value);
    Some(editor)
}

pub struct PropertyView<E: Copy> {
    pub filter: u32,
    pub rows: Vec<u32>,
    pub editors: BTreeMap<u32, E>,
    pub current: Option<E>,
    pub help: String,
    pub area: Area,
    pub flags: u32,
    initialized: bool,
}

impl<E: Copy> PropertyView<E> {
    pub fn new(filter: u32) -> Self {
        Self { filter, rows: Vec::new(), editors: BTreeMap::new(), current: None, help: "<help>".into(),
            area: Area::default(), flags: 0, initialized: false }
    }

    pub fn init(&mut self, ingredients: &Ingredients, exemplar: &PropertyHolder, parent: Area,
        host: &mut impl EditorHost<Editor = E>) {
        if self.initialized { return; }
        if self.area.right == self.area.left && self.area.bottom == self.area.top {
            self.area.left = (parent.right - parent.left - 400) / 2;
            self.area.top = (parent.bottom - parent.top - 450) / 2;
            self.area.right = self.area.left + 400;
            self.area.bottom = self.area.top + 450;
        }
        self.flags = standard_flags(self.flags);
        self.map_properties(ingredients, exemplar, host);
        self.initialized = true;
    }

    pub fn map_properties(&mut self, ingredients: &Ingredients, exemplar: &PropertyHolder,
        host: &mut impl EditorHost<Editor = E>) {
        self.editors.clear();
        // §6 publishes the editor map, but not the population of the row
        // vector (RE5). Rows are supplied explicitly by the list's caller.
        for (&id, definition) in &ingredients.properties {
            if definition.filter & self.filter != 0 {
                if let Some(editor) = get_value_editor(ingredients, id, None, exemplar.get(id), host) {
                    self.editors.insert(id, editor);
                }
            }
        }
    }

    fn select(&mut self, selected: usize, host: &mut impl EditorHost<Editor = E>) -> Option<u32> {
        let &id = self.rows.get(selected)?;
        if let Some(editor) = self.current { host.show(editor, false); }
        self.current = self.editors.get(&id).copied();
        if let Some(editor) = self.current { host.show(editor, true); }
        Some(id)
    }

    pub fn assign(&mut self, id: u32, target: &mut impl PropertyTarget, host: &mut impl EditorHost<Editor = E>) {
        if let Some(&editor) = self.editors.get(&id) {
            let value = host.value(editor);
            target.set_id(id);
            target.set_value(value);
        }
    }

    pub fn command(&mut self, message: u32, parameter: u32, selected: usize, ingredients: &Ingredients,
        target: &mut impl PropertyTarget, host: &mut impl EditorHost<Editor = E>) -> bool {
        if message == SELECTION_MESSAGE && parameter == PROPERTY_LIST {
            if let Some(id) = self.select(selected, host) { self.help = ingredients.help(id).into(); }
            return false;
        }
        if message != BUTTON_MESSAGE { return false; }
        match parameter {
            PROPERTY_DONE => {
                if let Some(id) = self.select(selected, host) { self.assign(id, target, host); }
                host.end_dialog(self.flags & 0x1000 != 0, 0)
            }
            PROPERTY_CANCEL => host.end_dialog(self.flags & 0x1000 != 0, 1),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogButton {
    pub label: &'static str,
    pub id: u32,
    pub style: u32,
    pub area: Area,
}

/// The toolkit decides button dimensions. The dialog owns all positioning.
pub trait ButtonMetrics {
    fn button_size(&mut self, label: &str, style: u32) -> (i32, i32);
}

pub struct EditValueDialog<E: Copy> {
    pub property: u32,
    pub editor: Option<E>,
    pub area: Area,
    pub flags: u32,
    pub buttons: Vec<DialogButton>,
    pub help: String,
    pub help_area: Area,
    initialized: bool,
}

impl<E: Copy> EditValueDialog<E> {
    pub fn new(property: u32, area: Area) -> Self {
        Self { property, editor: None, area, flags: 0, buttons: Vec::new(), help: String::new(),
            help_area: Area::default(), initialized: false }
    }

    pub fn init(&mut self, ingredients: &Ingredients, current: &Variant,
        metrics: &mut impl ButtonMetrics, host: &mut impl EditorHost<Editor = E>) {
        if self.initialized { return; }
        self.flags = standard_flags(self.flags);
        let width = self.area.right - self.area.left;
        let height = self.area.bottom - self.area.top;
        let mut right = width - 5;
        let mut top = height - 5;
        for (index, (label, id, style)) in [("Okay", EDIT_OKAY, 0x2001), ("Cancel", EDIT_CANCEL, 0x2007),
            ("Add Value", EDIT_ADD, 0x2010), ("Remove Value", EDIT_REMOVE, 0x2011)].into_iter().enumerate() {
            if index >= 2 && !ingredients.variable_length(self.property) { break; }
            let (w, h) = metrics.button_size(label, style);
            if index == 0 { top = height - 5 - h; }
            let area = Area { left: right - w, top, right, bottom: top + h };
            right = area.left - 5;
            self.buttons.push(DialogButton { label, id, style, area });
        }
        self.help = ingredients.help(self.property).into();
        self.help_area = Area { left: 5, top, right: right + 5, bottom: height - 5 };
        self.editor = get_value_editor(ingredients, self.property, Some(current), None, host);
        if let Some(editor) = self.editor {
            host.set_area(editor, Area { left: 5, top: 5, right: width - 5, bottom: top });
            host.child_add(editor);
        }
        self.initialized = true;
    }

    pub fn command(&mut self, message: u32, button: u32, ingredients: &Ingredients,
        target: &mut impl PropertyTarget, host: &mut impl EditorHost<Editor = E>) -> bool {
        if message != BUTTON_MESSAGE { return false; }
        match button {
            EDIT_OKAY => {
                let result = if let Some(editor) = self.editor {
                    target.set_value(host.value(editor));
                    1
                } else { 0 };
                host.end_dialog(true, result)
            }
            EDIT_CANCEL => host.end_dialog(true, 0),
            EDIT_ADD | EDIT_REMOVE => {
                if let Some(editor) = self.editor {
                    let value = host.value(editor);
                    if button == EDIT_REMOVE && value.count() < 2 { return true; }
                    let new_value = resize_dialog_value(&value, button == EDIT_ADD);
                    let area = host.area(editor);
                    host.child_remove(editor);
                    self.editor = get_value_editor(ingredients, self.property, Some(&new_value), None, host);
                    if let Some(replacement) = self.editor {
                        host.set_area(replacement, area);
                        host.child_add(replacement);
                    }
                    // No guessed Release of either opaque handle: RE7.
                }
                true
            }
            _ => true,
        }
    }
}

pub fn resize_dialog_value(value: &Variant, add: bool) -> Variant {
    let ty = value.mem_type() & !ARRAY;
    let zero = match ty {
        mem::BOOL => Elem::Bool(false), mem::UINT8 => Elem::U8(0), mem::SINT8 => Elem::I8(0),
        mem::UINT16 => Elem::U16(0), mem::SINT16 => Elem::I16(0), mem::UINT32 => Elem::U32(0),
        mem::SINT32 => Elem::I32(0), mem::FLOAT32 => Elem::F32(0.0),
        _ => return Variant::Empty,
    };
    let mut items = value.items();
    if add { items.push(zero); } else { items.pop(); }
    Variant::array(ty, items)
}

/// Useful to callers building their own property lists; does not infer the
/// unspecified native row population policy.
pub fn matching_properties(ingredients: &Ingredients, mask: u32) -> Vec<u32> {
    ingredients.properties.iter().filter_map(|(&id, p)| (p.filter & mask != 0).then_some(id)).collect()
}

pub fn has_variable_buttons(ingredients: &Ingredients, id: u32) -> bool {
    ingredients.properties.get(&id).is_some_and(|p| p.count == 0 && p.editor_class != RESPONSE_CURVE_CLASS)
}
