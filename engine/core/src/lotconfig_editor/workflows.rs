// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn release_identity(i: &Identity, h: &mut impl Host) {
    // Independently release slots, even if the two token values coincide.
    for p in i.references.into_iter().flatten() {
        h.effect(Effect::Release(p));
    }
}
fn retain_identity(i: &Identity, h: &mut impl Host) {
    for p in i.references.into_iter().flatten() {
        h.effect(Effect::AddRef(p));
    }
}
/// Windows separators are intentional; no slash normalization is performed.
pub fn default_path(data: &str) -> String {
    format!("{data}\\Exemplar\\Lot Templates")
}
pub fn split_identity_path(path: &str) -> crate::paths::PathParts {
    crate::paths::helpers::splitpath(path.as_bytes())
}
/// §24.2 C-locale framework transform, composing string-encoding §§3.1–3.2.
/// Non-C wide tables belong to foreign question RZS2. This is deliberately
/// not Rust Unicode folding or its UTF-8 decoder (four bytes produce one word).
pub fn lowercase_directory(path: &[u8], c_locale: bool) -> Result<Vec<u8>> {
    if !c_locale {
        return Err(Unsupported::Dependency("RZS2 non-C lowercase tables"));
    }
    if path.iter().all(|&b| b < 0x7f) {
        return Ok(path.to_ascii_lowercase());
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < path.len() {
        let first = path[at];
        let (n, mask, minimum) = match first {
            0..=0x7f => (1, 0x7f, 0),
            0xc2..=0xdf => (2, 0x1f, 0x80),
            0xe0..=0xef => (3, 0x0f, if first == 0xe0 { 0xa0 } else { 0x80 }),
            0xf0..=0xf7 => (4, 0x07, if first == 0xf0 { 0x90 } else { 0x80 }),
            0xf8..=0xfb => (5, 0x03, if first == 0xf8 { 0x88 } else { 0x80 }),
            0xfc..=0xfd => (6, 0x01, if first == 0xfc { 0x84 } else { 0x80 }),
            _ => break,
        };
        if path.len() - at < n
            || (n > 1 && (path[at + 1] < minimum || path[at + 1..at + n].iter().any(|b| !(0x80..=0xbf).contains(b))))
        {
            break;
        }
        let mut word = u16::from(first & mask);
        for &byte in &path[at + 1..at + n] {
            word = (word << 6) | u16::from(byte & 0x3f);
        }
        if (0x41..=0x5a).contains(&word) {
            word += 0x20;
        }
        if word < 0x80 {
            out.push(word as u8);
        } else if word < 0x800 {
            out.extend([0xc0 | (word >> 6) as u8, 0x80 | (word & 0x3f) as u8]);
        } else {
            out.extend([
                0xe0 | (word >> 12) as u8,
                0x80 | ((word >> 6) & 0x3f) as u8,
                0x80 | (word & 0x3f) as u8,
            ]);
        }
        at += n;
    }
    Ok(out)
}
pub fn identity_from_path(path: &str, h: &mut impl Host) -> Result<Option<Identity>> {
    let parts = split_identity_path(path);
    let directory = [parts.drive, parts.directory].concat();
    if !h.directory_exists(&directory) {
        return Ok(None);
    }
    let providers = h.providers()?;
    let mut result = None;
    let mut unsupported = false;
    for provider in providers {
        if result.is_none() && !unsupported {
            if let Some(interface) = h.query_provider(provider) {
                let supplied = h.provider_directory(interface);
                let compared = h
                    .lowercase_path(&directory)
                    .and_then(|directory| h.lowercase_path(&supplied).map(|supplied| directory == supplied));
                unsupported = compared.is_err();
                if compared == Ok(true) {
                    if let Some(key) = h.provider_key(interface, &parts.name, &parts.extension) {
                        let i = Identity {
                            id: key[2],
                            key,
                            path: path.into(),
                            references: [Some(provider), Some(interface)],
                        };
                        retain_identity(&i, h);
                        result = Some(i);
                    }
                }
                h.effect(Effect::Release(interface));
            }
        }
        h.effect(Effect::Release(provider));
    }
    if unsupported {
        Err(Unsupported::Dependency("RZS2 directory lowercase tables"))
    } else {
        Ok(result)
    }
}
pub fn identity_from_key(key: [u32; 3], h: &mut impl Host) -> Result<Option<Identity>> {
    let Some(provider) = h.provider_for_key(key)? else {
        return Ok(None);
    };
    let result = h.query_provider(provider).map(|interface| {
        let i = Identity {
            id: key[2],
            key,
            path: h.provider_path(interface, key),
            references: [Some(provider), Some(interface)],
        };
        retain_identity(&i, h);
        h.effect(Effect::Release(interface));
        i
    });
    h.effect(Effect::Release(provider));
    Ok(result)
}
pub fn identity_from_type(ty: u32, h: &mut impl Host) -> Result<Option<Identity>> {
    let Some(id) = h.configuration_id_for_type(ty)? else {
        return Ok(None);
    };
    identity_from_key([0x6534284A, 0xA8FBD372, id], h)
}

impl Editor {
    pub fn create_new(&mut self, h: &mut impl Host) -> bool {
        let old = std::mem::take(&mut self.identity);
        release_identity(&old, h);
        self.current = Some(Configuration {
            name: "New Lot".into(),
            size: [1, 1],
            numeric: [5.0, 90.0, 0.0],
            ..Default::default()
        });
        // Rust-owned configurations have no COM reference; no borrowed pointer
        // is released. Selection is deliberately not repaired by this method.
        self.set_dirty(0, h);
        true
    }
    pub fn read(&mut self, key: [u32; 3], h: &mut impl Host) -> Result<bool> {
        let Some(config) = h.read_configuration(key)? else {
            return Ok(false);
        };
        self.current = Some(config);
        self.renumber_loaded_records();
        let c = self.config_mut()?;
        let code = h
            .configuration_manager()?
            .fix_records(
                (u32::from(c.size[0]), u32::from(c.size[1])),
                &mut c.objects,
                &mut c.retaining_walls,
                |r| &mut r.data,
            )
            .0;
        if code != 0 {
            self.set_dirty(1, h);
            let reason = match code {
                1 => "The family has changed.",
                2 => "The family is now empty.",
                3 => "The footprint has changed.",
                4 => "The footprint was out of bounds and needs to be fixed.",
                5 => "The center has changed.",
                6 => "The retaining walls have changed.",
                _ => "",
            };
            h.effect(Effect::ReconciliationWarning {
                introduction: "This lot has components that have changed since the last edit.",
                reason,
            });
        }
        Ok(true)
    }
    fn renumber_loaded_records(&mut self) {
        if let Some(c) = &mut self.current {
            for r in &mut c.objects {
                r.id = self.next_id;
                self.next_id += 1;
            }
        }
    }
    pub fn open(&mut self, key: [u32; 3], h: &mut impl Host) -> Result<bool> {
        // PostInit is first, before read/new, and is not fully published.
        let _ = (key, h);
        Err(Unsupported::Dependency(
            "LE5 PostInit; LE4 SetLotSize; LE5 Prepare3DView",
        ))
    }
    /// The published portion after an independently initialized lifecycle.
    /// This is not Open and never claims to complete it.
    pub fn open_configuration_stage(&mut self, key: [u32; 3], h: &mut impl Host) -> Result<bool> {
        if key[0] == 0 {
            self.create_new(h);
        } else {
            if !self.read(key, h)? {
                return Ok(false);
            }
            let Some(identity) = identity_from_key(key, h)? else {
                return Ok(false);
            };
            self.identity = identity;
        }
        h.effect(Effect::Title);
        Err(Unsupported::Dependency("LE4 SetLotSize"))
    }
    /// Â§9.2 finite-only verification. Predicate unordered behavior is LE3.
    pub fn verify_bounds(&self, partial: bool, prompt: bool, h: &mut impl Host) -> Result<bool> {
        let c = self.config()?;
        for r in &c.objects {
            if !matches!(r.data.kind, 0 | 4 | 1) {
                continue;
            }
            let [l, t, right, bottom] = bounds(&r.data);
            if [l, t, right, bottom].iter().any(|x| !x.is_finite()) {
                return Err(Unsupported::Dependency("LE3 verification unordered geometry"));
            }
            let [width, depth] = c.size.map(|v| f32::from(v) * 16.0);
            let valid = if partial {
                right >= 0.0 && bottom >= 0.0 && l <= width && t <= depth
            } else {
                l >= 0.0 && t >= 0.0 && right <= width && bottom <= depth
            };
            if !valid {
                return Ok(prompt
                    && h.message_box(
                        "One or more lot objects don't fit within the lot bounds. Do you want to continue or cancel?",
                        &c.name,
                        (1, 1, 0),
                    ) == 0x5301814A);
            }
        }
        Ok(true)
    }
    pub fn save(&mut self, success: bool, error: bool, h: &mut impl Host) -> Result<bool> {
        if self.identity.key[0] == 0 {
            h.effect(Effect::SharedSaveAs(true));
            let result = self.save_as(false, success, error, h);
            h.effect(Effect::SharedSaveAs(false));
            return result;
        }
        if !self.verify_bounds(true, true, h)? {
            return Ok(false);
        }
        if !h.persist(&self.identity, self.config()?)? {
            if error {
                h.effect(Effect::Message("Error saving lot configuration.".into(), 0x10000));
            }
            return Ok(false);
        }
        self.set_dirty(0, h);
        h.effect(Effect::Saved(self.identity.clone()));
        if success {
            h.effect(Effect::Message("Lot configuration saved.".into(), 0));
        }
        Ok(true)
    }
    pub fn save_as(&mut self, copy: bool, success: bool, error: bool, h: &mut impl Host) -> Result<bool> {
        self.config()?;
        let initial = if self.identity.key[0] == 0 {
            default_path(&h.data_directory()?)
        } else {
            let parts = split_identity_path(&self.identity.path);
            // Counted byte strings preserve the splitter's 255-byte cut even
            // when it falls inside a UTF-8 sequence.
            let initial = [parts.drive, parts.directory].concat();
            return self.save_as_browser(copy, success, error, initial, h);
        };
        self.save_as_browser(copy, success, error, initial.into_bytes(), h)
    }
    fn save_as_browser(
        &mut self,
        copy: bool,
        success: bool,
        error: bool,
        initial: Vec<u8>,
        h: &mut impl Host,
    ) -> Result<bool> {
        let Some(w) = h.create_dialog(DialogKind::Browser) else {
            return Err(Unsupported::AllocationFault);
        };
        h.effect(Effect::AddRef(w));
        h.effect(Effect::Seed(w, 0, Value::Bytes(initial)));
        // Raw browser primitives: size 200x200, mode 2, window flag 0x100.
        h.effect(Effect::BrowserSetup(w, [200, 200], 2, 0x100, true));
        h.effect(Effect::Register(w));
        let accepted = h.run_modal(w) == ACCEPT;
        let selected = if accepted {
            match h.browser_selection(w) {
                None => {
                    h.effect(Effect::Message("Unknown error in file selection.".into(), 0));
                    Ok(None)
                }
                Some((kind, _)) if kind != 2 => {
                    h.effect(Effect::Message("You must select a file.".into(), 0));
                    Ok(None)
                }
                Some((_, path)) => {
                    let result = identity_from_path(&path, h);
                    if matches!(result, Ok(None)) {
                        h.effect(Effect::Message("The file you selected was somehow invalid.".into(), 0));
                    }
                    result
                }
            }
        } else {
            Ok(None)
        };
        // Browser is already gone before installing identity or calling Save.
        h.effect(Effect::Unregister(w));
        h.effect(Effect::RemoveDialog(w));
        h.effect(Effect::Release(w));
        h.effect(Effect::Focus);
        let Some(identity) = selected? else {
            return Ok(false);
        };
        let old = std::mem::replace(&mut self.identity, identity);
        self.config_mut()?.id = self.identity.id;
        let result = self.save(success, error, h);
        if result == Ok(true) {
            if !h.saved_id_present(self.identity.id) {
                h.register_configuration(self.config()?, true);
                h.effect(Effect::RegisteredConfiguration(self.identity.id));
            }
            if self.chooser {
                h.effect(Effect::RefreshChooser);
            }
        }
        if copy || result != Ok(true) {
            let selected = std::mem::replace(&mut self.identity, old);
            release_identity(&selected, h);
            self.config_mut()?.id = self.identity.id;
        } else {
            release_identity(&old, h);
        }
        // No extra title refresh after restoration, even on successful copy.
        result
    }
    pub fn close(&mut self, ask: bool, save: bool, h: &mut impl Host) -> Result<bool> {
        let mut permitted = true;
        let mut skip_save = false;
        if ask && self.dirty != 0 {
            permitted = false;
            match h.prompt("Do you want to save this lot configuration?", 4) {
                0x5301814C => permitted = true,
                0x53018149 => {
                    permitted = true;
                    skip_save = true;
                }
                0x53018147 => return Ok(false),
                _ => (),
            }
        }
        if !skip_save && save && self.dirty != 0 {
            permitted = self.save(false, true, h)?;
        }
        if permitted {
            if self.chooser {
                h.effect(Effect::NotifyChooser);
            }
            h.effect(Effect::RemoveEditor);
        }
        Ok(permitted)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Save,
    SaveAs(bool),
    Name,
    Size,
    Foundation,
    Wall,
    Compatibility,
    AddBuilding,
    AddFlora,
    AddProp(bool, bool),
    AddTexture,
    AddNetwork,
    AddWater,
    AddTerrain,
    Fences,
    Go3D,
    Refresh,
    Condition,
    Slope,
    Help(u32),
}
pub fn edit_action(row: i32) -> Option<Command> {
    Some(match row {
        0 => Command::Save,
        1 => Command::SaveAs(false),
        2 => Command::SaveAs(true),
        4 => Command::Name,
        5 => Command::Size,
        6 => Command::Foundation,
        7 => Command::Wall,
        8 => Command::Compatibility,
        10 => Command::AddBuilding,
        11 => Command::AddFlora,
        12 => Command::AddProp(true, true),
        13 => Command::AddTexture,
        14 => Command::AddNetwork,
        15 => Command::AddWater,
        16 => Command::AddTerrain,
        17 => Command::Fences,
        19 => Command::Go3D,
        20 => Command::Refresh,
        21 => Command::Condition,
        22 => Command::Slope,
        24 => Command::Help(0),
        _ => return None,
    })
}
impl Editor {
    pub fn execute(&mut self, command: Command, h: &mut impl Host) -> Result<()> {
        match command {
            Command::Save => {
                self.save(true, true, h)?;
            }
            Command::SaveAs(copy) => {
                self.save_as(copy, true, true, h)?;
            }
            Command::Name => {
                self.name_dialog(h)?;
            }
            Command::Size => {
                self.size_dialog(h)?;
            }
            Command::Compatibility => {
                self.compatibility_dialog(h)?;
            }
            Command::AddNetwork => self.placement_mode(4, h)?,
            Command::Fences => self.placement_mode(8, h)?,
            Command::Condition => self.cycle_condition(h),
            Command::Slope => {
                self.slope_dialog(h)?;
            }
            // Refresh/help/rendering/title are explicit pending effects, never
            // generic WCM guesses or a claim of completed native workflows.
            Command::Refresh => h.effect(Effect::Refresh),
            _ => return Err(Unsupported::Dependency("LE4/LE5 command body")),
        }
        Ok(())
    }
    pub fn edit_area_menu(&mut self, h: &mut impl Host) -> Result<()> {
        let labels = [
            "Save (Control-S)",
            "Save As (Control-Shift-S)",
            "Save a Copy As (Control-Alt-S)",
            "",
            "Set lot name",
            "Set lot size",
            "Set lot foundation",
            "Set retaining wall",
            "Set lot compatibilities",
            "",
            "Add building",
            "Add flora",
            "Add prop",
            "Add lot base texture",
            "Add network node",
            "Add water constraint",
            "Add terrain constraint",
            "Add/edit fences",
            "",
            "Go to 3D view (G)",
            "Refresh 3D view (R)",
            "Cycle lot condition (C)",
            "Set 3D view slope",
            "",
            "Help (F1)",
        ];
        let row = h.popup(&labels.map(str::to_owned));
        h.effect(Effect::Focus);
        if let Some(command) = edit_action(row) {
            self.execute(command, h)?;
        }
        Ok(())
    }
    pub fn object_action(&mut self, id: RecordId, row: i32, h: &mut impl Host) -> Result<()> {
        let kind = self.record(id).ok_or(Unsupported::MissingRecord)?.kind;
        let last = match kind {
            0 => 7,
            1 => 11,
            2 => 10,
            4 => 8,
            6 => return Ok(()),
            7 => 6,
            _ => return Err(Unsupported::Dependency("LE4 unknown object type popup")),
        };
        if row < 0 || row > last {
            return Ok(());
        }
        let detail = match kind {
            1 => Some(7),
            2 => Some(6),
            4 => Some(4),
            _ => None,
        };
        if let Some(first) = detail {
            if (first..=first + 2).contains(&row) {
                let o = self.record_mut(id).unwrap();
                o.flags = (o.flags & 1) + ((row - first) as u8 * 0x10);
                return Ok(());
            }
        }
        match (kind, row) {
            (6, _) => (),
            (1, 3) => self.record_mut(id).unwrap().flags ^= 1,
            (0, 2) => {
                if self.record(id).unwrap().foundation != 0 {
                    self.record_mut(id).unwrap().foundation = 0;
                    self.set_dirty(1, h);
                    h.effect(Effect::RedrawRequired);
                }
            }
            (0, 4) | (1, 4) | (2, 3) | (4, 1) => {
                self.duplicate(id, h)?;
            }
            (0, 5) | (1, 5) | (2, 4) | (4, 2) | (7, 0) => {
                self.delete(id, true, h)?;
            }
            (2, 2) => self.mirror(id, h)?,
            (0, 3) | (1, 1) | (2, 1) => h.effect(Effect::Instructions("Use Page Up and Page Down to rotate")),
            (1, 2) => h.effect(Effect::Instructions(
                "Hold Space while pressing Up/Down to change elevation",
            )),
            (7, 4) => {
                self.network_path_dialog(id, h)?;
            }
            (0 | 1, 6) | (1, 10) | (2, 5 | 9) | (4, 3 | 7) | (7, 5) => (),
            _ => {
                return Err(Unsupported::Dependency(
                    "LE4 object popup selector/edge table/help or unknown row",
                ))
            }
        }
        Ok(())
    }
    pub fn network_type(&mut self, id: RecordId, row: i32) -> Result<()> {
        if (0..13).contains(&row) {
            let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
            *o.ids
                .first_mut()
                .ok_or(Unsupported::Dependency("LE3 missing network payload"))? = row as u32;
        }
        Ok(())
    }
    pub fn fence_menu(&mut self, h: &mut impl Host) -> Result<()> {
        let row = h.popup(&["Set Fence Type...".into(), String::new(), "Help... (F1)".into()]);
        h.effect(Effect::Focus);
        match row {
            0 => {
                h.message_box(
                "Sorry, the fence options menu isn't implemented yet and won't be implemented until fence functionality is in place.",
                self.current.as_ref().map_or("", |c| c.name.as_str()), (0, 1, 0));
            }
            2 => return Err(Unsupported::Dependency("LE5 help page 0")),
            _ => (),
        }
        Ok(())
    }
    pub fn key_down(&mut self, key: u32, modifiers: u32, space: bool, h: &mut impl Host) -> Result<bool> {
        if self.capturing {
            return Ok(true);
        }
        self.drag = false;
        match key {
            0x1B => {
                self.mode = 0;
                h.effect(Effect::Cursor(0));
            }
            0x43 => self.cycle_condition(h),
            0x53 => {
                if modifiers & 2 != 0 {
                    let command = if modifiers & 1 != 0 {
                        Command::SaveAs(false)
                    } else if modifiers & 4 != 0 {
                        Command::SaveAs(true)
                    } else {
                        Command::Save
                    };
                    self.execute(command, h)?;
                }
            }
            0x42 => self.execute(Command::AddBuilding, h)?,
            0x54 => self.execute(Command::AddTexture, h)?,
            0x50 => {
                let args = if modifiers & 1 != 0 {
                    (false, true)
                } else if modifiers & 2 != 0 {
                    (true, false)
                } else {
                    (true, true)
                };
                self.execute(Command::AddProp(args.0, args.1), h)?;
            }
            0x46 => self.execute(
                if modifiers & 3 != 0 {
                    Command::Fences
                } else {
                    Command::AddFlora
                },
                h,
            )?,
            0x47 => self.execute(Command::Go3D, h)?,
            0x52 => self.execute(Command::Refresh, h)?,
            0x70 => self.execute(Command::Help(0), h)?,
            _ => {
                if let Some(id) = self.selected {
                    match key {
                        0x25..=0x28 => self.move_key(id, key, modifiers, space, h)?,
                        0x21 | 0x22 => self.rotate(id, h)?,
                        0x4D => self.mirror(id, h)?,
                        0x44 if modifiers & 2 != 0 => {
                            self.duplicate(id, h)?;
                        }
                        0x2E | 0x08 => {
                            self.delete(id, true, h)?;
                        }
                        0x24 => {
                            let o = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
                            if o.offset[1] != 0.0 {
                                o.offset[1] = 0.0;
                                self.set_dirty(1, h);
                                h.effect(Effect::RedrawRequired);
                            }
                        }
                        0x4E => return Err(Unsupported::Dependency("LE5 CycleSelectedObject")),
                        _ => (),
                    }
                }
            }
        }
        Ok(true)
    }
}
