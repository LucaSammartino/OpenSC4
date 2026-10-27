// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl MiniMap {
    pub fn init(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<bool> {
        if windows.get(self.window).flag(0x4000) {
            return Ok(true);
        }
        let Some(surface) = host.create_surface(0xc416025c, 0x0073283c)? else {
            return Ok(false);
        };
        self.surface = Some(surface);
        match host.init_surface(surface, self.n, [9, 32]) {
            Ok(true) => {}
            result => {
                host.release(surface);
                self.surface = None;
                return result.map(|_| false);
            }
        }
        let lock = match host.surface_lock(surface, 0x8001) {
            Ok(lock) => lock,
            Err(error) => {
                host.release(surface);
                self.surface = None;
                return Err(error);
            }
        };
        if let Some(mut lock) = lock {
            host.surface_clear(surface, [100, 100, 127, 0], &mut lock);
            host.surface_unlock(surface, lock);
        }
        if !windows.init(host, self.window) {
            self.shutdown(windows, host);
            return Ok(false);
        }
        for message in SUBSCRIPTIONS {
            host.subscribe(self.window, message, true);
        }
        self.terrain_palette.fill(0);
        self.water_palette.fill(0);
        self.set_default_zone_color(0xe57300);
        for i in 1..=3 {
            self.set_zone_color(i, 0x00ec00);
        }
        for i in 4..=6 {
            self.set_zone_color(i, 0x7979ff);
        }
        for i in 7..=9 {
            self.set_zone_color(i, 0xc7bc00);
        }
        self.load_tunable_values(host)?;
        self.on_city_init(host)?;
        Ok(true)
    }

    pub fn shutdown(&mut self, windows: &mut Windows, host: &mut impl Host) -> bool {
        if windows.get(self.window).flag(0x4000) {
            self.unhook_occupant_messages(host);
            for message in SUBSCRIPTIONS {
                host.subscribe(self.window, message, false);
            }
            if let Some(surface) = self.surface.take() {
                host.shutdown_surface(surface);
                host.release(surface);
            }
            windows.shutdown(host, self.window);
        }
        true
    }

    /// Explicit native-style destruction: adapters call Shutdown/OnCityShutdown
    /// at their specified lifecycle events before destroying a live window.
    /// This consumes all references held by this owner, independent of Init.
    pub fn deleting_destructor(&mut self, windows: &mut Windows, host: &mut impl Host, flags: u32) -> Window {
        if windows.valid(self.window) {
            self.networks = Vec::new();
            self.map.free();
            self.zone_colors.fill(0);
            if let Some(overlay) = self.overlay.take() {
                host.release(overlay);
            }
            if let Some(surface) = self.surface.take() {
                host.release(surface);
            }
            windows.deleting_destructor(host, self.window, flags);
        }
        self.window
    }

    pub fn add_ref(&self, windows: &mut Windows) -> u32 {
        windows.add_ref(self.window)
    }

    pub fn release(&mut self, windows: &mut Windows, host: &mut impl Host) -> u32 {
        if windows.get(self.window).references > 1 {
            windows.release(host, self.window)
        } else {
            self.deleting_destructor(windows, host, 1);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    #[test]
    fn init_failure_success_defaults_subscriptions_and_idempotent_shutdown() {
        let (mut w, mut h, mut m) = fixture();
        m.set_area(&mut w, &mut h, base::Rect(0, 0, 128, 128)).unwrap();
        assert_eq!(m.init(&mut w, &mut h), Err(Unsupported::Open("MMW4")));
        h.factory = Some(None);
        assert!(!m.init(&mut w, &mut h).unwrap());
        h.factory = Some(Some(10));
        assert!(!m.init(&mut w, &mut h).unwrap());
        assert_eq!(m.surface, None);
        assert_eq!(h.events.last().unwrap(), "release:10");
        h.init_surface_ok = true;
        h.cells = [256, 256];
        assert!(m.init(&mut w, &mut h).unwrap());
        assert!(w.get(m.window).flag(0x4000));
        assert_eq!(m.zone_colors[0], 0);
        assert_eq!(&m.zone_colors[1..4], &[0xff00ec00; 3]);
        assert_eq!(&m.zone_colors[4..7], &[0xff7979ff; 3]);
        assert_eq!(&m.zone_colors[7..10], &[0xffc7bc00; 3]);
        assert_eq!(m.zone_colors[15], 0xffe57300);
        assert_eq!(
            h.events
                .iter()
                .filter(|s| s.starts_with("sub:") && s.ends_with(":true"))
                .count(),
            11
        );
        let count = h.events.len();
        assert!(m.init(&mut w, &mut h).unwrap());
        assert_eq!(h.events.len(), count);
        assert!(m.shutdown(&mut w, &mut h));
        assert_eq!(m.surface, None);
        assert!(m.attached); // Shutdown is distinct from city shutdown
        let count = h.events.len();
        assert!(m.shutdown(&mut w, &mut h));
        assert_eq!(h.events.len(), count);
        m.on_city_shutdown(&mut h);
        assert!(!m.attached);
    }

    #[test]
    fn final_release_and_non_deleting_destructor_release_owned_resources_once() {
        for flags in [0, 1] {
            let (mut w, mut h, mut m) = fixture();
            m.surface = Some(10); // created reference
            m.set_overlay(&mut h, Some(20));
            m.map.init(16, 16).unwrap();
            m.networks = vec![3; 16];
            h.events.clear();
            let handle = m.deleting_destructor(&mut w, &mut h, flags);
            assert_eq!(handle, m.as_igzwin());
            assert!(!w.valid(handle));
            assert!(m.map.pixels().is_none() && m.networks.is_empty());
            assert_eq!(h.events, ["release:20", "release:10"]);
            m.deleting_destructor(&mut w, &mut h, flags);
            assert_eq!(h.events.len(), 2);
        }
        let (mut w, mut h, mut m) = fixture();
        assert_eq!(m.add_ref(&mut w), 1);
        assert_eq!(m.add_ref(&mut w), 2);
        assert_eq!(m.release(&mut w, &mut h), 1);
        assert!(w.valid(m.window));
        assert_eq!(m.release(&mut w, &mut h), 0);
        assert!(!w.valid(m.window));
    }
}
