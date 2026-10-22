// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable snapshot album/taker owners from `docs/specs/snapshots.md` §§1-4/6.
//!
//! File, clock, image and UI services are typed primitives, never native snapshot
//! owner callbacks. All paths are virtual; no OS filesystem adapter is supplied.
//! See OWNER_INVENTORY.md for the complete published surface and residuals.
//! Provisional choices are labelled SN3-SN9; this is not a native/full-owner claim.

mod album;
mod info;
mod primitives;
mod taker;

pub use album::{album_directory_name, AlbumAction, SnapshotAlbum};
pub use info::{CaptionFields, CityContext, GraphicInfo, Information, ViewPosition};
pub use primitives::{
    DateText, Entry, EntryKind, FileSystem, ImageDepth, ImagePrimitives, MemoryFs, MemoryUi, SnapshotClock,
    UiPrimitives, VPath, Zoom,
};
pub use taker::{
    capture_target, CaptureDelivery, CaptureTarget, Key, KeyResult, Modifiers, Rect, Size, SnapshotTaker,
    CAPTURE_LUA_EVENT, CAPTURE_MESSAGE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    FilenameSanitizing,
    CollisionNaming,
    RegionOnlyNaming,
    MalformedInformation,
    EmbeddedLineFeed,
    OversizedFrame,
    NestedAlbumDirectory,
    NativeToolkit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Gap(Gap),
    Io(crate::rz_files::OsError),
    InvalidPath,
    InvalidIndex,
    InvalidSize,
    NoAlbum,
    NoPicture,
    MissingFile,
    InvalidState,
    Image,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests;
