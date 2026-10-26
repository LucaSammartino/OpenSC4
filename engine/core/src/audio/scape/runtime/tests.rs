// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[derive(Default)]
struct Host {
    posts: Vec<(u32, [u32; 4])>,
    releases: Vec<u64>,
}
impl ScapeHost for Host {
    fn acquire(&mut self, _: u32, _: u32) -> Option<u64> {
        None
    }
    fn add_ref(&mut self, _: u64) {}
    fn release(&mut self, p: u64) {
        self.releases.push(p);
    }
    fn framework_available(&mut self) -> bool {
        false
    }
    fn register_service(&mut self, _: u32, _: bool) {}
    fn start_timer(&mut self, _: u32) {}
    fn sound_event(&mut self, _: u64, e: u32, a: [u32; 4]) {
        self.posts.push((e, a));
    }
}
fn setup() -> (ScapeManager, ScapeGlobals, Host) {
    (
        ScapeManager::default(),
        ScapeGlobals { sound_system: Some(1) },
        Host::default(),
    )
}
#[test]
fn au42_definition_bands_and_copied_input_worked_case() {
    let (mut m, g, mut h) = setup();
    let mut name = String::from("test");
    assert!(m.create_scape(&name, 7, 0, 10.0));
    name.clear();
    assert_eq!(m.groups[&7].name, "test");
    assert!(m.add_sound(7, 9, 255, 123, 2, 3));
    assert!(!m.add_sound(7, 9, 0, 0, 0, 0));
    for key in 1..=4 {
        assert!(m.add_emanation(7, 0, 0, key, [0.0; 3], 999.0, &g, &mut h));
        assert_eq!(
            m.groups[&7].clusters[0].playing.len(),
            usize::from(key == 2 || key == 3)
        );
    }
    assert_eq!(h.posts.iter().filter(|p| p.0 == 0xEA45_1903).count(), 1);
    assert_eq!(h.posts.iter().filter(|p| p.0 == 0x6A45_1908).count(), 1);
    assert!(!m.add_emanation(7, 0, 0, 1, [0.0; 3], 0.0, &g, &mut h));
    let d = &m.groups[&7].definitions[0];
    assert_eq!((d.flag, d.extra, d.f), (255, 123, 50.0));
}
#[test]
fn au42_wrap_channels_cache_and_pending_worked_cases() {
    let (mut m, g, mut h) = setup();
    m.create_scape("a", 7, 1, 10.0);
    m.add_emanation(7, 0, 0, 1, [0.0; 3], 0.0, &g, &mut h);
    h.posts.clear();
    let mut pending = Cluster::new(1.0);
    pending.countdown = 60;
    m.pending = vec![
        Some(pending.clone()),
        Some(Cluster {
            countdown: 20,
            ..pending
        }),
    ];
    m.elapsed = 0xFFFF_FFF0;
    assert!(m.on_tick(0x20, &g, &mut h));
    assert_eq!(m.pending.len(), 1);
    assert_eq!(m.pending[0].as_ref().unwrap().countdown, 12);
    let playing = &m.groups[&7].clusters[0].playing[0];
    let instance = (&*playing.definition as *const Definition) as usize as u32;
    assert_eq!(playing.instance, instance);
    assert_eq!(std::mem::size_of::<Definition>(), 24);
    assert_eq!(
        h.posts,
        [
            (0x4A45_1915, [instance, 146, 0, 0]),
            (0x4A45_1915, [instance, 146, 0, 0])
        ]
    );
    m.add_sound(7, 9, 0, 0, 1, 9);
    m.add_emanation(7, 0, 0, 2, [0.0; 3], 0.0, &g, &mut h);
    assert_eq!(m.groups[&7].clusters[0].cache, 20);
    for now in 0x21..0x45 {
        m.on_tick(now, &g, &mut h);
    }
    assert_eq!(m.groups[&7].clusters[0].channels, [1024; 2]);
    assert_eq!(m.groups[&7].clusters[0].cache, 20);
    m.groups.get_mut(&7).unwrap().clusters[0].channels[0] = 1030;
    m.on_tick(70, &g, &mut h);
    assert_eq!(m.groups[&7].clusters[0].channels[0], 1024);
    m.pending.push(None);
    h.posts.clear();
    assert!(!m.on_tick(71, &g, &mut h));
    assert!(h.posts.is_empty());
}
#[test]
fn au42_capacity_sparse_grid_edges_and_near_marks() {
    let (mut m, g, mut h) = setup();
    m.create_scape("grid", 7, 0, -1.0);
    for key in 0..10 {
        let p = if key == 9 { [25.0, 0.0, 25.0] } else { [5.0, 0.0, 5.0] };
        m.add_emanation(7, 0, 0, key, p, 0.0, &g, &mut h);
    }
    let mut out = [[99.0; 3]; 10];
    let mut n = 1;
    let mut total = 77;
    assert!(!m.positions(7, &mut out, &mut n, &mut total));
    assert_eq!((n, total, out), (1, 77, [[99.0; 3]; 10]));
    n = 9;
    assert_eq!(
        m.grid(7, [0.0, 0.0, 30.0, 30.0], &mut out, &mut n, &mut total, true),
        Ok(true)
    );
    assert_eq!((n, total), (2, 10));
    assert_eq!(out[0], [5.0, 0.0, 5.0]);
    assert_eq!(out[8], [25.0, 0.0, 25.0]);
    assert_eq!(out[1..8], [[99.0; 3]; 7]);
    n = 10;
    m.grid(7, [40.0, 40.0, 50.0, 50.0], &mut out, &mut n, &mut total, true)
        .unwrap();
    assert_eq!((n, total), (0, 10));
    total = 77;
    m.grid(99, [0.0; 4], &mut out, &mut n, &mut total, false).unwrap();
    assert_eq!((n, total), (0, 77));
    m.groups.get_mut(&7).unwrap().clusters[0].average = [30.0, 0.0, 5.0];
    n = 9;
    m.grid(7, [0.0, 0.0, 30.0, 30.0], &mut out, &mut n, &mut total, true)
        .unwrap();
    assert_eq!(out[3], [30.0, 0.0, 5.0]);
    m.create_scape("b", 8, 0, 40.0);
    m.add_emanation(8, 0, 0, 99, [0.0; 3], 0.0, &g, &mut h);
    n = 10;
    m.near(7, 8, [0.0; 4], &mut out, &mut n, &mut total, false).unwrap();
    assert_eq!((n, total), (10, 10));
    assert!(m.groups[&7].clusters.iter().all(|c| c.adds == 1));
}
#[test]
fn scape_average_move_clear_and_flag_quirks() {
    let (mut m, g, mut h) = setup();
    m.create_scape("a", 7, 1, 10.0);
    m.add_emanation(7, 0, 0, 1, [10.0, 0.0, 0.0], 0.0, &g, &mut h);
    m.add_emanation(7, 0, 0, 2, [20.0, 0.0, 0.0], 0.0, &g, &mut h);
    m.remove_emanation(7, 2, &g, &mut h);
    assert_eq!(m.groups[&7].clusters[0].average[0], 5.0);
    m.create_scape("b", 8, 0x10, 10.0);
    m.create_scape("c", 9, 4, 1.0);
    m.remove_group(8, &g, &mut h);
    assert_eq!((m.groups[&7].flags, m.groups[&9].flags), (0, 4));
    assert!(m.add_emanation(99, 123, 255, 5, [0.0; 3], 0.0, &g, &mut h));
    assert_eq!(m.groups[&99].name, "nameless");
    assert_eq!(m.groups[&99].clusters[0].playing[0].definition.sound, 99);
    m.move_emanation(0, 5, [100.0, 0.0, 0.0], 0.0, &g, &mut h);
    assert_eq!(m.groups[&99].clusters[0].average, [100.0, 0.0, 0.0]);
}

#[test]
fn au42_service_getters_and_destructor_do_not_shutdown_or_stop_copies() {
    let (mut m, g, mut h) = setup();
    m.create_scape("live", 7, 1, 10.0);
    m.add_emanation(7, 0, 0, 1, [0.0; 3], 0.0, &g, &mut h);
    m.initialized = true;
    m.sound_manager = Some(2);
    m.sound_system = Some(3);
    assert_eq!(m.priority(), 0x8000_0000);
    assert!(!m.on_idle());
    m.set_enabled(true);
    assert!(m.enabled);
    assert_eq!(m.add_ref(), 1);
    assert_eq!(m.add_ref(), 2);
    assert_eq!(m.release(&mut h), 1);
    h.posts.clear();
    assert_eq!(m.release(&mut h), 0);
    assert_eq!(h.releases, [2, 3]);
    assert!(h.posts.is_empty());
    assert!(m.initialized);
    assert_eq!(g.sound_system, Some(1));
}
