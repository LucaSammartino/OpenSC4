// SPDX-License-Identifier: GPL-3.0-or-later
//! Vehicle owners from vehicles.md and automata.md. See README.md for closure gaps.
//! City services remain foreign interfaces; no game vehicle implements these owners.

mod traffic;
mod train;
mod vehicle;
mod init;
mod states;
mod effects;
mod routing;

pub use traffic::*;
pub use train::*;
pub use vehicle::*;
pub use init::*;
pub use effects::*;
pub use routing::*;

#[cfg(test)]
mod tests;

use crate::automata::Vec3;

/// A missing specification, never a request to run the original owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    PlayerDrivingVh1,
    TrainCompositionVh5,
    TrainFollowingVh5,
    TrainTripVh5,
    CollisionDirectionVh6,
    RouteGeometryVh7,
    BaseStateVh9,
}

pub type Result<T> = std::result::Result<T, Unsupported>;

pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(crate) fn length(a: Vec3) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
pub(crate) fn unit(a: Vec3) -> Vec3 {
    scale(a, 1.0 / length(a))
}

/// vehicles.md §2/automata-managers.md §5 slot 155.
pub fn direction_toward(from: Vec3, to: Vec3) -> Vec3 {
    let delta = sub(to, from);
    if length(delta) < 0.00001 {
        [0.0, 0.0, 1.0]
    } else {
        unit(delta)
    }
}
