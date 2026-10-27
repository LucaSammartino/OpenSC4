// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Kind(u32),
    Zoom(u32),
    MissingFont,
    ImageSize,
    NativeProcessorAllocation,
    MissingMainWindow,
    /// DVV30-1: Init's three packed-state values are not specified.
    OpenRenderStates,
    /// DVV30-2: which row-2 spin product remains unrounded is unspecified.
    OpenSpinRow2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    Occupant,
    Automaton,
    Model,
    Font,
    Surface,
    Image,
    Resource,
    Factory,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResourceOp {
    Acquire(Reference, Token),
    Release(Reference, Token),
    ReleaseTexture(Token),
    FontStyle(Token, u32),
    ClearSurface(Token, i32, i32),
    UnlockSurface(Token, u32),
    DrawText {
        font: Token,
        surface: Token,
        text: String,
        colour: u32,
        flags: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub scale: f32,
    pub pitch: f32,
    pub zoom: u32,
    pub rotation: u32,
    pub forward_x: f32,
    pub forward_z: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            scale: 1.0,
            pitch: 0.0,
            zoom: 0,
            rotation: 0,
            forward_x: 0.0,
            forward_z: -1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    pub token: Token,
    pub width: i32,
    pub height: i32,
}

/// Foreign operations only. Optional services default to absent, never success.
/// Image/resource/factory results are created references. `font` is borrowed.
/// ResourceOp implements exactly the acquisitions/releases requested by core.
pub trait DisplayServices {
    fn camera(&mut self) -> Camera;
    fn resource_op(&mut self, op: ResourceOp);
    fn font(&mut self, _id: u32) -> Option<Token> {
        None
    }
    fn font_style(&mut self, _font: Token) -> u32 {
        0
    }
    fn measure_text(&mut self, font: Token, text: &str) -> [i32; 2];
    fn create_surface(&mut self) -> Option<Token> {
        None
    }
    fn init_surface(&mut self, _surface: Token, _side: u32, _descriptor: u32) -> bool {
        false
    }
    fn lock_surface(&mut self, _surface: Token, _flags: u32) -> bool {
        false
    }
    fn texture_service_present(&mut self) -> bool {
        false
    }
    /// SetupTexture(source, service, out, 1, 0, 0, 0).
    fn setup_texture(&mut self, source: Token) -> Option<Token>;
    /// Resource key (856DDBAC,46A006B0,instance), IID 06F1568E, then res slot 3.
    fn resource_image(&mut self, _instance: u32) -> Option<(Token, Image)> {
        None
    }
    /// GZCOM class A2FFB5D3, IID 02FFBB60.
    fn image_factory(&mut self) -> Option<Token> {
        None
    }
    fn factory_image(&mut self, _factory: Token, _label: &str, _flags: u32) -> Option<Image> {
        None
    }
}

fn power_of_two(n: i32) -> Result<u32, Unsupported> {
    if n <= 0 {
        return Err(Unsupported::ImageSize);
    }
    (n as u32).checked_next_power_of_two().ok_or(Unsupported::ImageSize)
}

/// Rebuild releases the old texture before all ordinary failure paths. Model
/// entries return before that release (dispatch §4.1.3).
pub fn build_display(info: &mut TrackInfo, host: &mut impl DisplayServices) -> Result<(), Unsupported> {
    if info.has(flags::MODEL) {
        return Ok(());
    }
    if let Some(texture) = info.texture.take() {
        host.resource_op(ResourceOp::ReleaseTexture(texture));
    }
    let (width, height, side, left, right, icon_depth, display_depth) = if info.kind == 3 || info.kind == 4 {
        let (resource, image) = if info.image_instance != 0 {
            let Some((resource, image)) = host.resource_image(info.image_instance) else {
                return Ok(());
            };
            (Some(resource), image)
        } else {
            let Some(factory) = host.image_factory() else {
                return Ok(());
            };
            let image = host.factory_image(factory, &info.label, 8);
            host.resource_op(ResourceOp::Release(Reference::Factory, factory));
            let Some(image) = image else {
                return Ok(());
            };
            (None, image)
        };
        if host.texture_service_present() {
            info.texture = host.setup_texture(image.token);
        }
        host.resource_op(ResourceOp::Release(Reference::Image, image.token));
        if let Some(resource) = resource {
            host.resource_op(ResourceOp::Release(Reference::Resource, resource));
        }
        if info.kind == 4 {
            info.display_size = [35.0; 2];
            let side = power_of_two(image.width)?;
            let t = crate::x87::ftol32(f64::from(image.width) * 0.25);
            let left = crate::x87::ftol32(f64::from(t) * f64::from(info.frame as u8));
            let right = crate::x87::ftol32(f64::from(t) * f64::from(info.frame as u8) + f64::from(t));
            (right.wrapping_sub(left), image.height, side, left, right, 0.0, 0.3)
        } else {
            info.display_size = [32.0; 2];
            (image.width, image.height, 64, 0, image.width, 0.3, 0.0)
        }
    } else if info.has(flags::DISPLAY) {
        let font = host.font(0x6896_3c4c).ok_or(Unsupported::MissingFont)?;
        host.resource_op(ResourceOp::Acquire(Reference::Font, font));
        let old_style = host.font_style(font);
        host.resource_op(ResourceOp::FontStyle(font, 3));
        let text = (info.number as i32).to_string();
        let [width, height] = host.measure_text(font, &text);
        let side = if width > 32 || height > 32 {
            power_of_two(width.max(height))
        } else {
            Ok(32)
        };
        if let Ok(side) = side {
            if let Some(surface) = host.create_surface() {
                if host.init_surface(surface, side, 9) {
                    if host.lock_surface(surface, 0x8001) {
                        host.resource_op(ResourceOp::ClearSurface(surface, 0, 0));
                        host.resource_op(ResourceOp::UnlockSurface(surface, 0x8001));
                    }
                    host.resource_op(ResourceOp::DrawText {
                        font,
                        surface,
                        text,
                        colour: 0xff00_0000,
                        flags: 0x0800_0000,
                    });
                    if host.texture_service_present() {
                        info.texture = host.setup_texture(surface);
                    }
                }
                host.resource_op(ResourceOp::Release(Reference::Surface, surface));
            }
        }
        host.resource_op(ResourceOp::FontStyle(font, old_style));
        host.resource_op(ResourceOp::Release(Reference::Font, font));
        info.display_size = [width as f32, 14.0];
        (width, height, side?, 0, width, 0.0, 0.3)
    } else {
        info.display_size = [0.0; 2];
        (0, 0, 32, 0, 0, 0.0, 0.3)
    };
    let camera = host.camera();
    let table = [20.0_f32, 30.0, 40.0, 50.0, 60.0, 60.0];
    let spacing = table.get(camera.zoom as usize).ok_or(Unsupported::Zoom(camera.zoom))?;
    let scale = f64::from(camera.scale);
    let cos_pitch = f64::from(camera.pitch).cos();
    let separation = (f64::from(*spacing) * scale / cos_pitch) as f32;
    let icon_height = (64.0 * scale / cos_pitch) as f32;
    let icon_half_width = (64.0 * scale * 0.5) as f32;
    let icon_half_height = (f64::from(icon_height) * 0.5) as f32;
    let number_half_width = f64::from(width as f32) * scale * 0.5;
    let number_half_height = (f64::from((f64::from(height as f32) * scale) as f32) / cos_pitch * 0.5) as f32;
    let icon_top = (f64::from(icon_height) + f64::from(separation)) as f32;
    let number_top = ((f64::from(number_half_height) + f64::from(icon_half_height)) + f64::from(separation)) as f32;
    let number_bottom = ((f64::from(icon_half_height) + f64::from(separation)) - f64::from(number_half_height)) as f32;
    let r = (1.0 / f64::from(side)) as f32;
    let u0 = (f64::from(left as f32) * f64::from(r)) as f32;
    let u1 = (f64::from(right as f32) * f64::from(r)) as f32;
    let v0 = r * 0.0;
    let v1 = (f64::from(height as f32) * f64::from(r)) as f32;
    info.icon_quad = quad(
        -icon_half_width,
        separation,
        icon_half_width,
        icon_top,
        icon_depth,
        [0.0, 0.0, 1.0, 1.0],
    );
    if info.has(flags::DISPLAY) {
        info.display_quad = quad(
            (-number_half_width) as f32,
            number_bottom,
            number_half_width as f32,
            number_top,
            display_depth,
            [u0, v0, u1, v1],
        );
    }
    rotate_quad(&mut info.icon_quad, camera);
    if info.has(flags::DISPLAY) {
        rotate_quad(&mut info.display_quad, camera);
    }
    Ok(())
}

fn quad(left: f32, bottom: f32, right: f32, top: f32, z: f32, uv: [f32; 4]) -> Quad {
    let [u0, v0, u1, v1] = uv;
    [
        Vertex {
            position: [left, top, z],
            uv: [u0, v0],
        },
        Vertex {
            position: [left, bottom, z],
            uv: [u0, v1],
        },
        Vertex {
            position: [right, bottom, z],
            uv: [u1, v1],
        },
        Vertex {
            position: [right, top, z],
            uv: [u1, v0],
        },
    ]
}

pub(crate) fn rotate_quad(quad: &mut Quad, camera: Camera) {
    let a = f64::from(-camera.forward_x).atan2(f64::from(-camera.forward_z));
    let c = f64::from(a.cos() as f32);
    let s = f64::from(a.sin() as f32);
    for vertex in quad {
        let [x, y, z] = vertex.position.map(f64::from);
        vertex.position = [
            ((z * s + x * c) + 0.0 * y) as f32,
            ((z * -0.0 + x * 0.0) + y) as f32,
            ((z * c - x * s) + 0.0 * y) as f32,
        ];
    }
}
