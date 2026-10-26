// SPDX-License-Identifier: GPL-3.0-or-later
//! AU44 native owner; the parent module's old fixture is not a native lifecycle.
pub trait Host {
    fn service(&mut self, id: u32, iid: u32) -> Option<u64>;
    fn retain(&mut self, p: u64);
    fn release(&mut self, p: u64);
    fn subscribe(&mut self, server: u64, owner: u64, message: u32, add: bool);
    fn create_scape(&mut self, scape: u64, name: &str, id: u32, flags: u32, radius: f32) -> bool;
    fn add_sound(&mut self, scape: u64, group: u32, sound: u32, band: [u32; 2]);
    fn query(&mut self, p: u64, iid: u32) -> (bool, Option<u64>);
    fn call(&mut self, p: u64, offset: u32) -> u64;
    fn position(&mut self, p: u64, out: &mut [f32; 3]);
    fn emanation(&mut self, scape: u64, key: u64, position: Option<[f32; 3]>);
    /// Renderer slot 44, with the embedded fire filter; outputs are borrowed.
    fn enumerate(&mut self, owner: u64, filter: u64) -> Vec<u64>;
}
#[derive(Debug, Default)]
pub struct Owner {
    pub pointer: u64,
    pub refcount: u32,
    pub filter_count: u32,
    pub initialized: bool,
    pub scape: Option<u64>,
    pub words: [u32; 4],
}
impl Owner {
    pub fn factory(pointer: Option<u64>) -> Option<Self> {
        pointer.map(|pointer| Self {
            pointer,
            ..Self::default()
        })
    }
    pub fn query_interface(&mut self, iid: u32, out: &mut u64) -> bool {
        let offset = match iid {
            1 | 0x89E5_1472 => 4,
            0x4522_94AA => 0,
            0xBB52_3BB2 => 0x14,
            _ => return false,
        };
        *out = self.pointer + offset;
        self.add_ref();
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }
    pub fn release(&mut self, h: &mut impl Host) -> u32 {
        if self.refcount as i32 > 1 {
            self.refcount -= 1;
            self.refcount
        } else {
            self.destroy(h);
            self.refcount = 0;
            0
        }
    }
    pub fn destroy(&mut self, h: &mut impl Host) {
        if let Some(p) = self.scape.take() {
            h.release(p);
        }
    }
    pub fn init(&mut self, h: &mut impl Host) -> bool {
        if self.initialized {
            return true;
        }
        let Some(server) = h.service(0x04FA_845B, 0x6522_94C7) else {
            return false;
        };
        for m in [0x99EF_1142, 0x99EF_1143, 0x26D3_1EC1] {
            h.subscribe(server, self.pointer, m, true);
        }
        let incoming = h.service(0x49E5_0BA1, 0x29E3_9DB4);
        if let Some(p) = incoming {
            h.retain(p);
        }
        if let Some(p) = self.scape.take() {
            h.release(p);
        }
        self.scape = incoming;
        let result = if let Some(scape) = self.scape {
            if h.create_scape(scape, "Fires", 0x49E4_CE85, 0, 100.0) {
                for (sound, band) in [
                    (0x29DC_19EF, [1, 9]),
                    (0x09DC_19A2, [10, 49]),
                    (0x09DC_1982, [50, 0x7FFF_FFFF]),
                ] {
                    h.add_sound(scape, 0x49E4_CE85, sound, band);
                }
                self.initialized = true;
                true
            } else {
                false
            }
        } else {
            false
        };
        if let Some(p) = incoming {
            h.release(p);
        }
        h.release(server);
        result
    }
    pub fn shutdown(&mut self, h: &mut impl Host) -> bool {
        if !self.initialized {
            return true;
        }
        if let Some(server) = h.service(0x04FA_845B, 0x6522_94C7) {
            for m in [0x26D3_1EC1, 0x99EF_1142, 0x99EF_1143] {
                h.subscribe(server, self.pointer, m, false);
            }
            h.release(server);
        }
        self.destroy(h);
        self.initialized = false;
        true
    }
    pub fn predicate(p: u64, h: &mut impl Host) -> bool {
        let (ok, view) = h.query(p, 0x69D2_5C60);
        if let Some(v) = view {
            h.release(v);
        }
        ok
    }
    pub fn callback(&mut self, occupant: Option<u64>, h: &mut impl Host) -> bool {
        let Some(p) = occupant else { return false };
        if self.pointer == 0 {
            return false;
        }
        self.add_ref();
        let mut position = [0.0; 3];
        h.position(p, &mut position);
        h.emanation(self.scape.expect("unchecked scape"), p, Some(position));
        self.release(h);
        true
    }
    pub fn do_message(&mut self, message: Option<u64>, h: &mut impl Host) -> bool {
        let Some(m) = message else { return true };
        let (ok, payload) = h.query(m, 0x6529_7976);
        if ok {
            let p = payload.expect("successful message QI");
            let object = h.call(p, 0x80);
            if object != 0 {
                h.retain(object);
            }
            let kind = h.call(m, 0x0C) as u32;
            match kind {
                0x99EF_1142 | 0x99EF_1143 if object != 0 => {
                    let (accepted, position) = h.query(object, 0x26BB_767E);
                    if accepted {
                        let view = position.expect("successful position QI");
                        if Self::predicate(view, h) {
                            let mut p = [0.0; 3];
                            h.position(view, &mut p);
                            let insert = kind == 0x99EF_1142;
                            h.emanation(self.scape.expect("unchecked scape"), view, insert.then_some(p));
                        }
                    }
                    if let Some(v) = position {
                        h.release(v);
                    }
                }
                0x26D3_1EC1 => {
                    for o in h.enumerate(self.pointer, self.pointer + 8) {
                        self.callback(Some(o), h);
                    }
                }
                _ => {}
            }
            if object != 0 {
                h.release(object);
            }
        }
        if let Some(p) = payload {
            h.release(p);
        }
        true
    }
    pub fn read_version(version: u16, error: bool) -> bool {
        version == 3 && !error
    }
    pub fn write_version() -> u16 {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Trace {
        missing: u32,
        register: bool,
        kind: u32,
        subscriptions: Vec<(u32, bool)>,
        bands: Vec<(u32, [u32; 2])>,
        releases: Vec<u64>,
        retains: Vec<u64>,
        emanations: Vec<(u64, Option<[f32; 3]>)>,
    }
    impl Host for Trace {
        fn service(&mut self, id: u32, _: u32) -> Option<u64> {
            (id != self.missing).then_some(u64::from(id))
        }
        fn retain(&mut self, p: u64) {
            self.retains.push(p);
        }
        fn release(&mut self, p: u64) {
            self.releases.push(p);
        }
        fn subscribe(&mut self, _: u64, _: u64, m: u32, a: bool) {
            self.subscriptions.push((m, a));
        }
        fn create_scape(&mut self, _: u64, n: &str, id: u32, f: u32, r: f32) -> bool {
            assert_eq!((n, id, f, r), ("Fires", 0x49E4_CE85, 0, 100.0));
            self.register
        }
        fn add_sound(&mut self, _: u64, _: u32, s: u32, b: [u32; 2]) {
            self.bands.push((s, b));
        }
        fn query(&mut self, _: u64, i: u32) -> (bool, Option<u64>) {
            (
                true,
                Some(match i {
                    0x6529_7976 => 20,
                    0x26BB_767E => 30,
                    _ => 40,
                }),
            )
        }
        fn call(&mut self, p: u64, o: u32) -> u64 {
            if p == 20 && o == 0x80 {
                10
            } else {
                u64::from(self.kind)
            }
        }
        fn position(&mut self, p: u64, out: &mut [f32; 3]) {
            assert!(p == 30 || p == 10);
            *out = [1.0, 2.0, 3.0];
        }
        fn emanation(&mut self, _: u64, k: u64, p: Option<[f32; 3]>) {
            self.emanations.push((k, p));
        }
        fn enumerate(&mut self, p: u64, f: u64) -> Vec<u64> {
            assert_eq!(f, p + 8);
            vec![10]
        }
    }
    #[test]
    fn au44_identity_partial_init_and_shutdown_worked_cases() {
        assert!(Owner::factory(None).is_none());
        let mut o = Owner::factory(Some(100)).unwrap();
        assert_eq!((o.refcount, o.filter_count, o.words), (0, 0, [0; 4]));
        for (iid, p) in [(1, 104), (0x4522_94AA, 100), (0x89E5_1472, 104), (0xBB52_3BB2, 120)] {
            let mut out = 0;
            assert!(o.query_interface(iid, &mut out));
            assert_eq!(out, p);
        }
        let mut out = 999;
        assert!(!o.query_interface(99, &mut out));
        assert_eq!(out, 999);
        let mut h = Trace {
            missing: 0x04FA_845B,
            ..Trace::default()
        };
        assert!(!o.init(&mut h));
        assert!(h.subscriptions.is_empty());
        h.missing = 0x49E5_0BA1;
        assert!(!o.init(&mut h));
        assert_eq!(h.subscriptions.len(), 3);
        o.shutdown(&mut h);
        assert_eq!(h.subscriptions.len(), 3);
        h.missing = 0;
        assert!(!o.init(&mut h));
        assert!(o.scape.is_some());
        assert!(h.bands.is_empty());
        h.register = true;
        assert!(o.init(&mut h));
        assert!(o.init(&mut h));
        assert_eq!(
            h.bands,
            [
                (0x29DC_19EF, [1, 9]),
                (0x09DC_19A2, [10, 49]),
                (0x09DC_1982, [50, 0x7FFF_FFFF])
            ]
        );
        o.shutdown(&mut h);
        assert_eq!(
            &h.subscriptions[h.subscriptions.len() - 3..],
            &[(0x26D3_1EC1, false), (0x99EF_1142, false), (0x99EF_1143, false)]
        );
    }
    #[test]
    fn au44_dispatch_position_keys_callback_and_destructor() {
        let mut o = Owner::factory(Some(100)).unwrap();
        o.refcount = 1;
        o.scape = Some(2);
        let mut h = Trace {
            kind: 0x99EF_1142,
            ..Trace::default()
        };
        assert!(o.do_message(None, &mut h));
        assert!(o.do_message(Some(1), &mut h));
        h.kind = 0x99EF_1143;
        o.do_message(Some(1), &mut h);
        h.kind = 0x26D3_1EC1;
        o.do_message(Some(1), &mut h);
        assert_eq!(
            h.emanations,
            [(30, Some([1.0, 2.0, 3.0])), (30, None), (10, Some([1.0, 2.0, 3.0]))]
        );
        assert_eq!(h.retains, [10, 10, 10]);
        assert_eq!(o.refcount, 1);
        assert!(!o.callback(None, &mut h));
        o.initialized = true;
        o.destroy(&mut h);
        assert!(h.subscriptions.is_empty());
        assert_eq!(h.releases.last(), Some(&2));
    }
}
