// SPDX-License-Identifier: GPL-3.0-or-later
//! Sound-manager foreign stages from audio §§8.2–8.3/10.6.
//! Legacy fixture stages. Settled AU46/AU49 execution is in `execution`.
use super::AudioSoundManager;

pub trait SoundManagerHost {
    /// cIGZSndMgr slot +48(sound, 0, 0, IID, out); acquired output.
    fn get_sound(&mut self, manager: u64, sound: u32, zero1: u32, zero2: u32, iid: u32) -> Option<u64>;
    fn query(&mut self, sound: u64, iid: u32) -> Option<u64>;
    fn property_38(&mut self, sound: u64) -> f32;
    /// Slot +14 on the resolved holder, after tSound construction.
    fn play(&mut self, holder: u64, instance: u32, word_14: u32, flags: u32, zero: u32);
    fn release(&mut self, reference: u64);
}

/// Owned references resolved by the two known foreign calls. No native failure
/// result or tSound-constructor value is fabricated when QI fails (AU49).
#[derive(Debug)]
pub struct ResolvedSample {
    pub holder: u64,
    pub properties: Option<u64>,
    pub pause_radius: Option<f32>,
    pub resume_radius: Option<f32>,
}

impl ResolvedSample {
    pub fn release(self, host: &mut impl SoundManagerHost) {
        if let Some(properties) = self.properties {
            host.release(properties);
        }
        host.release(self.holder);
    }

    /// The caller must supply the constructed tSound+14 value: its source is
    /// unsettled (AU49). This is only the settled playback continuation.
    pub fn play_constructed(&self, instance: u32, word_14: u32, host: &mut impl SoundManagerHost) {
        host.play(self.holder, instance, word_14, 0x200, 0);
    }
}

pub fn resolve_sample(manager: u64, sound: u32, host: &mut impl SoundManagerHost) -> Option<ResolvedSample> {
    let holder = host.get_sound(manager, sound, 0, 0, 0x2060_77D2)?;
    let properties = host.query(holder, 0xFD58_4467);
    let pause_radius = properties.map(|properties| host.property_38(properties));
    let resume_radius = pause_radius.map(|radius| radius * 4.0);
    Some(ResolvedSample {
        holder,
        properties,
        pause_radius,
        resume_radius,
    })
}

/// Acquiring QI, unlike the legacy fixture's membership predicate (§8.2).
pub fn query_interface(manager: &mut AudioSoundManager, iid: u32) -> bool {
    if !manager.query_interface(iid) {
        return false;
    }
    manager.refcount = manager.refcount.wrapping_add(1);
    true
}

/// All handled message types except tuning change are subscribed (§8.3).
pub fn subscriptions() -> [u32; 7] {
    let messages = super::handled_message_types();
    std::array::from_fn(|i| messages[i])
}

/// Preserve the event domain separately from message dispatch. A route is not
/// a successful event result; use `execution` for owner side effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRoute {
    Start,
    Stop,
    Position,
    UpdateAll,
    Reset,
    Enumerate,
    Volume,
    Focus,
    StopAll,
    Ignore,
}
pub fn event_route(event: u32) -> EventRoute {
    match event {
        0xEA45_1903 | 1 => EventRoute::Start,
        0x6A45_1908 | 2 => EventRoute::Stop,
        0xCA45_190F => EventRoute::Position,
        0x4A8C_52C8 => EventRoute::UpdateAll,
        0x4A4D_35E0 => EventRoute::Reset,
        0x4A4F_E6F6 => EventRoute::Enumerate,
        0x4A45_1915 => EventRoute::Volume,
        0x6A95_CBA2 => EventRoute::Focus,
        0xCAA4_4CA7 => EventRoute::StopAll,
        _ => EventRoute::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        calls: Vec<(u32, u64)>,
        qi: bool,
    }
    impl SoundManagerHost for Host {
        fn get_sound(&mut self, manager: u64, sound: u32, zero1: u32, zero2: u32, iid: u32) -> Option<u64> {
            assert_eq!((manager, sound, zero1, zero2, iid), (9, 77, 0, 0, 0x2060_77D2));
            self.calls.push((0, manager));
            Some(1)
        }
        fn query(&mut self, holder: u64, iid: u32) -> Option<u64> {
            assert_eq!((holder, iid), (1, 0xFD58_4467));
            self.qi.then_some(2)
        }
        fn property_38(&mut self, sound: u64) -> f32 {
            assert_eq!(sound, 2);
            12.5
        }
        fn play(&mut self, holder: u64, instance: u32, word_14: u32, flags: u32, zero: u32) {
            assert_eq!((holder, instance, word_14, flags, zero), (1, 11, 33, 0x200, 0));
            self.calls.push((1, holder));
        }
        fn release(&mut self, reference: u64) {
            self.calls.push((2, reference));
        }
    }
    #[test]
    fn sample_resolution_playback_and_owned_reference_cleanup() {
        let mut host = Host {
            qi: true,
            ..Host::default()
        };
        let sample = resolve_sample(9, 77, &mut host).unwrap();
        assert_eq!((sample.pause_radius, sample.resume_radius), (Some(12.5), Some(50.0)));
        sample.play_constructed(11, 33, &mut host);
        sample.release(&mut host);
        assert_eq!(host.calls, [(0, 9), (1, 1), (2, 2), (2, 1)]);
        host.qi = false;
        let sample = resolve_sample(9, 77, &mut host).unwrap();
        assert_eq!(sample.pause_radius, None);
        sample.release(&mut host);
        assert_eq!(host.calls.last(), Some(&(2, 1)));
    }
    #[test]
    fn acquiring_qi_subscription_exception_and_event_aliases() {
        let mut manager = AudioSoundManager::new();
        let initial = manager.refcount;
        assert!(query_interface(&mut manager, 0x4A47_DC0D));
        assert!(!query_interface(&mut manager, 0xFD58_4467));
        assert_eq!(manager.refcount, initial + 1);
        assert!(!subscriptions().contains(&0xC53D_10AA));
        assert_eq!(event_route(1), EventRoute::Start);
        assert_eq!(event_route(2), EventRoute::Stop);
        assert_eq!(event_route(0x8A38_5DD8), EventRoute::Ignore);
        assert_eq!(event_route(0xAA39_6960), EventRoute::Ignore);
    }
}
