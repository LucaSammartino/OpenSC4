// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl RegionView {
    pub fn draw_position(&self, host: &impl image::SurfaceHost, city: &CityEntry) -> (i32, i32) {
        let height = host.dimensions(city.image).1;
        let x = city.position.0.floor() as i32 - self.scroll.0;
        let y = ((city.position.1.floor() as i32 - self.scroll.1) as f32 - height as f32) as i32;
        (x, y)
    }

    pub fn playfield_rect(&self, host: &impl image::SurfaceHost, city: &CityEntry) -> Rect {
        let (x, y) = self.draw_position(host, city);
        image::rect(host, city.image).offset(x + self.scroll.0, y + self.scroll.1)
    }

    pub fn city_at_point(&self, host: &impl image::SurfaceHost, x: i32, y: i32) -> Option<CityId> {
        self.cities
            .iter()
            .rev()
            .find(|city| {
                let p = self.draw_position(host, city);
                let (px, py) = (x - p.0, y - p.1);
                image::rect(host, city.image).contains(px, py) && image::hit(&city.hit_runs, px, py)
            })
            .map(|city| city.id)
    }

    pub fn invalidate_all(&mut self, host: &mut impl Host) {
        if let Some(canvas) = &mut self.canvas {
            canvas.invalidate_all();
        }
        for city in &mut self.cities {
            city.current = false;
        }
        host.redraw();
    }

    pub fn invalidate_region(&mut self, host: &mut impl Host, id: CityId) -> Result<()> {
        let Some(city) = self.cities.iter().find(|c| c.id == id) else {
            return Ok(());
        };
        let rect = self.playfield_rect(host, city);
        if let Some(canvas) = &mut self.canvas {
            canvas.invalidate_rect(rect);
        }
        host.redraw();
        self.update_signpost(host, id)
    }

    pub fn select_city(&mut self, host: &mut impl Host, x: i32, y: i32) -> Result<Option<CityId>> {
        let next = self.city_at_point(host, x, y);
        if next != self.selected {
            let old = std::mem::replace(&mut self.selected, next);
            if let Some(id) = next {
                self.invalidate_region(host, id)?;
            }
            if let Some(id) = old {
                self.invalidate_region(host, id)?;
            }
        }
        Ok(next)
    }

    pub fn clear_selection(&mut self, host: &mut impl Host) -> Result<()> {
        if let Some(old) = self.selected.take() {
            self.invalidate_region(host, old)?;
        }
        Ok(())
    }

    pub fn enable_labels(&mut self, host: &mut impl Host, other: bool, selected: bool) {
        self.labels = (other, selected);
        self.invalidate_all(host);
    }
    pub fn enable_grid(&mut self, host: &mut impl Host, enabled: bool) {
        self.grid = enabled;
        self.invalidate_all(host);
    }
    pub fn set_map_mode(&mut self, host: &mut impl Host, mode: u32) {
        self.map_mode = mode;
        self.invalidate_all(host);
    }
    pub fn clear_cities(&mut self, host: &mut impl Host) {
        self.selected = None;
        for city in &mut self.cities {
            for effect in city.effects.drain(..) {
                host.stop_release_effect(effect);
            }
        }
        // Entries and their signposts/images are owned by the caller, not this vector.
        self.cities.clear();
        if let Some(canvas) = &mut self.canvas {
            canvas.invalidate_all();
        }
        host.redraw();
    }
}
