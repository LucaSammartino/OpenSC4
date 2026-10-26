// SPDX-License-Identifier: GPL-3.0-or-later
//! Lighting and dynamic textures, docs/specs/model-maker.md §11.
use super::{Handle, ModelMaker, Port};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel {
    Rgba8888,
    Rgb888,
    Argb4444,
    Argb1555,
    Dxt1,
    Dxt35,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TintError {
    InvalidLayout,
    ShortBuffer,
    SizeOverflow,
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub width: usize,
    pub height: usize,
    pub source_padding: usize,
    pub destination_padding: usize,
}

impl Kernel {
    pub fn from_format(format: u32, sub_format: u32) -> Option<Self> {
        match (format, sub_format) {
            (3, 1) => Some(Self::Rgba8888),
            (2, 1) => Some(Self::Rgb888),
            (3, 10) => Some(Self::Argb1555),
            (3, 13) => Some(Self::Argb4444),
            (7 | 8, _) => Some(Self::Dxt1),
            (9 | 10, _) => Some(Self::Dxt35),
            _ => None,
        }
    }

    fn bits(self) -> usize {
        match self {
            Self::Rgba8888 => 32,
            Self::Rgb888 => 24,
            Self::Argb4444 | Self::Argb1555 => 16,
            Self::Dxt1 => 4,
            Self::Dxt35 => 8,
        }
    }
    fn block(self) -> usize {
        match self {
            Self::Dxt1 | Self::Dxt35 => 4,
            _ => 1,
        }
    }

    /// Row padding is in bytes. For the two 16-bit kernels, native row starts
    /// advance by four bytes per texel, although only two bytes are tinted.
    pub fn tint(self, destination: &mut [u8], source: &[u8], layout: Layout, colour: u32) -> Result<(), TintError> {
        let block = self.block();
        if layout.width % block != 0 || layout.height % block != 0 {
            return Err(TintError::InvalidLayout);
        }
        let row_bytes = layout
            .width
            .checked_mul(self.bits())
            .ok_or(TintError::SizeOverflow)?
            .checked_mul(block)
            .ok_or(TintError::SizeOverflow)?
            / 8;
        let row_advance = if matches!(self, Self::Argb1555 | Self::Argb4444) {
            layout.width.checked_mul(4).ok_or(TintError::SizeOverflow)?
        } else {
            row_bytes
        };
        let source_stride = row_advance
            .checked_add(layout.source_padding)
            .ok_or(TintError::SizeOverflow)?;
        let destination_stride = row_advance
            .checked_add(layout.destination_padding)
            .ok_or(TintError::SizeOverflow)?;
        let rows = layout.height / block;
        let required = |stride: usize| -> Result<usize, TintError> {
            if rows == 0 {
                Ok(0)
            } else {
                (rows - 1)
                    .checked_mul(stride)
                    .and_then(|v| v.checked_add(row_bytes))
                    .ok_or(TintError::SizeOverflow)
            }
        };
        // SPEC-GAP: malformed descriptors/buffers are rejected before writing.
        if required(source_stride)? > source.len() || required(destination_stride)? > destination.len() {
            return Err(TintError::ShortBuffer);
        }
        let colours = colour.to_le_bytes();
        for row in 0..rows {
            let source = &source[row * source_stride..][..row_bytes];
            let destination = &mut destination[row * destination_stride..][..row_bytes];
            match self {
                Self::Rgba8888 | Self::Rgb888 => {
                    let bytes = if self == Self::Rgba8888 { 4 } else { 3 };
                    for (src, dst) in source.chunks_exact(bytes).zip(destination.chunks_exact_mut(bytes)) {
                        for k in 0..bytes {
                            dst[k] = ((u32::from(src[k]) * u32::from(colours[k]) + 128) >> 8) as u8;
                        }
                    }
                }
                Self::Argb4444 | Self::Argb1555 => {
                    for (src, dst) in source.chunks_exact(2).zip(destination.chunks_exact_mut(2)) {
                        let value = u16::from_le_bytes([src[0], src[1]]);
                        let tinted = if self == Self::Argb4444 {
                            let mut tinted = 0;
                            for k in 0..4 {
                                tinted |=
                                    ((((value >> (4 * k)) & 15) * u16::from(colours[k] >> 4) + 8) >> 4) << (4 * k);
                            }
                            tinted
                        } else {
                            let mut tinted = if colours[3] & 128 != 0 { value & 0x8000 } else { 0 };
                            for k in 0..3 {
                                tinted |=
                                    ((((value >> (5 * k)) & 31) * u16::from(colours[k] >> 3) + 16) >> 5) << (5 * k);
                            }
                            tinted
                        };
                        dst.copy_from_slice(&tinted.to_le_bytes());
                    }
                }
                Self::Dxt1 | Self::Dxt35 => {
                    let bytes = if self == Self::Dxt1 { 8 } else { 16 };
                    for (src, dst) in source.chunks_exact(bytes).zip(destination.chunks_exact_mut(bytes)) {
                        let (src, dst) = if bytes == 16 {
                            dst[..8].copy_from_slice(&src[..8]);
                            (&src[8..], &mut dst[8..])
                        } else {
                            (src, dst)
                        };
                        tint_dxt_colour(src, dst, colours, self == Self::Dxt35);
                    }
                }
            }
        }
        Ok(())
    }
}

fn tint_565(value: u16, colour: [u8; 4]) -> u16 {
    let b = ((value & 31) * u16::from(colour[0] >> 3) + 16) >> 5;
    let g = (((value >> 5) & 63) * u16::from(colour[1] >> 3) + 16) >> 5;
    let r = (((value >> 11) & 31) * u16::from(colour[2] >> 3) + 16) >> 5;
    (r << 11) | (g << 5) | b
}

fn tint_dxt_colour(source: &[u8], destination: &mut [u8], colour: [u8; 4], keep_order: bool) {
    let e0 = u16::from_le_bytes([source[0], source[1]]);
    let e1 = u16::from_le_bytes([source[2], source[3]]);
    let t0 = tint_565(e0, colour);
    let t1 = tint_565(e1, colour);
    let mut indices = u32::from_le_bytes(source[4..8].try_into().unwrap());
    let swap = if e1 < e0 {
        if t0 < t1 {
            indices = !indices;
            true
        } else {
            if t0 == t1 {
                indices = 0;
            }
            false
        }
    } else if t1 < t0 {
        indices ^= (!indices >> 1) & 0x5555_5555;
        true
    } else {
        false
    };
    let (first, second) = if swap && !keep_order { (t1, t0) } else { (t0, t1) };
    destination[..2].copy_from_slice(&first.to_le_bytes());
    destination[2..4].copy_from_slice(&second.to_le_bytes());
    destination[4..8].copy_from_slice(&indices.to_le_bytes());
}

pub fn light_tint(colour: [f32; 3]) -> u32 {
    let channel = |f: f32| {
        let v = (f + 49152.0).to_bits().wrapping_sub(0x4740_0000);
        v.wrapping_sub(v >> 8) & 255
    };
    0xFF00_0000 | (channel(colour[0]) << 16) | (channel(colour[1]) << 8) | channel(colour[2])
}

/// Eight descriptor words, with word 7 represented portably by `mips`.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub words: [u32; 8],
    pub mips: Vec<Vec<u8>>,
}

pub trait TextureHost {
    fn global_colour_seed(&mut self) -> f32;
    fn light_colour(&mut self, colour: &mut [f32; 3]);
    /// Acquired R (0x056B906E/0x656B8EFC), T (0x1AC0E11A/0xFAC0E219).
    fn acquire_texture_services(&mut self) -> Option<[Handle; 2]>;
    fn texture_group(&mut self, texture_service: Handle) -> u32;
    /// Type 0x7AB50E44, IID 0x1AB50C3F, acquired on success.
    fn load_texture(&mut self, resource_service: Handle, group: u32, instance: u32) -> Option<Handle>;
    fn image(&mut self, texture: Handle) -> Image;
    fn update_binding(&mut self, service: Handle, binding: Handle, image: &Image);
    fn create_binding(&mut self, service: Handle, image: &Image) -> Option<Handle>;
    fn retain(&mut self, handle: Handle);
    fn release(&mut self, handle: Handle);
    /// Acquired occupants of type 0x278128A0 in manager-cell query order.
    fn building_occupants(&mut self) -> Vec<Handle>;
    fn holder(&mut self, occupant: Handle) -> Option<Handle>;
}

impl ModelMaker {
    pub fn lighting_changed(&mut self, host: &mut impl TextureHost) -> Result<(), TintError> {
        let seed = host.global_colour_seed();
        let mut colour = [seed; 3];
        host.light_colour(&mut colour);
        if colour
            .iter()
            .zip(self.light_colour)
            .all(|(new, old)| (*new - old).abs() < 1e-4)
        {
            return Ok(());
        }
        self.light_colour = colour;
        for k in 0..5 {
            self.rebuild_dynamic_texture(k, host)?;
        }
        if self.port(Port::OccupantManager).is_some() {
            for o in host.building_occupants() {
                if let Some(h) = host.holder(o) {
                    host.retain(h);
                    host.release(h);
                }
                host.release(o);
            }
        }
        Ok(())
    }

    pub fn rebuild_dynamic_texture(&mut self, k: usize, host: &mut impl TextureHost) -> Result<bool, TintError> {
        if k >= 5 {
            return Err(TintError::InvalidLayout);
        }
        let Some([resource_service, texture_service]) = host.acquire_texture_services() else {
            return Ok(false);
        };
        let group = host.texture_group(texture_service);
        let instance =
            (if k == 4 { 0x894F_D650u32 } else { 0x294E_CDD0u32 }).wrapping_add(self.saved.target_view[0] as u32);
        let Some(texture) = host.load_texture(resource_service, group, instance) else {
            host.release(texture_service);
            host.release(resource_service);
            return Ok(false);
        };
        let result = (|| {
            let mut image = host.image(texture);
            let Some(kernel) = Kernel::from_format(image.words[5], image.words[6]) else {
                return Ok(false);
            };
            let mut width = image.words[2] as usize;
            let mut height = image.words[3] as usize;
            let mut mips = Vec::new();
            for level in 0..image.words[1] as usize {
                width = width.max(kernel.block());
                height = height.max(kernel.block());
                let size = width.checked_mul(height).ok_or(TintError::SizeOverflow)?;
                let bytes = size.checked_mul(kernel.bits()).ok_or(TintError::SizeOverflow)? / 8;
                let mut destination = vec![0; bytes];
                let source = image.mips.get(level).ok_or(TintError::ShortBuffer)?;
                kernel.tint(
                    &mut destination,
                    source,
                    Layout {
                        width: size / kernel.block(),
                        height: kernel.block(),
                        source_padding: 0,
                        destination_padding: 0,
                    },
                    light_tint(self.light_colour),
                )?;
                mips.push(destination);
                width >>= 1;
                height >>= 1;
            }
            image.mips = mips;
            if let Some(binding) = self.textures[k] {
                host.update_binding(texture_service, binding, &image);
            } else {
                self.textures[k] = host.create_binding(texture_service, &image);
            }
            Ok(self.textures[k].is_some())
        })();
        host.release(texture);
        host.release(texture_service);
        host.release(resource_service);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layout(width: usize, height: usize) -> Layout {
        Layout {
            width,
            height,
            source_padding: 0,
            destination_padding: 0,
        }
    }

    #[test]
    fn raw_kernels_round_alpha_and_keep_the_16_bit_row_stride_quirk() {
        let mut dst = [0; 4];
        Kernel::Rgba8888
            .tint(&mut dst, &[255; 4], layout(1, 1), light_tint([0.5; 3]))
            .unwrap();
        assert_eq!(dst, [128, 128, 128, 254]);
        let mut dst = [99; 6];
        Kernel::Argb4444
            .tint(&mut dst, &[255, 255, 0, 0, 255, 255], layout(1, 2), 0xFFFF_FFFF)
            .unwrap();
        assert_eq!(dst, [0xEE, 0xEE, 99, 99, 0xEE, 0xEE]);
        Kernel::Argb1555
            .tint(&mut dst, &[255, 255, 0, 0, 255, 255], layout(1, 2), 0x00FF_FFFF)
            .unwrap();
        assert_eq!(u16::from_le_bytes([dst[0], dst[1]]), 0x7BDE);
        assert_eq!(light_tint([0.0, 0.5, 1.0]), 0xFF00_80FF);
        let original = dst;
        assert_eq!(
            Kernel::Argb1555.tint(&mut dst, &[0; 2], layout(1, 2), 0),
            Err(TintError::ShortBuffer)
        );
        assert_eq!(dst, original);
    }

    #[test]
    fn dxt_endpoint_remapping_and_dxt35_keeps_unswapped_endpoints() {
        // Source red > green; tint removes red, reversing endpoint order.
        let mut source = vec![0x00, 0xF8, 0xE0, 0x07, 0xE4, 0xE4, 0xE4, 0xE4];
        let mut dst = [0; 8];
        Kernel::Dxt1.tint(&mut dst, &source, layout(4, 4), 0xFF00_FF00).unwrap();
        assert_eq!(&dst[..4], &[0xA0, 0x07, 0, 0]);
        assert_eq!(&dst[4..], &[0x1B; 4]);
        let mut full_source = vec![17; 8];
        full_source.append(&mut source);
        let mut dst = [0; 16];
        Kernel::Dxt35
            .tint(&mut dst, &full_source, layout(4, 4), 0xFF00_FF00)
            .unwrap();
        assert_eq!(&dst[..8], &[17; 8]);
        assert_eq!(&dst[8..12], &[0, 0, 0xA0, 0x07]);
        assert_eq!(&dst[12..], &[0x1B; 4]);
        let mut equal = [0; 8];
        Kernel::Dxt1
            .tint(&mut equal, &full_source[8..], layout(4, 4), 0)
            .unwrap();
        assert_eq!(equal, [0; 8]);
    }

    struct Textures {
        colour: [f32; 3],
        instances: Vec<u32>,
        releases: Vec<Handle>,
        updates: Vec<(Handle, Image)>,
        creates: usize,
        retains: Vec<Handle>,
        occupants: usize,
    }
    impl TextureHost for Textures {
        fn global_colour_seed(&mut self) -> f32 {
            0.0
        }
        fn light_colour(&mut self, c: &mut [f32; 3]) {
            *c = self.colour;
        }
        fn acquire_texture_services(&mut self) -> Option<[Handle; 2]> {
            Some([1, 2])
        }
        fn texture_group(&mut self, _: Handle) -> u32 {
            9
        }
        fn load_texture(&mut self, _: Handle, g: u32, i: u32) -> Option<Handle> {
            assert_eq!(g, 9);
            self.instances.push(i);
            Some(3)
        }
        fn image(&mut self, _: Handle) -> Image {
            Image {
                words: [0, 1, 1, 1, 0, 3, 1, 0],
                mips: vec![vec![255; 4]],
            }
        }
        fn update_binding(&mut self, _: Handle, b: Handle, i: &Image) {
            self.updates.push((b, i.clone()));
        }
        fn create_binding(&mut self, _: Handle, _: &Image) -> Option<Handle> {
            self.creates += 1;
            Some(100 + self.creates as u64)
        }
        fn retain(&mut self, h: Handle) {
            self.retains.push(h);
        }
        fn release(&mut self, h: Handle) {
            self.releases.push(h);
        }
        fn building_occupants(&mut self) -> Vec<Handle> {
            self.occupants += 1;
            vec![20]
        }
        fn holder(&mut self, _: Handle) -> Option<Handle> {
            Some(21)
        }
    }
    #[test]
    fn m22_light_threshold_rebuild_order_and_existing_binding_identity() {
        let mut maker = ModelMaker::default();
        maker.light_colour = [0.5; 3];
        maker.saved.target_view[0] = 3;
        maker.ports.insert(Port::OccupantManager, 99);
        let mut host = Textures {
            colour: [0.5, 0.5, 0.50009],
            instances: vec![],
            releases: vec![],
            updates: vec![],
            creates: 0,
            retains: vec![],
            occupants: 0,
        };
        maker.lighting_changed(&mut host).unwrap();
        assert!(host.instances.is_empty());
        host.colour[2] = 0.5002;
        maker.lighting_changed(&mut host).unwrap();
        assert_eq!(
            host.instances,
            [0x294E_CDD3, 0x294E_CDD3, 0x294E_CDD3, 0x294E_CDD3, 0x894F_D653]
        );
        assert_eq!(host.creates, 5);
        assert_eq!(host.retains, [21]);
        assert_eq!(host.occupants, 1);
        let bindings = maker.textures;
        host.colour[0] = 0.6;
        maker.lighting_changed(&mut host).unwrap();
        assert_eq!(host.creates, 5);
        assert_eq!(host.updates.len(), 5);
        assert_eq!(maker.textures, bindings);
        assert_eq!(host.updates[0].1.words, [0, 1, 1, 1, 0, 3, 1, 0]);
        assert_eq!(host.releases.iter().filter(|v| **v == 3).count(), 10);
    }
}
