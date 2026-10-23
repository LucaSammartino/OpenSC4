// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned cameras, written from `docs/specs/camera.md` §§1–2, 4, 6.
//!
//! No original camera/control methods are host callbacks. Foreign services
//! supply terrain queries and synchronous messages. Transform storage reuses Xform.
//! Getters take `&self`, with no interior mutability or lazy RefCell caches.
//! See `README.md` beside this module for the complete method inventory.

mod control;
pub(crate) mod geometry;
mod s3d;

pub use control::{
    CameraControl, CameraControlAllocation, CameraHandle, CameraViewUtilities, LiveViewMessage, TerrainView,
    ViewMessage, ViewMessageData, ViewMessageKind, ViewMessageServer, ZoomRotation, CAMERA_CONTROL_CLASS_ID,
};
pub use geometry::{intersect_with_ground_plane, Mat4, Vec2, Vec3};
pub use s3d::{Ray, S3dCamera, ViewVolume, Viewport, CAMERA_CLASS_ID};

#[cfg(test)]
mod tests;
