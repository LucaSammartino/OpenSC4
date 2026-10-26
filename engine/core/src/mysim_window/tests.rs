// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Fake {
    sims: Vec<Sim>,
    refs: BTreeMap<Sim, i32>,
    buttons: Vec<(Handle, SlotImage)>,
    subscriptions: Vec<(u32, bool)>,
    calls: Vec<String>,
    flags: BTreeMap<Advice, u32>,
    advice: Vec<Advice>,
}
impl WindowHost for Fake {}
impl Host for Fake {
    fn add_ref_sim(&mut self, sim: Sim) {
        *self.refs.entry(sim).or_default() += 1;
    }
    fn release_sim(&mut self, sim: Sim) {
        let count = self.refs.get_mut(&sim).unwrap();
        *count -= 1;
        assert!(*count >= 0);
    }
    fn subscribe(&mut self, message: u32, subscribe: bool) {
        self.subscriptions.push((message, subscribe));
    }
    fn unregister_service(&mut self, service: u32) {
        self.calls.push(format!("unregister:{service:x}"));
    }
    fn bind_button(&mut self, _: Handle, slot: usize) {
        self.calls.push(format!("bind:{slot}"));
    }
    fn current_sims(&mut self) -> Vec<Sim> {
        self.sims.clone()
    }
    fn sim_slot(&mut self, sim: Sim) -> usize {
        sim as usize - 1
    }
    fn portrait(&mut self, sim: Sim) -> Portrait {
        Portrait::Resource {
            group: 0x46a0_06b0,
            instance: sim as u32,
        }
    }
    fn name(&mut self, sim: Sim) -> String {
        format!("Sim {sim}")
    }
    fn button_image(&mut self, button: Handle, image: SlotImage) {
        self.buttons.push((button, image));
    }
    fn caption(&mut self, control: Handle, text: &str) {
        self.calls.push(format!("caption:{control}:{text}"));
    }
    fn image(&mut self, _: Handle, _: &Portrait) {
        self.calls.push("portrait".into());
    }
    fn create_pick_dialog(&mut self, slot: usize) -> Option<Handle> {
        self.calls.push(format!("create:{slot}"));
        Some(100)
    }
    fn run_pick_dialog(&mut self, _: Handle) {
        self.calls.push("run".into());
    }
    fn close_pick_dialog(&mut self, _: Handle) {
        self.calls.push("close".into());
    }
    fn release_pick_dialog(&mut self, _: Handle) {
        self.calls.push("release-dialog".into());
    }
    fn drop_sim(&mut self, sim: Sim) {
        self.sims.retain(|s| *s != sim);
    }
    fn advice_owner(&mut self, advice: Advice) -> Option<Sim> {
        (advice < 100).then_some(1)
    }
    fn advice_items(&mut self, _: Sim) -> Vec<Advice> {
        self.advice.clone()
    }
    fn advice_flags(&mut self, advice: Advice) -> u32 {
        self.flags[&advice]
    }
    fn set_advice_flags(&mut self, advice: Advice, flags: u32) {
        self.flags.insert(advice, flags);
    }
    fn invalidate_text(&mut self, control: Handle) {
        self.calls.push(format!("redraw:{control}"));
    }
}

fn setup() -> (MySimWindow, Windows, Fake, Bindings) {
    let mut host = Fake::default();
    let mut windows = Windows::default();
    let root = windows.create_default(&mut host);
    windows.add_ref(root); // The caller owns a reference independently of the procedure.
    for id in PANELS {
        let child = windows.create_default(&mut host);
        windows.get_mut(child).id = id;
        windows.child_add(&mut host, root, child);
    }
    let bindings = Bindings {
        root,
        buttons: [20, 21, 22, 23, 24],
        body: [30, 31],
        mode_name: 32,
        mode_portrait: 33,
    };
    let mut owner = MySimWindow::default();
    assert!(owner.init(bindings.clone(), &mut windows, &mut host));
    (owner, windows, host, bindings)
}

fn visible(windows: &Windows, root: Window) -> Vec<u32> {
    PANELS
        .into_iter()
        .filter(|id| {
            windows
                .get(windows.child_from_id_recursive(root, *id).unwrap())
                .flags
                & 1
                != 0
        })
        .collect()
}

#[test]
fn three_sims_main_add_buttons_and_balanced_lifecycle() {
    let (mut owner, mut windows, mut host, bindings) = setup();
    assert!(visible(&windows, bindings.root).is_empty());
    assert_eq!(host.subscriptions, MESSAGES.map(|id| (id, true)));
    host.sims = vec![3, 1, 2];
    owner.show(&mut windows, &mut host).unwrap();
    assert_eq!(owner.slots(), &[Some(1), Some(2), Some(3), None, None]);
    assert_eq!(
        host.buttons
            .iter()
            .filter(|(_, image)| matches!(image, SlotImage::Sim(_)))
            .count(),
        3
    );
    assert_eq!(host.buttons[3..], [(23, SlotImage::Add), (24, SlotImage::Add)]);
    assert_eq!(visible(&windows, bindings.root), [PANELS[0]]);
    assert!(owner.shown());
    owner.hide(&mut windows, &mut host);
    assert!(visible(&windows, bindings.root).is_empty());
    owner.shutdown(&mut windows, &mut host);
    assert!(host.refs.values().all(|count| *count == 0));
    assert_eq!(windows.get(bindings.root).references, 1);
    assert_eq!(&host.subscriptions[6..], MESSAGES.map(|id| (id, false)));
    assert!(host.calls.contains(&"unregister:ebe0e860".into()));
    owner.shutdown(&mut windows, &mut host);
}

#[test]
fn empty_slot_dialog_disables_panels_and_releases_created_reference() {
    let (mut owner, mut windows, mut host, bindings) = setup();
    owner.set_current_sim(3, &mut windows, &mut host).unwrap();
    for id in [0xea1f_1e4d, 0xaa1f_1ec5] {
        assert_eq!(
            windows
                .get(windows.child_from_id_recursive(bindings.root, id).unwrap())
                .flags
                & 2,
            0
        );
    }
    assert_eq!(&host.calls[5..], ["create:3", "run", "release-dialog"]);
    owner.shutdown(&mut windows, &mut host);
    assert!(!host.calls.contains(&"close".into()));
}

#[test]
fn occupied_slot_mode_boot_and_stat_boundary() {
    let (mut owner, mut windows, mut host, bindings) = setup();
    host.sims = vec![1];
    owner.show(&mut windows, &mut host).unwrap();
    assert_eq!(
        owner.set_current_sim(0, &mut windows, &mut host),
        Err(Unsupported::TransportButtonImage)
    );
    owner.set_mode_window(&mut windows, &mut host).unwrap();
    assert_eq!(visible(&windows, bindings.root), [0xea1f_1e4e]);
    assert!(host.calls.contains(&"caption:32:Sim 1".into()));
    assert_eq!(owner.set_stat_window(), Err(Unsupported::StatisticsLayout));
    owner.set_dialog_state(&mut windows, &mut host);
    assert_eq!(visible(&windows, bindings.root), [0xea1f_1e5e]);
    owner.boot_sim(&mut windows, &mut host).unwrap();
    assert_eq!(owner.slots(), &[None; 5]);
    assert_eq!(owner.selected_slot(), None);
    assert_eq!(host.refs[&1], 0);
    owner.shutdown(&mut windows, &mut host);
}

#[test]
fn text_redraw_keeps_list_and_raw_set_aside_flags_follow_spec() {
    let (mut owner, mut windows, mut host, _) = setup();
    host.sims = vec![1];
    host.advice = vec![10, 11, 12];
    host.flags = [(10, 0x20), (11, 0x22), (12, 0x60)].into();
    owner.show(&mut windows, &mut host).unwrap();
    let _ = owner.set_current_sim(0, &mut windows, &mut host);
    assert_eq!(owner.set_small_window(&mut windows, &mut host), Err(Unsupported::MessageHtmlLayout));
    for id in [10, 11] {
        assert_eq!(owner.do_message(0x8a09_fa64, Some(id), &mut windows, &mut host), Err(Unsupported::MessageHtmlLayout));
    }
    assert_eq!(owner.set_aside_items(), &[10, 11].into());
    assert_eq!(host.flags[&10], 0);
    assert_eq!(host.flags[&11], 2);
    host.flags.insert(10, 0x60);
    host.flags.insert(11, 0x22);
    owner.get_message_list(&mut host).unwrap();
    assert_eq!(host.flags[&10], 0x40);
    assert_eq!(host.flags[&11], 0x22);
    assert!(owner.set_aside_items().is_empty());
    let before = owner.advice_items().to_vec();
    let redraws = host.calls.len();
    assert_eq!(owner.do_message(0x8a09_fa65, None, &mut windows, &mut host), Err(Unsupported::MessageHtmlLayout));
    assert_eq!(owner.advice_items(), before);
    assert_eq!(&host.calls[redraws..], ["redraw:30"]);
    let _ = owner.do_message(0x8a09_fa64, Some(12), &mut windows, &mut host);
    let _ = owner.do_message(0xca09_fa6c, Some(12), &mut windows, &mut host);
    assert!(owner.set_aside_items().is_empty());
    assert_eq!(owner.visible_advice(), Err(Unsupported::MessageVisibility));
    owner.shutdown(&mut windows, &mut host);
}

#[test]
fn changed_sims_retains_occupied_selection_or_returns_main_only_while_shown() {
    let (mut owner, mut windows, mut host, bindings) = setup();
    host.sims = vec![1, 2];
    owner.show(&mut windows, &mut host).unwrap();
    let _ = owner.set_current_sim(1, &mut windows, &mut host);
    owner.set_mode_window(&mut windows, &mut host).unwrap();
    host.sims = vec![2];
    assert_eq!(owner.do_message(0xaa25_827c, None, &mut windows, &mut host), Ok(true));
    assert_eq!(owner.selected_slot(), Some(1));
    assert_eq!(visible(&windows, bindings.root), [0xea1f_1e4e]);
    host.sims.clear();
    owner.do_message(0xaa25_827c, None, &mut windows, &mut host).unwrap();
    assert_eq!(visible(&windows, bindings.root), [PANELS[0]]);
    assert_eq!(owner.selected_slot(), None);
    owner.hide(&mut windows, &mut host);
    host.sims = vec![3];
    owner.do_message(0xaa25_827c, None, &mut windows, &mut host).unwrap();
    assert_eq!(owner.slots(), &[None; 5]);
    assert_eq!(owner.do_message(0xac08_746e, None, &mut windows, &mut host), Err(Unsupported::TransportChoice));
    owner.shutdown(&mut windows, &mut host);
}
