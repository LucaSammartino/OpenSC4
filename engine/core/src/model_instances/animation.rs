// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// cS3DModelInstanceAnimated. The unpublished/never-set subrange table is
/// intentionally absent (graphics-formats §3.9); SetAnimation is therefore inert.
pub struct AnimatedInstance {
    pub instance: StaticInstance,
    frame: u16,
    mode: u8,
    direction: i8,
    speed: f32,
    accumulator: f32,
}
impl AnimatedInstance {
    /// Native constructor state, before the resource supplies frame zero.
    pub fn unbound(attributes: DrawAttributes) -> Self {
        Self {
            instance: StaticInstance::new(attributes),
            frame: 0,
            mode: 3,
            direction: 1,
            speed: -100.,
            accumulator: 0.,
        }
    }
    /// Portable bind-and-build convenience, separate from construction.
    pub fn new(model: &ModelReference, attributes: DrawAttributes) -> Option<Self> {
        let mut result = Self::unbound(attributes);
        if !result.instance.set_model(Some(model)) {
            return None;
        }
        Some(result)
    }
    pub fn frame(&self) -> u16 {
        self.frame
    }
    pub fn frame_count(&self) -> u16 {
        self.animation_header().map_or(0, |h| h.frame_count)
    }
    pub fn animation_header(&self) -> Option<AnimationHeader> {
        self.instance.animation_header()
    }
    pub fn mode(&self) -> u8 {
        self.mode
    }
    pub fn direction(&self) -> i8 {
        self.direction
    }
    pub fn speed(&self) -> f32 {
        self.speed
    }
    pub fn accumulator(&self) -> f32 {
        self.accumulator
    }
    pub fn set_animation(&mut self, _index: u32) -> u32 {
        0
    }
    pub fn set_frame(&mut self, frame: u16) -> u32 {
        if self.build_frame(frame) {
            2
        } else {
            0
        }
    }
    /// Same-frame success bypasses the provider. Failed builds preserve the
    /// frame word; range validation belongs to GetModelFrame (§3.12).
    pub fn build_frame(&mut self, frame: u16) -> bool {
        if frame == self.frame {
            return true;
        }
        if !self.instance.build_frame(frame) {
            return false;
        }
        self.frame = frame;
        true
    }
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = if speed == -200. {
            f32::from(self.animation_header().map_or(0, |h| h.frame_rate)) * 0.001_f32
        } else {
            speed
        };
    }
    pub fn set_anim_mode(&mut self, mode: u8) {
        self.mode = mode;
    }
    pub fn step(&mut self) {
        let count = self.frame_count();
        if count <= 1 || self.direction == 0 {
            return;
        }
        let next = if self.direction > 0 {
            if self.frame < count - 1 {
                self.frame + 1
            } else if self.mode == 3 {
                0
            } else if self.mode == 2 {
                self.direction = -1;
                self.frame - 1
            } else {
                self.direction = 0;
                self.accumulator = 0.;
                return;
            }
        } else if self.frame > 0 {
            self.frame - 1
        } else if self.mode == 3 {
            count - 1
        } else if self.mode == 2 {
            self.direction = 1;
            self.frame + 1
        } else {
            self.direction = 0;
            self.accumulator = 0.;
            return;
        };
        self.set_frame(next);
    }
    // Keep the published (dt * speed) * direction + acc operand order.
    #[allow(clippy::assign_op_pattern)]
    pub fn advance(&mut self, dt: f32) -> u32 {
        if self.speed.to_bits() == (-100_f32).to_bits() {
            self.set_speed(-200.);
        }
        if self.speed == 0. {
            return 0;
        }
        // MI9: transient precision differs from the binary32 accumulator store.
        // MI11: f64 approximates x87 here; control-word dependent last bits are a
        // portable numeric limit, not blanket per-operation f32 rounding.
        let transient =
            (f64::from(dt) * f64::from(self.speed)) * f64::from(self.direction) + f64::from(self.accumulator);
        self.accumulator = transient as f32;
        if transient.is_nan() || transient.abs() < 1. {
            return 0;
        }
        if transient.abs() < 2. {
            self.step();
            self.accumulator -= f32::from(self.direction);
        } else {
            self.step();
            self.accumulator = 1.;
        }
        2
    }
}
impl Interfaces for AnimatedInstance {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        if iid == IID_ANIMATED {
            Some(InterfacePart::Animated)
        } else {
            self.instance.interface(iid)
        }
    }
}

#[derive(Clone)]
pub enum InstanceReference {
    Basic(Reference<BasicInstance>),
    Static(Reference<StaticInstance>),
    Animated(Reference<AnimatedInstance>),
}
pub(crate) enum BorrowedInstanceReference {
    Basic(BorrowedReference<BasicInstance>),
    Static(BorrowedReference<StaticInstance>),
    Animated(BorrowedReference<AnimatedInstance>),
}
impl BorrowedInstanceReference {
    pub(crate) fn access(&self) -> Option<InstanceReference> {
        match self {
            Self::Basic(v) => v.access().map(InstanceReference::Basic),
            Self::Static(v) => v.access().map(InstanceReference::Static),
            Self::Animated(v) => v.access().map(InstanceReference::Animated),
        }
    }
}
impl InstanceReference {
    pub(crate) fn downgrade(&self) -> BorrowedInstanceReference {
        match self {
            Self::Basic(v) => BorrowedInstanceReference::Basic(v.downgrade()),
            Self::Static(v) => BorrowedInstanceReference::Static(v.downgrade()),
            Self::Animated(v) => BorrowedInstanceReference::Animated(v.downgrade()),
        }
    }
    pub(crate) fn borrowed_access(&self) -> Self {
        self.downgrade().access().expect("live container")
    }
    pub fn has_mesh_container(&self) -> bool {
        !matches!(self, Self::Basic(_))
    }
    /// Portable container accessor; requires has_mesh_container().
    pub fn with_instance<R>(&self, f: impl FnOnce(&StaticInstance) -> R) -> R {
        match self {
            Self::Basic(_) => panic!("basic instance has no mesh container"),
            Self::Static(v) => f(&v.read()),
            Self::Animated(v) => f(&v.read().instance),
        }
    }
    pub fn with_instance_mut<R>(&self, f: impl FnOnce(&mut StaticInstance) -> R) -> R {
        match self {
            Self::Basic(_) => panic!("basic instance has no mesh container"),
            Self::Static(v) => f(&mut v.write()),
            Self::Animated(v) => f(&mut v.write().instance),
        }
    }
    pub fn animated_frame_count(&self) -> Option<u16> {
        match self {
            Self::Basic(_) | Self::Static(_) => None,
            Self::Animated(v) => Some(v.read().frame_count()),
        }
    }
}
