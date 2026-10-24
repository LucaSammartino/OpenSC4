// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled inherited arithmetic from watercraft §5.1.4.
use super::{math, Event, Publication, Watercraft};
use crate::automata::Vec3;

impl Watercraft {
    /// Prefix of inherited animation, before slots 83/103 and state dispatch.
    pub fn advance_speed_tween(&mut self, animation_time: f32) {
        let tween = &mut self.player.speed_tween;
        if tween.elapsed < tween.duration {
            tween.elapsed += animation_time * 0.001_f32;
            if tween.elapsed >= tween.duration {
                tween.elapsed = 0.0;
                tween.duration = 0.0;
                tween.start = tween.target;
            }
        }
    }

    /// Generic-seek branch after state dispatch and flag gates. The callback
    /// is only the foreign city service's segment adjustment, slot 44.
    /// Scale is the inherited slot-125 property; speed is sampled locally.
    pub fn generic_seek(&mut self, movement_time: f32, scale: f32, mut adjust: impl FnMut(Vec3, &mut Vec3, u32)) {
        let p = self.inherited_position;
        if self.inherited_destination.iter().any(|c| *c < 0.0) {
            self.inherited_destination = p;
        }
        let delta = math::sub(self.inherited_destination, p);
        let length = math::length(delta);
        if length <= 0.01_f32 {
            self.record.base.automata_flags |= 4;
            self.events.push(Event::Notify(4, 0));
            return;
        }
        let heading = math::scale(delta, 1.0 / length);
        self.inherited_heading = heading;
        let velocity = math::scale(heading, self.player.speed_tween.sample());
        let scaled = math::scale(velocity, scale);
        let mut proposed = math::add(p, math::scale(scaled, movement_time));
        if self.record.base.automata_flags & 0x20000 == 0 {
            adjust(p, &mut proposed, 0xC49);
        }
        self.events.push(Event::Publish(Publication::MoveTo(proposed, heading)));
        self.inherited_position = proposed;
        self.inherited_measured_speed = math::length(math::sub(proposed, p)) / movement_time;
    }
}
