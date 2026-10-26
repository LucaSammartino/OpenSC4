// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4WinSnapshotAlbum control composition, snapshots.md sections 3.1-3.3.
//! Reuses the base window tree, ListBox and TextEdit byte-stage owners. UI-script
//! resolution and the cWinBMPView/renderer are foreign, not album algorithms.

use crate::ui_windows::base::{Host, Window, Windows};
use crate::widget_controls::list_box::ListBox;
use crate::widget_controls::text_edit::{TextEdit, TextForeign};
use crate::widget_controls::Foreign;

use super::album_picture::{AlbumPicture, PictureHost};
use super::{
    AlbumAction, CaptionFields, CityContext, Error, FileSystem, Gap, Result, SnapshotAlbum, UiPrimitives, VPath, Zoom,
};

/// Already resolved, borrowed UI-script controls. Native control IDs are SN7.
/// The adapter retains ownership and executes TextEdit's pending layout repairs.
#[derive(Debug)]
pub struct Controls {
    pub albums: ListBox,
    pub description: TextEdit,
    pub picture: AlbumPicture,
    pub next: Window,
    pub previous: Window,
    pub count: Window,
}

/// Only image loading and presentation are foreign. The album owner supplies
/// the selected path and the five raw caption inputs; SN2 formatting is absent.
pub trait AlbumHost: Host + Foreign + TextForeign + PictureHost + UiPrimitives {
    fn load_picture(&mut self, window: Window, path: Option<&VPath>) -> Result<()>;
    fn information_fields(&mut self, window: Window, fields: Option<&CaptionFields>);
}

#[derive(Debug)]
pub struct AlbumWindow {
    window: Window,
    store: SnapshotAlbum,
    controls: Controls,
    initialized: bool,
}

impl AlbumWindow {
    /// Bind a window built by the UI-script adapter, rather than constructing a
    /// duplicate native control tree. All handles must belong to `windows`.
    pub fn bind(windows: &Windows, window: Window, controls: Controls, store: SnapshotAlbum) -> Result<Self> {
        let handles = [
            window,
            controls.albums.window,
            controls.description.window,
            controls.picture.window,
            controls.next,
            controls.previous,
            controls.count,
        ];
        if handles.iter().any(|&handle| !windows.valid(handle)) {
            return Err(Error::InvalidState);
        }
        Ok(Self {
            window,
            controls,
            store,
            initialized: false,
        })
    }

    pub fn store(&self) -> &SnapshotAlbum {
        &self.store
    }

    pub fn controls(&self) -> &Controls {
        &self.controls
    }

    /// Return borrowed controls to their adapter for its explicit widget shutdown.
    /// The caller must finish the album modal lifecycle before reclaiming them.
    pub fn into_controls(self) -> Controls {
        self.controls
    }

    /// UI adapters drive the existing text owner, including its layout repairs.
    pub fn description_control(&mut self) -> &mut TextEdit {
        &mut self.controls.description
    }

    pub fn on_init(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
    ) -> Result<()> {
        if self.initialized {
            return Err(Error::InvalidState);
        }
        self.store.refresh_albums(fs)?;
        self.controls.picture.init(host)?;
        windows.get_mut(self.window).id = SnapshotAlbum::WINDOW_ID;
        self.sync(windows, host)?;
        self.initialized = true;
        Ok(())
    }

    fn require_init(&self) -> Result<()> {
        if self.initialized {
            Ok(())
        } else {
            Err(Error::InvalidState)
        }
    }

    fn save_edit(&mut self, fs: &mut impl FileSystem) -> Result<()> {
        if self.controls.description.text != self.store.description() {
            self.store.edit_description(self.controls.description.text.clone());
        }
        self.store.save_description(fs)
    }

    fn sync(&mut self, windows: &mut Windows, host: &mut impl AlbumHost) -> Result<()> {
        let path = if self.store.current_picture().is_some() {
            Some(self.store.current_path()?)
        } else {
            None
        };
        host.load_picture(self.controls.picture.window, path.as_ref())?;
        let fields = self.store.information().map(|info| info.caption_fields());
        host.information_fields(self.controls.picture.window, fields.as_ref());
        self.controls.albums.clear(host);
        for (index, name) in self.store.albums().iter().enumerate() {
            self.controls
                .albums
                .insert(-2, name.as_bytes().to_vec(), index as u32, host);
        }
        self.controls
            .albums
            .select(self.store.album_index().map_or(-1, |i| i as i32), false, host);
        let text = self.store.description();
        let count = u32::try_from(text.len()).map_err(|_| Error::Gap(Gap::TextEncoding))?;
        self.controls
            .description
            .set_text_bytes(Some(text), count, false, host)?;
        // SPEC-GAP: SN4 is open; no picture uses empty count text in the portable UI.
        windows.get_mut(self.controls.count).caption = self.store.count_text().unwrap_or_default();
        Ok(())
    }

    pub fn show_current_city(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
        city: &CityContext,
        create: bool,
    ) -> Result<isize> {
        self.require_init()?;
        if self.store.is_modal() {
            return Err(Error::InvalidState);
        }
        self.save_edit(fs)?;
        let index = self.store.go_to_city(fs, city, create)?;
        self.sync(windows, host)?;
        self.store.begin_modal(host)?;
        windows.show(host, self.window);
        Ok(index)
    }

    #[allow(clippy::too_many_arguments)] // Published name/number/create operation plus the three host boundaries.
    pub fn go_to_album(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
        name: &str,
        number: u32,
        create: bool,
    ) -> Result<isize> {
        self.require_init()?;
        self.save_edit(fs)?;
        let index = self.store.go_to_album(fs, name, number, create)?;
        self.sync(windows, host)?;
        Ok(index)
    }

    pub fn set_current_file(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
        name: &str,
    ) -> Result<()> {
        self.require_init()?;
        self.save_edit(fs)?;
        self.store.set_current_file(fs, name)?;
        self.sync(windows, host)
    }

    /// Typed semantic routing; raw button/message identities remain SN7.
    pub fn action(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
        action: AlbumAction,
    ) -> Result<()> {
        self.require_init()?;
        if let AlbumAction::Zoom(direction) = action {
            // The album's zoom buttons select large/small, not a numeric zoom factor.
            self.controls.picture.set_size(windows, host, direction == Zoom::In);
            return Ok(());
        }
        if action != AlbumAction::DeletePicture {
            self.save_edit(fs)?;
        }
        let closing = action == AlbumAction::Close;
        let saving = action == AlbumAction::SaveDescription;
        self.store.action(fs, host, action)?;
        if closing {
            windows.hide(host, self.window);
            Ok(())
        } else if saving {
            Ok(())
        } else {
            self.sync(windows, host)
        }
    }

    pub fn on_shutdown(
        &mut self,
        fs: &mut impl FileSystem,
        windows: &mut Windows,
        host: &mut impl AlbumHost,
    ) -> Result<()> {
        self.require_init()?;
        self.save_edit(fs)?;
        self.store.on_shutdown(fs, host)?;
        windows.hide(host, self.window);
        self.sync(windows, host)?;
        self.initialized = false;
        Ok(())
    }

    pub fn do_win_message(&mut self, _message: [u32; 5]) -> Result<()> {
        // SPEC-GAP: SN7 is open; native button/control/message IDs are Unsupported.
        Err(Error::Gap(Gap::NativeToolkit))
    }
}

#[cfg(test)]
mod tests;
