// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Font {
    mode: u32,
    w: i32,
    h: i32,
    draws: Vec<(i32, i32, u32)>,
}
impl TextFont for Font {
    fn mode(&self) -> u32 {
        self.mode
    }
    fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }
    fn width(&mut self, _text: &str) -> i32 {
        assert_eq!(self.mode, 3);
        self.w
    }
    fn height(&mut self) -> i32 {
        assert_eq!(self.mode, 3);
        self.h
    }
    fn draw(&mut self, text: &str, surface: &mut Surface, x: i32, y: i32, colour: u32) {
        assert_eq!(text, "test");
        assert_eq!(colour, 0xFFFF_FFFF);
        assert!(surface.pixels.iter().all(|p| *p == [0; 4]));
        self.draws.push((x, y, self.mode));
        // Deliberately draw through the border; the utility must erase it.
        for (i, pixel) in surface.pixels.iter_mut().enumerate() {
            *pixel = [7, 13, 23, (i as u8).wrapping_mul(17)];
        }
    }
}
struct Host {
    default: Option<Font>,
    image: Option<TextureImage>,
    surface_failure: bool,
    upload_result: bool,
    requests: Vec<(u32, u32, u32)>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            default: None,
            image: None,
            surface_failure: false,
            upload_result: true,
            requests: vec![],
        }
    }
}
impl TextureHost for Host {
    type Font = Font;
    type Texture = usize;
    fn default_font(&mut self, id: u32) -> Option<Font> {
        assert_eq!(id, 0x6896_3C4C);
        self.default.take()
    }
    fn create_surface(&mut self, w: u32, h: u32, code: u32) -> Option<Surface> {
        self.requests.push((w, h, code));
        if self.surface_failure {
            None
        } else {
            let mut surface = Surface::new(w, h)?;
            surface.pixels.fill([255; 4]);
            Some(surface)
        }
    }
    fn upload(&mut self, image: &TextureImage, texture: &mut usize) -> bool {
        self.image = Some(image.clone());
        *texture += 1;
        self.upload_result
    }
}

#[test]
fn published_text_texture_layout_plain_pixels_and_font_restore() {
    let mut host = Host::default();
    let mut font = Font {
        mode: 7,
        w: 30,
        h: 12,
        ..Font::default()
    };
    let mut size = [99; 2];
    let mut rect = [99.; 4];
    let mut texture = 0;
    assert!(create_texture_from_text(
        "test",
        &mut texture,
        Some(&mut font),
        Some(&mut size),
        Some(&mut rect),
        false,
        &mut host
    ));
    assert_eq!(font.mode, 7);
    assert_eq!(font.draws, [(2, 1, 3)]);
    assert_eq!(size, [32, 16]);
    assert_eq!(rect, [32., 13., 2., 1.]);
    assert_eq!(host.requests, [(32, 16, 9)]);
    assert_eq!(texture, 1);
    let image = host.image.unwrap();
    assert_eq!((image.format, image.element_type), (3, 1));
    assert_eq!(image.bytes.len(), 32 * 16 * 4);
    for y in 0..16 {
        for x in 0..32 {
            let at = (y * 32 + x) * 4;
            if x == 0 || x == 31 || y == 0 || y == 15 {
                assert_eq!(&image.bytes[at..at + 4], [0; 4]);
            } else {
                assert_eq!(&image.bytes[at..at + 3], [7, 13, 23]);
            }
        }
    }
}

#[test]
fn dxt3_alpha_nibbles_and_white_colour_blocks() {
    let mut host = Host {
        default: Some(Font {
            w: 2,
            h: 2,
            ..Font::default()
        }),
        ..Host::default()
    };
    assert!(create_texture_from_text(
        "test", &mut 0, None, None, None, true, &mut host
    ));
    let image = host.image.unwrap();
    assert_eq!((image.width, image.height, image.format), (4, 4, 9));
    assert_eq!(
        image.bytes,
        [0, 0, 0x50, 0x06, 0x90, 0x0a, 0, 0, 0, 0, 255, 255, 0x55, 0x55, 0x55, 0x55]
    );
    let mut surface = Surface::new(8, 4).unwrap();
    surface
        .pixels
        .iter_mut()
        .enumerate()
        .for_each(|(i, p)| p[3] = if i % 8 < 4 { 0x9f } else { 0x27 });
    let encoded = encode_surface(&surface, true).unwrap();
    assert_eq!(&encoded.bytes[..8], [0x99; 8]);
    assert_eq!(&encoded.bytes[16..24], [0x22; 8]);
    assert!(encode_surface(&Surface::new(2, 4).unwrap(), true).is_none());
    surface.pixel_code = 8;
    assert!(encode_surface(&surface, true).is_none());
}

#[test]
fn text_failure_paths_and_published_common_output_policy() {
    let mut host = Host::default();
    let mut texture = 0;
    let mut size = [99; 2];
    assert!(!create_texture_from_text(
        "test",
        &mut texture,
        None,
        Some(&mut size),
        None,
        false,
        &mut host
    ));
    assert_eq!(size, [99; 2]);
    assert!(host.requests.is_empty());
    let mut font = Font {
        mode: 9,
        w: 30,
        h: 12,
        ..Font::default()
    };
    host.surface_failure = true;
    assert!(!create_texture_from_text(
        "test",
        &mut texture,
        Some(&mut font),
        Some(&mut size),
        None,
        false,
        &mut host
    ));
    assert_eq!(font.mode, 9);
    // Public GU4 DIFFERS: surface failure also writes dimensions.
    assert_eq!(size, [32, 16]);
    assert!(font.draws.is_empty());
    host.surface_failure = false;
    host.upload_result = false;
    // Public integer wrapping domain includes negative width -1 -> dimension 1.
    assert!(create_texture_from_text(
        "test",
        &mut texture,
        Some(&mut font),
        Some(&mut size),
        None,
        false,
        &mut host
    ));
    assert_eq!(font.mode, 9);
    assert_eq!(size, [32, 16]);
    assert_eq!(texture, 1);
    font.w = -1;
    host.image = None;
    host.upload_result = true;
    assert!(create_texture_from_text(
        "test",
        &mut texture,
        Some(&mut font),
        None,
        None,
        false,
        &mut host
    ));
    assert_eq!(font.mode, 9);
    assert!(host.image.is_some());
}

#[test]
fn small_text_truncated_offsets_and_compression_rejection() {
    let mut host = Host::default();
    let mut font = Font {
        w: 1,
        h: 1,
        ..Font::default()
    };
    assert!(create_texture_from_text(
        "test",
        &mut 0,
        Some(&mut font),
        None,
        None,
        false,
        &mut host
    ));
    assert_eq!(font.draws, [(2, 0, 3)]);
    font.w = 0;
    font.h = 0;
    host.image = None;
    assert!(!create_texture_from_text(
        "test",
        &mut 0,
        Some(&mut font),
        None,
        None,
        true,
        &mut host
    ));
    assert!(host.image.is_none());
    assert_eq!(font.mode, 0);
}
