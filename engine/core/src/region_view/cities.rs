// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl RegionView {
    pub fn add_city(&mut self, host: &mut impl Host, city: CityEntry) -> Result<()> {
        let id = city.id;
        self.cities.push(city);
        self.invalidate_region(host, id)?;
        self.update_city(host, id)
    }

    pub fn set_signpost_type(&mut self, host: &mut impl Host, instance: u32, id: u32) -> Result<()> {
        self.signpost_type = (instance, id);
        for id in self.cities.iter().map(|c| c.id).collect::<Vec<_>>() {
            self.update_signpost(host, id)?;
        }
        Ok(())
    }

    pub fn set_signpost_mode(&mut self, every: bool, unclaimed: bool) {
        self.signposts = (every, unclaimed);
    }
    pub fn enable_tutorial_arrow(&mut self, enabled: bool) {
        self.tutorial_arrow = enabled;
    }

    pub fn update_city(&mut self, host: &mut impl Host, id: CityId) -> Result<()> {
        let index = self
            .cities
            .iter()
            .position(|c| c.id == id)
            .ok_or(Unsupported::Precondition("city entry"))?;
        self.cities[index].current = false;
        self.cities[index].icons.clear();
        for effect in self.cities[index].effects.drain(..) {
            host.stop_release_effect(effect);
        }
        self.invalidate_all(host);
        let key = self.key()?;
        let cell = self.cities[index].cell;
        let Some(record) = host.city_record(key, cell) else {
            return Ok(());
        };
        let (x, z) = ((cell.0 as f32) * 1024.0, (cell.1 as f32) * 1024.0);
        for (i, name) in ["region_airport", "region_seaport"].into_iter().enumerate() {
            let Some(icon) = self.icons[i] else {
                continue;
            };
            for point in record.points.get(&(i as u32)).into_iter().flatten() {
                let position = [x + point[0], point[1], z + point[2]];
                let px = round_projection(0.0883883 * position[0] - 0.0366117 * position[2]);
                let py = round_projection(0.0183058 * position[0] - 0.0828534 * position[1] + 0.0441942 * position[2]);
                self.cities[index].icons.push(Icon {
                    position: (px, py),
                    image: icon,
                });
                if let Some(effect) = host.create_effect(name) {
                    // The adapter sets flags 1/1, identity rotation and scale 1 before Start(0).
                    host.start_effect(effect, position);
                    self.cities[index].effects.push(effect);
                }
            }
        }
        Ok(())
    }

    pub fn render_canvas(&mut self, host: &mut impl Host, destination: Surface) -> Result<bool> {
        let mut canvas = self.canvas.take().ok_or(Unsupported::Precondition("view canvas"))?;
        let result = canvas.render(host, destination, 0, 0, |host, tile, x, y| {
            self.render_tile(host, tile, x, y)
        });
        self.canvas = Some(canvas);
        result
    }

    pub fn render_tile(&mut self, host: &mut impl Host, destination: Surface, wx: i32, wy: i32) -> Result<()> {
        if let Some(background) = self.background {
            host.tiled_background(destination, background, (-(self.scroll.0 + wx), -(self.scroll.1 + wy)));
        } else {
            host.fill(destination, 0xff00_0000);
        }
        let target = image::rect(host, destination);
        for city in &mut self.cities {
            let p = (
                city.position.0.floor() as i32 - wx,
                city.position.1.floor() as i32 - host.dimensions(city.image).1 - wy,
            );
            let bounds = image::rect(host, city.image);
            if !bounds.offset(p.0, p.1).overlaps(target) {
                continue;
            }
            let Some(composed) = city.composed else {
                continue;
            };
            if !city.current {
                let source = if self.map_mode == 1 {
                    city.alternate_image.unwrap_or(city.image)
                } else {
                    city.image
                };
                image::copy_and_tint(
                    host,
                    source,
                    composed,
                    self.grid.then_some(city.grid.as_slice()),
                    self.grid_colour,
                )?;
                image::build_runs(host, composed, &mut city.runs)?;
                city.current = true;
            }
            image::blit_image_spans(host, composed, destination, bounds, bounds.offset(p.0, p.1), &city.runs)?;
        }
        for city in &self.cities {
            let p = (
                city.position.0.floor() as i32 - wx,
                city.position.1.floor() as i32 - host.dimensions(city.image).1 - wy,
            );
            let bounds = image::rect(host, city.image);
            if !bounds.offset(p.0, p.1).overlaps(target) {
                continue;
            }
            let Some(record) = host.city_record(self.key()?, city.cell) else {
                continue;
            };
            let tutorial = record.is_tutorial();
            if (tutorial || record.established)
                && (self.selected != Some(city.id) || self.labels.1)
                && (tutorial || self.labels.0)
            {
                let mayor = if tutorial && host.hide_tutorial_mayor(record.tutorial_id) {
                    Vec::new()
                } else {
                    record.mayor_name
                };
                // SPEC-GAP: RGV4 leaves the mayor baseline unresolved; do not invent one.
                if !mayor.is_empty() {
                    return Err(Unsupported::Open("RGV4"));
                }
                let font_id = 0x8a8c_c984 - 2 * u32::from(city.size_code);
                if let Some(font) = host.font(font_id) {
                    let height = host.font_height(font);
                    let mut text = record.city_name;
                    host.layout_text(font, &mut text, Rect(0, 0, 4 * bounds.width() / 3, height), 0x5377_be31);
                    let width = host.text_width(font, &text);
                    let x = p.0 + city.label_anchor.0 - width / 2;
                    let y = p.1 + city.label_anchor.1 - height;
                    host.draw_control(
                        destination,
                        Draw::FittedText {
                            font,
                            count: text.len(),
                            text,
                            rect: [x, y, x + width, y + height],
                            color: 0x00de_e8e3,
                            flags: 0x0800_0000,
                        },
                    );
                }
            }
            // SPEC-GAP: RGV3 leaves the native icon blitter interface unresolved.
            if self.map_mode != 0 && !city.icons.is_empty() {
                return Err(Unsupported::Open("RGV3"));
            }
        }
        if let Some(city) = self.cities.iter().find(|c| Some(c.id) == self.selected) {
            let p = (
                city.position.0.floor() as i32 - wx,
                city.position.1.floor() as i32 - host.dimensions(city.image).1 - wy,
            );
            let bounds = image::rect(host, city.image).offset(p.0, p.1);
            image::fill_coverage(host, destination, bounds, &city.selection, self.selection_colour)?;
        }
        Ok(())
    }

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

fn round_projection(value: f32) -> i32 {
    (value + 12_582_912.0).to_bits().wrapping_sub(0x4b40_0000) as i32
}
