// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn stop(h: &mut impl Host, token: u64) {
    if h.effect_running(token) {
        h.effect(Effect::StopEffect(token, 0));
    }
}

fn attributes(visible: bool, valid: bool) -> Attributes {
    Attributes {
        flag: if visible { 3 } else { 2 },
        highlight: if visible {
            if valid {
                3
            } else {
                4
            }
        } else {
            0
        },
        rgb: [1.0; 3],
        alpha: if visible { 0.4 } else { 0.0 },
    }
}

fn draw(h: &mut impl Host, preview: &mut Preview, visible: bool, valid: bool, point: Vec3, rotation: u8) {
    // Renderer applies this record to every contained instance in mode 5.
    h.effect(Effect::OccupantDraw(preview.token, 5, attributes(visible, valid)));
    if !visible {
        if preview.registered {
            h.effect(Effect::RemovePreview(preview.token));
            preview.registered = false;
        }
        return;
    }
    let (a, b) = (preview.local.x, preview.local.z);
    let (dx, dz) = match rotation {
        0 => (-a, -b),
        1 => (b, -a),
        2 => (a, b),
        _ => (-b, a),
    };
    let orientation = (preview.rotation + rotation) % 4;
    h.effect(Effect::OccupantTransform(
        preview.token,
        Vec3 {
            x: point.x + dx,
            y: point.y,
            z: point.z + dz,
        },
        orientation,
    ));
    if preview.registered {
        if preview.shown_rotation != orientation {
            h.effect(Effect::OrientationMessage(preview.token, 0x99EF_1145, 0));
        }
    } else {
        h.effect(Effect::InsertPreview(preview.token));
        preview.registered = true;
    }
    preview.shown_rotation = orientation;
}

impl PlaceLot {
    pub(super) fn build_preview(&mut self, h: &mut impl Host) {
        let Some(cfg) = h.configuration(self.configuration) else {
            return;
        };
        let entries = h.entries(self.configuration);
        let w = h.terrain().cell_width;
        let center = |p: Vec3| Vec3 {
            x: p.x - cfg.size[0] as u32 as f32 * w * 0.5,
            y: p.y,
            z: p.z - cfg.size[1] as u32 as f32 * w * 0.5,
        };
        if !self.building_preview {
            if let Some(entry) = h.first_building_entry(self.configuration) {
                let candidates = h.building_candidates(&entry.identifiers);
                if let Some(&exemplar) = candidates.first() {
                    if h.has_profile(exemplar) {
                        if let Some(token) = h.create_preview_building(exemplar) {
                            self.prepare_preview(h, token);
                            let mut preview = Preview {
                                token,
                                local: center(entry.position),
                                rotation: entry.rotation,
                                registered: false,
                                shown_rotation: 0,
                                prop: false,
                            };
                            draw(h, &mut preview, false, false, self.position, self.rotation);
                            self.previews.insert(0, preview);
                            self.building_preview = true;
                            let info = h.preview_info(token);
                            let mut categories = 0;
                            for (matched, bit, name) in [
                                (info.police, 1, "Police"),
                                (info.fire, 2, "Fire"),
                                (info.health, 4, "Health"),
                                (info.education, 8, "Education"),
                            ] {
                                if matched {
                                    categories |= bit;
                                    let (radius, auxiliary) = h.coverage_radius(token, bit).unwrap_or((32.0, 0));
                                    self.radius = radius;
                                    self.radius_auxiliary = auxiliary;
                                    self.coverage_names =
                                        Some([format!("PlopMode_{name}_Plop"), format!("PlopMode_{name}_Inactive")]);
                                }
                            }
                            h.effect(Effect::CoverageCategories(categories));
                            h.effect(Effect::RecalculateCoverage);
                            let info = h.occupant_info(token);
                            if info.groups.contains(&0x1401) {
                                self.coverage_view = 0xA;
                            } else if info.groups.contains(&0x1302) {
                                self.coverage_view = 0xF2;
                            }
                        }
                    }
                }
            }
        }
        for entry in entries
            .iter()
            .filter(|e| e.kind == 1 && e.flags & 1 == 0 && e.identifiers.len() == 1)
        {
            if let Some(token) = h.create_preview_prop(entry.identifiers[0]) {
                if h.preview_info(token).prop_preview {
                    self.prepare_preview(h, token);
                    h.effect(Effect::AcquirePreview(token));
                    self.previews.push(Preview {
                        token,
                        local: center(entry.position),
                        rotation: entry.rotation,
                        registered: false,
                        shown_rotation: 0,
                        prop: true,
                    });
                }
                h.effect(Effect::ReleaseCreatedPreview(token));
            }
        }
        if self.texture.is_none() {
            self.texture = h.footprint_texture(self.configuration);
        }
    }

    fn prepare_preview(&self, h: &mut impl Host, token: u64) {
        h.effect(Effect::OccupantFlag(token, 0x10));
        for property in [0x0292_44DB, 0x29F0_13E6] {
            h.effect(Effect::OccupantProperty(token, property, true));
        }
    }

    pub(super) fn release_configuration_effects(&mut self, h: &mut impl Host) {
        self.stop_previews(h);
        for mut preview in self.previews.drain(..) {
            draw(h, &mut preview, false, false, self.position, self.rotation);
            h.effect(if preview.prop {
                Effect::ReleasePreview(preview.token)
            } else {
                Effect::ReleaseCreatedPreview(preview.token)
            });
        }
        self.building_preview = false;
        self.clear_constraints(h);
        if let Some((token, _)) = self.texture.take() {
            h.effect(Effect::ReleaseTexture(token));
        }
    }

    fn clear_constraints(&mut self, h: &mut impl Host) {
        for array in &mut self.constraint_effects {
            for token in array.drain(..) {
                h.effect(Effect::StopEffect(token, 0));
                h.effect(Effect::ReleaseCreatedEffect(token));
            }
        }
    }

    pub fn feedback(&mut self, h: &mut impl Host, valid: bool, visible: bool, point: Vec3) {
        h.effect(Effect::FeedbackState(valid, visible, point, self.rotation));
        if !visible {
            h.effect(Effect::GridVisible(false));
            for token in self.ring.iter().chain(self.preview_effects.iter().flatten()) {
                stop(h, *token);
            }
            for preview in &mut self.previews {
                draw(h, preview, false, valid, point, self.rotation);
            }
            self.clear_constraints(h);
        } else {
            h.effect(Effect::GridPosition(point));
            h.effect(Effect::GridVisible(!h.city_grid_hidden()));
            if let Some(ring) = self.ring {
                if self.radius > 0.0 {
                    h.effect(Effect::EffectTransform(ring, point, 0.0, 2.0 * self.radius));
                    if !h.effect_running(ring) {
                        h.effect(Effect::StartEffect(ring, 0));
                    }
                }
            }
            for index in 0..2 {
                let recreate = self.feedback_valid != valid
                    || self.preview_effects[index].is_none()
                    || (index == 0 && self.preview_effects[index].is_some_and(|t| !h.effect_running(t)));
                if recreate {
                    if let Some(old) = self.preview_effects[index].take() {
                        stop(h, old);
                        h.effect(Effect::ReleaseCreatedEffect(old));
                    }
                    self.preview_effects[index] = h.create_effect(if index == 1 {
                        "Lot_Direction_Arrow"
                    } else if valid {
                        "building_footprint_ok"
                    } else {
                        "building_footprint_notok"
                    });
                }
                if let Some(token) = self.preview_effects[index] {
                    if index == 0 {
                        if let Some((texture, _)) = self.texture {
                            h.effect(Effect::EffectTexture(token, 6, texture));
                        }
                        let angle = f32::from_bits(0x3FC9_0FD0) * f32::from(self.rotation);
                        let scale = self.texture.map_or(0.0, |(_, extent)| extent) * f32::from_bits(0x3F7F_FF58);
                        h.effect(Effect::EffectTransform(token, point, f64::from(angle), scale));
                    } else if let Some(cfg) = h.configuration(self.configuration) {
                        let d = (f64::from(cfg.size[1]) * f64::from(h.terrain().cell_width) * 0.5 - 6.0).trunc() as f32;
                        let (dx, dz) = match self.rotation {
                            0 => (0.0, -d),
                            1 => (d, 0.0),
                            2 => (0.0, d),
                            _ => (-d, 0.0),
                        };
                        let p = Vec3 {
                            x: point.x + dx,
                            y: point.y,
                            z: point.z + dz,
                        };
                        h.effect(Effect::EffectTransform(
                            token,
                            p,
                            f64::from_bits(0xBFF9_21F9_F01B_866E) * f64::from(self.rotation),
                            1.0,
                        ));
                    }
                    if !h.effect_running(token) {
                        h.effect(Effect::StartEffect(token, 0));
                    }
                }
            }
            for preview in &mut self.previews {
                draw(h, preview, true, valid, point, self.rotation);
            }
            self.constraint_feedback(h, point);
        }
        if self.feedback_valid != valid {
            self.set_cursor_id(h, if valid { 0x417E_E36B } else { CURSOR });
        }
        self.feedback_valid = valid;
    }

    pub fn constraint_feedback(&mut self, h: &mut impl Host, point: Vec3) {
        let Some(cfg) = h.configuration(self.configuration) else {
            return;
        };
        let coverage = super::placement::cell_coverage(cfg.size, h.terrain(), point, self.rotation);
        let mut used = [0; 2];
        if coverage.valid {
            for entry in h
                .entries(self.configuration)
                .iter()
                .filter(|e| e.kind == 5 || e.kind == 6)
            {
                let index = usize::from(entry.kind == 6);
                let u0 = h.position_to_cell(entry.position.x + 8.0);
                let v0 = h.position_to_cell(entry.position.z + 8.0);
                let u1 = h.position_to_cell(entry.maximum.x + 8.0) - 1;
                let v1 = h.position_to_cell(entry.maximum.z + 8.0) - 1;
                let [w, d] = cfg.size;
                let (a, b, c, e) = rotate_constraint([u0, v0, u1, v1], [w, d], self.rotation);
                let array = &mut self.constraint_effects[index];
                if used[index] == array.len() {
                    if let Some(token) = h.create_effect(if index == 0 { "selection_blue" } else { "selection_red" }) {
                        array.push(token);
                    } else {
                        continue;
                    }
                }
                let token = array[used[index]];
                used[index] += 1;
                let r = coverage.rect;
                h.effect(Effect::EffectParameter(
                    token,
                    0xC,
                    vec![
                        (r.x0 + a) as f32,
                        (r.z0 + b) as f32,
                        (r.x0 + c + 1) as f32,
                        (r.z0 + e + 1) as f32,
                    ],
                ));
                h.effect(Effect::StartEffect(token, 0));
            }
        }
        for (array, count) in self.constraint_effects.iter().zip(used) {
            for &token in &array[count..] {
                h.effect(Effect::StopEffect(token, 0));
            }
        }
    }
}

pub fn rotate_constraint([u0, v0, u1, v1]: [i32; 4], [w, d]: [i32; 2], rotation: u8) -> (i32, i32, i32, i32) {
    match rotation {
        0 => (w - 1 - u1, d - 1 - v1, w - 1 - u0, d - 1 - v0),
        1 => (v0, w - 1 - u1, v1, w - 1 - u0),
        2 => (u0, v0, u1, v1),
        _ => (d - 1 - v1, u0, d - 1 - v0, u1),
    }
}
