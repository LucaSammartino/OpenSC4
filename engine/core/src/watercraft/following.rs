// SPDX-License-Identifier: GPL-3.0-or-later
use super::{math, BoatId, Event, Fleet, Publication, Result, Terrain};

impl Fleet {
    /// §6.3 endpoint-gap constraint. Translation is independent of dt.
    pub fn following_tick(&mut self, id: BoatId, dt: f32, terrain: &dyn Terrain) -> Result<bool> {
        let Some(tug_id) = self.boat(id)?.tug else {
            return Ok(false);
        };
        let tug_back = self.boat(tug_id)?.rear_endpoint();
        let boat = self.boat_mut(id)?;
        boat.inherited_position[1] = terrain.water_height(boat.inherited_position);
        let mut a = tug_back;
        a[1] = terrain.water_height(a);
        let v = math::sub(a, boat.front_endpoint());
        let length = math::length(v);
        let gap = length - (boat.radius + 10.0);
        let target = math::normalize(math::flat(math::sub(tug_back, boat.inherited_position)));
        let heading = math::normalize(boat.inherited_heading);
        let angle = math::dot(heading, target).acos().to_degrees();
        let angle = if angle.is_nan() { 0.0 } else { angle };
        if (angle >= 1.0 || length >= 1.0) && gap > 0.0 {
            let position = math::add(boat.inherited_position, math::scale(v, gap / length));
            let sign = super::sign(target[0] * heading[2] - target[2] * heading[0]);
            let heading = math::yaw(heading, sign * angle.min(dt * boat.drive.0[38]) * 0.017_453_292);
            boat.events.push(Event::Publish(Publication::MoveTo(position, heading)));
            boat.previous_position = boat.position;
            boat.previous_heading = boat.heading;
            boat.position = position;
            boat.heading = heading;
            boat.inherited_position = position;
            boat.inherited_heading = heading;
        }
        Ok(true)
    }
}
