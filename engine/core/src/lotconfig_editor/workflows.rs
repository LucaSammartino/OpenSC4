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
fn split_path(path: &str) -> (&str, &str, &str) {
    // Literal backslash paths only. Non-Windows syntax is not guessed (LE6).
    let (directory, file) = path.rsplit_once('\\').unwrap_or(("", path));
    let (stem, ext) = file.rsplit_once('.').unwrap_or((file, ""));
    (directory, stem, ext)
}
pub fn identity_from_path(path: &str, h: &mut impl Host) -> Result<Option<Identity>> {
    if path.contains('/') {
        return Err(Unsupported::Dependency("LE6 non-Windows path splitting"));
    }
    let (directory, stem, ext) = split_path(path);
    if !h.directory_exists(directory) {
        return Ok(None);
    }
    let providers = h.providers()?;
    let mut result = None;
    let mut unsupported = false;
    for provider in providers {
        if result.is_none() && !unsupported {
            if let Some(interface) = h.query_provider(provider) {
                let supplied = h.provider_directory(interface);
                // Only byte-oriented ASCII lowercasing is supported here. LE6
                // tracks non-ASCII native lowercasing and path splitter details.
                if !directory.is_ascii() || !supplied.is_ascii() {
                    unsupported = true;
                }
                if !unsupported && supplied.eq_ignore_ascii_case(directory) {
                    if let Some(key) = h.provider_key(interface, stem, ext) {
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
        Err(Unsupported::Dependency("LE6 non-ASCII directory comparison"))
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
        self.current = Some(Configuration::default());
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
        // The existing LotConfigManager::fix requires a configuration-slot
        // adapter. Never let a host supply a precomputed editor reconciliation.
        Err(Unsupported::Dependency("LotConfigManager::fix configuration adapter"))
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
                return Ok(prompt && h.prompt("Continue with objects outside lot bounds?", 0) == 0x5301814A);
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
            split_path(&self.identity.path).0.to_string()
        };
        let Some(w) = h.create_dialog(DialogKind::Browser) else {
            return Err(Unsupported::AllocationFault);
        };
        h.effect(Effect::AddRef(w));
        h.effect(Effect::Seed(w, 0, Value::Text(initial)));
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
                    h.effect(Effect::Draw);
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
            0 => h.effect(Effect::Message(
                "Fence options are unimplemented until fence functionality is in place".into(),
                0,
            )),
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
                                h.effect(Effect::Draw);
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
