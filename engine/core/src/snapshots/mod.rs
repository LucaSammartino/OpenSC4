// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable snapshot album/taker owners from `docs/specs/snapshots.md` §§1-4/6.
//!
//! File, clock, image and UI services are typed primitives, never native snapshot
//! owner callbacks. All paths are virtual; no OS filesystem adapter is supplied.
//! See OWNER_INVENTORY.md for the complete published surface and residuals.
//! Provisional choices are labelled SN3-SN9; this is not a native/full-owner claim.

pub mod album;
mod info;
mod primitives;
pub mod taker;

pub use album::{album_directory_name, AlbumAction, SnapshotAlbum};
pub use info::{CaptionFields, CityContext, GraphicInfo, Information, ViewPosition};
pub use primitives::{
    DateText, Entry, EntryKind, FileSystem, ImageDepth, ImagePrimitives, MemoryFs, MemoryUi, SnapshotClock, UiEvent,
    UiPrimitives, VPath, Zoom,
};
pub use taker::{
    capture_target, CaptureDelivery, CaptureTarget, Key, KeyResult, Modifiers, Rect, Size, SnapshotTaker,
    CAPTURE_LUA_EVENT, CAPTURE_MESSAGE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    FilenameSanitizing,
    CollisionNaming,
    RegionOnlyNaming,
    MalformedInformation,
    EmbeddedLineFeed,
    OversizedFrame,
    NestedAlbumDirectory,
    NativeToolkit,
    AlbumNumberMeaning,
    NinthInformationField,
    PercentageMeaning,
    CaptionLayout,
    AlbumOrdering,
    PictureSelection,
    TextEncoding,
    PixelFormat,
    FileFailurePolicy,
}

/// Compatibility name for existing callers of the portable workflow.
pub type Gap = Unsupported;

impl Unsupported {
    /// Open specification question that prevents a native-exact implementation.
    pub const fn question(self) -> &'static str {
        match self {
            Self::AlbumNumberMeaning | Self::NinthInformationField | Self::PercentageMeaning => "SN1",
            Self::CaptionLayout => "SN2",
            Self::FilenameSanitizing | Self::CollisionNaming | Self::RegionOnlyNaming => "SN3",
            Self::AlbumOrdering | Self::PictureSelection => "SN4",
            Self::MalformedInformation | Self::EmbeddedLineFeed | Self::TextEncoding => "SN5",
            Self::OversizedFrame => "SN6",
            Self::NativeToolkit => "SN7",
            Self::PixelFormat => "SN8",
            Self::NestedAlbumDirectory | Self::FileFailurePolicy => "SN9",
        }
    }
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
