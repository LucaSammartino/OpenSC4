// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use image::SurfaceHost;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Recording {
    pub(crate) next: u64,
    pub(crate) pixels: BTreeMap<Surface, (i32, i32, Vec<u32>)>,
    pub events: Vec<String>,
    pub records: BTreeMap<(i32, i32), CityRecord>,
    pub fail_size: bool,
    pub fail_lock: bool,
    pub tutorial: Option<(i32, i32)>,
    pub children_8810: bool,
    pub load_succeeds: bool,
    pub new_succeeds: bool,
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
    fn signpost_has_child(&mut self, _: Window, _: u32) -> bool {
        true
    }
    fn signpost_children(&mut self, _: Window) -> Vec<Window> {
        if self.children_8810 {
            vec![999]
        } else {
            vec![]
        }
    }
    fn clear_child_flag(&mut self, _: Window, _: u32) {
        self.events.push("child-flag".into());
    }
    fn child_has_8810(&mut self, _: Window) -> bool {
        self.children_8810
    }
    fn install_animator(&mut self, _: Window) {
        self.events.push("animator".into());
    }
    fn manager_state(&mut self) -> Handle {
        1
    }
    fn manager_state_active(&mut self, _: Handle) -> bool {
        true
    }
    fn restore_manager_state(&mut self, _: Handle) {
        self.events.push("restore".into());
    }
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

#[test]
fn rgv_c5_signpost_values_negative_budget_and_open_price() {
    for (rating, expected) in [(-100, 0.0), (0, 0.5), (100, 1.0)] {
        assert_eq!(rating_fraction(rating), expected);
    }
    let mut record = CityRecord::new(2);
    record.established = true;
    record.tutorial_id = 4;
    record.mayor_name = b"Mayor".to_vec();
    record.budget = -123.9;
    let data = signpost_data(Some(&record), true, true).unwrap();
    assert!(data.fields.contains(&(0x4a55_2006, Field::Money(-123))));
    assert!(data.fields.contains(&(0x4a55_2001, Field::Text(vec![]))));
    assert!(data.visible.contains(&(0x4a55_2002, false)));
    assert!(data.visible.contains(&(0xcc06_f4cf, true)));
    let normal = signpost_data(Some(&record), false, false).unwrap();
    assert!(normal.fields.contains(&(0x4a55_2002, Field::Money(-123))));
    assert_eq!(signpost_data(None, false, false), Err(Unsupported::Open("RGV2")));
}

#[test]
fn poi_projection_effect_replacement_and_global_invalidation() {
    let mut host = Recording::default();
    let mut windows = Windows::new(1);
    let window = windows.create_default(&mut host);
    windows.set_area(&mut host, window, Rect(0, 0, 32, 32));
    let mut view = RegionView::new(window);
    view.region_key = Some(1);
    assert!(view.init(&mut windows, &mut host));
    let surface = host.image(1, 1, vec![0xffff_ffff]);
    let mut record = CityRecord::new(0);
    record.points.insert(0, vec![[0.0, 0.0, 0.0]]);
    host.records.insert((1, 2), record);
    view.cities.push(CityEntry::new(1, (1, 2), 0, surface));
    view.cities.push(CityEntry::new(2, (3, 2), 0, surface));
    for c in &mut view.cities {
        c.current = true;
    }
    view.update_city(&mut host, 1).unwrap();
    assert_eq!(view.cities[0].icons[0].position, (16, 109));
    assert!(view.cities.iter().all(|c| !c.current));
    let old_effect = view.cities[0].effects[0];
    view.update_city(&mut host, 1).unwrap();
    assert_eq!(
        host.events
            .iter()
            .filter(|e| **e == format!("stop:{old_effect}"))
            .count(),
        1
    );
    assert_eq!(view.cities[0].effects.len(), 1);
    view.clear_cities(&mut host);
    assert_eq!(host.events.iter().filter(|e| e.starts_with("stop:")).count(), 2);
}

#[test]
fn selection_settings_signpost_placement_and_open_child_contract() {
    let mut host = Recording::default();
    let surface = host.image(8, 4, vec![0xffff_ffff; 32]);
    let mut view = RegionView::new(1);
    assert_eq!(view.key(), Err(Unsupported::Open("RGV1")));
    view.region_key = Some(3);
    let mut record = CityRecord::new(0);
    record.established = true;
    host.records.insert((0, 0), record);
    let mut city = CityEntry::new(1, (0, 0), 0, surface);
    city.position = (20.0, 24.0);
    city.hit_runs = vec![0, 8, 1 << 16, (1 << 16) | 8];
    view.add_city(&mut host, city).unwrap();
    view.set_signpost_type(&mut host, 0xca53_9340, 0x0a55_1c50).unwrap();
    assert!(host.events.contains(&"move:(14, 12)".to_owned()));
    assert_eq!(view.select_city(&mut host, 20, 20), Ok(Some(1)));
    view.set_scroll_pos(&mut host, 10, -10);
    assert!(host.events.contains(&"move:(4, 22)".to_owned()));
    view.clear_selection(&mut host).unwrap();
    assert_eq!(view.selected, None);
    host.children_8810 = true;
    assert_eq!(view.update_signpost(&mut host, 1), Err(Unsupported::Open("RGV6")));
    assert_eq!(host.events.last().unwrap(), "restore");
    assert!(host.events.contains(&"child-flag".to_owned()));
}

#[test]
fn tile_composition_map_selection_and_open_render_contracts() {
    let mut host = Recording::default();
    let src = host.image(1, 1, vec![0xff00_0000]);
    let alternate = host.image(1, 1, vec![0xff20_4060]);
    let composed = host.image(1, 1, vec![0]);
    let dst = host.image(1, 1, vec![0]);
    let mut city = CityEntry::new(1, (0, 0), 0, src);
    city.position = (0.0, 1.0);
    city.alternate_image = Some(alternate);
    city.composed = Some(composed);
    city.selection.push(image::Coverage {
        x: 0,
        y: 0,
        values: vec![128],
    });
    let mut view = RegionView::new(1);
    view.region_key = Some(1);
    view.selected = Some(1);
    view.map_mode = 1;
    view.selection_colour = 0x00ff_ffff;
    view.cities.push(city);
    view.render_tile(&mut host, dst, 0, 0).unwrap();
    assert_eq!(host.pixel(dst, 0, 0), 0xff90_a0b0);
    assert!(view.cities[0].current);
    let mut record = CityRecord::new(0);
    record.established = true;
    record.mayor_name = b"Mayor".to_vec();
    host.records.insert((0, 0), record);
    view.enable_labels(&mut host, true, true);
    assert_eq!(view.render_tile(&mut host, dst, 0, 0), Err(Unsupported::Open("RGV4")));
    host.records.get_mut(&(0, 0)).unwrap().mayor_name.clear();
    view.cities[0].icons.push(Icon {
        position: (0, 0),
        image: src,
    });
    assert_eq!(view.render_tile(&mut host, dst, 0, 0), Err(Unsupported::Open("RGV3")));
    assert!(host.events.contains(&"text".to_owned()));
}

struct Bob {
    now: u32,
    retained: i32,
    position: (i32, i32),
}
impl AnimatorHost for Bob {
    fn clock(&mut self) -> u32 {
        self.now
    }
    fn find_child(&mut self, _: Window, id: u32) -> Option<Window> {
        assert_eq!(id, 0x4a63_0000);
        Some(2)
    }
    fn retain_child(&mut self, _: Window) {
        self.retained += 1;
    }
    fn release_child(&mut self, _: Window) {
        self.retained -= 1;
    }
    fn parent_child_heights(&mut self, _: Window) -> (i32, i32) {
        (100, 20)
    }
    fn move_child(&mut self, _: Window, p: (i32, i32)) {
        self.position = p;
    }
}

#[test]
fn rgv_c6_bob_and_counted_child_lifetime() {
    let mut host = Bob {
        now: u32::MAX - 100,
        retained: 0,
        position: (0, 0),
    };
    let mut animator = Animator::default();
    assert!(animator.message(&mut host, 1, 0xa2bf_8ad5));
    assert_eq!(host.retained, 1);
    assert!(!animator.message(&mut host, 1, 0xa2bf_8acd));
    assert_eq!(host.position, (0, 80));
    host.now = host.now.wrapping_add(375);
    assert!(!animator.message(&mut host, 1, 0xa2bf_8acd));
    assert_eq!(host.position, (0, 0));
    assert!(animator.message(&mut host, 1, 0xa2bf_8ad6));
    animator.shutdown(&mut host);
    assert_eq!(host.retained, 0);
}

impl canvas::UpdateHost for Recording {
    fn update_device(&mut self) -> (Option<Surface>, i32, Rect) {
        (None, 600, Rect(10, 20, 30, 40))
    }
    fn update_operation(&mut self, op: canvas::UpdateOp) {
        self.events.push(format!("update:{op:?}"));
    }
}

#[test]
fn update_without_2d_interface_restores_renderer_and_viewport() {
    let mut canvas = Canvas::default();
    let mut host = Recording::default();
    canvas.update(&mut host, |_, _, _, _| panic!("no device")).unwrap();
    assert_eq!(
        host.events,
        [
            "update:Renderer(26)",
            "update:Renderer(24)",
            "update:Viewport(Rect(10, 20, 30, 40))",
            "update:DefaultRenderState"
        ]
    );
}
