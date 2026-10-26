// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use base::Rect;

pub const SUBSCRIPTIONS: [u32; 9] = [0x89171496, 0x678e6e3a, 0x278e6003, 0x07699455,
    0xa6b79602, 0xa6b79612, 0x26d31ec1, 0x26d31ec2, 0xc53d10aa];

#[derive(Clone, Copy, Debug, Default)]
pub enum Payload {
    #[default]
    None,
    /// Message slot 15 points to the four cell-rectangle words.
    Rect(Rect),
    /// Raw arrays addressed by message slots 10 and 15.
    ViewChange { a: [u32; 2], b: [u32; 2] },
    /// Borrowed occupant from message slot 10; QI takes a separate reference.
    Occupant(Handle),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Message {
    pub kind: u32,
    /// Message slots 9, 14 and 19, without type conversion.
    pub data1: u32,
    pub data2: u32,
    pub word19: u32,
    pub payload: Payload,
}

impl MiniMap {
    pub fn set_flags(&mut self, host: &mut impl Host, mask: u32, value: u32) -> u32 {
        self.unhook_occupant_messages(host);
        self.flags = (self.flags & !mask) ^ value;
        self.invalidate_tz_map();
        self.flags
    }

    pub fn tick(&mut self, windows: &mut Windows, host: &mut impl Host, word: u32) -> Result<()> {
        if !visible_recursive(windows, self.window) { self.map.free(); return Ok(()); }
        self.elapsed += f32::from_bits(word);
        if self.elapsed > 1000.0 {
            self.elapsed = 0.0;
            self.update(windows, host)?;
            if self.flags & 0x10 != 0 {
                if let Some(service) = host.marker_service()? {
                    if host.marker_active(service) { self.repaint = true; }
                }
            }
        }
        if self.repaint { self.request_repaint(windows, host); self.repaint = false; }
        Ok(())
    }

    pub fn do_message(&mut self, windows: &mut Windows, host: &mut impl Host, message: Message) -> Result<bool> {
        match message.kind {
            0x26d31ec1 => { self.on_city_init(host)?; self.update(windows, host)?; self.request_repaint(windows, host); }
            0x26d31ec2 => self.on_city_shutdown(host),
            0x07699455 => {
                let Payload::Rect(rect) = message.payload else { return Err(Unsupported::Precondition("rectangle message payload")); };
                self.invalidate_tz_map_rect(rect)?;
            }
            0x278e6003 | 0x678e6e3a | 0x89171496 => {
                self.invalidate_tz_map_rect(Rect((message.data1 >> 16) as i32, (message.data2 >> 16) as i32,
                    (message.data1 & 0xffff) as i32, (message.data2 & 0xffff) as i32))?;
            }
            0x86ad10ee => self.tick(windows, host, message.word19)?,
            0x99ef1142 | 0x99ef1143 => {
                if message.word19 == 0xc772bf98 {
                    let Payload::Occupant(occupant) = message.payload else { return Err(Unsupported::Precondition("network occupant message payload")); };
                    self.network_changed(host, occupant, message.word19, message.kind == 0x99ef1143)?;
                }
            }
            0xa6b79602 => {
                let Payload::ViewChange { a, b } = message.payload else { return Err(Unsupported::Precondition("view-change arrays")); };
                if a[1] != b[1] { self.invalidate_tz_map(); }
                if a[0] != b[0] { self.repaint = true; }
            }
            0xa6b79612 => self.repaint = true,
            0xc53d10aa => {
                if message.data1 == TUNING_KEY[1] && message.data2 == TUNING_KEY[2] { self.load_tunable_values(host)?; }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::fixture;

    #[test]
    fn mm_c8_tick_exceeds_threshold_once_and_hidden_releases_map() {
        let (mut w, mut h, mut m) = fixture();
        m.n = 1;
        m.map.init(1, 1).unwrap();
        m.surface = Some(10);
        m.flags = 0;
        m.surface_dirty = true;
        h.surfaces.insert(10, SurfacePixels { pixels: vec![0], pitch: 1 });
        m.tick(&mut w, &mut h, 600f32.to_bits()).unwrap();
        assert!(h.events.is_empty());
        m.tick(&mut w, &mut h, 500f32.to_bits()).unwrap();
        assert_eq!(h.events.iter().filter(|s| s.starts_with("lock:")).count(), 1);
        assert_eq!(m.elapsed, 0.0);
        m.surface_dirty = true;
        m.tick(&mut w, &mut h, 1000f32.to_bits()).unwrap();
        assert!(m.surface_dirty); // exactly 1000 does not update
        w.get_mut(m.window).flags &= !1;
        m.tick(&mut w, &mut h, 500f32.to_bits()).unwrap();
        assert!(m.map.pixels().is_none());
        assert_eq!(m.elapsed, 1000.0);
    }

    #[test]
    fn mm_c9_flags_unhook_before_change_and_xor_unmasked_value() {
        let (_, mut h, mut m) = fixture();
        m.attached = true;
        m.hook_occupant_messages(&mut h);
        h.events.clear();
        assert_eq!(m.set_flags(&mut h, 2, 0), 5); // MM-C9
        assert_eq!(h.events, ["sub:99ef1142:false", "sub:99ef1143:false"]);
        assert!(!m.hooked);
        assert_eq!(m.set_flags(&mut h, 0, 1), 4); // XOR, value not limited by mask
        assert!(m.map_dirty && m.surface_dirty);
        m.hook_occupant_messages(&mut h);
        assert!(m.hooked);
        assert_eq!(h.events.len(), 2); // marked hooked without subscriptions
    }

    #[test]
    fn zone_setters_and_overlay_reference_order() {
        let (_, mut h, mut m) = fixture();
        m.set_default_zone_color(0x123456);
        assert_eq!(m.zone_colors[0], 0);
        assert_eq!(m.zone_colors[15], 0xff123456);
        m.set_zone_color(0, 1);
        m.set_zone_color(16, 2);
        assert_eq!(m.zone_colors[0], 0xff000001);
        m.set_zone_color(1, 0);
        assert_eq!(m.zone_colors[1], 0);
        m.set_default_zone_color_to_trans();
        assert_eq!(m.zone_colors, [0; 16]);
        assert!(!m.map_dirty && !m.surface_dirty); // setters do not invalidate
        m.set_overlay(&mut h, Some(10));
        m.set_overlay(&mut h, Some(10));
        m.set_overlay(&mut h, None);
        assert_eq!(h.events, ["add:10", "add:10", "release:10", "release:10"]);
        assert!(m.surface_dirty);
    }

    #[test]
    fn message_rects_view_changes_tuning_identity_and_unknown() {
        let (mut w, mut h, mut m) = fixture();
        for kind in [0x278e6003, 0x678e6e3a, 0x89171496] {
            m.dirty.fill(0);
            assert!(m.do_message(&mut w, &mut h, Message { kind, data1: (16 << 16) | 31,
                data2: (32 << 16) | 47, ..Message::default() }).unwrap());
            assert_eq!(m.dirty[2], 2);
        }
        m.dirty.fill(0);
        m.do_message(&mut w, &mut h, Message { kind: 0x07699455, payload: Payload::Rect(Rect(0, 0, 0, 0)), ..Message::default() }).unwrap();
        assert_eq!(m.dirty[0], 1);
        m.do_message(&mut w, &mut h, Message { kind: 0xa6b79602, payload: Payload::ViewChange { a: [1, 2], b: [3, 4] }, ..Message::default() }).unwrap();
        assert_eq!(m.dirty, [u32::MAX; 16]);
        assert!(m.repaint);
        m.map_dirty = false;
        m.do_message(&mut w, &mut h, Message { kind: 0xc53d10aa, data1: 0, data2: TUNING_KEY[2], ..Message::default() }).unwrap();
        assert!(!m.map_dirty);
        m.do_message(&mut w, &mut h, Message { kind: 0xc53d10aa, data1: TUNING_KEY[1], data2: TUNING_KEY[2], ..Message::default() }).unwrap();
        assert!(m.map_dirty);
        assert!(!m.do_message(&mut w, &mut h, Message::default()).unwrap());
    }
}
