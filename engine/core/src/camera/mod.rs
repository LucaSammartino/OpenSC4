// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned cameras, written from `docs/specs/camera.md` §§1–2, 4, 6.
//!
//! No original camera/control methods are host callbacks. The sole foreign
//! query is the terrain ray cast. Transform storage reuses the existing Xform.
//! Getters take `&self`, with no interior mutability or lazy RefCell caches.
//! See `README.md` beside this module for the complete method inventory.

mod control;
mod geometry;
mod s3d;

pub use control::{
    CameraControl, CameraHandle, TerrainView, ViewMessage, ViewMessageKind, ZoomRotation, CAMERA_CONTROL_CLASS_ID,
};
pub use geometry::{intersect_with_ground_plane, Mat4, Vec2, Vec3};
pub use s3d::{Ray, S3dCamera, ViewVolume, Viewport, CAMERA_CLASS_ID};

#[cfg(test)]
mod tests;
