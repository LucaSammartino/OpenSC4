// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
pub const ACCEPT: u32 = 0x42B7C356;
pub const TEXT: u32 = 0x89264D48;
pub const GRIDS: [u32; 3] = [0x49405340, 0x49405342, 0x49405343];
pub const NUMERIC: [u32; 4] = [0x294053DA, 0xE9B98665, 0xE9B98666, 0xC9B981C9];
pub const CHECK: u32 = 0xCA4A929A;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogKind {
    Compatibility,
    Name,
    NetworkPath,
    Slope,
    Selector(SelectorKind),
    Browser,
}
impl DialogKind {
    pub fn resource(self) -> (u32, [u32; 3]) {
        let (selector, instance) = match self {
            Self::Compatibility => (0xE9264B49, 0xE9263D4D),
            Self::Name | Self::NetworkPath | Self::Slope => (0xC9264BE2, 0xE9263D4C),
            Self::Selector(SelectorKind::Building) => (0x89264BF9, 0xE9263D4F),
            Self::Selector(SelectorKind::Flora) => (0x89264BFA, 0x89404462),
            Self::Selector(SelectorKind::Prop) => (0x097CEFB0, 0x697CEF4C),
            Self::Selector(SelectorKind::Texture) => (0x6987B3F8, 0x6987B3F7),
            Self::Browser => (0, 0), // Browser is not a script resource.
        };
        (selector, [0, 0x96A006B0, instance])
    }
}
/// Only primitive services. Unsupported defaults are explicit dependency gaps.
pub trait Host {
    fn effect(&mut self, effect: Effect);
    fn create_dialog(&mut self, kind: DialogKind) -> Option<u64>;
    fn has_control(&mut self, window: u64, id: u32) -> bool;
    fn run_modal(&mut self, window: u64) -> u32;
    fn text(&mut self, window: u64, id: u32) -> String;
    fn integer(&mut self, window: u64, id: u32) -> i32;
    fn selected_rows(&mut self, window: u64, id: u32) -> Vec<(u32, u32)>;
    fn grid_text(&mut self, _: u64, _: u32, _: u32, _: u32) -> Result<String> {
        Err(Unsupported::Dependency("grid text"))
    }
    fn grid_interval(&mut self, _: u64, _: u32) -> Option<(u32, u32)> {
        None
    }
    fn control_parent(&mut self, _: u64, _: u32) -> Option<u64> {
        None
    }
    fn prompt(&mut self, text: &str, flags: u32) -> u32;
    fn checked(&mut self, window: u64, id: u32) -> bool {
        self.integer(window, id) != 0
    }
    fn parse_hex(&mut self, _: &str) -> Result<u32> {
        Err(Unsupported::Dependency("RZAtoUint32HexA"))
    }
    fn parse_unsigned(&mut self, _: &str) -> Result<u32> {
        Err(Unsupported::Dependency("RZConvertToUint32A"))
    }
    fn persist(&mut self, _: &Identity, _: &Configuration) -> Result<bool> {
        Err(Unsupported::Dependency("foreign persistence format"))
    }
    fn browser_selection(&mut self, _: u64) -> Option<(u32, String)> {
        None
    }
    fn data_directory(&mut self) -> Result<String> {
        Err(Unsupported::Dependency("data directory"))
    }
    fn directory_exists(&mut self, _: &str) -> bool {
        false
    }
    /// Enumeration returns individually acquired provider references.
    fn providers(&mut self) -> Result<Vec<u64>> {
        Err(Unsupported::Dependency("provider manager"))
    }
    fn provider_for_key(&mut self, _: [u32; 3]) -> Result<Option<u64>> {
        Err(Unsupported::Dependency("provider manager"))
    }
    fn query_provider(&mut self, _: u64) -> Option<u64> {
        None
    }
    fn provider_directory(&mut self, _: u64) -> String {
        String::new()
    }
    fn provider_key(&mut self, _: u64, _: &str, _: &str) -> Option<[u32; 3]> {
        None
    }
    /// Path query status is intentionally not used by the editor.
    fn provider_path(&mut self, _: u64, _: [u32; 3]) -> String {
        String::new()
    }
    fn read_configuration(&mut self, _: [u32; 3]) -> Result<Option<Configuration>> {
        Err(Unsupported::Dependency("foreign exemplar read"))
    }
    fn configuration_id_for_type(&mut self, _: u32) -> Result<Option<u32>> {
        Err(Unsupported::Dependency("configuration type catalogue"))
    }
    fn saved_id_present(&mut self, _: u32) -> bool {
        false
    }
    fn register_configuration(&mut self, config: &Configuration, option: bool);
    fn create_cursor(&mut self, _: u32, _: u32) -> Result<Option<u64>> {
        Err(Unsupported::Dependency("cursor resource factory"))
    }
    fn destroy_cursor_temporary(&mut self, p: u64) {
        self.effect(Effect::Release(p));
    }
    fn popup(&mut self, _: &[String]) -> i32 {
        -1
    }
}
fn modal(w: u64, h: &mut impl Host) -> u32 {
    h.effect(Effect::Register(w));
    let result = h.run_modal(w);
    h.effect(Effect::Unregister(w));
    h.effect(Effect::Focus);
    result
}
fn dispose(w: u64, h: &mut impl Host) {
    h.effect(Effect::ShutdownDialog(w));
    h.effect(Effect::Release(w));
}
fn seed(w: u64, id: u32, value: Value, h: &mut impl Host) {
    h.effect(Effect::Seed(w, id, value));
}

const DESCRIPTIONS: [&[&str]; 3] = [
    &[
        "Low Density Residential",
        "Medium Density Residential",
        "High Density Residential",
        "Low Density Commercial",
        "Medium Density Commercial",
        "High Density Commercial",
        "Resource Industrial",
        "Light Industrial",
        "Heavy Industrial",
        "Military",
        "Airport",
        "Seaport",
        "Spaceport",
        " Landfill",
        "Plopped Building",
    ],
    &["Low Wealth", "Medium Wealth", "High Wealth"],
    &[
        "Residence",
        "Services",
        "Office",
        "Tourism",
        "Agriculture",
        "Processing",
        "Manufacturing",
        "High Tech",
    ],
];
impl Editor {
    pub fn compatibility_dialog(&mut self, h: &mut impl Host) -> Result<bool> {
        self.config()?;
        let Some(w) = h.create_dialog(DialogKind::Compatibility) else {
            return Ok(false);
        };
        let ignored = h.has_control(w, 0x294053DB);
        if ignored {
            seed(
                w,
                0x294053DB,
                Value::Integer(self.config()?.ignored_numeric),
                h,
            );
        }
        let mut numeric = [false; 4];
        for (i, id) in NUMERIC.into_iter().enumerate() {
            numeric[i] = h.has_control(w, id);
            if numeric[i] {
                let v = if i == 3 {
                    i32::from(self.config()?.minimum)
                } else {
                    let v = f64::from(self.config()?.numeric[i]);
                    // LE6: the published phrase x87-to-integer does not state
                    // fractional rounding. Preserve the integral domain only.
                    if v.fract() != 0.0 && v.is_finite() {
                        dispose(w, h);
                        return Err(Unsupported::Dependency(
                            "LE6 fractional numeric initialization",
                        ));
                    }
                    if !v.is_finite() || v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
                        dispose(w, h);
                        return Err(Unsupported::NumericConversion);
                    }
                    v as i32
                };
                seed(w, id, Value::Integer(v), h);
                seed(w, id, Value::ClearFlag(0x4000), h);
            }
        }
        let mut grids = [false; 3];
        for (i, id) in GRIDS.into_iter().enumerate() {
            grids[i] = h.has_control(w, id);
            if grids[i] {
                h.effect(Effect::GridInit(w, id));
                for (row, label) in DESCRIPTIONS[i].iter().enumerate() {
                    h.effect(Effect::GridRow(
                        w,
                        id,
                        row,
                        (row + 1).to_string(),
                        Cell::Text((*label).into()),
                    ));
                    if self.config()?.compatibility[i].get(row + 1) == Some(&true) {
                        h.effect(Effect::GridSelect(w, id, row));
                    }
                }
            }
        }
        let mut checks = [false; 4];
        for (i, present) in checks.iter_mut().enumerate() {
            *present = h.has_control(w, CHECK + i as u32);
            if *present {
                seed(
                    w,
                    CHECK + i as u32,
                    Value::Checked(self.config()?.checkboxes & (1 << i) != 0),
                    h,
                );
            }
        }
        if modal(w, h) != ACCEPT {
            dispose(w, h);
            return Ok(false);
        }
        let result = (|| {
            for (i, id) in GRIDS.into_iter().enumerate() {
                // Native faults here, potentially after earlier vectors changed.
                if !grids[i] {
                    return Err(Unsupported::MissingAcceptedGrid(i));
                }
                let selections = h.selected_rows(w, id);
                let bits = &mut self.config_mut()?.compatibility[i];
                bits.fill(false);
                for (row, _) in selections.into_iter().take(32) {
                    let bit = row
                        .checked_add(1)
                        .ok_or(Unsupported::BitOutsideLogicalRange(row))?;
                    // Native does not validate or resize. Report unsupported storage
                    // instead of making an out-of-range result a successful no-op.
                    *bits
                        .get_mut(bit as usize)
                        .ok_or(Unsupported::BitOutsideLogicalRange(bit))? = true;
                }
            }
            for (i, id) in NUMERIC.into_iter().enumerate() {
                if numeric[i] {
                    let v = h.integer(w, id);
                    if i == 3 {
                        self.config_mut()?.minimum = (v as u8).max(1);
                    } else {
                        self.config_mut()?.numeric[i] = v as f32;
                    }
                }
            }
            self.config_mut()?.checkboxes = 0;
            for (i, present) in checks.into_iter().enumerate() {
                if present && h.checked(w, CHECK + i as u32) {
                    self.config_mut()?.checkboxes |= 1 << i;
                }
            }
            self.set_dirty(1, h);
            Ok(true)
        })();
        dispose(w, h);
        result
    }
    fn text_dialog(
        &self,
        kind: DialogKind,
        initial: String,
        title: Option<&str>,
        h: &mut impl Host,
    ) -> Option<(u64, bool, Option<String>)> {
        let w = h.create_dialog(kind)?;
        if let Some(title) = title {
            seed(w, 0, Value::Text(title.into()), h);
        }
        if h.has_control(w, TEXT) {
            seed(w, TEXT, Value::Text(initial), h);
        }
        let accepted = modal(w, h) == ACCEPT;
        let value = if accepted && h.has_control(w, TEXT) {
            Some(h.text(w, TEXT))
        } else {
            None
        };
        Some((w, accepted, value))
    }
    pub fn name_dialog(&mut self, h: &mut impl Host) -> Result<bool> {
        let initial = self.config()?.name.clone();
        let Some((w, accepted, text)) = self.text_dialog(DialogKind::Name, initial, None, h) else {
            return Ok(false);
        };
        if let Some(text) = text {
            self.config_mut()?.name = if text.is_empty() {
                "<unnamed>".into()
            } else {
                text
            };
            self.set_dirty(1, h);
            h.effect(Effect::Title);
        }
        dispose(w, h);
        Ok(accepted)
    }
    pub fn network_path_dialog(&mut self, id: RecordId, h: &mut impl Host) -> Result<bool> {
        let initial = self
            .record(id)
            .ok_or(Unsupported::MissingRecord)?
            .ids
            .get(3)
            .copied()
            .unwrap_or(0);
        let Some((w, accepted, text)) = self.text_dialog(
            DialogKind::NetworkPath,
            format!("{initial:x}"),
            Some("Set path ID for connection"),
            h,
        ) else {
            return Ok(false);
        };
        let result = (|| {
            if let Some(text) = text {
                let v = h.parse_hex(&text)?;
                let ids = &mut self.record_mut(id).ok_or(Unsupported::MissingRecord)?.ids;
                if ids.len() < 4 {
                    ids.resize(4, 0);
                }
                ids[3] = v;
            }
            Ok(accepted)
        })();
        dispose(w, h);
        result
    }
    pub fn slope_dialog(&mut self, h: &mut impl Host) -> Result<bool> {
        self.config()?;
        let Some((w, accepted, text)) = self.text_dialog(
            DialogKind::Slope,
            self.slope.to_string(),
            Some("Set Slope of Displayed Lot"),
            h,
        ) else {
            return Ok(false);
        };
        let result = (|| {
            if let Some(text) = text {
                let v = h.parse_unsigned(&text)?;
                let c = self.config()?;
                if f64::from(v) > f64::from(c.numeric[1]) || f64::from(v) < f64::from(c.numeric[2])
                {
                    h.effect(Effect::Message(
                        "Slope is outside min/max values for this lot".into(),
                        0,
                    ));
                } else if v > 80 {
                    h.effect(Effect::Message("Slope limited to 80 degrees".into(), 0));
                } else {
                    // Prepare3DView is still LE5; do not report a completed view refresh.
                    self.slope = v;
                    h.effect(Effect::Prepare3D);
                    h.effect(Effect::Refresh);
                }
            }
            Ok(accepted)
        })();
        dispose(w, h);
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorKind {
    Building,
    Flora,
    Prop,
    Texture,
}
impl SelectorKind {
    pub fn grid(self) -> u32 {
        match self {
            Self::Building => 0xE9264E71,
            Self::Flora => 0x69433083,
            Self::Prop => 0x097CEFB1,
            Self::Texture => 0x6987B3F9,
        }
    }
}
/// Inputs to the published row-construction kernel. Names must be known resource
/// strings; unknown LE4/LE5 name fallback is not replaceable by a host outcome.
#[derive(Clone, Debug)]
pub struct CatalogRow {
    pub id: u32,
    pub family: bool,
    pub name: Option<String>,
    pub image: Option<u64>,
}
pub fn selector_rows(
    kind: SelectorKind,
    rows: &[CatalogRow],
    options: [bool; 2],
) -> Result<Vec<(u32, Cell)>> {
    if kind == SelectorKind::Texture {
        return rows
            .iter()
            .map(|r| {
                Ok((
                    r.id,
                    Cell::Image(
                        r.image
                            .ok_or(Unsupported::Dependency("LE5 texture images"))?,
                    ),
                ))
            })
            .collect();
    }
    let mut out = Vec::new();
    for family in [true, false] {
        if kind == SelectorKind::Prop && !options[usize::from(!family)] {
            continue;
        }
        for r in rows.iter().filter(|r| r.family == family) {
            let cell = if kind == SelectorKind::Texture {
                Cell::Image(
                    r.image
                        .ok_or(Unsupported::Dependency("LE5 texture images"))?,
                )
            } else if kind == SelectorKind::Flora && !family {
                Cell::Text("<Flora>".into())
            } else {
                let name = r
                    .name
                    .as_ref()
                    .ok_or(Unsupported::Dependency("LE4/LE5 selector name fallback"))?;
                let prefix = match (kind, family) {
                    (SelectorKind::Building, true) => "Family: ",
                    (SelectorKind::Building, false) => "Bldg: ",
                    (SelectorKind::Prop, true) => " (F)",
                    (SelectorKind::Prop, false) => " (P)",
                    _ => "",
                };
                Cell::Text(format!("{prefix}{name}"))
            };
            out.push((r.id, cell));
        }
    }
    Ok(out)
}
pub fn selector_dialog(
    kind: SelectorKind,
    rows: &[(u32, Cell)],
    preselected: Option<&[u32]>,
    output: &mut Vec<u32>,
    h: &mut impl Host,
) -> Result<bool> {
    let Some(w) = h.create_dialog(DialogKind::Selector(kind)) else {
        return Ok(false);
    };
    let grid = kind.grid();
    if !h.has_control(w, grid) {
        dispose(w, h);
        return Ok(false);
    }
    h.effect(Effect::GridInit(w, grid));
    for (i, (id, cell)) in rows.iter().enumerate() {
        h.effect(Effect::GridRow(
            w,
            grid,
            i,
            format!("0x{id:08x}"),
            cell.clone(),
        ));
        if preselected.is_some_and(|ids| ids.contains(id)) {
            h.effect(Effect::GridSelect(w, grid, i));
        }
    }
    h.effect(Effect::Sort(grid, 0, false, 1, 0));
    h.effect(Effect::Callback(w, true));
    let accepted = modal(w, h) == ACCEPT;
    h.effect(Effect::Callback(w, false));
    let result = (|| {
        if accepted {
            for (row, _) in h.selected_rows(w, grid).into_iter().take(32) {
                // Read current displayed text after sorting; never use row IDs.
                let text = h.grid_text(w, grid, row, 0)?;
                let id = h.parse_hex(&text)?;
                if id != 0 {
                    output.push(id);
                }
            }
        }
        Ok(accepted)
    })();
    dispose(w, h);
    result
}

pub fn dialog_callback(
    w: u64,
    category: u32,
    command: u32,
    control: u32,
    keys_down: [bool; 2],
    h: &mut impl Host,
) -> Result<bool> {
    let sort = match control {
        0x097CEFB2 => Some((0x097CEFB1, 0)),
        0x097CEFB3 => Some((0x097CEFB1, 1)),
        0x69433011 => Some((0x69433083, 0)),
        0x69433012 => Some((0x69433083, 1)),
        0xE9432DD7 => Some((0xE9264E71, 0)),
        0xE9432DD8 => Some((0xE9264E71, 1)),
        0x6987B3FA => Some((0x6987B3F9, 0)),
        _ => None,
    };
    if category == 3 && command == 0x287259F6 {
        if let Some((grid, col)) = sort {
            if let Some(parent) = h.control_parent(w, control) {
                if h.has_control(parent, grid) {
                    h.effect(Effect::Sort(grid, col, keys_down[0] || keys_down[1], 1, 0));
                }
            }
            return Ok(true);
        }
    }
    if matches!(category, 3 | 5) && (0x41..=0x5A).contains(&command) {
        let grid = 0x097CEFB1;
        if h.has_control(w, grid) {
            if let Some((first, last)) = h.grid_interval(w, grid) {
                for row in first..=last {
                    let text = h.grid_text(w, grid, row, 1)?;
                    if text.as_bytes().first().copied() == Some(command as u8) {
                        h.effect(Effect::Scroll(grid, row));
                        break;
                    }
                }
            }
        }
        return Ok(true);
    }
    Ok(false)
}
