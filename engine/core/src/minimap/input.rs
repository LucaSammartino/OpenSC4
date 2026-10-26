// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use super::numeric::E;

pub trait Input {
    fn terrain_height_at(&mut self, x: f32, z: f32) -> Result<f32>;
    fn set_view_target(&mut self, target: [f32; 3]);
}

fn coordinate(x: i32, extent: i32) -> f32 {
    if extent == 0 { return (x as f32 / 0.0) - 0.5; }
    E::constant(f64::from(x)).div(E::constant(f64::from(extent))).sub(E::constant(0.5)).single()
}

fn clamp(x: f32) -> f32 {
    if x.is_nan() || x >= 1.0 { 1.0 } else if x <= 0.0 { 0.0 } else { x }
}

impl MiniMap {
    pub fn center_on_map_point(&self, windows: &Windows, host: &mut impl Host, x: i32, y: i32) -> Result<()> {
        let area = windows.get(self.window).area;
        let (mut u, mut v) = (coordinate(x, area.width()), coordinate(y, area.height()));
        let rotation = host.rotation();
        if rotation & 1 != 0 { (u, v) = (-v, u); }
        if rotation & 2 != 0 { u = -u; v = -v; }
        let [sx, sz] = host.world_size();
        let x = clamp(u + 0.5) * sx;
        let z = clamp(v + 0.5) * sz;
        let h = host.terrain_height_at(x, z)?;
        host.set_view_target([x, h, z]);
        Ok(())
    }

    pub fn mouse_down(&mut self, windows: &mut Windows, host: &mut impl Host, x: i32, y: i32) -> Result<bool> {
        self.center_on_map_point(windows, host, x, y)?;
        self.dragging = true;
        windows.capture = Some(self.window);
        host.capture(self.window, true);
        Ok(true)
    }

    pub fn mouse_move(&self, windows: &Windows, host: &mut impl Host, x: i32, y: i32) -> Result<bool> {
        if self.dragging { self.center_on_map_point(windows, host, x, y)?; }
        Ok(true)
    }

    pub fn mouse_up(&mut self, windows: &mut Windows, host: &mut impl Host) -> bool {
        self.dragging = false;
        windows.capture = None;
        host.capture(self.window, false);
        true
    }

    pub fn mouse_enter_or_exit(&mut self) -> bool {
        self.dragging = false;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::fixture;

    #[test]
    fn mm_c10_center_drag_capture_rotations_and_nan_clamp() {
        let (mut w, mut h, mut m) = fixture();
        m.set_area(&mut w, &mut h, base::Rect(0, 0, 200, 150)).unwrap();
        m.mouse_down(&mut w, &mut h, 100, 75).unwrap();
        assert_eq!(h.targets, [[50.0, 12.0, 50.0]]); // MM-C10
        assert_eq!(w.capture, Some(m.window));
        h.rotation = 1;
        m.mouse_move(&w, &mut h, 200, 0).unwrap();
        assert_eq!(h.targets.last().unwrap(), &[100.0, 12.0, 100.0]);
        m.mouse_enter_or_exit();
        let count = h.targets.len();
        m.mouse_move(&w, &mut h, 0, 0).unwrap();
        assert_eq!(h.targets.len(), count);
        assert_eq!(w.capture, Some(m.window)); // enter/exit only clears dragging
        assert!(m.mouse_up(&mut w, &mut h));
        assert_eq!(w.capture, None);
        h.rotation = 0;
        m.set_area(&mut w, &mut h, base::Rect(0, 0, 0, 0)).unwrap();
        m.center_on_map_point(&w, &mut h, 0, 0).unwrap();
        assert_eq!(h.targets.last().unwrap(), &[100.0, 12.0, 100.0]);
        assert_eq!(clamp(f32::NAN), 1.0);
        assert_eq!(clamp(f32::NEG_INFINITY), 0.0);
    }
}
