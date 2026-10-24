// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

pub trait Target: Send + Sync {
    fn area(&self) -> Rect;
    fn native_rgb(&self, colour: u32) -> u32;
    fn buffer(&self) -> Option<Arc<dyn Buffer>>;
    fn supports_simgl(&self) -> bool;
}
pub trait EngineFactory {
    fn create(&mut self, target: &Arc<dyn Target>) -> Result<Box<dyn RenderEngine>>;
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attributes {
    pub aa: u8,
    pub modulation: u8,
    pub alpha: i32,
    pub thickness: f32,
    pub raster_thickness: i32,
}
impl Default for Attributes {
    fn default() -> Self {
        Self {
            aa: 0,
            modulation: 0,
            alpha: 0,
            thickness: 1.,
            raster_thickness: 1,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum StateChange {
    Clip(Rect),
    Colours([u32; 4]),
    Alpha(i32),
    Modulation(u8),
    Antialias(u8),
    Thickness(f32),
    /// Twelve binary32 words, with binary32 anchor coordinates (§4.2).
    Gradient(Option<Vec<u32>>, f32, f32),
    Tiling(Option<[u32; 4]>),
}
#[derive(Clone)]
pub struct BlitBinding {
    pub source: Option<Arc<dyn Target>>,
    pub placement: Point,
    pub scale: Point,
}
impl BlitBinding {
    pub fn clear() -> Self {
        Self {
            source: None,
            placement: [0.; 2],
            scale: [1.; 2],
        }
    }
}
pub trait RenderEngine: Send + Sync {
    fn init(&mut self, target: &Arc<dyn Target>) -> bool;
    fn shutdown(&mut self);
    fn canvas(&self) -> Rect;
    fn state(&mut self, change: StateChange) -> Result<()>;
    fn bind(&mut self, binding: BlitBinding) -> Result<Option<Rect>>;
    fn flush(&mut self) -> Result<()>;
    fn suspend(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn lock(&self) -> Result<BufferRecord>;
    fn unlock(&mut self) -> Result<()>;
    fn draw(&mut self, kind: u32, points: &[Point]) -> Result<u32>;
    /// Generic geometry algorithms are GZG1. Explicit routes keep the settled
    /// result-code protocol independent of future tessellation/coverage rules.
    fn generic(&mut self, route: GeometryRoute, kind: u32, points: &[Point]) -> Result<()>;
    /// Slot 22: clipped, half-open spans in destination coordinates.
    fn spans(&mut self, _spans: &[PixelSpan]) -> Result<()> {
        Err(Error::Unsupported("GZG2"))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryRoute {
    Outline,
    Rasterize,
    Tessellate,
}

pub struct DrawContext {
    target: Option<Arc<dyn Target>>,
    engine: Option<Box<dyn RenderEngine>>,
    pub colours: [u32; 4],
    pub native_colours: [u32; 4],
    origin: Point,
    clip: Rect,
    canvas: Rect,
    pub attributes: Attributes,
    bounds: Rect,
    active: bool,
    tiling: bool,
    suspended: u32,
    transforms: Vec<(Point, Rect)>,
    states: Vec<Attributes>,
}
impl DrawContext {
    /// Caller supplies colours because native constructor leaves them undefined.
    /// Detached coordinates are safe placeholders, not native defaults (§2.2).
    pub fn new(colours: [u32; 4]) -> Self {
        Self {
            target: None,
            engine: None,
            colours,
            native_colours: [0; 4],
            origin: [0.; 2],
            clip: Rect::default(),
            canvas: Rect::default(),
            attributes: Attributes::default(),
            bounds: Rect::default(),
            active: false,
            tiling: false,
            suspended: 0,
            transforms: Vec::new(),
            states: Vec::new(),
        }
    }
    fn engine_mut(&mut self) -> Result<&mut (dyn RenderEngine + '_)> {
        match self.engine.as_mut() {
            Some(e) => Ok(e.as_mut()),
            None => Err(Error::Detached),
        }
    }
    fn notify(&mut self, c: StateChange) -> Result<()> {
        self.engine_mut()?.state(c)
    }
    fn optional_notify(&mut self, c: StateChange) -> Result<()> {
        if let Some(e) = self.engine.as_mut() {
            e.state(c)?;
        }
        Ok(())
    }
    pub fn engine(&self) -> Option<&dyn RenderEngine> {
        self.engine.as_deref()
    }
    pub fn target(&self) -> Option<&Arc<dyn Target>> {
        self.target.as_ref()
    }
    pub fn is_target_attached(&self) -> bool {
        self.target.is_some()
    }
    pub fn origin(&self) -> Point {
        self.origin
    }
    pub fn clip_rect(&self) -> Rect {
        self.clip
    }
    pub fn canvas_area(&self) -> Rect {
        self.canvas
    }
    pub fn transform_depth(&self) -> usize {
        self.transforms.len()
    }
    pub fn blit_active(&self) -> bool {
        self.active
    }
    pub fn tiling(&self) -> bool {
        self.tiling
    }
    pub fn is_suspended(&self) -> bool {
        self.suspended > 0
    }
    pub fn set_target(&mut self, target: Option<&Arc<dyn Target>>, factory: &mut dyn EngineFactory) -> Result<()> {
        if match (&self.target, target) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        } {
            return Ok(());
        }
        if let Some(mut e) = self.engine.take() {
            e.shutdown();
        }
        // Acquire replacement before releasing old target (§3.4).
        self.target = target.cloned();
        let Some(t) = self.target.clone() else {
            return Ok(());
        };
        self.origin = [0.; 2];
        self.clip = t.area();
        let result = (|| {
            let mut e = factory.create(&t)?;
            if !e.init(&t) {
                e.shutdown();
                return Err(Error::Foreign);
            }
            self.engine = Some(e);
            self.notify(StateChange::Clip(self.clip))?;
            self.native_colours = self.colours.map(|c| t.native_rgb(c));
            self.notify(StateChange::Colours(self.colours))?;
            self.notify(StateChange::Alpha(self.attributes.alpha))?;
            self.notify(StateChange::Modulation(self.attributes.modulation))?;
            self.notify(StateChange::Antialias(self.attributes.aa))?;
            self.notify(StateChange::Thickness(self.attributes.thickness))?;
            self.engine_mut()?.bind(BlitBinding::clear())?;
            self.notify(StateChange::Gradient(None, 0., 0.))?;
            self.canvas = self.engine_mut()?.canvas();
            self.active = false;
            self.tiling = false;
            self.suspended = 0;
            Ok(())
        })();
        if result.is_err() {
            if let Some(mut e) = self.engine.take() {
                e.shutdown();
            }
            self.target = None;
        }
        result
    }
    pub fn set_colour(&mut self, index: usize, colour: u32) -> Result<()> {
        if index >= 4 {
            return Err(Error::InvalidInput);
        }
        let t = self.target.as_ref().ok_or(Error::Detached)?;
        self.colours[index] = colour;
        self.native_colours[index] = t.native_rgb(colour);
        self.optional_notify(StateChange::Colours(self.colours))
    }
    pub fn set_alpha(&mut self, v: i32) -> Result<()> {
        if self.attributes.alpha != v {
            self.attributes.alpha = v;
            self.optional_notify(StateChange::Alpha(v))?;
        }
        Ok(())
    }
    pub fn set_modulation(&mut self, v: u8) -> Result<()> {
        if self.attributes.modulation != v {
            self.attributes.modulation = v;
            self.optional_notify(StateChange::Modulation(v))?;
        }
        Ok(())
    }
    pub fn set_antialias(&mut self, v: u8) -> Result<()> {
        if self.attributes.aa != v {
            self.attributes.aa = v;
            self.optional_notify(StateChange::Antialias(v))?;
        }
        Ok(())
    }
    pub fn set_thickness(&mut self, v: f32) -> Result<()> {
        if !v.is_finite() || v > (i32::MAX as f32 - 256.) {
            return Err(Error::InvalidInput);
        }
        if self.attributes.thickness != v {
            self.attributes.thickness = v.max(0.);
            self.attributes.raster_thickness = ((self.attributes.thickness + 0.5) as i32).max(1);
            self.optional_notify(StateChange::Thickness(self.attributes.thickness))?;
        }
        Ok(())
    }
    pub fn gradient(&mut self, words: Option<Vec<u32>>, x: f32, y: f32) -> Result<()> {
        self.notify(StateChange::Gradient(words, x, y))
    }
    pub fn push_state(&mut self) {
        self.states.push(self.attributes);
    }
    pub fn pop_state(&mut self) -> Result<()> {
        if let Some(a) = self.states.pop() {
            self.set_antialias(a.aa)?;
            self.set_modulation(a.modulation)?;
            self.set_alpha(a.alpha)?;
            self.set_thickness(a.thickness)?;
            self.attributes.raster_thickness = a.raster_thickness;
        }
        Ok(())
    }
    pub fn reset_defaults(&mut self) -> Result<()> {
        self.set_alpha(0)?;
        self.set_modulation(0)?;
        self.set_antialias(0)?;
        self.set_thickness(1.)?;
        self.clear_blit()?;
        self.clear_tiling()?;
        self.gradient(None, 0., 0.)
    }
    pub fn offset(&mut self, x: i32, y: i32) {
        self.origin[0] += x as f32;
        self.origin[1] += y as f32;
    }
    pub fn offset_absolute(&mut self, x: i32, y: i32) {
        self.origin = [x as f32, y as f32];
    }
    pub fn push_transform(&mut self) {
        self.transforms.push((self.origin, self.clip));
    }
    pub fn pop_transform(&mut self) -> Result<()> {
        if let Some((o, c)) = self.transforms.pop() {
            self.origin = o;
            self.clip = c;
            self.notify(StateChange::Clip(c))?;
        }
        Ok(())
    }
    pub fn clip_reset(&mut self) {
        self.clip = self.canvas;
    }
    fn effective_clip(&self) -> Rect {
        if self.active && !self.tiling {
            self.clip.intersect(self.bounds)
        } else {
            self.clip
        }
    }
    pub fn clip(&mut self, r: Rect) -> Result<()> {
        let [x, y] = self.origin.map(|v| v as i32);
        self.clip = self.clip.intersect(Rect(
            r.0.wrapping_add(x),
            r.1.wrapping_add(y),
            r.2.wrapping_add(x),
            r.3.wrapping_add(y),
        ));
        self.notify(StateChange::Clip(self.effective_clip()))
    }
    pub fn clip_absolute(&mut self, r: Rect) -> Result<()> {
        self.clip = self.clip.intersect(r);
        self.notify(StateChange::Clip(self.clip))
    }
    pub fn set_blit(&mut self, source: Arc<dyn Target>, placement: Point, scale: Point) -> Result<bool> {
        let b = BlitBinding {
            source: Some(source),
            placement: [placement[0] + self.origin[0], placement[1] + self.origin[1]],
            scale,
        };
        let bounds = self.engine_mut()?.bind(b)?;
        self.active = bounds.is_some();
        if let Some(r) = bounds {
            self.bounds = r;
        }
        self.notify(StateChange::Clip(self.effective_clip()))?;
        Ok(self.active)
    }
    pub fn clear_blit(&mut self) -> Result<()> {
        self.engine_mut()?.bind(BlitBinding::clear())?;
        self.active = false;
        self.notify(StateChange::Clip(self.clip))
    }
    pub fn set_tiling(&mut self, words: [u32; 4]) -> Result<()> {
        self.tiling = true;
        self.notify(StateChange::Tiling(Some(words)))?;
        self.notify(StateChange::Clip(self.clip))
    }
    pub fn clear_tiling(&mut self) -> Result<()> {
        self.tiling = false;
        self.notify(StateChange::Tiling(None))?;
        if self.active {
            self.notify(StateChange::Clip(self.effective_clip()))?;
        }
        Ok(())
    }
    pub fn flush(&mut self) -> Result<()> {
        self.engine_mut()?.flush()
    }
    pub fn suspend(&mut self) -> Result<()> {
        self.suspended = self.suspended.checked_add(1).ok_or(Error::InvalidInput)?;
        self.flush()?;
        self.engine_mut()?.suspend()
    }
    pub fn resume(&mut self) -> Result<()> {
        if self.suspended > 0 {
            self.suspended -= 1;
            if self.suspended == 0 {
                self.engine_mut()?.resume()?;
            }
        }
        Ok(())
    }
    pub fn lock(&self) -> Result<BufferRecord> {
        self.engine.as_ref().ok_or(Error::Detached)?.lock()
    }
    pub fn unlock(&mut self) -> Result<()> {
        self.engine_mut()?.unlock()
    }
    pub fn draw(&mut self, kind: u32, points: &[Point]) -> Result<()> {
        let p: Vec<_> = points
            .iter()
            .map(|p| [p[0] + self.origin[0], p[1] + self.origin[1]])
            .collect();
        self.draw_no_transform(kind, &p)
    }
    pub fn draw_no_transform(&mut self, kind: u32, points: &[Point]) -> Result<()> {
        if kind > 8 {
            return Ok(());
        }
        let e = self.engine_mut()?;
        if kind < 6 && kind != 3 {
            return e.generic(GeometryRoute::Outline, kind, points);
        }
        match e.draw(kind, points)? {
            2 => e.generic(GeometryRoute::Rasterize, kind, points),
            1 if kind == 7 || kind == 8 => e.generic(GeometryRoute::Tessellate, kind, points),
            1 if kind == 3 => e.generic(GeometryRoute::Tessellate, kind, points),
            _ => Ok(()),
        }
    }
    pub fn point(&mut self, x: i32, y: i32) -> Result<()> {
        self.draw(0, &[[x as f32 + 0.5, y as f32 + 0.5]])
    }
    pub fn line(&mut self, a: [i32; 2], b: [i32; 2]) -> Result<()> {
        self.draw(
            1,
            &[
                [a[0] as f32 + 0.5, a[1] as f32 + 0.5],
                [b[0] as f32 + 0.5, b[1] as f32 + 0.5],
            ],
        )
    }
    pub fn fill(&mut self, r: Rect) -> Result<()> {
        self.draw(3, &r.points())
    }
    pub fn clear(&mut self) -> Result<()> {
        self.draw_no_transform(3, &self.clip.points())
    }
    pub fn convenience_blit(&mut self, source: Arc<dyn Target>, src: Rect, dest: Rect) -> Result<()> {
        if src.width() <= 0 || src.height() <= 0 || dest.width() <= 0 || dest.height() <= 0 {
            return Ok(());
        }
        self.set_blit(
            source,
            [dest.0.wrapping_sub(src.0) as f32, dest.1.wrapping_sub(src.1) as f32],
            [
                (dest.width() as f64 / src.width() as f64) as f32,
                (dest.height() as f64 / src.height() as f64) as f32,
            ],
        )?;
        self.clear_tiling()?;
        let drawing = self.fill(dest);
        let cleanup = self.clear_blit();
        drawing.and(cleanup)
    }
    pub fn blit_tiled(&mut self, source: Arc<dyn Target>, tile: Rect, dest: Rect, offset: [i32; 2]) -> Result<()> {
        if tile.width() <= 0 || tile.height() <= 0 || dest.width() <= 0 || dest.height() <= 0 {
            return Ok(());
        }
        self.set_blit(
            source,
            [
                dest.0.wrapping_add(offset[0]) as f32,
                dest.1.wrapping_add(offset[1]) as f32,
            ],
            [1.; 2],
        )?;
        self.set_tiling([tile.0 as u32, tile.1 as u32, tile.2 as u32, tile.3 as u32])?;
        let drawing = self.fill(dest);
        let tiling = self.clear_tiling();
        let cleanup = self.clear_blit();
        drawing.and(tiling).and(cleanup)
    }
    pub fn ellipse(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<()> {
        self.draw(5, &[[x0 as f32, y0 as f32], [x1 as f32, y1 as f32]])
    }
    pub fn filled_ellipse(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<()> {
        self.ellipse(x0, y0, x1, y1)
    }
}
impl Drop for DrawContext {
    fn drop(&mut self) {
        // §2.1 / V45: shutdown precedes vector frees and independent releases.
        if let Some(engine) = self.engine.as_mut() {
            engine.shutdown();
        }
        self.transforms = Vec::new();
        self.states = Vec::new();
        drop(self.engine.take());
        drop(self.target.take());
    }
}
