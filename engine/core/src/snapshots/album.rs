// SPDX-License-Identifier: GPL-3.0-or-later
use super::primitives::valid_component;
use super::{CityContext, EntryKind, Error, FileSystem, Gap, GraphicInfo, Information, Result, UiPrimitives, VPath, Zoom};

/// SN3: never guess SC4SanitizeFileName's substitutions. Already-valid UTF-8
/// components are accepted; unsupported/reserved names report the missing contract.
fn safe_name(name: &str) -> Result<&str> {
    let reserved = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if !valid_component(name)
        || name.chars().any(|c| c.is_control() || "<>\"|?*".contains(c))
        || name.ends_with([' ', '.'])
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(Error::Gap(Gap::FilenameSanitizing));
    }
    Ok(name)
}

fn city_prefix(name: &str) -> Result<&str> {
    safe_name(name)?;
    let mut end = name.len().min(48);
    while !name.is_char_boundary(end) { end -= 1; }
    Ok(&name[..end])
}

pub fn album_directory_name(city: &str, number: u32) -> Result<String> {
    Ok(format!("{}--{number}", city_prefix(city)?))
}

fn recognized_image(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| {
        ["png", "jpg", "jpeg", "jp2", "gif", "bmp"].iter().any(|e| ext.eq_ignore_ascii_case(e))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlbumAction {
    Next,
    Previous,
    SaveDescription,
    DeletePicture,
    SelectAlbum(usize),
    Zoom(Zoom),
    Close,
}

#[derive(Clone, Debug, Default)]
struct Loaded {
    description: Vec<u8>,
    information: Option<Information>,
}

/// Owns album directories, sorted picture names, both selected indices, description
/// edit state and decoded information. Indices never come back from the host.
#[derive(Clone, Debug)]
pub struct SnapshotAlbum {
    root: VPath,
    albums: Vec<String>,
    album_index: Option<usize>,
    pictures: Vec<String>,
    picture_index: Option<usize>,
    loaded: Loaded,
    description_dirty: bool,
    modal_previous_pause: Option<bool>,
}

impl SnapshotAlbum {
    pub const WINDOW_ID: u32 = 0x0a8c_d3ee;

    pub fn new(root: VPath) -> Self {
        Self { root, albums: vec![], album_index: None, pictures: vec![], picture_index: None,
            loaded: Loaded::default(), description_dirty: false, modal_previous_pause: None }
    }

    pub fn root(&self) -> &VPath { &self.root }
    pub fn albums(&self) -> &[String] { &self.albums }
    pub fn pictures(&self) -> &[String] { &self.pictures }
    pub fn album_index(&self) -> Option<usize> { self.album_index }
    pub fn picture_index(&self) -> Option<usize> { self.picture_index }
    pub fn current_album(&self) -> Option<&str> { self.album_index.map(|i| self.albums[i].as_str()) }
    pub fn current_picture(&self) -> Option<&str> { self.picture_index.map(|i| self.pictures[i].as_str()) }
    pub fn information(&self) -> Option<&Information> { self.loaded.information.as_ref() }
    pub fn description(&self) -> &[u8] { &self.loaded.description }
    pub fn description_dirty(&self) -> bool { self.description_dirty }
    pub fn is_modal(&self) -> bool { self.modal_previous_pause.is_some() }

    pub fn album_directory_index(&self, name: &str) -> isize {
        self.albums.iter().position(|s| s == name).map_or(-1, |i| i as isize)
    }

    pub fn count_text(&self) -> Option<String> {
        self.picture_index.map(|i| format!("({}/{})", i + 1, self.pictures.len()))
    }

    pub fn current_path(&self) -> Result<VPath> {
        self.root.join(self.current_album().ok_or(Error::NoAlbum)?)?
            .join(self.current_picture().ok_or(Error::NoPicture)?)
    }

    pub fn album_exists(&self, fs: &mut impl FileSystem, directory: &str) -> Result<bool> {
        self.root.join(directory)?;
        Ok(fs.entries(&self.root)?.iter().any(|e| e.kind == EntryKind::Directory && e.name == directory))
    }

    pub fn create_album(&mut self, fs: &mut impl FileSystem, directory: &str) -> Result<()> {
        let path = self.root.join(directory)?;
        if !self.album_exists(fs, directory)? {
            fs.create_dir(&path)?;
            fs.write(&path.join("album.ini")?, b"")?;
        }
        self.refresh_albums(fs)
    }

    pub fn refresh_albums(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        let old = self.current_album().map(str::to_owned);
        let mut albums = vec![];
        for entry in fs.entries(&self.root)? {
            if entry.kind != EntryKind::Directory { continue; }
            let path = self.root.join(&entry.name)?;
            // §1.1: JPEG-only folders are not albums despite JPEG being a recognized picture.
            if fs.entries(&path)?.iter().any(|e| e.kind == EntryKind::File
                && (e.name == "album.ini" || e.name.rsplit_once('.').is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("png")))) {
                albums.push(entry.name);
            }
        }
        // SN4: literal "sorted" means bytewise order until native collation is published.
        albums.sort();
        albums.dedup();
        self.album_index = old.and_then(|name| albums.iter().position(|s| *s == name));
        self.albums = albums;
        if self.album_index.is_none() { self.clear_pictures(); }
        Ok(())
    }

    fn list_pictures(fs: &mut impl FileSystem, folder: &VPath) -> Result<Vec<String>> {
        let mut pictures = vec![];
        for entry in fs.entries(folder)? {
            folder.join(&entry.name)?;
            if entry.kind == EntryKind::File && recognized_image(&entry.name) { pictures.push(entry.name); }
        }
        // SN4: picture order is unspecified; bytewise sorting is the portable literal policy.
        pictures.sort();
        pictures.dedup();
        Ok(pictures)
    }

    fn read_picture(fs: &mut impl FileSystem, path: &VPath) -> Result<Loaded> {
        if fs.read(path)?.is_none() { return Err(Error::MissingFile); }
        let info = fs.read(&path.companion("info")?)?;
        let information = info.as_deref().map(Information::decode).transpose()?.flatten();
        let description = fs.read(&path.companion("txt")?)?.unwrap_or_default();
        Ok(Loaded { description, information })
    }

    pub fn refresh_pictures(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        self.save_description(fs)?;
        let folder = self.root.join(self.current_album().ok_or(Error::NoAlbum)?)?;
        let old = self.current_picture().map(str::to_owned);
        let pictures = Self::list_pictures(fs, &folder)?;
        let index = old.and_then(|name| pictures.iter().position(|s| *s == name));
        let loaded = match index { Some(i) => Self::read_picture(fs, &folder.join(&pictures[i])?)?, None => Loaded::default() };
        self.pictures = pictures;
        self.picture_index = index;
        self.loaded = loaded;
        self.description_dirty = false;
        Ok(())
    }

    pub fn go_to_album_index(&mut self, fs: &mut impl FileSystem, index: usize) -> Result<()> {
        let name = self.albums.get(index).ok_or(Error::InvalidIndex)?;
        let folder = self.root.join(name)?;
        // SN4: save before changing albums; errors leave the edit and selection in place.
        self.save_description(fs)?;
        let pictures = Self::list_pictures(fs, &folder)?;
        let loaded = if let Some(first) = pictures.first() { Self::read_picture(fs, &folder.join(first)?)? } else { Loaded::default() };
        self.album_index = Some(index);
        self.picture_index = (!pictures.is_empty()).then_some(0);
        self.pictures = pictures;
        self.loaded = loaded;
        self.description_dirty = false;
        Ok(())
    }

    pub fn go_to_album(&mut self, fs: &mut impl FileSystem, name: &str, number: u32, create: bool) -> Result<isize> {
        let directory = album_directory_name(name, number)?;
        if create && !self.album_exists(fs, &directory)? { self.create_album(fs, &directory)?; }
        self.refresh_albums(fs)?;
        let index = self.album_directory_index(&directory);
        if index >= 0 { self.go_to_album_index(fs, index as usize)?; }
        Ok(index)
    }

    pub fn go_to_city(&mut self, fs: &mut impl FileSystem, city: &CityContext, create: bool) -> Result<isize> {
        self.go_to_album(fs, &city.name, city.number, create)
    }

    pub fn set_initial_file(&mut self, fs: &mut impl FileSystem, name: Option<&str>) -> Result<()> {
        // SN4: absent initial filename selects the first, with an explicit no-image state if empty.
        if let Some(name) = name { return self.set_current_file(fs, name); }
        if self.pictures.is_empty() { self.clear_pictures(); return Ok(()); }
        self.go_to_graphic(fs, 0)
    }

    pub fn set_current_file(&mut self, fs: &mut impl FileSystem, name: &str) -> Result<()> {
        let index = self.pictures.iter().position(|s| s == name).ok_or(Error::InvalidIndex)?;
        self.go_to_graphic(fs, index)
    }

    pub fn go_to_graphic(&mut self, fs: &mut impl FileSystem, index: usize) -> Result<()> {
        let name = self.pictures.get(index).ok_or(Error::InvalidIndex)?;
        let path = self.root.join(self.current_album().ok_or(Error::NoAlbum)?)?.join(name)?;
        self.save_description(fs)?;
        let loaded = Self::read_picture(fs, &path)?;
        self.picture_index = Some(index);
        self.loaded = loaded;
        self.description_dirty = false;
        Ok(())
    }

    pub fn edit_description(&mut self, description: Vec<u8>) {
        if self.picture_index.is_some() {
            self.loaded.description = description;
            self.description_dirty = true;
        }
    }

    pub fn save_description(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        if self.picture_index.is_none() || !self.description_dirty { return Ok(()); }
        let file = self.current_path()?.companion("txt")?;
        Self::write_description(fs, &file, &self.loaded.description)?;
        self.description_dirty = false;
        Ok(())
    }

    fn write_description(fs: &mut impl FileSystem, file: &VPath, description: &[u8]) -> Result<()> {
        if description.is_empty() { fs.remove_file(file) } else { fs.write(file, description) }
    }

    pub fn next(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        self.save_description(fs)?;
        if let Some(index) = self.picture_index {
            // SN4: no wrap is specified for album navigation; clamp at the endpoint.
            self.go_to_graphic(fs, (index + 1).min(self.pictures.len() - 1))?;
        }
        Ok(())
    }

    pub fn previous(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        self.save_description(fs)?;
        if let Some(index) = self.picture_index { self.go_to_graphic(fs, index.saturating_sub(1))?; }
        Ok(())
    }

    pub fn delete_current(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        let index = self.picture_index.ok_or(Error::NoPicture)?;
        let path = self.current_path()?;
        fs.remove_file(&path)?;
        fs.remove_file(&path.companion("info")?)?;
        fs.remove_file(&path.companion("txt")?)?;
        self.description_dirty = false;
        self.picture_index = None;
        self.loaded = Loaded::default();
        self.refresh_pictures(fs)?;
        // SN4: choose the successor at the same index, or the last survivor.
        if !self.pictures.is_empty() { self.go_to_graphic(fs, index.min(self.pictures.len() - 1))?; }
        Ok(())
    }

    pub fn delete_album(&mut self, fs: &mut impl FileSystem, directory: &str) -> Result<()> {
        let folder = self.root.join(directory)?;
        let entries = fs.entries(&folder)?;
        // SN9: spec says every file, not recursive directories. Refuse before deleting anything.
        if entries.iter().any(|e| e.kind == EntryKind::Directory) { return Err(Error::Gap(Gap::NestedAlbumDirectory)); }
        for entry in entries { fs.remove_file(&folder.join(&entry.name)?)?; }
        fs.remove_dir(&folder)?;
        self.refresh_albums(fs)
    }

    pub fn new_picture_path(&self, fs: &mut impl FileSystem, city: &CityContext) -> Result<VPath> {
        let directory = album_directory_name(&city.name, city.number)?;
        let base = format!("{}-{}.png", city_prefix(&city.name)?, safe_name(&city.info.date)?);
        let folder = self.root.join(&directory)?;
        let path = folder.join(&base)?;
        // SN3: collision suffix/search order is unpublished; never overwrite or invent one.
        if fs.entries(&folder)?.iter().any(|e| e.name == base) { return Err(Error::Gap(Gap::CollisionNaming)); }
        Ok(path)
    }

    pub fn new_region_picture_path(&self) -> Result<VPath> { Err(Error::Gap(Gap::RegionOnlyNaming)) }

    pub fn add_new_graphic(&mut self, fs: &mut impl FileSystem, source: &VPath, info: &GraphicInfo,
        description: &[u8], city: &CityContext, move_source: bool) -> Result<VPath> {
        let encoded = info.encode()?;
        self.save_description(fs)?;
        let directory = album_directory_name(&city.name, city.number)?;
        self.create_album(fs, &directory)?;
        let folder = self.root.join(&directory)?;
        let destination = if source.parent().as_ref() == Some(&folder) {
            if fs.read(source)?.is_none() { return Err(Error::MissingFile); }
            source.clone()
        } else {
            let destination = self.new_picture_path(fs, city)?;
            let bytes = fs.read(source)?.ok_or(Error::MissingFile)?;
            fs.write(&destination, &bytes)?;
            // §3.3 explicitly orders removal after successful copy. SN9: no rollback invented.
            if move_source { fs.remove_file(source)?; }
            destination
        };
        fs.write(&destination.companion("info")?, &encoded)?;
        Self::write_description(fs, &destination.companion("txt")?, description)?;
        // Save an existing edit before rebuilding the target album's selection.
        self.go_to_city(fs, city, false)?;
        self.set_current_file(fs, destination.file_name().ok_or(Error::InvalidPath)?)?;
        Ok(destination)
    }

    pub fn add_new_graphic_for_current_city(&mut self, fs: &mut impl FileSystem, source: &VPath,
        description: &[u8], city: &CityContext, move_source: bool) -> Result<VPath> {
        self.add_new_graphic(fs, source, &city.info, description, city, move_source)
    }

    pub fn begin_modal(&mut self, ui: &mut impl UiPrimitives) -> Result<()> {
        if self.is_modal() { return Err(Error::InvalidState); }
        self.modal_previous_pause = Some(ui.paused());
        ui.set_paused(true);
        ui.set_album_visible(true);
        Ok(())
    }

    pub fn end_dialog(&mut self, fs: &mut impl FileSystem, ui: &mut impl UiPrimitives) -> Result<()> {
        self.save_description(fs)?;
        if let Some(previous) = self.modal_previous_pause.take() { ui.set_paused(previous); }
        ui.set_album_visible(false);
        Ok(())
    }

    pub fn on_shutdown(&mut self, fs: &mut impl FileSystem, ui: &mut impl UiPrimitives) -> Result<()> {
        self.end_dialog(fs, ui)?;
        self.albums.clear();
        self.album_index = None;
        self.clear_pictures();
        Ok(())
    }

    pub fn action(&mut self, fs: &mut impl FileSystem, ui: &mut impl UiPrimitives, action: AlbumAction) -> Result<()> {
        match action {
            AlbumAction::Next => self.next(fs),
            AlbumAction::Previous => self.previous(fs),
            AlbumAction::SaveDescription => self.save_description(fs),
            AlbumAction::DeletePicture => self.delete_current(fs),
            AlbumAction::SelectAlbum(index) => self.go_to_album_index(fs, index),
            AlbumAction::Zoom(direction) => { ui.zoom_picture(direction); Ok(()) }
            AlbumAction::Close => self.end_dialog(fs, ui),
        }
    }

    fn clear_pictures(&mut self) {
        self.pictures.clear();
        self.picture_index = None;
        self.loaded = Loaded::default();
        self.description_dirty = false;
    }
}
