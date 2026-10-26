// SPDX-License-Identifier: GPL-3.0-or-later
//! Fixed-view container rectangles and grid rebuild (render3d §14).
use super::{ContainerRect, GridElement, Handle, QuadGrid, Rect, Render3d, WorldBox};

pub trait ContainerServices {
    fn rotation(&self) -> u32;
    fn project_fixed(&mut self, point: [f32; 3]) -> [f32; 2];
    fn viewport_to_fixed(&mut self, point: [f32; 2]) -> [f32; 2];
    fn viewport_to_world(&mut self, point: [f32; 3]) -> [f32; 3];
    fn utility_viewport_size(&self) -> [f32; 2];
    fn utility_draw_rect(&self) -> Rect;
    fn model_world_box(&mut self, model: Handle, out: &mut WorldBox);
    /// True binds the newly constructed element, false clears the model's slot.
    fn bind_model_grid_element(&mut self, model: Handle, present: bool);
}

#[derive(Debug, Default)]
pub(super) struct ContainerGeometry {
    margins: [[f32; 4]; 2],
    view: ContainerRect,
    draw: ContainerRect,
}

impl Render3d {
    pub fn initialize_container_margins(&mut self) {
        let scale = self.state.map_scale as f32;
        self.geometry.margins = [
            [3000.0, 1400.0, 5200.0, 6800.0].map(|v| scale * v),
            [4500.0, 6400.0, 7200.0, 5300.0].map(|v| scale * v),
        ];
    }

    pub fn container_margins(&self) -> [[f32; 4]; 2] {
        self.geometry.margins
    }

    fn container_size(&self) -> f32 {
        self.grid.as_ref().map_or(0.0, QuadGrid::size)
    }

    fn offset_fixed(&self, [x, y]: [f32; 2], rotation: u32) -> [f32; 2] {
        // SPEC-GAP: an invalid rotation uses rotation zero rather than an invalid array access.
        let index = if rotation < 4 { rotation as usize } else { 0 };
        [
            x + self.geometry.margins[0][index],
            -y + self.geometry.margins[1][index],
        ]
    }

    pub fn world_to_container_rect(
        &self,
        host: &mut impl ContainerServices,
        bounds: WorldBox,
        rotation: u32,
    ) -> ContainerRect {
        let [lx, ly, lz] = bounds.min;
        let [hx, hy, hz] = bounds.max;
        let extent = if rotation == 1 || rotation == 3 {
            hx - lx
        } else {
            hz - lz
        };
        let a = 0.353_553_38_f32 * extent;
        let b = 0.146_446_6_f32 * extent;
        let (first, second) = match rotation {
            1 => ([hx - b, hy, lz - a], [lx + b, ly, hz + a]),
            2 => ([hx + a, hy, hz - b], [lx - a, ly, lz + b]),
            3 => ([lx + b, hy, hz + a], [hx - b, ly, lz - a]),
            _ => ([lx - a, hy, lz + b], [hx + a, ly, hz - b]),
        };
        let first = self.offset_fixed(host.project_fixed(first), rotation);
        let second = self.offset_fixed(host.project_fixed(second), rotation);
        let mut rect = ContainerRect::new(first[0] - 2.0, first[1] - 2.0, second[0] + 2.0, second[1] + 2.0);
        let size = self.container_size();
        if rect.left <= 0.5 {
            rect.left = 0.5;
        }
        if rect.top <= 0.5 {
            rect.top = 0.5;
        }
        if rect.right >= size - 0.5 {
            rect.right = size - 0.5;
        }
        if rect.bottom >= size - 0.5 {
            rect.bottom = size - 0.5;
        }
        if rect.right <= rect.left {
            rect.left = rect.right - 0.1;
        }
        if rect.bottom <= rect.top {
            rect.top = rect.bottom - 0.1;
        }
        if rect.right - rect.left > size / 2.0 {
            rect.right = rect.left + 1.0;
        }
        if rect.bottom - rect.top > size / 2.0 {
            rect.bottom = rect.top + 1.0;
        }
        rect
    }

    pub fn model_container_rect(
        &self,
        host: &mut impl ContainerServices,
        model: Handle,
        rotation: u32,
    ) -> ContainerRect {
        let mut bounds = WorldBox::default();
        host.model_world_box(model, &mut bounds);
        self.world_to_container_rect(host, bounds, rotation)
    }

    pub fn move_grid_element(&mut self, host: &mut impl ContainerServices, model: Handle) -> bool {
        let mut bounds = WorldBox::default();
        if let Some(element) = self.grid.as_ref().and_then(|g| g.element(model)) {
            bounds = element.bounds;
        }
        host.model_world_box(model, &mut bounds);
        let rotation = host.rotation();
        let rect = self.world_to_container_rect(host, bounds, rotation);
        let result = if let Some(grid) = self.grid.as_mut() {
            if let Some(element) = grid.element_mut(model) {
                element.bounds = bounds;
                let flag = u16::from(element.flags & 8 == 0 || element.flags & 0x80 != 0);
                grid.move_element(model, rect, flag)
            } else {
                false
            }
        } else {
            false
        };
        if !result {
            host.bind_model_grid_element(model, false);
        }
        result
    }

    fn frustum_container_rect(&self, host: &mut impl ContainerServices, draw: bool) -> ContainerRect {
        let (first, second) = if draw {
            let r = host.utility_draw_rect();
            ([r.left as f32, r.top as f32], [r.right as f32, r.bottom as f32])
        } else {
            ([0.0; 2], host.utility_viewport_size())
        };
        let rotation = host.rotation();
        let first = self.offset_fixed(host.viewport_to_fixed(first), rotation);
        let second = self.offset_fixed(host.viewport_to_fixed(second), rotation);
        ContainerRect::new(first[0], first[1], second[0], second[1])
    }

    pub fn view_frustum_container_rect(&self, host: &mut impl ContainerServices) -> ContainerRect {
        self.frustum_container_rect(host, false)
    }

    pub fn draw_frustum_container_rect(&self, host: &mut impl ContainerServices) -> ContainerRect {
        self.frustum_container_rect(host, true)
    }

    pub fn visible_container_rect(&mut self, host: &mut impl ContainerServices, which: i32) -> ContainerRect {
        let draw = which == 0 || which == 1;
        let valid = if draw {
            self.state.visible_valid
        } else {
            self.state.frustum_valid
        };
        if !valid {
            let mut rect = self.frustum_container_rect(host, draw);
            let last = self.container_size() - 1.0;
            if rect.left < 0.0 {
                rect.left = 0.0;
            }
            if rect.top < 0.0 {
                rect.top = 0.0;
            }
            if rect.right > last {
                rect.right = last;
            }
            if rect.bottom > last {
                rect.bottom = last;
            }
            if draw {
                self.geometry.draw = rect;
                self.state.visible_valid = true;
            } else {
                self.geometry.view = rect;
                self.state.frustum_valid = true;
            }
        }
        if draw {
            self.geometry.draw
        } else {
            self.geometry.view
        }
    }

    pub fn viewport_ray_container_rect(&self, host: &mut impl ContainerServices, x: i32, y: i32) -> ContainerRect {
        let world = host.viewport_to_world([x as f32, y as f32, 0.0]);
        let point = host.project_fixed(world);
        let [x, y] = self.offset_fixed(point, host.rotation());
        let mut rect = ContainerRect::new(x, y, x + 2.0, y + 2.0);
        let last = self.container_size() - 2.0;
        if x < 0.0 {
            rect.left = 0.0;
        }
        if rect.left > last {
            rect.left = last;
        }
        if rect.right < rect.left + 1.0 {
            rect.right = rect.left + 1.0;
        }
        if y < 0.0 {
            rect.top = 0.0;
        }
        if rect.top > last {
            rect.top = last;
        }
        if rect.bottom < rect.top + 1.0 {
            rect.bottom = rect.top + 1.0;
        }
        rect
    }

    pub fn rebuild_quad_grid(
        &mut self,
        host: &mut impl ContainerServices,
        old_zoom: i32,
        old_rotation: u32,
        new_zoom: i32,
        new_rotation: u32,
    ) {
        if old_zoom == new_zoom && old_rotation == new_rotation {
            return;
        }
        let Some(old) = self.grid.as_mut() else {
            return;
        };
        if old.element_count() == 0 {
            return;
        }
        let models = old.all_elements(old.element_count());
        let mut grid = QuadGrid::for_map_scale(self.state.map_scale);
        for model in models {
            let old = self.grid.as_ref().unwrap().element(model).unwrap();
            let mut element = GridElement::new(model);
            element.flags = old.flags;
            element.bounds = old.bounds;
            let rect = self.model_container_rect(host, model, new_rotation);
            host.bind_model_grid_element(model, true);
            let flag = u16::from(element.flags & 8 == 0 || element.flags & 0x80 != 0);
            if !grid.add_element(element.clone(), rect, flag)
                && !grid.add_element(element, ContainerRect::new(0.0, 0.0, 1.0, 1.0), flag)
            {
                host.bind_model_grid_element(model, false);
            }
        }
        self.grid = Some(grid);
    }
}
