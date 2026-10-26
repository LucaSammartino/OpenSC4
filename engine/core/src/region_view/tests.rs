// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use image::SurfaceHost;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Recording {
    next: u64,
    pixels: BTreeMap<Surface, (i32, i32, Vec<u32>)>,
    pub events: Vec<String>,
    pub records: BTreeMap<(i32, i32), CityRecord>,
    pub fail_size: bool,
    pub fail_lock: bool,
    pub tutorial: Option<(i32, i32)>,
}

impl Recording {
    pub fn image(&mut self, w: i32, h: i32, pixels: Vec<u32>) -> Surface {
        self.next += 1;
        self.pixels.insert(self.next, (w, h, pixels));
        self.next
    }
}

impl base::Host for Recording {}
impl SurfaceHost for Recording {
    fn dimensions(&self, image: Surface) -> (i32, i32) {
        let p = &self.pixels[&image];
        (p.0, p.1)
    }
    fn lock(&mut self, image: Surface, code: u32) -> bool {
        self.events.push(format!("lock:{image}:{code:x}"));
        !self.fail_lock
    }
    fn unlock(&mut self, image: Surface, code: u32) {
        self.events.push(format!("unlock:{image}:{code:x}"));
    }
    fn pixel(&self, image: Surface, x: i32, y: i32) -> u32 {
        let p = &self.pixels[&image];
        p.2[(y * p.0 + x) as usize]
    }
    fn set_pixel(&mut self, image: Surface, x: i32, y: i32, colour: u32) {
        let p = self.pixels.get_mut(&image).unwrap();
        p.2[(y * p.0 + x) as usize] = colour;
    }
    fn fill(&mut self, image: Surface, colour: u32) {
        self.pixels.get_mut(&image).unwrap().2.fill(colour);
    }
    fn tiled_background(&mut self, _: Surface, _: Surface, offset: (i32, i32)) {
        self.events.push(format!("background:{offset:?}"));
    }
}
impl CanvasHost for Recording {
    fn display_depth(&mut self) -> u32 {
        32
    }
    fn create_surface(&mut self) -> Option<Surface> {
        Some(self.image(0, 0, Vec::new()))
    }
    fn size_surface(&mut self, surface: Surface, w: i32, h: i32, format: u32, bits: u32) -> bool {
        self.events.push(format!("size:{format}:{bits}"));
        if self.fail_size {
            return false;
        }
        self.pixels.insert(surface, (w, h, vec![0; (w * h) as usize]));
        true
    }
    fn shutdown_surface(&mut self, surface: Surface) {
        self.events.push(format!("shutdown:{surface}"));
    }
    fn release_surface(&mut self, surface: Surface) {
        self.events.push(format!("release:{surface}"));
        self.pixels.remove(&surface);
    }
    fn blit(&mut self, _: Surface, _: Surface, _: Rect, dst: Rect, clip: Rect) {
        self.events.push(format!("blit:{dst:?}:{clip:?}"));
    }
}
impl Host for Recording {
    fn load_image(&mut self, _: (u32, u32, u32, u32, u32)) -> Option<Surface> {
        Some(self.image(1, 1, vec![0xff00_0000]))
    }
    fn register_canvas(&mut self, on: bool) {
        self.events.push(format!("register:{on}"));
    }
    fn redraw(&mut self) {
        self.events.push("redraw".into());
    }
    fn child_at_screen(&mut self, _: Window, _: (i32, i32)) -> bool {
        false
    }
    fn city_record(&mut self, _: RegionKey, cell: (i32, i32)) -> Option<CityRecord> {
        self.records.get(&cell).cloned()
    }
    fn hide_tutorial_mayor(&mut self, _: i32) -> bool {
        false
    }
    fn tutorial_tile(&mut self, _: RegionKey) -> Option<(i32, i32)> {
        self.tutorial
    }
    fn create_effect(&mut self, name: &str) -> Option<Handle> {
        self.next += 1;
        self.events.push(format!("effect:{name}"));
        Some(self.next)
    }
    fn start_effect(&mut self, _: Handle, position: [f32; 3]) {
        self.events.push(format!("start:{position:?}"));
    }
    fn stop_release_effect(&mut self, effect: Handle) {
        self.events.push(format!("stop:{effect}"));
    }
    fn remove_release_signpost(&mut self, _: Window, signpost: Window) {
        self.events.push(format!("remove:{signpost}"));
    }
    fn create_signpost(&mut self, _: Window, instance: u32, _: u32) -> Option<Window> {
        self.next += 1;
        self.events.push(format!("signpost:{instance:x}"));
        Some(self.next)
    }
    fn signpost_size(&mut self, _: Window) -> (i32, i32) {
        (20, 10)
    }
    fn move_signpost(&mut self, _: Window, position: (i32, i32)) {
        self.events.push(format!("move:{position:?}"));
    }
    fn fill_signpost(&mut self, _: Window, _: &SignpostData) {}
    fn font(&mut self, _: u32) -> Option<Handle> {
        Some(1000)
    }
    fn font_height(&mut self, _: Handle) -> i32 {
        8
    }
    fn layout_text(&mut self, _: Handle, _: &mut Vec<u8>, _: Rect, _: u32) {}
    fn text_width(&mut self, _: Handle, text: &[u8]) -> i32 {
        text.len() as i32
    }
    fn draw_control(&mut self, _: Surface, _: Draw) {
        self.events.push("text".into());
    }
}

#[test]
fn rgv_c1_c2_c3_canvas_worked_cases() {
    let mut host = Recording::default();
    let mut canvas = Canvas::default();
    assert!(canvas.init(&mut host, 800, 600, 256, 256));
    assert_eq!((canvas.columns, canvas.rows), (5, 4));
    for tile in &mut canvas.tiles {
        tile.dirty = false;
    }
    canvas.invalidate_rect(Rect(0, 0, 256, 256));
    assert_eq!(canvas.tiles.iter().filter(|t| t.dirty).count(), 1);
    assert!(canvas.tiles[0].dirty);
    for tile in &mut canvas.tiles {
        tile.dirty = false;
    }
    let old = canvas.tiles.clone();
    canvas.scroll_to(300, -10);
    assert_eq!(canvas.origin, (1, -1));
    assert_eq!(canvas.offset, (44, 246));
    for row in 0..4 {
        for column in 0..5 {
            let tile = canvas.tiles[row * 5 + column];
            assert_eq!(tile.dirty, column == 4 || row == 0);
            assert_eq!(tile.surface, old[((row + 3) % 4) * 5 + (column + 1) % 5].surface);
        }
    }
}

#[test]
fn rgv_c4_runs_and_reverse_overlap() {
    assert!(image::hit(&[0x0003_0002, 0x0003_0006], 5, 3));
    assert!(!image::hit(&[0x0003_0002, 0x0003_0006], 6, 3));
    let mut host = Recording::default();
    let surface = host.image(8, 5, vec![0; 40]);
    let mut city = CityEntry::new(1, (0, 0), 0, surface);
    city.position = (-0.2, 5.9);
    city.hit_runs = vec![0x0003_0002, 0x0003_0006];
    let mut view = RegionView::new(1);
    view.cities.push(city.clone());
    city.id = 2;
    view.cities.push(city);
    assert_eq!(view.draw_position(&host, &view.cities[0]), (-1, 0));
    assert_eq!(view.city_at_point(&host, 4, 3), Some(2));
    assert_eq!(view.city_at_point(&host, 5, 3), None);
}

#[test]
fn failed_canvas_init_leaves_view_initialised_and_retry_succeeds_without_canvas() {
    let mut host = Recording {
        fail_size: true,
        ..Default::default()
    };
    let mut windows = Windows::new(1);
    let window = windows.create_default(&mut host);
    windows.set_area(&mut host, window, Rect(0, 0, 800, 600));
    let mut view = RegionView::new(window);
    assert!(!view.init(&mut windows, &mut host));
    assert!(windows.get(window).flag(0x4000));
    assert!(!view.canvas.as_ref().unwrap().renderer);
    assert!(view.init(&mut windows, &mut host));
    assert!(!host.events.iter().any(|e| e == "register:true"));
    view.shutdown(&mut windows, &mut host);
    assert!(!windows.get(window).flag(0x4000));
    assert_eq!(host.events.iter().filter(|e| e.starts_with("release:")).count(), 1);
}

#[test]
fn canvas_render_cleans_once_and_lock_failure_does_no_work() {
    let mut host = Recording::default();
    let mut canvas = Canvas::default();
    assert!(canvas.init(&mut host, 2, 2, 2, 2));
    let dst = host.image(2, 2, vec![0; 4]);
    let mut renders = Vec::new();
    assert_eq!(
        canvas.render(&mut host, dst, 7, 9, |_, _, x, y| {
            renders.push((x, y));
            Ok(())
        }),
        Ok(true)
    );
    assert_eq!(renders, [(0, 0), (2, 0), (0, 2), (2, 2)]);
    canvas
        .render(&mut host, dst, 0, 0, |_, _, _, _| panic!("clean tile"))
        .unwrap();
    host.fail_lock = true;
    canvas.invalidate_all();
    assert_eq!(
        canvas.render(&mut host, dst, 0, 0, |_, _, _, _| panic!("failed lock")),
        Ok(false)
    );
    assert!(canvas.tiles.iter().all(|t| t.dirty));
}

#[test]
fn image_edges_runs_tint_lock_quirk_and_clipped_blend() {
    let mut host = Recording::default();
    let src = host.image(3, 1, vec![0, 0xff10_2030, 0]);
    let mut runs = vec![123];
    image::build_runs(&mut host, src, &mut runs).unwrap();
    assert_eq!(host.pixel(src, 0, 0), 0xfd10_2030); // Two transparent-over steps each attenuate alpha.
    assert_eq!(runs, [0, 1, 1, 2]); // partial and opaque are separate runs.
    let dst = host.image(3, 1, vec![0; 3]);
    image::copy_and_tint(
        &mut host,
        src,
        dst,
        Some(&[image::Coverage {
            x: 1,
            y: 0,
            values: vec![255],
        }]),
        0x00ff_ffff,
    )
    .unwrap();
    assert_eq!(host.pixel(dst, 1, 0), 0xff87_8f97);
    assert!(host.events.contains(&format!("lock:{src}:800")));
    assert!(!host.events.contains(&format!("unlock:{src}:800")));
    let target = host.image(2, 1, vec![0xff00_0000; 2]);
    image::blit_image_spans(&mut host, src, target, Rect(0, 0, 3, 1), Rect(-1, 0, 2, 1), &runs).unwrap();
    assert_eq!(host.pixel(target, 0, 0), 0xff10_2030);
    assert_eq!(host.pixel(target, 1, 0), 0xff00_0000);
}
