// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl<D: Driver> DrawContext<D> {
    pub fn set_vertex_buffer(&mut self, format: u32, data: usize) -> Result<(), Error> {
        self.ready()?;
        self.emit(Command::InterleavedArrays {
            format,
            stride: 0,
            data,
        })
    }
    pub fn set_index_buffer(&mut self, base: usize) {
        self.index_base = base;
    }
    pub fn draw_arrays(&mut self, primitive: u32, first: u32, count: u32) -> Result<(), Error> {
        self.ready()?;
        self.emit(Command::DrawArrays {
            primitive,
            first,
            count,
        })
    }
    pub fn draw_prims(&mut self, primitive: u32, format: u32, count: u32, data: usize) -> Result<(), Error> {
        self.set_vertex_buffer(format, data)?;
        self.draw_arrays(primitive, 0, count)
    }
    pub fn draw_indexed(&mut self, primitive: u32, first: u32, count: u32) -> Result<(), Error> {
        let offset = (first as usize).checked_mul(2).ok_or(Error::AddressOverflow)?;
        let indices = self.index_base.checked_add(offset).ok_or(Error::AddressOverflow)?;
        self.draw_indices(primitive, count, indices)
    }
    pub fn draw_indices(&mut self, primitive: u32, count: u32, indices: usize) -> Result<(), Error> {
        self.ready()?;
        self.emit(Command::DrawElements {
            primitive,
            count,
            index_type: 3,
            indices,
        })
    }
    pub fn render_mesh(&mut self, mesh: &Mesh) -> Result<(), Error> {
        self.ready()?;
        let Some(vertices) = mesh.vertices else {
            return Ok(());
        };
        self.set_vertex_buffer(mesh.format, vertices)?;
        if let Some(indices) = mesh.indices {
            self.set_index_buffer(indices);
        }
        for group in &mesh.groups {
            if mesh.indices.is_some() {
                self.draw_indexed(group.primitive, group.first, group.count)?;
            } else {
                self.draw_arrays(group.primitive, group.first, group.count)?;
            }
        }
        Ok(())
    }

    pub fn set_material_state(&mut self, material: &Material, flags: DrawFlags) -> Result<(), Error> {
        self.ready()?;
        self.qualified_numeric()?;
        if material.textures.len() > 2 {
            return Err(Error::InvalidValue);
        }
        let bit = |n: u32| flags.bits & (1u8 << n) != 0;
        let grey = bit(3);
        let fade = bit(1) && !grey;
        let reference = numeric::material_threshold(material.alpha_threshold);
        let packed = PackedState::pack(PackInput {
            color_write: material.flags & 0x100 != 0,
            depth_write: material.flags & 0x80 != 0 && !bit(4) && !bit(5),
            depth_test: !bit(4) && (material.flags & 2 != 0 || bit(5)),
            alpha_test: material.flags & 1 != 0,
            blend: material.flags & 0x10 != 0 || fade,
            textures: material.textures.len(),
            cull: material.flags & 8 != 0,
            depth_func: if bit(5) { 4 } else { material.depth_func as i32 },
            source_blend: if fade { 4 } else { material.blend[0] as i32 },
            destination_blend: if fade { 5 } else { material.blend[1] as i32 },
            alpha_func: material.alpha_func as i32,
            alpha_reference: if fade && flags.a < 1.0 {
                numeric::store(numeric::mul(f64::from(reference), f64::from(flags.a))?)?
            } else {
                reference
            },
            stage0_mode: material.textures.first().map_or(-1, |slot| slot.mode as i32),
            stage1_mode: material.textures.get(1).map_or(-1, |slot| slot.mode as i32),
        })?;
        if grey {
            self.emit(Command::ColorMultiplier([flags.a; 3]))?;
        } else if fade && flags.a < 1.0 {
            self.set_transparency(flags.a)?;
        }
        self.set_render_state(packed)?;
        if material.flags & 0x20 != 0 {
            for (stage, slot) in material.textures.iter().enumerate() {
                self.set_texture(slot.texture.clone(), stage)?;
                self.set_tex_coord(slot.coordinate_set, stage)?;
                self.set_tex_wrap(i32::from(slot.wrap[0]), i32::from(slot.wrap[1]), stage)?;
                self.set_tex_filter(i32::from(slot.filter[0]), i32::from(slot.filter[1]), stage)?;
                if slot.mode == 4 {
                    self.set_tex_combiner(&DEFAULT_COMBINER_BYTES.map(|byte| byte.unwrap()), stage)?;
                } else {
                    self.set_tex_env_mode(slot.mode as i32, stage)?;
                }
                if slot.flags & 1 != 0 {
                    self.qualified_numeric()?;
                    let k = numeric::reciprocal(slot.scale)?;
                    let tx = self.composed.translation[0];
                    let ty = self.composed.translation[1];
                    let mut matrix = [0.0; 16];
                    matrix[0] = numeric::store(k)?;
                    matrix[5] = numeric::store(-k)?;
                    matrix[12] = numeric::store(numeric::mul(-k, f64::from(tx))?)?;
                    matrix[13] = numeric::store(numeric::mul(k, f64::from(ty))?)?;
                    self.set_tex_transform(matrix, stage)?;
                } else {
                    self.clear_tex_transform(stage)?;
                }
            }
        }
        Ok(())
    }

    pub fn set_highlight_material_state(&mut self, material: &Material, flags: DrawFlags) -> Result<(), Error> {
        self.set_render_state_highlight(flags.highlight_color as usize)?;
        if let Some(slot) = material.textures.first() {
            self.set_texture(slot.texture.clone(), 0)?;
            self.set_tex_coord(slot.coordinate_set, 0)?;
            self.set_tex_wrap(i32::from(slot.wrap[0]), i32::from(slot.wrap[1]), 0)?;
            self.set_tex_filter(i32::from(slot.filter[0]), i32::from(slot.filter[1]), 0)?;
        }
        Ok(())
    }

    pub fn render_model_instance(
        &mut self,
        instance: &dyn Instance,
        meshes: &[Mesh],
        flags: DrawFlags,
        highlight: bool,
    ) -> Result<(), Error> {
        self.ready()?;
        if !highlight && self.properties.show_boxes {
            self.draw_bounding_box(instance.bounds()?, [1.0, 0.0, 0.0, 1.0])?;
            if let Some(bounds) = instance.effects_collection_bounds()? {
                self.draw_bounding_box(bounds, [0.0, 0.0, 1.0, 0.5])?;
            }
            return Ok(());
        }
        if highlight && flags.bits & (1 << 3) != 0 {
            return Ok(());
        }
        if self.properties.bit30 {
            let size = instance.bounds()?.largest_extent();
            let pixels = self.view.as_ref().ok_or(Error::MissingView)?.pixels_per_metre()?;
            if size * pixels < self.properties.model_threshold {
                return Ok(());
            }
        }
        self.set_model_transform(Some(instance.transform()?))?;
        let override_shade = highlight || flags.bits & (1 << 2) != 0;
        if override_shade {
            self.set_self_lit_shade(flags.color)?;
        } else {
            self.set_model_shade(instance.token(), Some(flags.color))?;
        }
        for mesh in meshes {
            if highlight {
                self.set_highlight_material_state(&mesh.material, flags)?;
            } else {
                self.set_material_state(&mesh.material, flags)?;
            }
            self.render_mesh(mesh)?;
            if !self.properties.use_second_stage && mesh.material.textures.len() == 2 {
                self.prepare_second_pass()?;
                self.render_mesh(mesh)?;
            }
        }
        // DC6: normal end-of-loop work only. Provider/nonreturn boundaries above
        // propagate immediately, preserving the completed prefix without cleanup.
        self.reset_shade()?;
        self.reset_transparency()
    }
}
