// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Result, Unsupported};
use crate::ui_windows::base::Rect;

pub type Surface = u64;

/// Created surfaces belong to Canvas; image handles supplied by city entries are borrowed.
pub trait CanvasHost: super::image::SurfaceHost {
    fn display_depth(&mut self) -> u32;
    fn create_surface(&mut self) -> Option<Surface>;
    fn size_surface(&mut self, surface: Surface, width: i32, height: i32, format: u32, bits: u32) -> bool;
    fn shutdown_surface(&mut self, surface: Surface);
    fn release_surface(&mut self, surface: Surface);
    fn blit(&mut self, destination: Surface, source: Surface, src: Rect, dst: Rect, clip: Rect);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOp {
    Renderer(u32),
    Begin2d,
    Clip(Rect),
    End2d,
    Present2d,
    Viewport(Rect),
    DefaultRenderState,
}

pub trait UpdateHost: CanvasHost {
    /// The device's AB300B2B interface, height and viewport, when available.
    fn update_device(&mut self) -> (Option<Surface>, i32, Rect);
    fn update_operation(&mut self, operation: UpdateOp);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub surface: Surface,
    pub dirty: bool,
}

#[derive(Debug, Default)]
pub struct Canvas {
    pub tiles: Vec<Tile>,
    pub origin: (i32, i32),
    pub offset: (i32, i32),
    pub tile_size: (i32, i32),
    pub view_size: (i32, i32),
    pub columns: i32,
    pub rows: i32,
    pub renderer: bool,
}

impl Canvas {
    pub fn update<H: UpdateHost>(
        &mut self,
        host: &mut H,
        render_tile: impl FnMut(&mut H, Surface, i32, i32) -> Result<()>,
    ) -> Result<()> {
        host.update_operation(UpdateOp::Renderer(26));
        let (device, height, vp) = host.update_device();
        let mut result = Ok(());
        if let Some(device) = device {
            host.update_operation(UpdateOp::Begin2d);
            host.update_operation(UpdateOp::Clip(Rect(vp.0, height - vp.3, vp.2, height - vp.1)));
            result = self.render(host, device, 0, 0, render_tile).map(|_| ());
            host.update_operation(UpdateOp::End2d);
            host.update_operation(UpdateOp::Present2d);
        }
        host.update_operation(UpdateOp::Renderer(24));
        host.update_operation(UpdateOp::Viewport(vp));
        host.update_operation(UpdateOp::DefaultRenderState);
        result
    }

    pub fn init(&mut self, host: &mut impl CanvasHost, width: i32, height: i32, tw: i32, th: i32) -> bool {
        self.view_size = (width, height);
        self.tile_size = (tw, th);
        // SPEC-GAP: non-positive canvas dimensions have no specified native contract.
        if width <= 0 || height <= 0 || tw <= 0 || th <= 0 {
            self.shutdown(host);
            return false;
        }
        self.columns = (width - 1) / tw + 2;
        self.rows = (height - 1) / th + 2;
        self.renderer = true;
        let format = if host.display_depth() > 16 { 9 } else { 4 };
        let bits = if format == 9 { 32 } else { 16 };
        // Existing surfaces are released before replacement, never retained twice.
        for tile in self.tiles.drain(..) {
            host.release_surface(tile.surface);
        }
        for _ in 0..i64::from(self.columns) * i64::from(self.rows) {
            let Some(surface) = host.create_surface() else {
                self.shutdown(host);
                return false;
            };
            self.tiles.push(Tile { surface, dirty: true });
            if !host.size_surface(surface, tw, th, format, bits) {
                self.shutdown(host);
                return false;
            }
        }
        self.origin = (0, 0);
        self.offset = (0, 0);
        true
    }

    pub fn shutdown(&mut self, host: &mut impl CanvasHost) {
        for tile in self.tiles.drain(..) {
            host.shutdown_surface(tile.surface);
            host.release_surface(tile.surface);
        }
        self.renderer = false;
    }

    pub fn invalidate_all(&mut self) {
        for tile in &mut self.tiles {
            tile.dirty = true;
        }
    }

    pub fn invalidate_rect(&mut self, rect: Rect) {
        if self.tiles.is_empty() || rect.width() <= 0 || rect.height() <= 0 {
            return;
        }
        let (tw, th) = self.tile_size;
        let (ox, oy) = self.origin;
        let x0 = (rect.0 - ox * tw).div_euclid(tw).max(0);
        let y0 = (rect.1 - oy * th).div_euclid(th).max(0);
        let x1 = (rect.2 - ox * tw - 1).div_euclid(tw).min(self.columns - 1);
        let y1 = (rect.3 - oy * th - 1).div_euclid(th).min(self.rows - 1);
        for row in y0..=y1 {
            for column in x0..=x1 {
                self.tiles[(row * self.columns + column) as usize].dirty = true;
            }
        }
    }

    pub fn scroll_to(&mut self, x: i32, y: i32) {
        if self.tiles.is_empty() {
            return;
        }
        let (tw, th) = self.tile_size;
        let tx = x.div_euclid(tw);
        let ty = y.div_euclid(th);
        let dx = tx - self.origin.0;
        let dy = ty - self.origin.1;
        if dx.abs() < self.columns && dy.abs() < self.rows {
            let old = self.tiles.clone();
            for row in 0..self.rows {
                for column in 0..self.columns {
                    let sx = column + dx;
                    let sy = row + dy;
                    let mut tile =
                        old[(sy.rem_euclid(self.rows) * self.columns + sx.rem_euclid(self.columns)) as usize];
                    tile.dirty |= sx < 0 || sx >= self.columns || sy < 0 || sy >= self.rows;
                    self.tiles[(row * self.columns + column) as usize] = tile;
                }
            }
        } else {
            self.invalidate_all();
        }
        self.origin = (tx, ty);
        self.offset = (x - tx * tw, y - ty * th);
    }

    pub fn render<H: CanvasHost>(
        &mut self,
        host: &mut H,
        destination: Surface,
        dx: i32,
        dy: i32,
        mut render_tile: impl FnMut(&mut H, Surface, i32, i32) -> Result<()>,
    ) -> Result<bool> {
        if !self.renderer {
            return Err(Unsupported::Precondition("canvas renderer"));
        }
        if !host.lock(destination, 0x8010) {
            return Ok(false);
        }
        let clip = Rect(dx, dy, dx + self.view_size.0, dy + self.view_size.1);
        for (index, tile) in self.tiles.iter_mut().enumerate() {
            let column = index as i32 % self.columns;
            let row = index as i32 / self.columns;
            let (tw, th) = self.tile_size;
            if tile.dirty {
                if let Err(error) = render_tile(
                    host,
                    tile.surface,
                    (self.origin.0 + column) * tw,
                    (self.origin.1 + row) * th,
                ) {
                    host.unlock(destination, 0x8010);
                    return Err(error);
                }
                tile.dirty = false;
            }
            let x = tw * column - self.offset.0;
            let y = th * row - self.offset.1;
            // Compatibility: dx/dy affect the clip, not the tile destination.
            host.blit(
                destination,
                tile.surface,
                host_rect(tw, th),
                Rect(x, y, x + tw, y + th),
                clip,
            );
        }
        host.unlock(destination, 0x8010);
        Ok(true)
    }
}

fn host_rect(width: i32, height: i32) -> Rect {
    Rect(0, 0, width, height)
}
