// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

type ModelEvents = Rc<RefCell<Vec<(&'static str, Handle)>>>;

struct TestResources {
    render: bool,
    service: bool,
    models: bool,
    null_bindings: Vec<u32>,
    events: Vec<(char, u64)>,
    model_events: Option<ModelEvents>,
}
impl Default for TestResources {
    fn default() -> Self {
        Self {
            render: true,
            service: true,
            models: true,
            null_bindings: Vec::new(),
            events: Vec::new(),
            model_events: None,
        }
    }
}
impl Resources for TestResources {
    fn render_service_available(&mut self) -> bool {
        self.render
    }
    fn resource_service_available(&mut self) -> bool {
        self.service
    }
    fn binding(&mut self, key: ResourceKey) -> Option<Handle> {
        self.events.push(('b', u64::from(key.instance)));
        (!self.null_bindings.contains(&key.instance)).then_some(u64::from(key.instance))
    }
    fn model(&mut self, key: ResourceKey) -> Option<Handle> {
        self.events.push(('m', u64::from(key.instance)));
        self.models.then_some(u64::from(key.instance))
    }
    fn lot_filter(&mut self, mask: u32) -> Option<Handle> {
        self.events.push(('l', u64::from(mask)));
        Some(99)
    }
    fn retain(&mut self, handle: Handle) {
        self.events.push(('+', handle));
        if let Some(events) = &self.model_events {
            events.borrow_mut().push(("retain", handle));
        }
    }
    fn release(&mut self, handle: Handle) {
        self.events.push(('-', handle));
        if let Some(events) = &self.model_events {
            events.borrow_mut().push(("release", handle));
        }
    }
    fn base_cleanup(&mut self, primary: u32) {
        self.events.push(('c', u64::from(primary)));
    }
    fn free_object(&mut self, primary: u32) {
        self.events.push(('f', u64::from(primary)));
    }
}
struct Trace {
    commands: Vec<Command>,
    depth: i32,
    model_events: Option<ModelEvents>,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            depth: 9,
            model_events: None,
        }
    }
}
impl Renderer for Trace {
    fn depth_offset(&self) -> i32 {
        self.depth
    }
    fn emit(&mut self, command: Command) {
        if let Some(events) = &self.model_events {
            let event = match &command {
                Command::ModelState { model, .. } => Some(("state", *model)),
                Command::ModelTransparency { model, .. } => Some(("transparency", *model)),
                Command::DrawModel { model, .. } => Some(("draw", *model)),
                _ => None,
            };
            if let Some(event) = event {
                events.borrow_mut().push(event);
            }
        }
        if let Command::DepthOffset(value) = command {
            self.depth = value;
        }
        self.commands.push(command);
    }
}
impl Trace {
    fn draws(&self) -> Vec<&Draw> {
        self.commands
            .iter()
            .filter_map(|c| match c {
                Command::Draw(d) => Some(d),
                _ => None,
            })
            .collect()
    }
}
struct Terrain {
    width: u32,
    height: u32,
    split: bool,
    lot_flags: Option<u32>,
    show_marks: bool,
    visible: bool,
    frustum: Vec<Vec3>,
    samples: Vec<(i32, i32)>,
    lookups: usize,
}
impl Default for Terrain {
    fn default() -> Self {
        Self {
            width: 64,
            height: 64,
            split: false,
            lot_flags: None,
            show_marks: true,
            visible: true,
            frustum: Vec::new(),
            samples: Vec::new(),
            lookups: 0,
        }
    }
}
impl CellTerrain for Terrain {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn terrain_height(&mut self, x: i32, z: i32) -> Option<f32> {
        self.samples.push((x, z));
        (x >= 0 && z >= 0 && x as u32 <= self.width && z as u32 <= self.height).then_some(2.0)
    }
    fn water_height(&mut self, _x: i32, _z: i32) -> f32 {
        10.0
    }
    fn split(&mut self, _cell: Cell) -> bool {
        self.split
    }
    fn in_frustum(&mut self, point: Vec3) -> bool {
        self.frustum.push(point);
        self.visible
    }
    fn lookup_lot(&mut self, _cell: Cell, filter: Handle) -> Option<Lot> {
        assert_eq!(filter, 99);
        self.lookups += 1;
        self.lot_flags.map(|flags| Lot {
            occupant_reference: 401,
            lot_reference: 402,
            flags,
            show_marks: self.show_marks,
        })
    }
    fn lot_altitude(&mut self, _lot: Handle, _x: f32, _z: f32) -> f32 {
        3.0
    }
}
struct Network {
    available: bool,
    min: Cell,
    max: Cell,
    occupants: BTreeMap<(i32, i32), Vec<Handle>>,
    diagnostic: BTreeMap<Handle, Vec<NetworkPath>>,
    routes: BTreeMap<(Handle, u8), RouteSet>,
    paths: BTreeMap<(Handle, u32), NetworkPath>,
    origin: Vec3,
    altitude: f32,
    terrain: f32,
    cells_visited: Vec<Cell>,
    occupants_visited: Vec<Handle>,
    edges_visited: Vec<(Handle, u8)>,
    altitude_queries: Vec<(f32, f32)>,
    terrain_queries: Vec<(f32, f32)>,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            available: true,
            min: Cell(0, 0),
            max: Cell(0, 0),
            occupants: BTreeMap::from([((0, 0), vec![1])]),
            diagnostic: BTreeMap::new(),
            routes: BTreeMap::new(),
            paths: BTreeMap::new(),
            origin: Vec3::new(0.0, 7.0, 8.0),
            altitude: 20.0,
            terrain: 5.0,
            cells_visited: Vec::new(),
            occupants_visited: Vec::new(),
            edges_visited: Vec::new(),
            altitude_queries: Vec::new(),
            terrain_queries: Vec::new(),
        }
    }
}
impl NetworkHost for Network {
    fn manager_available(&mut self) -> bool {
        self.available
    }
    fn visible_cells(&mut self) -> (Cell, Cell) {
        (self.min, self.max)
    }
    fn occupants(&mut self, cell: Cell) -> Vec<Handle> {
        self.cells_visited.push(cell);
        self.occupants.get(&(cell.0, cell.1)).cloned().unwrap_or_default()
    }
    fn paths(&mut self, occupant: Handle) -> Vec<NetworkPath> {
        self.occupants_visited.push(occupant);
        self.diagnostic.get(&occupant).cloned().unwrap_or_default()
    }
    fn altitude(&mut self, _occupant: Handle, x: f32, z: f32) -> f32 {
        self.altitude_queries.push((x, z));
        self.altitude
    }
    fn routes(&mut self, occupant: Handle, entry: u8) -> RouteSet {
        self.edges_visited.push((occupant, entry));
        self.routes.get(&(occupant, entry)).cloned().unwrap_or(RouteSet {
            keys: Vec::new(),
            chosen: 0,
        })
    }
    fn path(&mut self, occupant: Handle, key: u32) -> Option<NetworkPath> {
        self.paths.get(&(occupant, key)).cloned()
    }
    fn route_origin(&mut self, _occupant: Handle, _first_key: u32) -> Option<Vec3> {
        Some(self.origin)
    }
    fn terrain_altitude(&mut self, x: f32, z: f32) -> f32 {
        self.terrain_queries.push((x, z));
        self.terrain
    }
}
fn p(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00002, "{a} != {b}");
}
fn near_point(a: Vec3, b: Vec3) {
    near(a.x, b.x);
    near(a.y, b.y);
    near(a.z, b.z);
}
fn ribbon(points: &[Vec3], flags: u32, thickness: f32, repeat: f32) -> Vec<Draw> {
    let mut view = PathView::default();
    view.set_thickness(thickness);
    view.new_path(0x1234_5678, 0.0, flags);
    for &point in points {
        view.add_path_vertex(point).unwrap();
    }
    let mut resources = TestResources::default();
    view.set_path_texture(arrow_key(1), repeat, &mut resources).unwrap();
    draw_path(&view.paths()[0], thickness)
}
fn network_path(key: u32, points: &[Vec3]) -> NetworkPath {
    NetworkPath {
        key,
        flags: 0,
        has_heights: true,
        points: points.to_vec(),
    }
}
fn diagnostic(path: NetworkPath) -> (Network, Trace) {
    let mut network = Network::default();
    network.diagnostic.insert(1, vec![path]);
    let mut trace = Trace::default();
    PathInfoView::default()
        .update(&mut Pulse::with_observed_value(0), &mut network, &mut trace)
        .unwrap();
    (network, trace)
}

#[test]
fn ov_l1_l2_order_colour_selection_and_unordered_priority() {
    let mut view = PathView::default();
    for (colour, priority) in [(1, 2.0), (2, -1.0), (3, 0.5), (4, 2.0), (5, f32::NAN)] {
        view.new_path(colour, priority, 0);
        view.add_path_vertex(p(0.0, 0.0, 0.0)).unwrap();
        view.add_path_vertex(p(10.0, 0.0, 0.0)).unwrap();
    }
    assert_eq!(
        view.paths().iter().map(|p| p.colour).collect::<Vec<_>>(),
        [2, 3, 4, 1, 5]
    );
    assert!(view.paths().iter().all(|p| p.points.len() == 2 && p.repeat.is_none()));
    let mut trace = Trace::default();
    view.update(&mut trace);
    assert_eq!(trace.draws().len(), 5);
    assert_eq!(trace.draws()[0].vertices[0].colour, Some(Colour::packed(2)));
    assert_eq!(trace.draws()[0].vertices[0].uv, None);
    assert_eq!(trace.depth, 9);
    let points = [p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0)];
    let draw = ribbon(&points, 0, 2.0, 1.0);
    assert!(draw[0]
        .vertices
        .iter()
        .all(|v| v.colour == Some(Colour::packed(0x1234_5678))));
    assert_eq!(
        draw[0].vertices[0].colour.unwrap().bgra(),
        Some([0x78, 0x56, 0x34, 0x12])
    );
}
#[test]
fn path_spec_order_strip_duplicate_and_defaults() {
    let mut view = PathView::default();
    assert_eq!(view.thickness, 2.0);
    assert!(!view.initialized);
    view.add_path_vertex(p(1.0, 0.0, 0.0)).unwrap();
    let mut resources = TestResources::default();
    assert!(view.set_path_texture(arrow_key(1), 2.0, &mut resources).is_err());
    for (colour, priority) in [(1, 2.0), (2, 1.0), (3, 2.0)] {
        view.new_path(colour, priority, 0);
        for point in [
            p(0.0, 0.0, 0.0),
            p(0.001, 100.0, 0.0),
            p(5.0, 1.0, 0.0),
            p(10.0, 0.0, 0.0),
        ] {
            view.add_path_vertex(point).unwrap();
        }
    }
    assert_eq!(view.paths().iter().map(|p| p.colour).collect::<Vec<_>>(), [2, 3, 1]);
    let draws = draw_path(&view.paths()[0], 2.0);
    assert_eq!(draws[0].vertices.len(), 6);
    near(
        draws[0].vertices[0]
            .position
            .minus(draws[0].vertices[1].position)
            .length(),
        4.0,
    );
    view.init(false, &mut resources);
    let state = view.plain_state.unwrap();
    assert_eq!(
        (state.depth_test, state.depth_write, state.blend),
        (DepthTest::Off, false, Blend::SourceAlpha)
    );
    view.init(true, &mut resources);
    assert_eq!(view.plain_state.unwrap(), state);
    view.shutdown();
    view.init(true, &mut resources);
    assert_eq!(view.plain_state.unwrap().blend, Blend::Off);
    view.clear_paths(&mut resources);
    assert!(view.add_path_vertex(p(1.0, 0.0, 0.0)).is_err());
    assert!(view.set_path_texture(arrow_key(1), 1.0, &mut resources).is_err());
    view.new_path(0, f32::NAN, 0);
    view.add_path_vertex(p(f32::NAN, 0.0, 0.0)).unwrap();
    view.add_path_vertex(p(f32::NAN, 0.0, 0.0)).unwrap();
    assert_eq!(view.paths()[0].points.len(), 2);
}
#[test]
fn ov_l3_l4_offsets_uv_and_reversal() {
    let draws = ribbon(&[p(0.0, 0.0, 0.0), p(6.0, 8.0, 0.0)], 0, 2.0, 4.0);
    let vertices = &draws[0].vertices;
    assert_eq!(vertices[0].position, p(0.0, 0.0, -2.0));
    assert_eq!(vertices[3].position, p(6.0, 8.0, 2.0));
    assert_eq!(vertices[0].uv, Some([0.0, 0.0]));
    assert_eq!(vertices[3].uv, Some([10.0, 1.0]));
    let draws = ribbon(&[p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0), p(10.0, 0.0, 10.0)], 0, 2.0, 4.0);
    assert_eq!(draws[0].vertices[2].position, p(12.0, 0.0, -2.0));
    assert_eq!(draws[0].vertices[3].position, p(8.0, 0.0, 2.0));
    assert_eq!(draws[0].vertices[2].uv, Some([8.0, 0.0]));
    assert_eq!(draws[0].vertices[3].uv, Some([12.0, 1.0]));
    assert_eq!(draws[0].vertices[5].uv, Some([20.0, 1.0]));
    assert_eq!(
        joint_offsets(p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 2.0),
        [p(10.0, 0.0, 1.0), p(10.0, 0.0, -1.0)]
    );
}
#[test]
fn ov_l5_l6_arrows_sequence_and_stored_points() {
    let mut tip = p(0.0, 0.0, 0.0);
    let draw = draw_arrowhead(&mut tip, p(6.0, 8.0, 0.0), 5.0, 7).unwrap();
    assert_eq!(tip, p(3.0, 4.0, 0.0));
    assert_eq!(
        draw.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        [p(0.0, 0.0, 0.0), p(3.0, 4.0, 10.0), p(3.0, 4.0, -10.0)]
    );
    assert!(draw.vertices.iter().all(|v| v.uv == Some([0.0, 0.0])));
    let points = [p(0.0, 0.0, 0.0), p(13.0, 0.0, 0.0)];
    let mut view = PathView::default();
    view.new_path(0, 0.0, 3);
    for point in points {
        view.add_path_vertex(point).unwrap();
    }
    let draws = draw_path(&view.paths()[0], 2.0);
    assert_eq!(draws.iter().map(|d| d.primitive).collect::<Vec<_>>(), [0, 0, 1]);
    near(draws[0].vertices[1].position.x, 9.0);
    near(draws[1].vertices[1].position.x, 11.0);
    near(draws[2].vertices[0].position.x, 9.0);
    near(draws[2].vertices[3].position.x, 11.0);
    assert_eq!(view.paths()[0].points, points);
    let draws = ribbon(&[p(0.0, 0.0, 0.0), p(8.0, 0.0, 0.0)], 3, 2.0, 1.0);
    assert_eq!(draws.len(), 2);
    near(draws[0].vertices[1].position.x, 6.0);
    assert!(draw_arrowhead(&mut tip, p(3.0, 4.0, 1.0), 5.0, 0).is_none());
}
#[test]
fn path_texture_lifetime_state_and_empty_update() {
    let mut view = PathView::default();
    let mut resources = TestResources::default();
    let mut trace = Trace::default();
    view.update(&mut trace);
    assert!(trace.commands.is_empty());
    view.new_path(0, 0.0, 0);
    view.add_path_vertex(p(0.0, 0.0, 0.0)).unwrap();
    view.add_path_vertex(p(10.0, 0.0, 0.0)).unwrap();
    view.init(false, &mut resources);
    view.set_path_texture(arrow_key(11), 3.0, &mut resources).unwrap();
    view.set_path_texture(arrow_key(12), 4.0, &mut resources).unwrap();
    assert_eq!(
        &resources.events[resources.events.len() - 3..],
        [('b', 11), ('-', 11), ('b', 12)]
    );
    view.update(&mut trace);
    assert!(trace.commands.contains(&Command::RepeatWrap));
    assert_eq!(
        trace.commands[trace.commands.len() - 2],
        Command::State(view.plain_state.unwrap())
    );
    resources.null_bindings.push(13);
    view.set_path_texture(arrow_key(13), 7.0, &mut resources).unwrap();
    assert_eq!(view.paths()[0].texture, None);
    assert_eq!(view.paths()[0].repeat, Some(7.0));
    view.clear_paths(&mut resources);
    assert!(view.paths().is_empty());
}

#[test]
fn ov_l7_colour_conversion_mutations_and_unvalidated_sizes() {
    let mut view = MarkedCellView::default();
    assert_eq!(view.colour, 0x80C0_00C0);
    assert_eq!(view.size, [16.0; 2]);
    assert_eq!(view.inverse, [0.0625; 2]);
    assert!(view.ignore_depth && !view.use_water && !view.use_lots && view.outline.is_none());
    view.copy_colours(&[1, 2, 3]);
    let capacity = view.colours.capacity();
    view.set_cell_colour([2.0, -0.5, 1.0 / 512.0, 3.0 / 512.0]);
    assert_eq!(view.colour, 0x02FE_8100);
    assert!(view.colours.is_empty());
    assert_eq!(view.colours.capacity(), capacity);
    view.set_outline_colour([2.0, -0.5, 1.0 / 512.0, 3.0 / 512.0]);
    assert_eq!(view.outline, Some(view.colour));
    view.set_cell_colour([1.0, 0.5, 0.0, 1.0]);
    assert_eq!(view.colour, 0xFFFF_8000);
    view.copy_colours(&[3]);
    view.append_cell(Cell(1, 1));
    view.clear_cells();
    assert_eq!(view.colours, [3]);
    view.add_region_cells(Cell(-1, 0), Cell(1, 1), |c| c.0 != 0);
    assert_eq!(view.cells, [Cell(-1, 0), Cell(-1, 1), Cell(1, 0), Cell(1, 1)]);
    view.set_cell_size(0.0, -2.0);
    assert!(view.inverse[0].is_infinite());
    assert_eq!(view.inverse[1], -0.5);
    view.set_cell_size(f32::INFINITY, f32::NAN);
    assert_eq!(view.inverse[0], 0.0);
    assert!(view.inverse[1].is_nan());
}
#[test]
fn ovv4_worked_mutations_preserve_additive_outline_until_opaque_override() {
    let mut view = MarkedCellView::default();
    let mut resources = TestResources::default();
    let mut terrain = Terrain::default();
    view.init(&mut resources);
    view.append_cell(Cell(2, 3));
    view.append_cell(Cell(2, 3));
    let mut source = vec![0x1122_3344, 0x5566_7788];
    let destination = view.copy_colours(&source) as *mut Vec<u32>;
    assert_eq!(destination, &mut view.colours as *mut Vec<u32>);
    source[0] = 0;
    assert_eq!(view.colours, [0x1122_3344, 0x5566_7788]);
    view.set_cell_colour([1.0, 0.5, 0.0, 1.0]);
    assert_eq!(view.cells, [Cell(2, 3), Cell(2, 3)]);
    assert!(view.colours.is_empty());
    assert_eq!(view.colour.to_le_bytes(), [0, 0x80, 0xFF, 0xFF]);
    view.set_outline_colour([1.0, 0.0, 0.0, 0.0]);
    assert_eq!(view.outline, Some(0x00FF_0000));
    let mut trace = Trace::default();
    view.update(false, &mut terrain, &mut resources, &mut trace).unwrap();
    assert_eq!(
        trace.draws().iter().map(|d| d.vertices.len()).collect::<Vec<_>>(),
        [8, 8]
    );
    assert_eq!(trace.draws()[1].vertices[0].colour, Some(Colour::packed(0x00FF_0000)));
    assert!(trace
        .commands
        .iter()
        .any(|c| matches!(c, Command::State(s) if s.blend == Blend::Additive)));
    view.clear_cells();
    view.append_cell(Cell(2, 3));
    view.shutdown(&mut resources);
    view.init(&mut resources);
    assert_eq!(view.outline, Some(0x00FF_0000));
    for (prefer_opaque, count) in [(true, 1), (false, 2)] {
        trace.commands.clear();
        view.update(prefer_opaque, &mut terrain, &mut resources, &mut trace)
            .unwrap();
        assert_eq!(trace.draws().len(), count);
        assert_eq!(view.outline, Some(0x00FF_0000));
    }
    // SPEC-GAP: OVV4's external inline outline reset still requires the OV 2.3 watchpoint observation.
}
#[test]
fn ov_l8_other_size_clamp_floor_and_no_lots() {
    let mut view = MarkedCellView::default();
    view.set_cell_size(8.0, 8.0);
    view.set_use_lots(true);
    view.append_cell(Cell(1, 1));
    let mut terrain = Terrain {
        width: 4,
        height: 4,
        ..Terrain::default()
    };
    let mut trace = Trace::default();
    view.draw_cells(None, &mut terrain, &mut TestResources::default(), &mut trace)
        .unwrap();
    assert_eq!(terrain.samples, [(0, 0), (0, 1), (1, 1), (1, 0)]);
    assert_eq!(
        trace.draws()[0]
            .vertices
            .iter()
            .map(|v| (v.position.x, v.position.z))
            .collect::<Vec<_>>(),
        [(8.0, 8.0), (8.0, 16.0), (16.0, 16.0), (16.0, 8.0)]
    );
    assert_eq!(trace.draws()[0].indices, [0, 1, 2, 0, 2, 3]);
    view.cells = vec![Cell(7, 7)];
    trace.commands.clear();
    view.draw_cells(None, &mut terrain, &mut TestResources::default(), &mut trace)
        .unwrap();
    assert!(trace.draws()[0]
        .vertices
        .iter()
        .all(|v| v.position == p(32.0, 2.0, 32.0)));
    assert_eq!(terrain.lookups, 0);
    for (input, output) in [(0.0, 0), (0.5, 0), (0.75, 0), (1.0, 1), (1.5, 1), (2.0, 2)] {
        assert_eq!(lower_terrain_vertex(input), output);
    }
}
#[test]
fn ov_l9_water_frustum_lot_order_hidden_and_raised() {
    let mut view = MarkedCellView::default();
    let mut resources = TestResources::default();
    view.init(&mut resources);
    assert_eq!(resources.events, [('l', 0x1F4F)]);
    view.set_use_lots(true);
    view.set_should_use_water_altitude(true);
    view.append_cell(Cell(0, 0));
    let mut terrain = Terrain {
        lot_flags: Some(0),
        ..Terrain::default()
    };
    let mut trace = Trace::default();
    view.draw_cells(None, &mut terrain, &mut resources, &mut trace).unwrap();
    assert_eq!(terrain.frustum, [p(0.0, 10.0, 0.0)]);
    assert!(trace.draws()[0].vertices.iter().all(|v| v.position.y == 3.0));
    assert_eq!(&resources.events[1..], [('-', 402), ('-', 401)]);
    for flags in [4, 0x100, 0x200] {
        terrain.lot_flags = Some(flags);
        trace.commands.clear();
        view.draw_cells(None, &mut terrain, &mut resources, &mut trace).unwrap();
        assert!(trace.draws()[0].vertices.iter().all(|v| v.position.y == 18.5));
    }
    terrain.lot_flags = Some(0x10_0000);
    terrain.show_marks = false;
    trace.commands.clear();
    view.draw_cells(None, &mut terrain, &mut resources, &mut trace).unwrap();
    assert!(trace.draws().is_empty());
    assert_eq!(
        &resources.events[resources.events.len() - 2..],
        [('-', 402), ('-', 401)]
    );
    terrain.visible = false;
    let lookups = terrain.lookups;
    view.draw_cells(None, &mut terrain, &mut resources, &mut trace).unwrap();
    assert_eq!(terrain.lookups, lookups);
    assert_eq!(
        &terrain.frustum[terrain.frustum.len() - 2..],
        [p(0.0, 10.0, 0.0), p(16.0, 10.0, 16.0)]
    );
}
#[test]
fn cells_batch_split_colours_outline_state_and_bounds() {
    let mut resources = TestResources::default();
    let mut view = MarkedCellView::default();
    view.set_should_ignore_depth_buffer(false);
    view.init(&mut resources);
    let state = view.state.unwrap();
    assert_eq!(state.depth_test, DepthTest::LessEqual);
    view.set_should_ignore_depth_buffer(true);
    assert_eq!(view.state, Some(state));
    let mut terrain = Terrain::default();
    for (count, expected) in [(100, vec![400]), (300, vec![1024, 176])] {
        view.cells = (0..count).map(|i| Cell(i % 64, i / 64)).collect();
        let mut trace = Trace::default();
        view.update(false, &mut terrain, &mut resources, &mut trace).unwrap();
        assert_eq!(
            trace.draws().iter().map(|d| d.vertices.len()).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(trace.depth, 9);
        assert!(trace
            .draws()
            .iter()
            .all(|d| d.indices.len() == d.vertices.len() / 4 * 6));
    }
    view.cells = vec![Cell(-1, 0), Cell(64, 0), Cell(1, 2)];
    view.copy_colours(&[1, 2, 0x1122_3344]);
    terrain.split = true;
    view.set_outline_colour([1.0; 4]);
    let mut trace = Trace::default();
    view.update(false, &mut terrain, &mut resources, &mut trace).unwrap();
    assert_eq!(trace.draws().len(), 2);
    assert_eq!(trace.draws()[0].indices, [0, 1, 3, 3, 1, 2]);
    assert_eq!(trace.draws()[0].vertices[0].colour, Some(Colour::packed(0x1122_3344)));
    assert_eq!(trace.draws()[1].vertices[0].colour, Some(Colour::packed(u32::MAX)));
    assert!(trace.commands.iter().any(|c| matches!(c, Command::State(s)
        if s.depth_test == DepthTest::Greater && s.blend == Blend::Additive)));
    trace.commands.clear();
    view.update(true, &mut terrain, &mut resources, &mut trace).unwrap();
    assert_eq!(trace.draws().len(), 1);
    view.clear_cells();
    trace.commands.clear();
    view.update(false, &mut terrain, &mut resources, &mut trace).unwrap();
    assert!(trace.commands.is_empty());
    view.shutdown(&mut resources);
    assert_eq!(view.filter, None);
    assert!(!view.initialized);
}

#[derive(Default)]
struct Driver {
    format: u32,
    capacity: usize,
    fail_lock: bool,
    acquisitions: usize,
    calls: Vec<&'static str>,
    copies: Vec<(usize, Vec<TerrainVertex>)>,
    draws: Vec<Vec<u16>>,
}
impl VertexDriver for Driver {
    fn acquire(&mut self, format: u32) -> Option<DriverBuffer> {
        assert_eq!(format, 11);
        self.acquisitions += 1;
        Some(DriverBuffer {
            interface: 1,
            handle: 2,
            format: self.format,
            capacity: self.capacity,
        })
    }
    fn release_interface(&mut self, interface: Handle) {
        assert_eq!(interface, 1);
        self.calls.push("release");
    }
    fn lock(&mut self, interface: Handle, handle: Handle, capacity: usize) -> Option<Handle> {
        assert_eq!((interface, handle, capacity), (1, 2, self.capacity));
        self.calls.push("lock");
        (!self.fail_lock).then_some(3)
    }
    fn copy_vertices(&mut self, lock: Handle, start: usize, vertices: &[TerrainVertex]) {
        assert_eq!(lock, 3);
        self.calls.push("copy");
        self.copies.push((start, vertices.to_vec()));
    }
    fn unlock(&mut self, _interface: Handle, _handle: Handle) {
        self.calls.push("unlock");
    }
    fn draw_indexed(&mut self, _interface: Handle, _handle: Handle, first: usize, indices: &[u16]) {
        assert_eq!(first, 0);
        self.calls.push("draw");
        self.draws.push(indices.to_vec());
    }
}
fn buffer() -> (VertexBufferExtension, Driver, Vec<TerrainVertex>) {
    let mut buffer = VertexBufferExtension::default();
    let mut driver = Driver {
        format: 11,
        capacity: 10,
        ..Driver::default()
    };
    assert!(buffer.get(true, 4, 8, &mut driver).unwrap());
    let terrain = (0..45).map(|i| [i; 32]).collect();
    (buffer, driver, terrain)
}
#[test]
fn vertex_buffer_acquire_gate_format_capacity_no_initial_lock() {
    let mut buffer = VertexBufferExtension::default();
    let mut driver = Driver {
        format: 11,
        capacity: 10,
        ..Driver::default()
    };
    assert!(!buffer.get(false, 4, 8, &mut driver).unwrap());
    assert_eq!(driver.acquisitions, 0);
    driver.format = 10;
    assert!(!buffer.get(true, 4, 8, &mut driver).unwrap());
    driver.format = 11;
    driver.capacity = 9;
    assert!(!buffer.get(true, 4, 8, &mut driver).unwrap());
    assert_eq!(driver.calls, ["release", "release"]);
    driver.capacity = 10;
    assert!(buffer.get(true, 4, 8, &mut driver).unwrap());
    assert_eq!(buffer.indices.len(), 48);
    assert!(buffer.lock_pointer.is_none());
}
#[test]
fn ov_l10_l11_vertex_compaction_source_offsets_and_word_wrap() {
    let (mut buffer, mut driver, terrain) = buffer();
    buffer.create_buffer(&mut driver).unwrap();
    buffer.used_vertices = 4;
    let mut indices = vec![0; 66];
    indices[54..60].copy_from_slice(&[1, 2, 6, 2, 7, 6]);
    let source = buffer.set_index_buffer(&indices, 1);
    assert_eq!(source.offset_bytes(4, 2, 1), 108);
    buffer
        .draw_prims_indexed(2, 1, 3, &terrain, &source, &mut driver)
        .unwrap();
    assert_eq!((buffer.used_vertices, buffer.used_indices), (10, 12));
    assert_eq!(&buffer.indices[..6], [4, 5, 7, 5, 8, 7]);
    assert_eq!(driver.calls, ["lock", "copy", "copy"]);
    assert_eq!(
        driver.copies,
        [(4, terrain[11..14].to_vec()), (7, terrain[16..19].to_vec())]
    );
    buffer.flush_buffer(&mut driver).unwrap();
    buffer.create_buffer(&mut driver).unwrap();
    let indices = vec![0; 18];
    let source = buffer.set_index_buffer(&indices, 0);
    assert_eq!(source.offset_bytes(4, 2, 1), 12);
    buffer
        .draw_prims_indexed(2, 1, 3, &terrain, &source, &mut driver)
        .unwrap();
    assert_eq!(buffer.indices[0], 65535);
}
#[test]
fn ov_l12_l13_flush_relock_failure_and_failed_create_counts() {
    let (mut buffer, mut driver, terrain) = buffer();
    buffer.create_buffer(&mut driver).unwrap();
    buffer.used_vertices = 8;
    buffer.used_indices = 12;
    driver.calls.clear();
    driver.fail_lock = true;
    let source = buffer.set_index_buffer(&[], 0);
    buffer
        .draw_prims_indexed(0, 0, 2, &terrain, &source, &mut driver)
        .unwrap();
    assert_eq!(driver.calls, ["unlock", "draw", "lock"]);
    assert_eq!(driver.draws[0].len(), 12);
    assert_eq!(
        (buffer.used_vertices, buffer.used_indices, buffer.lock_pointer),
        (0, 0, None)
    );
    buffer.used_vertices = 4;
    buffer.used_indices = 6;
    assert!(!buffer.create_buffer(&mut driver).unwrap());
    driver.calls.clear();
    buffer.flush_buffer(&mut driver).unwrap();
    assert_eq!((buffer.used_vertices, buffer.used_indices), (4, 6));
    assert!(driver.calls.is_empty());
}
#[test]
fn vertex_empty_range_growth_flush_and_release_are_distinct() {
    let (mut buffer, mut driver, terrain) = buffer();
    buffer.create_buffer(&mut driver).unwrap();
    let source = buffer.set_index_buffer(&[0; 12], 0);
    buffer
        .draw_prims_indexed(0, 2, 2, &terrain, &source, &mut driver)
        .unwrap();
    assert_eq!((buffer.used_vertices, buffer.used_indices), (2, 0));
    buffer.flush_buffer(&mut driver).unwrap();
    assert_eq!(driver.calls, ["lock", "copy", "copy", "unlock"]);
    driver.calls.clear();
    buffer
        .draw_prims_indexed(0, 0, 1, &terrain, &source, &mut driver)
        .unwrap();
    assert!(driver.calls.is_empty());
    buffer.create_buffer(&mut driver).unwrap();
    buffer.indices.clear();
    buffer
        .draw_prims_indexed(0, 0, 1, &terrain, &source, &mut driver)
        .unwrap();
    assert_eq!(buffer.indices.len(), 22);
    assert!(!driver.calls.contains(&"unlock"));
    buffer.used_indices = 5;
    buffer.flush_buffer(&mut driver).unwrap();
    assert_eq!(driver.draws.last().unwrap().len(), 3);
    buffer.create_buffer(&mut driver).unwrap();
    buffer.used_vertices = 4;
    buffer.used_indices = 6;
    driver.calls.clear();
    buffer.release(&mut driver);
    assert_eq!(driver.calls, ["release"]);
    assert_eq!(
        (buffer.used_vertices, buffer.used_indices, buffer.lock_pointer),
        (4, 6, Some(3))
    );
    assert!(buffer.flush_buffer(&mut driver).is_err());
}

#[test]
fn no_l1_colours_and_process_pulse_no_manager_or_constructor_reset() {
    for (kind, expected) in [
        (1, [128, 255, 128, 200]),
        (2, [0, 0, 255, 200]),
        (4, [255, 255, 0, 200]),
        (6, [255, 0, 255, 200]),
        (7, [255, 0, 255, 200]),
        (3, [255, 128, 128, 200]),
    ] {
        let c = path_info_colour(kind << 24, 0, 0);
        assert_eq!([c.r, c.g, c.b, c.a.unwrap()], expected);
    }
    assert_eq!(path_info_colour(1 << 24, 0, 0).bgra(), Some([128, 255, 128, 200]));
    let mut pulse = Pulse::with_observed_value(250);
    let mut network = Network::default();
    let mut path = network_path(0x0400_0000, &[p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0)]);
    path.flags = 2;
    network.diagnostic.insert(1, vec![path]);
    for expected in [Colour::rgba(255, 255, 255, 200), Colour::rgba(255, 0, 0, 200)] {
        let mut trace = Trace::default();
        PathInfoView::default()
            .update(&mut pulse, &mut network, &mut trace)
            .unwrap();
        assert_eq!(trace.draws()[0].vertices[0].colour, Some(expected));
    }
    network.available = false;
    let mut trace = Trace::default();
    PathInfoView::default()
        .update(&mut pulse, &mut network, &mut trace)
        .unwrap();
    assert_eq!(pulse.value(), 5);
    assert!(trace.commands.is_empty());
    let mut fresh = Pulse::default();
    assert_eq!(fresh.value(), 0);
    for n in 1..=104 {
        assert_eq!(fresh.advance(), ((5 * n) % 260) as u8);
    }
    let mut wrapped = Pulse::with_observed_value(253);
    assert_eq!(wrapped.advance(), 2);
}
#[test]
fn nov2_worked_state_filter_and_fresh_process_pulse() {
    let mut process = OverlayProcess::default();
    let mut resources = TestResources::default();
    let mut a = View::new(0x1000, PathInfoView::default());
    a.owner.init(&mut resources);
    let depth_tested = a.owner.state;
    assert_eq!(depth_tested.unwrap().depth_test, DepthTest::LessEqual);
    a.owner.set_should_ignore_depth_buffer(true);
    a.owner.init(&mut resources);
    assert!(a.owner.ignore_depth);
    assert_eq!(a.owner.state, depth_tested);
    a.owner.set_filter(256);
    a.owner.shutdown();
    assert!(!a.owner.initialized);
    assert_eq!(a.owner.state, depth_tested);
    assert_eq!(a.owner.filter, 256);
    assert_eq!(process.pulse.value(), 0);
    a.owner.init(&mut resources);
    assert_eq!(a.owner.state.unwrap().depth_test, DepthTest::Off);
    assert_eq!(
        a.owner.state.unwrap().tuple,
        Some([1, 0, 0, 0, 1, 0, 0, -1, 4, 5, 4, 0, 1, -1])
    );
    let mut network = Network {
        available: false,
        ..Network::default()
    };
    let mut trace = Trace::default();
    a.owner.update(&mut process.pulse, &mut network, &mut trace).unwrap();
    assert_eq!(process.pulse.value(), 5);
    assert!(trace.commands.is_empty());
    let mut b = PathInfoView::default();
    b.init(&mut resources);
    let mut path = network_path(0, &[p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0)]);
    path.flags = 2;
    network.available = true;
    network.diagnostic.insert(1, vec![path]);
    b.update(&mut process.pulse, &mut network, &mut trace).unwrap();
    assert_eq!(
        trace.draws()[0].vertices[0].colour.unwrap().bgra(),
        Some([0x0A, 0x0A, 0xFF, 0xC8])
    );
    a.release(&mut resources).unwrap();
    assert_eq!(process.pulse.value(), 10);
    b.set_filter(256);
    trace.commands.clear();
    b.update(&mut process.pulse, &mut network, &mut trace).unwrap();
    assert!(trace.draws().is_empty());
    b.set_filter(u32::MAX);
    b.update(&mut process.pulse, &mut network, &mut trace).unwrap();
    assert_eq!(trace.draws().len(), 2);
}
#[test]
fn no_l2_sloped_arrow_uses_stored_height_and_3d_chord() {
    let mut path = network_path(0, &[p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), p(3.0, 4.0, 0.0)]);
    path.has_heights = false;
    let (network, trace) = diagnostic(path);
    let draws = trace.draws();
    assert!(draws[0].vertices.iter().all(|v| v.position.y == 20.5));
    assert_eq!(network.altitude_queries.len(), 3);
    near_point(draws[1].vertices[0].position, p(1.727208, 4.5, -1.272792));
    near_point(draws[1].vertices[1].position, p(3.0, 4.5, 0.0));
    near_point(draws[1].vertices[2].position, p(1.727208, 4.5, 1.272792));
    assert!(draws[1].vertices.iter().all(|v| v.colour.unwrap().a == Some(200)));
}
#[test]
fn no_l3_l4_full_vector_arrows_bars_precedence_and_scratch_alpha() {
    let (_, trace) = diagnostic(network_path(
        0x0000_FFFF,
        &[p(0.0, 0.0, 0.0), p(10.0, 0.0, 0.0), p(20.0, 0.0, 0.0)],
    ));
    let draws = trace.draws();
    assert_eq!(draws.len(), 3);
    assert!(draws[2]
        .vertices
        .iter()
        .all(|v| v.position.x == 0.0 && v.position.y == 0.0));
    assert_eq!(draws[2].vertices[0].colour, Some(Colour::rgba(255, 0, 0, 200)));
    let points: Vec<_> = (0..40).map(|i| p(i as f32, 0.0, 0.0)).collect();
    let (_, trace) = diagnostic(network_path(4 << 24, &points));
    assert_eq!(trace.draws()[0].vertices.len(), 25);
    near_point(trace.draws()[1].vertices[1].position, p(39.0, 0.5, 0.0));
    near(trace.draws()[1].vertices[0].position.x, 39.0 - 3.0 / 2.0_f32.sqrt());
    let (_, trace) = diagnostic(network_path(0, &[p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)]));
    assert_eq!(trace.draws()[1].vertices[2].colour.unwrap().a, None);
    let (_, trace) = diagnostic(network_path(0xFF, &[p(0.0, 1.0, 0.0), p(1.0, 3.0, 0.0)]));
    assert!(trace.draws()[2]
        .vertices
        .iter()
        .all(|v| v.position.x == 1.0 && v.position.y == 3.0));
}
#[test]
fn no_l5_cross_axes_height_queries_and_opaque_alpha() {
    let (_, trace) = diagnostic(network_path(0x1000_0000, &[p(10.0, 7.0, 20.0)]));
    let draws = trace.draws();
    assert_eq!(draws.len(), 2);
    assert_eq!(
        draws[0].vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        [p(8.0, 7.5, 20.0), p(12.0, 7.5, 20.0)]
    );
    assert_eq!(
        draws[1].vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        [p(10.0, 7.5, 18.0), p(10.0, 7.5, 22.0)]
    );
    assert!(draws
        .iter()
        .flat_map(|d| &d.vertices)
        .all(|v| v.colour == Some(Colour::rgba(255, 0, 0, 255))));
    let mut path = network_path(0x1000_0000, &[p(10.0, 7.0, 20.0)]);
    path.has_heights = false;
    let (network, trace) = diagnostic(path);
    assert_eq!(
        network.altitude_queries,
        [(8.0, 20.0), (12.0, 20.0), (10.0, 18.0), (10.0, 22.0)]
    );
    assert!(trace
        .draws()
        .iter()
        .flat_map(|d| &d.vertices)
        .all(|v| v.position.y == 20.5));
}
#[test]
fn network_arrow_backward_search_vertical_threshold_and_degenerate_boundaries() {
    let (_, trace) = diagnostic(network_path(
        0,
        &[p(0.0, 0.0, 0.0), p(0.0, 0.001, 0.0), p(0.0, 0.001, 0.0)],
    ));
    assert_eq!(trace.draws().len(), 2);
    assert!(trace.draws()[1]
        .vertices
        .iter()
        .all(|v| v.position.x == 0.0 && v.position.z == 0.0));
    let (_, trace) = diagnostic(network_path(0, &[p(1.0, 0.0, 0.0); 3]));
    assert_eq!(trace.draws().len(), 1);
    let (_, trace) = diagnostic(network_path(0xFF00, &[p(1.0, 0.0, 0.0); 3]));
    assert!(trace.draws()[1].vertices.iter().all(|v| v.position.x.is_nan()));
    let mut network = Network::default();
    network.diagnostic.insert(1, vec![network_path(0, &[])]);
    assert!(PathInfoView::default()
        .update(&mut Pulse::with_observed_value(0), &mut network, &mut Trace::default())
        .is_err());
}
#[test]
fn network_traversal_filter_state_idempotence_and_empty_rectangle() {
    let mut network = Network {
        max: Cell(1, 1),
        ..Network::default()
    };
    network.occupants.insert((0, 0), vec![9, 2]);
    let points = [p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0)];
    network.diagnostic.insert(
        9,
        vec![network_path(0x0403_0000, &points), network_path(0x0203_0000, &points)],
    );
    network.diagnostic.insert(2, vec![network_path(0x0104_0000, &points)]);
    let mut view = PathInfoView::default();
    let mut resources = TestResources::default();
    view.init(&mut resources);
    assert_eq!(
        view.state.unwrap().tuple,
        Some([1, 0, 1, 0, 1, 0, 0, 3, 4, 5, 4, 0, 1, -1])
    );
    assert!(!view.state.unwrap().culling);
    let state = view.state;
    view.set_should_ignore_depth_buffer(true);
    view.set_filter(3);
    view.init(&mut resources);
    assert_eq!(view.state, state);
    let mut trace = Trace::default();
    view.update(&mut Pulse::with_observed_value(0), &mut network, &mut trace)
        .unwrap();
    assert_eq!(network.cells_visited, [Cell(0, 0), Cell(0, 1), Cell(1, 0), Cell(1, 1)]);
    assert_eq!(network.occupants_visited, [9, 2]);
    assert_eq!(trace.draws().len(), 4);
    assert_eq!(
        trace.draws()[0].vertices[0].colour,
        Some(Colour::rgba(255, 255, 0, 200))
    );
    assert_eq!(trace.draws()[2].vertices[0].colour, Some(Colour::rgba(0, 0, 255, 200)));
    network.min = Cell(2, 2);
    trace.commands.clear();
    view.update(&mut Pulse::with_observed_value(0), &mut network, &mut trace)
        .unwrap();
    assert!(trace.draws().is_empty());
}
#[test]
fn no_l6_l7_train_horizontal_chord_classification() {
    let e = p(1.0, 0.0, 0.0);
    let points = [
        p(0.0, 0.0, 0.0),
        p(0.0, 0.0, 10.0),
        p(10.0, 0.0, -10.0),
        p(10.0, 0.0, 0.0),
    ];
    assert_eq!(classify_path_bitmap(&points, e, false), Ok(0x6BE0_9921));
    assert_eq!(classify_path_bitmap(&points, e, true), Ok(0x6BE0_9922));
    let points = [
        p(0.0, 0.0, 0.0),
        p(10.0, 0.0, 10.0),
        p(10.0, 100.0, 10.0),
        p(10.0, 200.0, 10.0),
    ];
    assert_eq!(classify_path_bitmap(&points, e, false), Ok(0x6BE0_9923));
    assert_eq!(classify_path_bitmap(&[p(0.0, 0.0, 0.0), e], e, false), Ok(0x6BE0_9925));
    assert!(classify_path_bitmap(&[], e, false).is_err());
}
#[test]
fn no_l8_quad_world_space_height_winding_uv_and_degenerate_direction() {
    let mut queried = Vec::new();
    let draw = arrow_bitmap_quad(p(10.0, 7.0, 20.0), p(1.0, 0.0, 0.0), 7 << 24, 0.1, |x, z| {
        queried.push((x, z));
        9.0
    });
    assert_eq!(queried, [(22.0, 20.0)]);
    assert_eq!((draw.primitive, draw.format, draw.vertices.len()), (6, 2, 4));
    for (vertex, expected) in draw.vertices.iter().zip([
        p(10.0, 12.1, 26.0),
        p(22.0, 12.1, 26.0),
        p(22.0, 12.1, 14.0),
        p(10.0, 12.1, 14.0),
    ]) {
        near_point(vertex.position, expected);
    }
    assert_eq!(
        draw.vertices.iter().map(|v| v.uv.unwrap()).collect::<Vec<_>>(),
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
    );
    let draw = arrow_bitmap_quad(p(0.0, 7.0, 8.0), p(1.0, 0.0, 0.0), 0, 0.1, |_, _| 5.0);
    assert_eq!(draw.vertices[0].position, p(0.0, 7.1, 14.0));
    let draw = arrow_bitmap_quad(p(0.0, 7.0, 8.0), Vec3::default(), 0, 0.1, |_, _| 5.0);
    assert!(draw.vertices.iter().all(|v| v.position == p(0.0, 7.1, 8.0)));
}
fn junction_fixture() -> (TrainJunctionView, OverlayProcess, TestResources, Network) {
    let mut view = TrainJunctionView::default();
    let mut process = OverlayProcess::default();
    let mut resources = TestResources::default();
    view.init(&mut process, &mut resources).unwrap();
    let mut network = Network {
        max: Cell(0, 1),
        ..Network::default()
    };
    network.occupants.insert((0, 1), vec![2]);
    let keys = [0x0002, 0x0001, 0x0102];
    for occupant in [1, 2] {
        for key in [keys[0], keys[2]] {
            network.paths.insert(
                (occupant, key),
                network_path(key, &[p(0.0, 0.0, 0.0), p(5.0, 0.0, 0.0), p(10.0, 0.0, 0.0)]),
            );
        }
    }
    network.routes.insert(
        (1, 0),
        RouteSet {
            keys: keys.to_vec(),
            chosen: 0xABCD_0002,
        },
    );
    network.routes.insert(
        (2, 0),
        RouteSet {
            keys: vec![keys[0]],
            chosen: 0,
        },
    );
    (view, process, resources, network)
}
#[test]
fn no_l9_train_order_skips_frame_stacking_transparency_and_marker() {
    let (mut view, process, mut resources, mut network) = junction_fixture();
    let mut trace = Trace::default();
    let result = view.update(&process, &mut network, &mut resources, &mut trace);
    assert_eq!(result, Ok(()));
    let draws = trace.draws();
    assert_eq!(draws.len(), 3);
    for (i, draw) in draws.iter().enumerate() {
        near(draw.vertices[0].position.y, 7.0 + (0.1_f32 + i as f32 * 0.01));
    }
    let alpha: Vec<_> = trace
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::Transparency(a) => Some(*a),
            _ => None,
        })
        .collect();
    assert_eq!(alpha, [0.4, 0.5, 0.4, 0.5]);
    assert_eq!(
        network.edges_visited,
        [(1, 0), (1, 1), (1, 2), (1, 3), (2, 0), (2, 1), (2, 2), (2, 3)]
    );
    let markers: Vec<_> = trace
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::ModelState { model, state } => Some((*model, *state)),
            _ => None,
        })
        .collect();
    assert_eq!(markers[0].0, 0x8BB7_0000);
    near_point(markers[0].1.translation, p(0.0, 7.32, 8.0));
    assert_eq!(
        markers[0].1.rotation,
        [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]]
    );
    assert_eq!(
        &resources.events[resources.events.len() - 2..],
        [('+', 0x8BB7_0000), ('-', 0x8BB7_0000)]
    );
    view.set_hover_cell(Some(Cell(0, 0)));
    trace.commands.clear();
    view.update(&process, &mut network, &mut resources, &mut trace).unwrap();
    assert!(trace.commands.contains(&Command::Transparency(0.7)));
    assert!(trace.commands.contains(&Command::Transparency(0.8)));
    assert!(trace
        .commands
        .iter()
        .any(|c| matches!(c, Command::ModelState { model: 0x8BB7_0000, .. })));
    view.set_pressed_cell(Cell(0, 0));
    trace.commands.clear();
    view.update(&process, &mut network, &mut resources, &mut trace).unwrap();
    assert!(trace
        .commands
        .iter()
        .any(|c| matches!(c, Command::ModelState { model: 0x8BB8_0000, .. })));
}
#[test]
fn nov3_worked_hover_press_marker_basis_state_and_submission_order() {
    let (mut view, process, mut resources, mut network) = junction_fixture();
    let cell = Cell(3, 4);
    network.min = cell;
    network.max = cell;
    network.occupants = BTreeMap::from([((3, 4), vec![1])]);
    network.routes = BTreeMap::from([(
        (1, 0),
        RouteSet {
            keys: vec![0x0002],
            chosen: 0,
        },
    )]);
    network.origin = p(10.0, 7.0, 20.0);
    network.terrain = 30.0;
    let events = ModelEvents::default();
    resources.model_events = Some(events.clone());
    let mut trace = Trace {
        model_events: Some(events.clone()),
        ..Trace::default()
    };
    let unset = Cell(-1, -1);
    let state = view.state;
    let bindings = view.bindings;
    let models = view.models;
    for (step, (pairs, model, transparency)) in [
        ([unset, unset], 0x8BB7_0000, 0.5),
        ([cell, unset], 0x8BB7_0000, 0.8),
        ([cell, cell], 0x8BB8_0000, 0.8),
        ([cell, unset], 0x8BB7_0000, 0.8),
        ([unset, unset], 0x8BB7_0000, 0.5),
    ]
    .into_iter()
    .enumerate()
    {
        let resource_count = resources.events.len();
        match step {
            1 => {
                view.set_hover_cell(Some(cell));
                view.set_hover_cell(Some(cell));
            }
            2 => {
                view.set_pressed_cell(cell);
                view.set_pressed_cell(cell);
            }
            3 => view.clear_pressed_cell(),
            4 => view.set_hover_cell(None),
            _ => {}
        }
        assert_eq!(resources.events.len(), resource_count);
        assert_eq!(view.cell_pairs, pairs);
        assert_eq!((view.state, view.bindings, view.models), (state, bindings, models));
        trace.commands.clear();
        events.borrow_mut().clear();
        view.update(&process, &mut network, &mut resources, &mut trace).unwrap();
        assert_eq!(
            &*events.borrow(),
            &[
                ("retain", model),
                ("state", model),
                ("transparency", model),
                ("draw", model),
                ("release", model),
            ]
        );
        near(trace.draws()[0].vertices[0].position.y, 30.1);
        let Command::ModelState {
            model: submitted,
            state,
        } = trace.commands[trace.commands.len() - 3]
        else {
            panic!("expected marker state before transparency and draw");
        };
        assert_eq!(submitted, model);
        assert_eq!(state.kind, [2, 5]);
        assert_eq!(state.rotation, [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]]);
        near_point(state.translation, p(10.0, 7.31, 20.0));
        assert_eq!(state.scale, 1.0);
        assert_eq!(
            &trace.commands[trace.commands.len() - 2..],
            &[
                Command::ModelTransparency {
                    model,
                    mode: 1,
                    value: transparency
                },
                Command::DrawModel { model, flags: 0 },
            ]
        );
        assert!(trace.commands.contains(&Command::Transparency(transparency)));
    }
    assert_eq!(
        marker_state(Vec3::default(), Vec3::default()).rotation,
        [[0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
    );
    assert_eq!(
        marker_state(p(3.0, 4.0, 0.0), Vec3::default()).rotation,
        [[0.0, -4.0, 3.0], [0.0, 3.0, 4.0], [-1.0, 0.0, 0.0]]
    );
}
#[test]
fn train_resources_lifetime_state_missing_services_and_null_bindings() {
    let mut view = TrainJunctionView::default();
    assert_eq!(view.cell_pairs, [Cell(-1, -1); 2]);
    assert_eq!(view.bindings, [None; 6]);
    assert_eq!(view.models, [None; 2]);
    assert_eq!(view.stacking_offset, None);
    let mut process = OverlayProcess::default();
    let mut resources = TestResources::default();
    view.init(&mut process, &mut resources).unwrap();
    assert_eq!(
        view.state.unwrap().tuple,
        Some([1, 1, 1, 1, 1, 1, 1, 3, 4, 5, 4, 0, 1, -1])
    );
    assert_eq!(process.directions.unwrap()[0][2], p(1.0, 0.0, 0.0));
    assert_eq!(process.directions.unwrap()[3][3], Vec3::default());
    assert_eq!(
        arrow_key(1),
        ResourceKey {
            kind: 0x7AB5_0E44,
            group: 0x1ABE_787D,
            instance: 1
        }
    );
    assert_eq!(
        marker_key(1),
        ResourceKey {
            kind: 0x5AD0_E817,
            group: 0xBADB_57F1,
            instance: 1
        }
    );
    assert_eq!(
        &resources.events[..6],
        [
            ('b', 0x6BE0_9921),
            ('b', 0x6BE0_9922),
            ('b', 0x6BE0_9923),
            ('b', 0x6BE0_9924),
            ('b', 0x6BE0_9925),
            ('b', 0x6BE0_9926)
        ]
    );
    assert_eq!(
        &resources.events[6..9],
        [('m', 0x8BB7_0000), ('+', 0x8BB7_0000), ('-', 0x8BB7_0000)]
    );
    let events = resources.events.len();
    view.init(&mut process, &mut resources).unwrap();
    assert_eq!(events, resources.events.len());
    resources.events.clear();
    view.shutdown(&mut resources).unwrap();
    assert_eq!(
        resources.events,
        [
            ('-', 0x6BE0_9923),
            ('-', 0x6BE0_9924),
            ('-', 0x6BE0_9921),
            ('-', 0x6BE0_9922),
            ('-', 0x6BE0_9925),
            ('-', 0x6BE0_9926),
            ('-', 0x8BB7_0000),
            ('-', 0x8BB8_0000)
        ]
    );
    view.shutdown(&mut resources).unwrap();
    assert_eq!(resources.events.len(), 8);
    let mut missing = TrainJunctionView::default();
    resources.service = false;
    resources.render = false;
    assert!(missing.init(&mut process, &mut resources).is_err());
    assert!(missing.initialized && missing.state.is_none());
    let (mut view, process, mut resources, mut network) = junction_fixture();
    view.models = [None; 2];
    view.bindings = [Some(None); 6];
    let mut trace = Trace::default();
    view.update(&process, &mut network, &mut resources, &mut trace).unwrap();
    assert!(trace.draws().is_empty());
    assert_eq!(view.stacking_offset, Some(0.1));
    network.available = false;
    view.stacking_offset = None;
    trace.commands.clear();
    view.update(&process, &mut network, &mut resources, &mut trace).unwrap();
    assert!(trace.commands.is_empty());
    assert_eq!(view.stacking_offset, None);
}
#[test]
fn ov_l14_shared_query_reference_wrap_zero_release_and_pick() {
    let mut view = View::new(0x1000, PathInfoView::default());
    let mut output = 0xDEAD;
    assert_eq!(view.query_interface(2, &mut output), Ok(false));
    assert_eq!((output, view.count()), (0xDEAD, 0));
    assert_eq!(view.query_interface(1, &mut output), Ok(true));
    assert_eq!((output, view.count()), (0x1004, 1));
    assert_eq!(view.add_ref(), Ok(2));
    let mut resources = TestResources::default();
    assert_eq!(view.release(&mut resources), Ok(1));
    assert!(!view.pick(&mut output));
    assert_eq!(output, 0x1004);
    assert_eq!(view.release(&mut resources), Ok(0));
    assert!(view.deleted());
    assert_eq!(view.count(), 1); // No zero store before deletion.
    assert_eq!(resources.events, [('c', 0x1000), ('f', 0x1000)]);
    let mut zero = View::new(0x2000, PathInfoView::default());
    assert_eq!(zero.release(&mut resources), Ok(0));
    assert_eq!(zero.count(), 0);
    let mut wrap = View::new(0, PathInfoView::default());
    wrap.count = u32::MAX;
    assert_eq!(wrap.add_ref(), Ok(0));
    wrap.delete(2, &mut resources).unwrap();
    assert_eq!(resources.events.last(), Some(&('c', 0)));
}
#[test]
fn ovv6_secondary_remove_ref_never_deletes_any_owner() {
    fn check<T: ViewOwner>(owner: T) {
        let mut view = View::new(0x2000, owner);
        let mut resources = TestResources::default();
        assert_eq!(view.remove_ref(), Ok(0));
        assert_eq!(view.add_ref(), Ok(1));
        assert_eq!(view.remove_ref(), Ok(0));
        assert_eq!(view.count(), 0);
        assert!(!view.deleted());
        assert!(resources.events.is_empty());
        assert_eq!(view.release(&mut resources), Ok(0));
        assert_eq!(resources.events, [('c', 0x2000), ('f', 0x2000)]);
        assert_eq!(view.remove_ref(), Err(Boundary::Undefined("deleted view")));
    }
    check(PathView::default());
    check(MarkedCellView::default());
    check(PathInfoView::default());
    check(TrainJunctionView::default());
}
#[test]
fn ovv6_worked_lifetime_shutdown_then_release_has_no_double_model_release() {
    let mut view = View::new(0x3000, TrainJunctionView::default());
    let mut resources = TestResources::default();
    assert_eq!(view.add_ref(), Ok(1));
    let mut output = 0xDEAD;
    assert_eq!(view.query_interface(1, &mut output), Ok(true));
    assert_eq!((output, view.count()), (0x3004, 2));
    assert_eq!(view.remove_ref(), Ok(1));
    assert!(!view.deleted());
    output = 0xDEAD;
    assert_eq!(view.query_interface(2, &mut output), Ok(false));
    assert_eq!((output, view.count()), (0xDEAD, 1));
    view.owner.init(&mut OverlayProcess::default(), &mut resources).unwrap();
    resources.events.clear();
    // The owning caller unregisters externally before invoking Shutdown and Release.
    view.owner.shutdown(&mut resources).unwrap();
    assert_eq!(view.owner.models, [None; 2]);
    assert_eq!(view.release(&mut resources), Ok(0));
    assert_eq!(
        resources.events,
        [
            ('-', 0x6BE0_9923),
            ('-', 0x6BE0_9924),
            ('-', 0x6BE0_9921),
            ('-', 0x6BE0_9922),
            ('-', 0x6BE0_9925),
            ('-', 0x6BE0_9926),
            ('-', 0x8BB7_0000),
            ('-', 0x8BB8_0000),
            ('c', 0x3000),
            ('f', 0x3000),
        ]
    );
    assert!(view.deleted());
}
#[test]
fn deleting_cleanup_of_each_owner_train_does_not_shutdown() {
    let mut resources = TestResources::default();
    let mut path = PathView::default();
    path.new_path(0, 0.0, 0);
    path.set_path_texture(arrow_key(1), 1.0, &mut resources).unwrap();
    path.add_path_vertex(p(0.0, 0.0, 0.0)).unwrap();
    let mut path = View::new(10, path);
    resources.events.clear();
    path.release(&mut resources).unwrap();
    assert_eq!(resources.events, [('-', 1), ('c', 10), ('f', 10)]);
    assert!(path.owner.paths().is_empty());
    let mut cells = MarkedCellView::default();
    cells.init(&mut resources);
    cells.append_cell(Cell(0, 0));
    cells.copy_colours(&[1]);
    let mut cells = View::new(20, cells);
    resources.events.clear();
    cells.release(&mut resources).unwrap();
    assert_eq!(resources.events, [('-', 99), ('c', 20), ('f', 20)]);
    assert_eq!((cells.owner.cells.capacity(), cells.owner.colours.capacity()), (0, 0));
    let mut train = TrainJunctionView::default();
    train.init(&mut OverlayProcess::default(), &mut resources).unwrap();
    let mut train = View::new(30, train);
    resources.events.clear();
    train.release(&mut resources).unwrap();
    assert_eq!(
        resources.events,
        [('-', 0x8BB8_0000), ('-', 0x8BB7_0000), ('c', 30), ('f', 30)]
    );
    assert!(train.owner.initialized);
    assert!(train.owner.bindings.iter().all(|b| matches!(b, Some(Some(_)))));
}
#[test]
fn init_without_render_service_and_short_unsafe_sources_are_explicit() {
    let mut resources = TestResources {
        render: false,
        ..TestResources::default()
    };
    let mut path = PathView::default();
    assert!(path.init(false, &mut resources));
    assert!(path.initialized && path.plain_state.is_none());
    let mut cells = MarkedCellView::default();
    assert!(cells.init(&mut resources));
    assert!(cells.initialized && cells.state.is_none());
    let mut diagnostic = PathInfoView::default();
    assert!(diagnostic.init(&mut resources));
    assert!(diagnostic.initialized && diagnostic.state.is_none());
    cells.cells = vec![Cell(0, 0), Cell(1, 0)];
    cells.colours = vec![0];
    assert!(cells
        .draw_cells(None, &mut Terrain::default(), &mut resources, &mut Trace::default())
        .is_err());
    cells.colours.clear();
    cells.set_cell_size(16000.0, 16000.0);
    cells.cells = vec![Cell(1, 1)];
    assert!(cells
        .draw_cells(None, &mut Terrain::default(), &mut resources, &mut Trace::default())
        .is_err());
}
