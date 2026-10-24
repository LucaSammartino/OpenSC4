// SPDX-License-Identifier: GPL-3.0-or-later
//! Listener's settled dispatch/output stages, audio-owner-contracts §18.
//! Camera geometry, tuning resource decoding and failure lifetime remain AU40.

pub type Vector = [f32; 3];
const CITY_INIT: u32 = 0x26D3_1EC1;
const CITY_SHUTDOWN: u32 = 0x26D3_1EC2;
const TUNING: u32 = 0xC53D_10AA;
const CAMERAS: [u32; 2] = [0xA6B7_9602, 0xA6B7_9612];

pub trait ListenerHost {
    fn acquire_message_server(&mut self) -> Option<u64>;
    fn acquire_city_sound_system(&mut self) -> Option<u64>;
    fn subscribe(&mut self, server: u64, message: u32, add: bool);
    fn release(&mut self, reference: u64);
    fn sound_event(&mut self, system: u64, event: u32);
    fn set_orientation(&mut self, system: u64, orientation: Vector, companion: Vector);
    fn set_position(&mut self, system: u64, position: Vector);
    /// Both payloads are live addresses into the boxed listener, not snapshots.
    /// Allocation/delivery lifetime is explicitly outside this settled stage (AU40).
    fn notify_position(&mut self, message: u32, listener: usize, position: usize);
}

/// The geometry stage's result. Producing it from a camera is still AU40;
/// callers may replay measured geometry, but cannot claim camera closure.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedGeometry {
    pub position: Vector,
    pub orientation: Vector,
    pub companion: Vector,
    pub zoom: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenerOpen {
    Au40,
}

#[derive(Debug)]
pub struct AudioListener {
    pub refcount: u32,
    pub initialized: bool,
    pub position: Vector,
    pub orientation: Vector,
    pub companion: Vector,
    pub last_notified: Vector,
    pub arrays: [Vec<f32>; 4],
    pub scale: f32,
    pub scalar_70: f32,
    pub sound_system: Option<u64>,
    pub message_server: Option<u64>,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self {
            refcount: 0,
            initialized: false,
            position: [0.0; 3],
            orientation: [0.0; 3],
            companion: [0.0; 3],
            last_notified: [0.0; 3],
            arrays: std::array::from_fn(|_| Vec::new()),
            scale: 1.0,
            scalar_70: 4.0,
            sound_system: None,
            message_server: None,
        }
    }
}

impl AudioListener {
    /// Box keeps addresses in position notifications stable when the owner moves.
    pub fn construct() -> Box<Self> {
        Box::new(Self::default())
    }

    pub fn query_interface(&mut self, iid: u32) -> bool {
        if !matches!(iid, 1 | 0x4522_94AA | 0xCA4A_66B9) {
            return false;
        }
        self.add_ref();
        true
    }

    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }

    pub fn init(&mut self, host: &mut impl ListenerHost) -> Result<(), ListenerOpen> {
        if self.initialized {
            return Ok(());
        }
        if self.message_server.is_none() {
            self.message_server = host.acquire_message_server();
        }
        let server = self.message_server.ok_or(ListenerOpen::Au40)?;
        for message in [CITY_INIT, CITY_SHUTDOWN, TUNING] {
            host.subscribe(server, message, true);
        }
        self.initialized = true;
        Ok(())
    }

    pub fn shutdown(&mut self, host: &mut impl ListenerHost) {
        if self.initialized {
            if let Some(server) = self.message_server {
                for message in [CITY_INIT, CITY_SHUTDOWN, TUNING] {
                    host.subscribe(server, message, false);
                }
            }
            self.initialized = false;
        }
    }

    pub fn city_shutdown(&mut self, host: &mut impl ListenerHost) {
        if let Some(server) = self.message_server {
            for message in CAMERAS {
                host.subscribe(server, message, false);
            }
        }
        if let Some(system) = self.sound_system.take() {
            host.release(system);
        }
    }

    pub fn calculate_zoom(&self, global_heights: &mut [f32; 5]) -> Result<(), ListenerOpen> {
        if self.arrays[0].len() < 5 || self.arrays[1].len() < 5 {
            return Err(ListenerOpen::Au40);
        }
        for (i, height) in global_heights.iter_mut().enumerate() {
            // §18.3: no intermediate f32 store between the two multiplies.
            *height = (f64::from(self.arrays[1][i]) * f64::from(self.arrays[0][i]) * f64::from(self.scale)) as f32;
        }
        Ok(())
    }

    /// Executes only after the unresolved geometry stage. Thresholds are an
    /// explicit input because their tuning-array mapping remains AU40.
    pub fn publish_geometry(&mut self, geometry: ResolvedGeometry, threshold: f32, host: &mut impl ListenerHost) {
        self.position = geometry.position;
        self.orientation = geometry.orientation;
        self.companion = geometry.companion;
        if let Some(system) = self.sound_system {
            host.set_orientation(system, self.orientation, self.companion);
            host.set_position(system, self.position);
        }
        let squared: f64 = self
            .position
            .iter()
            .zip(self.last_notified)
            .map(|(a, b)| {
                let difference = f64::from(*a) - f64::from(b);
                difference * difference
            })
            .sum();
        if squared > f64::from(threshold) {
            self.last_notified = self.position;
            host.notify_position(
                0x8A4A_69C7,
                self as *const Self as usize,
                self.position.as_ptr() as usize,
            );
        }
    }

    /// Returns the outer bool and a diagnostic for work whose algorithm is
    /// explicitly unsettled. Unknown messages are successful no-ops (§18.2).
    pub fn do_message(&mut self, message: Option<u32>, host: &mut impl ListenerHost) -> (bool, Option<ListenerOpen>) {
        let Some(message) = message else {
            return (false, None);
        };
        match message {
            CITY_INIT => {
                self.sound_system = host.acquire_city_sound_system();
                if let Some(server) = self.message_server {
                    for camera in CAMERAS {
                        host.subscribe(server, camera, true);
                    }
                }
                // Read tuning, calculate zoom, then geometry update: AU40.
                (true, Some(ListenerOpen::Au40))
            }
            CITY_SHUTDOWN => {
                self.city_shutdown(host);
                (true, None)
            }
            TUNING => {
                // The event is after read/recalculation. Do not emit it early
                // while those stages are unsupported (AU40).
                (true, Some(ListenerOpen::Au40))
            }
            0xA6B7_9602 | 0xA6B7_9612 | 0x230E_27AC => (true, Some(ListenerOpen::Au40)),
            _ => (true, None),
        }
    }

    /// Settled continuation after tuning read and zoom recalculation.
    pub fn tuning_recalculated(&mut self, host: &mut impl ListenerHost) {
        if let Some(system) = self.sound_system {
            host.sound_event(system, 0xAA36_D9B6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        subscriptions: Vec<(u32, bool)>,
        releases: Vec<u64>,
        calls: Vec<u32>,
        payload: Option<(usize, usize)>,
    }
    impl ListenerHost for Host {
        fn acquire_message_server(&mut self) -> Option<u64> {
            Some(1)
        }
        fn acquire_city_sound_system(&mut self) -> Option<u64> {
            Some(2)
        }
        fn subscribe(&mut self, _: u64, message: u32, add: bool) {
            self.subscriptions.push((message, add));
        }
        fn release(&mut self, reference: u64) {
            self.releases.push(reference);
        }
        fn sound_event(&mut self, _: u64, event: u32) {
            self.calls.push(event);
        }
        fn set_orientation(&mut self, _: u64, _: Vector, _: Vector) {
            self.calls.push(1);
        }
        fn set_position(&mut self, _: u64, _: Vector) {
            self.calls.push(2);
        }
        fn notify_position(&mut self, message: u32, listener: usize, position: usize) {
            self.calls.push(message);
            self.payload = Some((listener, position));
        }
    }

    #[test]
    fn au9_01_02_dispatch_city_lifetime_and_explicit_geometry_gap() {
        let mut listener = AudioListener::construct();
        let mut host = Host::default();
        assert_eq!(listener.do_message(None, &mut host), (false, None));
        assert_eq!(listener.do_message(Some(99), &mut host), (true, None));
        listener.init(&mut host).unwrap();
        listener.init(&mut host).unwrap();
        assert_eq!(host.subscriptions.len(), 3);
        assert!(listener.query_interface(0xCA4A_66B9));
        assert_eq!(listener.refcount, 1);
        assert_eq!(
            listener.do_message(Some(CITY_INIT), &mut host),
            (true, Some(ListenerOpen::Au40))
        );
        listener.tuning_recalculated(&mut host);
        assert_eq!(host.calls, [0xAA36_D9B6]);
        listener.do_message(Some(CITY_SHUTDOWN), &mut host);
        assert_eq!(
            &host.subscriptions[3..],
            [
                (CAMERAS[0], true),
                (CAMERAS[1], true),
                (CAMERAS[0], false),
                (CAMERAS[1], false)
            ]
        );
        assert_eq!(host.releases, [2]);
        listener.shutdown(&mut host);
        assert!(!listener.initialized);
    }

    #[test]
    fn au9_03_04_threshold_order_and_live_pointer_payload() {
        let mut listener = AudioListener::construct();
        listener.sound_system = Some(2);
        let mut host = Host::default();
        let mut geometry = ResolvedGeometry {
            position: [2.0, 0.0, 0.0],
            orientation: [1.0; 3],
            companion: [0.0; 3],
            zoom: 0,
        };
        listener.publish_geometry(geometry, 4.0, &mut host);
        assert_eq!(host.calls, [1, 2]);
        geometry.position[0] = 3.0;
        listener.publish_geometry(geometry, 4.0, &mut host);
        assert_eq!(host.calls, [1, 2, 1, 2, 0x8A4A_69C7]);
        assert_eq!(
            host.payload,
            Some((
                &*listener as *const AudioListener as usize,
                listener.position.as_ptr() as usize
            ))
        );
        assert_eq!(listener.last_notified, [3.0, 0.0, 0.0]);
        geometry.position[0] = 100.0;
        listener.publish_geometry(geometry, f32::NAN, &mut host);
        assert_eq!(listener.last_notified, [3.0, 0.0, 0.0]);
    }

    #[test]
    fn au9_05_zoom_defaults_and_no_intermediate_store() {
        let mut listener = AudioListener::default();
        assert_eq!((listener.scale, listener.scalar_70), (1.0, 4.0));
        let mut heights = [0.0; 5];
        assert_eq!(listener.calculate_zoom(&mut heights), Err(ListenerOpen::Au40));
        listener.arrays[0] = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        listener.arrays[1] = vec![3.0; 5];
        listener.scale = 0.5;
        listener.calculate_zoom(&mut heights).unwrap();
        assert_eq!(heights, [3.0, 6.0, 9.0, 12.0, 15.0]);
    }
}
