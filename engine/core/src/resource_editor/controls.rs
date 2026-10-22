// SPDX-License-Identifier: GPL-3.0-or-later
use crate::variant::{mem, Elem, Variant};

use super::{grid_parse, property_value_text, EditorError, Ingredients, TextPrimitives, UnpublishedFormatting};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Area {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Shared published Init effects; callers may keep unrelated window flags.
pub fn standard_flags(flags: u32) -> u32 {
    (flags & !0x800) | 0x10000
}

/// §10: createComboBox's UINT8 item-index counter wraps even though the
/// list control's item count is wider. Kept separate from map-key narrowing.
pub fn combo_selection_index(element_type: u16, index: usize) -> usize {
    if element_type == mem::UINT8 {
        usize::from(index as u8)
    } else {
        index
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboControl {
    pub child_id: u32,
    pub area: Area,
    pub items: Vec<String>,
    pub selected: usize,
    pub text_color: u32,
    pub second_color: u32,
    pub background: Option<u32>,
}

pub struct ComboEditor {
    pub element_type: u16,
    pub value: Variant,
    pub index_values: Vec<u32>,
    pub controls: Vec<Option<ComboControl>>,
    pub flags: u32,
    pub background_fill: bool,
    initialized: bool,
}

impl ComboEditor {
    pub fn new(element_type: u16, value: Variant) -> Result<Self, EditorError> {
        if !matches!(element_type, mem::UINT8 | mem::UINT32) {
            return Err(EditorError::UnsupportedType(element_type));
        }
        Ok(Self {
            element_type,
            value,
            index_values: Vec::new(),
            controls: Vec::new(),
            flags: 0,
            background_fill: false,
            initialized: false,
        })
    }

    pub fn init(
        &mut self,
        ingredients: &Ingredients,
        id: u32,
        width: i32,
        unknown: &mut impl UnpublishedFormatting,
    ) -> Result<(), EditorError> {
        if self.initialized {
            return Ok(());
        }
        let index_values: Vec<u32> = ingredients
            .value_map(id, self.element_type)
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default();
        let base: u32 = if self.element_type == mem::UINT8 {
            0xC7C3_4B1A
        } else {
            0x46C6_0DBF
        };
        let items: Vec<String> = index_values
            .iter()
            .map(|&v| {
                let elem = if self.element_type == mem::UINT8 {
                    Elem::U8(v as u8)
                } else {
                    Elem::U32(v)
                };
                property_value_text(ingredients, id, &Variant::Scalar(elem), unknown)
            })
            .collect::<Result<_, _>>()?;
        self.controls = self
            .value
            .items()
            .iter()
            .enumerate()
            .map(|(i, current)| {
                let index = index_values
                    .iter()
                    .position(|&v| v == current.as_i64() as u32)
                    .unwrap_or(0);
                let selected = combo_selection_index(self.element_type, index);
                let top = 5 + 20 * i as i32;
                Some(ComboControl {
                    child_id: base.wrapping_add(i as u32),
                    area: Area {
                        left: 5,
                        top,
                        right: width + 5,
                        bottom: top + 18,
                    },
                    items: items.clone(),
                    selected,
                    text_color: 0,
                    second_color: 0x80,
                    background: (self.element_type == mem::UINT32).then_some(0xE0E0E0),
                })
            })
            .collect();
        self.index_values = index_values;
        self.flags = standard_flags(self.flags);
        self.background_fill = true;
        self.initialized = true;
        Ok(())
    }

    pub fn read(&self) -> Result<Variant, EditorError> {
        if self.element_type == mem::UINT8 && self.value.count() > 255 {
            return Err(EditorError::NonterminatingUint8List);
        }
        let mut items = Vec::with_capacity(self.value.count());
        for i in 0..self.value.count() {
            let control = self
                .controls
                .get(i)
                .and_then(Option::as_ref)
                .ok_or(EditorError::MissingControl(i))?;
            let &v = self
                .index_values
                .get(control.selected)
                .ok_or(EditorError::InvalidSelection(control.selected))?;
            items.push(if self.element_type == mem::UINT8 {
                Elem::U8(v as u8)
            } else {
                Elem::U32(v)
            });
        }
        Ok(if self.value.mem_type() == self.element_type {
            Variant::Scalar(items[0])
        } else {
            Variant::array(self.element_type, items)
        })
    }
}

/// The spinner widget owns the treatment of an initial out-of-range value
/// (§11.1, RE5). It receives the actual range and original UINT32 unchanged.
pub trait SpinnerPrimitives {
    fn initial_value(&mut self, requested: u32, minimum: u32, maximum: u32) -> u32;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpinnerControl {
    pub child_id: u32,
    pub area: Area,
    pub requested: u32,
    pub value: u32,
    pub minimum: u32,
    pub maximum: u32,
    pub background: u32,
}

impl SpinnerControl {
    pub fn set_user_value(&mut self, value: u32) {
        self.value = value.clamp(self.minimum, self.maximum);
    }
}

pub struct SpinnerEditor {
    pub value: Variant,
    pub controls: Vec<Option<SpinnerControl>>,
    pub flags: u32,
    pub background_fill: bool,
    initialized: bool,
}

impl SpinnerEditor {
    pub fn new(value: Variant) -> Self {
        Self {
            value,
            controls: Vec::new(),
            flags: 0,
            background_fill: false,
            initialized: false,
        }
    }

    pub fn init(&mut self, host: &mut impl SpinnerPrimitives) {
        if self.initialized {
            return;
        }
        self.controls = self
            .value
            .items()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let requested = item.as_i64() as u32;
                let top = 5 + 20 * i as i32;
                Some(SpinnerControl {
                    child_id: 0xBA29_3DA4u32.wrapping_add(i as u32),
                    area: Area {
                        left: 5,
                        top,
                        right: 53,
                        bottom: top + 20,
                    },
                    requested,
                    value: host.initial_value(requested, 0, 255),
                    minimum: 0,
                    maximum: 255,
                    background: 0xE0E0E0,
                })
            })
            .collect();
        self.flags = standard_flags(self.flags);
        self.background_fill = true;
        self.initialized = true;
    }

    pub fn read(&self) -> Result<Variant, EditorError> {
        if self.value.mem_type() == mem::UINT32 {
            return Ok(self
                .controls
                .first()
                .and_then(Option::as_ref)
                .map_or_else(|| self.value.clone(), |c| Variant::Scalar(Elem::U32(c.value))));
        }
        let items = (0..self.value.count())
            .map(|i| {
                self.controls
                    .get(i)
                    .and_then(Option::as_ref)
                    .map(|c| Elem::U32(c.value))
                    .ok_or(EditorError::MissingControl(i))
            })
            .collect::<Result<_, _>>()?;
        Ok(Variant::array(mem::UINT32, items))
    }
}

/// Native `%.3g` is a primitive of the C runtime, not a grid algorithm. It is
/// supplied separately from §7's incompletely specified property formatter.
pub trait GridFormatting {
    fn three_significant_digits(&mut self, value: f32) -> String;
}

pub struct GridEditor {
    pub value: Variant,
    pub rows: usize,
    pub cells: Vec<Option<String>>,
    pub row_headers: Vec<String>,
    pub column_headers: Vec<String>,
    pub area: Area,
    pub flags: u32,
    pub background_fill: bool,
    pub appearance: GridAppearance,
    initialized: bool,
}

impl GridEditor {
    pub fn new(value: Variant) -> Self {
        Self {
            value,
            rows: 0,
            cells: Vec::new(),
            row_headers: Vec::new(),
            column_headers: Vec::new(),
            area: Area::default(),
            flags: 0,
            background_fill: false,
            appearance: GridAppearance::default(),
            initialized: false,
        }
    }

    pub fn init(&mut self, width: i32, height: i32, formatter: &mut impl GridFormatting) {
        if self.initialized {
            return;
        }
        self.rows = self.value.count() / 16;
        self.cells = vec![None; self.rows * 16];
        if let Variant::Array { elem, items } = &self.value {
            if matches!(
                *elem,
                mem::UINT8 | mem::SINT8 | mem::UINT16 | mem::SINT16 | mem::UINT32 | mem::SINT32 | mem::FLOAT32
            ) {
                for (cell, item) in self.cells.iter_mut().zip(items) {
                    *cell = Some(match item {
                        Elem::F32(value) => formatter.three_significant_digits(*value),
                        item => item.as_i64().to_string(),
                    });
                }
            }
        }
        self.row_headers = (0..self.rows).map(|r| r.to_string()).collect();
        self.column_headers = (0..16).map(|c| c.to_string()).collect();
        self.area = Area {
            left: 5,
            top: 5,
            right: width - 5,
            bottom: height - 23,
        };
        self.flags = standard_flags(self.flags);
        self.background_fill = true;
        self.initialized = true;
    }

    pub fn read(&mut self, numbers: &mut impl TextPrimitives) -> Result<Variant, EditorError> {
        if let Variant::Array { elem, items } = &mut self.value {
            if matches!(
                *elem,
                mem::UINT8 | mem::SINT8 | mem::UINT16 | mem::SINT16 | mem::UINT32 | mem::SINT32 | mem::FLOAT32
            ) {
                for (i, item) in items.iter_mut().take(self.rows * 16).enumerate() {
                    let text = self
                        .cells
                        .get(i)
                        .and_then(Option::as_ref)
                        .ok_or(EditorError::MissingControl(i))?;
                    *item = grid_parse(*elem, text, numbers)?;
                }
            }
        }
        Ok(self.value.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorPair {
    pub position: u32,
    pub color: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorRow {
    pub position: String,
    pub value: String,
    pub swatch: u32,
}

impl From<ColorPair> for ColorRow {
    fn from(pair: ColorPair) -> Self {
        Self {
            position: (pair.position as i32).to_string(),
            value: format!("{:08x}", pair.color),
            swatch: pair.color & 0xFF_FFFF,
        }
    }
}

pub trait ColorPicker {
    fn pick_color(&mut self, current: u32) -> u32;
}

#[derive(Default)]
pub struct ColorSequenceEditor {
    pub pairs: Vec<ColorPair>,
    pub rows: Vec<ColorRow>,
    pub selected: Option<usize>,
    pub flags: u32,
    pub background_fill: bool,
    pub appearance: GridAppearance,
    initialized: bool,
}

impl ColorSequenceEditor {
    /// Command 3 is the published window-message command for these buttons;
    /// it is distinct from cSCPropertyView's GZOnCommand message.
    pub fn button_command(
        &mut self,
        command: u32,
        button: u32,
        numbers: &mut impl TextPrimitives,
    ) -> Result<bool, EditorError> {
        if command != 3 {
            return Ok(false);
        }
        match button {
            0xEA0C_B68D => self.add()?,
            0x4A0C_B691 => self.delete(),
            0x2A0D_BDEE => self.refresh(numbers),
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub fn set_value(&mut self, value: &Variant) -> Result<(), EditorError> {
        let Variant::Array {
            elem: mem::UINT32,
            items,
        } = value
        else {
            return Err(EditorError::UnsupportedType(value.mem_type()));
        };
        self.pairs = items
            .chunks_exact(2)
            .map(|words| ColorPair {
                position: words[0].as_i64() as u32,
                color: words[1].as_i64() as u32,
            })
            .collect();
        // §11.3 only publishes a vector rebuild on SetValue. Grid reload is Init
        // or an explicit row redraw; don't invent a post-Init reload (RE6).
        Ok(())
    }

    pub fn init(&mut self) {
        if self.initialized {
            return;
        }
        self.rows = self.pairs.iter().copied().map(ColorRow::from).collect();
        self.flags = standard_flags(self.flags);
        self.background_fill = true;
        self.initialized = true;
    }

    pub fn add(&mut self) -> Result<(), EditorError> {
        if self.pairs.is_empty() {
            return Err(EditorError::OutOfBoundsNativeRead);
        }
        let index = self.selected.unwrap_or(self.pairs.len() - 1).min(self.pairs.len() - 1);
        let pair = self.pairs[index];
        self.pairs.insert(index + 1, pair);
        // Only rows after the insertion are redrawn: earlier typed edits survive.
        self.rows.truncate(index + 1);
        self.rows
            .extend(self.pairs[index + 1..].iter().copied().map(ColorRow::from));
        Ok(())
    }

    pub fn delete(&mut self) {
        if let Some(index) = self.selected.filter(|&r| r < self.pairs.len() && self.pairs.len() >= 2) {
            self.pairs.remove(index);
            if index < self.rows.len() {
                self.rows.remove(index);
            }
        }
    }

    pub fn refresh(&mut self, numbers: &mut impl TextPrimitives) {
        // RE6: equal-position ordering is not published; choose stable order.
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .map(|row| (numbers.atol32(&row.position), row.value.clone()))
            .collect();
        rows.sort_by_key(|(position, _)| *position);
        self.pairs = rows
            .into_iter()
            .map(|(position, color)| ColorPair {
                position: position as u32,
                // strtoul(base16) shares the Rule N overflow/sign primitive. Force
                // hex base without reinterpreting the user's text as decimal.
                color: hex_word(&color),
            })
            .collect();
        self.rows = self.pairs.iter().copied().map(ColorRow::from).collect();
    }

    pub fn activate(&mut self, row: usize, column: usize, host: &mut impl ColorPicker) {
        if column == 2 && self.pairs.len() >= 2 {
            if let Some(pair) = self.pairs.get_mut(row) {
                pair.color = host.pick_color(pair.color);
                if let Some(grid_row) = self.rows.get_mut(row) {
                    *grid_row = ColorRow::from(*pair);
                }
            }
        }
    }

    pub fn paint(&self, row: usize, column: usize, area: Area) -> Option<(Area, u32)> {
        self.pairs.get(row).filter(|_| column == 2).map(|pair| {
            (
                Area {
                    left: area.left + 1,
                    top: area.top + 1,
                    right: area.right - 1,
                    bottom: area.bottom - 1,
                },
                pair.color & 0xFF_FFFF,
            )
        })
    }

    pub fn read(&self) -> Variant {
        Variant::array(
            mem::UINT32,
            self.pairs
                .iter()
                .flat_map(|pair| [Elem::U32(pair.position), Elem::U32(pair.color)])
                .collect(),
        )
    }
}

fn hex_word(text: &str) -> u32 {
    let trimmed = text.trim_start();
    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    // Rule N supplies C strtoul's saturation. Apply the sign only when there
    // was no overflow (including -0x100000000 => UINT32_MAX).
    let digits = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .unwrap_or(digits);
    let mut n = 0u32;
    let mut overflow = false;
    for d in digits.chars().map_while(|c| c.to_digit(16)) {
        match n.checked_mul(16).and_then(|n| n.checked_add(d)) {
            Some(v) => n = v,
            None => overflow = true,
        }
    }
    if overflow {
        u32::MAX
    } else if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

/// Published grid brush/style arguments; no meaning is invented for the
/// framework slots whose roles remain inferred in §11.2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridAppearance {
    pub colors: [u32; 3],
    pub styles: [u32; 3],
}

impl Default for GridAppearance {
    fn default() -> Self {
        Self {
            colors: [0, 0xFFFFFF, 0xE0E0E0],
            styles: [8, 0x10, 1],
        }
    }
}
