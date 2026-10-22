// SPDX-License-Identifier: GPL-3.0-or-later
// Expected sequences transcribed from fixtures.md, committed before the owner.
use super::*;
use std::collections::BTreeMap;

const A: Automaton = Automaton(11);
const B: Automaton = Automaton(22);
const W: DashboardWindow = DashboardWindow(88);
const P: Pixel = Pixel { x: 12, y: -9 };

#[derive(Clone)]
struct Auto {
    flags: u32,
    group: u32,
    state: u32,
    slot18: bool,
    form: Option<Automaton>,
    kind: u32,
    dashboard: Option<u32>,
    sound: Option<u32>,
    present: bool,
    byte: u8,
}
impl Auto {
    fn new(a: Automaton) -> Self {
        Self {
            flags: 0,
            group: 0x4006,
            state: 0,
            slot18: false,
            form: Some(a),
            kind: 0,
            dashboard: None,
            sound: Some(0x1234),
            present: true,
            byte: 0,
        }
    }
}

struct Foreign {
    objects: BTreeMap<u64, Auto>,
    operations: Vec<Operation>,
    queries: Vec<&'static str>,
    paused: bool,
    mission: bool,
    network: Option<NetworkOccupant>,
    accepted: bool,
    picked: Option<Automaton>,
    terrain: Option<World>,
    command: u32,
    fail: Option<&'static str>,
}
impl Default for Foreign {
    fn default() -> Self {
        Self {
            objects: [(11, Auto::new(A)), (22, Auto::new(B))].into(),
            operations: vec![],
            queries: vec![],
            paused: false,
            mission: false,
            network: None,
            accepted: false,
            picked: None,
            terrain: None,
            command: 0,
            fail: None,
        }
    }
}
impl Foreign {
    fn get(&mut self, a: Automaton, name: &'static str) -> Raw<Auto> {
        self.queries.push(name);
        if self.fail == Some(name) {
            return Err(Unsupported::ForeignPrimitive);
        }
        self.objects.get(&a.0).cloned().ok_or(Unsupported::ForeignPrimitive)
    }
    fn clear(&mut self) {
        self.operations.clear();
        self.queries.clear();
    }
}
impl Primitives for Foreign {
    fn emit(&mut self, op: Operation) {
        // Only individual raw setters are applied here, never owner outcomes.
        match op {
            Operation::SimulatorPause => self.paused = true,
            Operation::SimulatorResume => self.paused = false,
            Operation::State52 { automaton, words } => self.objects.get_mut(&automaton.0).unwrap().state = words[0],
            Operation::State13 { automaton, value } => self.objects.get_mut(&automaton.0).unwrap().state = value,
            Operation::DriveByte229 { automaton, value } => self.objects.get_mut(&automaton.0).unwrap().byte = value,
            _ => (),
        }
        self.operations.push(op);
    }
    fn simulator_paused(&mut self) -> Raw<bool> {
        self.queries.push("paused");
        Ok(self.paused)
    }
    fn situation_slot32(&mut self) -> Raw<bool> {
        self.queries.push("mission32");
        Ok(self.mission)
    }
    fn flags31(&mut self, a: Automaton) -> Raw<u32> {
        Ok(self.get(a, "flags31")?.flags)
    }
    fn group(&mut self, a: Automaton) -> Raw<u32> {
        Ok(self.get(a, "group")?.group)
    }
    fn state(&mut self, a: Automaton) -> Raw<u32> {
        Ok(self.get(a, "state")?.state)
    }
    fn slot18(&mut self, a: Automaton) -> Raw<bool> {
        Ok(self.get(a, "slot18")?.slot18)
    }
    fn drivable_form50(&mut self, a: Automaton) -> Raw<Option<Automaton>> {
        Ok(self.get(a, "form50")?.form)
    }
    fn occupant_type(&mut self, a: Automaton) -> Raw<u32> {
        Ok(self.get(a, "type")?.kind)
    }
    fn property(&mut self, a: Automaton, id: u32) -> Raw<Option<u32>> {
        let data = self.get(a, "property")?;
        match id {
            0x2BE8_FBF3 => Ok(data.dashboard),
            0xCBBB_A52F => Ok(data.sound),
            _ => panic!("unexpected property {id:X}"),
        }
    }
    fn drive_state_present(&mut self, a: Automaton) -> Raw<bool> {
        Ok(self.get(a, "present")?.present)
    }
    fn drive_byte229(&mut self, a: Automaton) -> Raw<u8> {
        Ok(self.get(a, "byte229")?.byte)
    }
    fn renderer_pick_automaton(&mut self, at: Pixel, iid: u32) -> Raw<Option<Automaton>> {
        assert_eq!((at, iid), (P, 0xA9B4_0F05));
        self.queries.push("pick_auto");
        Ok(self.picked)
    }
    fn renderer_pick_network(&mut self, at: Pixel, iid: u32) -> Raw<Option<NetworkOccupant>> {
        assert_eq!((at, iid), (P, 0xA821_EF94));
        self.queries.push("pick_network");
        Ok(self.network)
    }
    fn network_slot42(&mut self, _: NetworkOccupant) -> Raw<bool> {
        self.queries.push("accept42");
        Ok(self.accepted)
    }
    fn network_position(&mut self, _: NetworkOccupant) -> Raw<World> {
        self.queries.push("network_position");
        Ok(World([1, 2, 3]))
    }
    fn terrain_pick(&mut self, at: Pixel) -> Raw<Option<World>> {
        assert_eq!(at, P);
        self.queries.push("terrain");
        Ok(self.terrain)
    }
    fn position_to_cell(&mut self, _: World) -> Raw<Cell> {
        self.queries.push("cell");
        Ok(Cell(3, 4))
    }
    fn script_keymap14(&mut self, _: u32) -> Raw<u32> {
        self.queries.push("keymap14");
        Ok(self.command)
    }
    fn can_zoom_in(&mut self) -> Raw<bool> {
        self.queries.push("can_in");
        Ok(true)
    }
    fn can_zoom_out(&mut self) -> Raw<bool> {
        self.queries.push("can_out");
        Ok(false)
    }
}

fn driving() -> (PlayerDrive, Foreign) {
    let mut owner = PlayerDrive::new(99);
    let mut foreign = Foreign::default();
    owner.attach_dashboard_window(Some(W));
    assert!(matches!(
        owner.enter_driving_mode(&mut foreign, A, CameraFields::default()),
        Ok(Progress::Partial(_))
    ));
    foreign.clear();
    (owner, foreign)
}

#[test]
fn construction_has_literal_bits_and_empty_state() {
    let owner = PlayerDrive::new(99);
    assert_eq!((ID, CURSOR), (0x2B70_9676, 0xC16C_4E1B));
    assert_eq!(owner.camera_speed_bits(), 0x3D2C_0831);
    assert_eq!(owner.cell_scales(), (0x3F80_0000, 0x3F80_0000));
    assert_eq!(owner.toggles(), [false, false]);
    assert_eq!((owner.current(), owner.hover(), owner.selected()), (None, None, None));
    assert_eq!(owner.mode(), Mode::Choosing);
    assert_eq!(owner.timer(), TimerState::Constructed);
}

#[test]
fn choosing_pauses_all_four_but_simulator_only_if_needed() {
    for paused in [false, true] {
        let mut owner = PlayerDrive::new(99);
        let mut f = Foreign {
            paused,
            ..Foreign::default()
        };
        assert_eq!(
            owner.enter_choosing_mode(&mut f),
            Ok(Progress::Partial(vec![Unsupported::HoverOwnershipAndText]))
        );
        let mut expected = vec![Operation::ClearCursorText(0xAB73_510C)];
        if !paused {
            expected.push(Operation::SimulatorPause);
        }
        expected.extend([
            Operation::ManagerPause(43),
            Operation::ManagerPause(44),
            Operation::ManagerPause(45),
            Operation::ManagerPause(46),
        ]);
        assert_eq!(f.operations, expected);
    }
}

#[test]
fn driving_projection_orders_raw_operations_and_resumes_conditionally() {
    for paused in [false, true] {
        let mut owner = PlayerDrive::new(99);
        let mut f = Foreign {
            paused,
            ..Foreign::default()
        };
        f.objects.get_mut(&11).unwrap().form = Some(B);
        f.objects.get_mut(&22).unwrap().kind = 0x480E_6151;
        owner.attach_dashboard_window(Some(W));
        assert_eq!(
            owner.enter_driving_mode(&mut f, A, CameraFields::default()),
            Ok(Progress::Partial(vec![
                Unsupported::DashboardShowUi,
                Unsupported::AudioTable,
                Unsupported::SettingsSelection,
                Unsupported::HoverOwnershipAndText
            ]))
        );
        let mut expected = vec![
            Operation::DisasterLayer(false),
            Operation::ClearCursorText(0xAB73_510C),
            Operation::MinimizeCityUi,
            Operation::ShowCityWindow(0x43),
            Operation::DashboardIdSelected(0x6BE8_846C),
            Operation::Gap(Unsupported::DashboardShowUi),
            Operation::OccupantSound {
                id: 0x1234,
                start: true,
            },
            Operation::EngineEvent { event: 0, start: true },
            Operation::Gap(Unsupported::AudioTable),
            Operation::AcquireDriven(B),
            Operation::State52 {
                automaton: B,
                words: [0x10, 0, 0xBF80_0000, 0, 0, 0, 0],
            },
            Operation::VehicleManagerTrack(Some(B)),
            Operation::Gap(Unsupported::SettingsSelection),
            Operation::ZoomLimits(0, 4),
            Operation::Zoom(4),
            Operation::WindowEnabled {
                window: W,
                id: 0x4992_046A,
                enabled: true,
            },
            Operation::WindowEnabled {
                window: W,
                id: 0xE992_0494,
                enabled: false,
            },
            Operation::DispatchTrack(Some(B)),
            Operation::Callback39(B),
            Operation::TimerReset,
        ];
        if paused {
            expected.push(Operation::SimulatorResume);
        }
        expected.extend([
            Operation::ManagerResume(43),
            Operation::ManagerResume(44),
            Operation::ManagerResume(45),
            Operation::ManagerResume(46),
        ]);
        assert_eq!(f.operations, expected);
        assert_eq!(owner.current(), Some(B));
        assert_eq!(owner.mode(), Mode::Driving);
        assert_eq!(owner.toggles(), [true, false]);
        assert_eq!(owner.timer(), TimerState::Reset);
        assert_eq!(owner.dashboard_id(), Some(0x6BE8_846C));
        assert_eq!(owner.camera_speed_bits(), 0x3D2C_0831);
    }
}

#[test]
fn drivable_form_gate_has_only_disaster_prefix_and_keeps_existing_current() {
    let (mut owner, mut f) = driving();
    f.objects.get_mut(&22).unwrap().form = None;
    assert_eq!(
        owner.enter_driving_mode(&mut f, B, CameraFields::default()),
        Ok(Progress::NoDrivableForm)
    );
    assert_eq!(f.operations, [Operation::DisasterLayer(false)]);
    assert_eq!(f.queries, ["form50"]);
    assert_eq!(owner.current(), Some(A));
}

#[test]
fn dashboard_selection_is_id_selection_only_and_preserves_zero_override() {
    assert_eq!(PlayerDrive::dashboard_selection(0x480E_6151, None), 0x6BE8_846C);
    assert_eq!(PlayerDrive::dashboard_selection(7, None), 0x0BE8_824F);
    assert_eq!(PlayerDrive::dashboard_selection(0x480E_6151, Some(5)), 5);
    assert_eq!(PlayerDrive::dashboard_selection(7, Some(0)), 0);
    let mut owner = PlayerDrive::new(99);
    let mut f = Foreign::default();
    let result = owner.enter_driving_mode(&mut f, A, CameraFields::default()).unwrap();
    assert!(matches!(result, Progress::Partial(gaps) if gaps.contains(&Unsupported::DashboardWindowMissing)));
    assert!(!f
        .operations
        .iter()
        .any(|op| matches!(op, Operation::WindowEnabled { .. })));
    assert_eq!(owner.show_ui(5), Err(Unsupported::DashboardShowUi));
}

#[test]
fn accepted_network_pick_wins_in_choosing_and_driving() {
    for already_driving in [false, true] {
        let (mut owner, mut f) = if already_driving {
            driving()
        } else {
            (PlayerDrive::new(99), Foreign::default())
        };
        f.network = Some(NetworkOccupant(55));
        f.accepted = true;
        f.picked = Some(B);
        assert_eq!(
            owner.on_mouse_down_l(&mut f, P, CameraFields::default()),
            Ok(Progress::Applied)
        );
        assert_eq!(
            f.operations,
            [
                Operation::PositionedSound {
                    id: 0x8BE8_D9AC,
                    position: World([1, 2, 3])
                },
                Operation::JunctionSelected(Some(Cell(3, 4)))
            ]
        );
        assert_eq!(f.queries, ["pick_network", "accept42", "network_position", "cell"]);
        assert_eq!(owner.current(), if already_driving { Some(A) } else { None });
        assert_eq!(owner.selected(), Some(Cell(3, 4)));
    }
}

#[test]
fn rejected_network_pick_falls_back_to_automaton_only_when_not_driving() {
    let mut owner = PlayerDrive::new(99);
    let mut f = Foreign {
        network: Some(NetworkOccupant(55)),
        picked: Some(B),
        ..Foreign::default()
    };
    assert!(matches!(
        owner.on_mouse_down_l(&mut f, P, CameraFields::default()),
        Ok(Progress::Partial(_))
    ));
    assert_eq!(&f.queries[..4], ["pick_network", "accept42", "pick_auto", "form50"]);
    assert_eq!(owner.current(), Some(B));
    f.clear();
    owner.on_mouse_down_l(&mut f, P, CameraFields::default()).unwrap();
    assert!(!f.queries.contains(&"pick_auto"));
    assert_eq!(
        f.operations,
        [
            Operation::DriveCommand {
                automaton: B,
                command: 7,
                value: 1
            },
            Operation::EngineEvent { event: 4, start: true }
        ]
    );
}

#[test]
fn flags_gate_independently_for_commands_and_mouse_drive_fields() {
    for (flags, valid) in [
        (0, true),
        (2, true),
        (1, false),
        (0x0800_0000, false),
        (0x0800_0001, false),
    ] {
        let (mut owner, mut f) = driving();
        f.objects.get_mut(&11).unwrap().flags = flags;
        assert_eq!(owner.is_drive_auto_valid(&mut f), Ok(valid));
        owner.apply_numeric_command(&mut f, 0, true).unwrap();
        assert_eq!(!f.operations.is_empty(), valid);
        f.clear();
        owner.on_mouse_move(&mut f, P).unwrap();
        assert_eq!(
            f.operations.iter().any(|op| matches!(op, Operation::DriveMouse { .. })),
            valid
        );
        assert_eq!(f.queries.contains(&"pick_auto"), !valid);
    }
}

#[test]
fn numeric_non_toggle_table_down_and_up_has_literal_sound_and_light_order() {
    // These values come from the published table, not from owner output.
    for (command, sound, light) in [
        (0, Some(1), None),
        (1, Some(2), None),
        (2, Some(3), Some(0x8BCB_93FC)),
        (3, Some(3), Some(0x6BCB_93F5)),
        (5, Some(6), None),
        (6, None, None),
        (7, Some(4), None),
        (11, None, None),
        (12, None, None),
    ] {
        for down in [true, false] {
            let (mut owner, mut f) = driving();
            assert_eq!(
                owner.apply_numeric_command(&mut f, command, down),
                Ok(Progress::Applied)
            );
            let mut expected = vec![];
            if let Some(id) = light {
                expected.push(Operation::WindowLight {
                    window: W,
                    id,
                    on: down,
                });
            }
            expected.push(Operation::DriveCommand {
                automaton: A,
                command,
                value: u32::from(down),
            });
            if let Some(event) = sound {
                expected.push(Operation::EngineEvent { event, start: down });
            }
            assert_eq!(f.operations, expected, "command {command}, down={down}");
        }
    }
}

#[test]
fn two_toggles_keep_independent_state_and_release_has_no_effect() {
    let (mut owner, mut f) = driving();
    for (command, expected) in [(4, 0), (4, 1), (10, 1), (10, 0)] {
        f.clear();
        owner.apply_numeric_command(&mut f, command, true).unwrap();
        assert_eq!(
            f.operations,
            [Operation::DriveCommand {
                automaton: A,
                command,
                value: expected
            }]
        );
        f.clear();
        assert_eq!(
            owner.apply_numeric_command(&mut f, command, false),
            Ok(Progress::Ignored)
        );
        assert!(f.operations.is_empty());
    }
    assert_eq!(owner.toggles(), [true, false]);
}

#[test]
fn command9_toggles_only_boolean_raw_byte_after_command_setter() {
    let (mut owner, mut f) = driving();
    for expected in [1, 0] {
        f.clear();
        owner.apply_numeric_command(&mut f, 9, true).unwrap();
        assert_eq!(
            f.operations,
            [
                Operation::DriveCommand {
                    automaton: A,
                    command: 9,
                    value: 1
                },
                Operation::DriveByte229 {
                    automaton: A,
                    value: expected
                }
            ]
        );
    }
    f.clear();
    owner.apply_numeric_command(&mut f, 9, false).unwrap();
    assert_eq!(
        f.operations,
        [Operation::DriveCommand {
            automaton: A,
            command: 9,
            value: 0
        }]
    );
    assert!(!f.queries.contains(&"byte229"));
    f.clear();
    f.objects.get_mut(&11).unwrap().byte = 2;
    assert_eq!(
        owner.apply_numeric_command(&mut f, 9, true),
        Err(Unsupported::NonBooleanDriveByte)
    );
    assert!(f.operations.is_empty());
    f.objects.get_mut(&11).unwrap().present = false;
    assert_eq!(
        owner.apply_numeric_command(&mut f, 9, true),
        Err(Unsupported::MissingDriveState)
    );
}

#[test]
fn autorepeat_suppresses_all_queries_and_escape_effects() {
    let (mut owner, mut f) = driving();
    for key in [0x41, 0x1B] {
        assert_eq!(owner.on_key_down(&mut f, key, 0x4000_0000), Ok(Progress::Ignored));
        assert!(f.operations.is_empty());
        assert!(f.queries.is_empty());
        assert_eq!(owner.current(), Some(A));
    }
}

#[test]
fn mission_predicate_blocks_lookup_mouse_and_direct_commands_but_not_selection_clear() {
    let (mut owner, mut f) = driving();
    f.mission = true;
    owner.on_key_down(&mut f, 0x41, 0).unwrap();
    owner.on_key_up(&mut f, 0x41, 0).unwrap();
    owner.apply_numeric_command(&mut f, 4, true).unwrap();
    owner.on_mouse_down_r(&mut f).unwrap();
    owner.on_mouse_up_r(&mut f).unwrap();
    owner.on_mouse_down_l(&mut f, P, CameraFields::default()).unwrap();
    assert!(!f.queries.contains(&"keymap14"));
    assert!(f.operations.is_empty());
    assert_eq!(owner.toggles(), [true, false]);
    owner.on_mouse_up_l(&mut f).unwrap();
    assert_eq!(f.operations, [Operation::JunctionSelected(None)]);
}

#[test]
fn ordinary_keys_never_falsely_bypass_unpublished_key_effects() {
    let (mut owner, mut f) = driving();
    for command in [0, 4, 9, 13] {
        f.command = command;
        assert_eq!(owner.on_key_down(&mut f, 0x41, 0), Err(Unsupported::HandleKeyEffects));
        assert_eq!(owner.on_key_up(&mut f, 0x41, 0), Err(Unsupported::HandleKeyEffects));
    }
    assert!(f.queries.contains(&"keymap14"));
    assert!(f.operations.is_empty());
    f.command = 8;
    assert_eq!(owner.on_key_down(&mut f, 0x41, 0), Err(Unsupported::KeyboardCommand8));
    assert_eq!(
        owner.on_key_up(&mut f, 0x41, 0x4000_0000),
        Err(Unsupported::ReleaseRepeatPolicy)
    );
}

#[test]
fn mouse_release_and_right_button_keep_numeric_effects() {
    let (mut owner, mut f) = driving();
    f.network = Some(NetworkOccupant(55));
    f.accepted = true;
    owner.on_mouse_down_l(&mut f, P, CameraFields::default()).unwrap();
    f.clear();
    owner.on_mouse_up_l(&mut f).unwrap();
    assert_eq!(owner.selected(), None);
    assert_eq!(
        f.operations,
        [
            Operation::JunctionSelected(None),
            Operation::DriveCommand {
                automaton: A,
                command: 7,
                value: 0
            },
            Operation::EngineEvent { event: 4, start: false }
        ]
    );
    f.clear();
    owner.on_mouse_down_r(&mut f).unwrap();
    owner.on_mouse_up_r(&mut f).unwrap();
    assert_eq!(
        f.operations,
        [
            Operation::DriveCommand {
                automaton: A,
                command: 8,
                value: 1
            },
            Operation::DriveCommand {
                automaton: A,
                command: 8,
                value: 0
            }
        ]
    );
}

#[test]
fn stopping_state10_selects_slot13_or_slot53_using_only_published_bits() {
    for (flags, group, special) in [
        (0, 0x4006, false),
        (1, 0x4006, false),
        (0x0800_0000, 0x4006, true),
        (0, 0x4003, true),
        (0x0800_0001, 0x4003, true),
    ] {
        let (mut owner, mut f) = driving();
        let data = f.objects.get_mut(&11).unwrap();
        data.flags = flags;
        data.group = group;
        owner.stop_tracking(&mut f).unwrap();
        assert_eq!(
            f.operations,
            [
                Operation::Callback40(A),
                Operation::DispatchTrack(None),
                Operation::VehicleManagerTrack(None),
                Operation::ZoomLimits(0, 5),
                Operation::EngineEvent { event: 0, start: false },
                Operation::OccupantSound {
                    id: 0x1234,
                    start: false
                },
                if special {
                    Operation::Slot53(A)
                } else {
                    Operation::State13 { automaton: A, value: 0 }
                },
                Operation::ReleaseDriven(A)
            ]
        );
        assert_eq!(owner.current(), None);
        f.clear();
        assert_eq!(owner.stop_tracking(&mut f), Ok(Progress::Ignored));
        assert!(f.operations.is_empty());
        assert!(f.queries.is_empty());
    }
}

#[test]
fn end_input_and_escape_order_callbacks_slot18_cleanup_and_base_stack_removal() {
    let (mut owner, mut f) = driving();
    owner.add_callback_target(CallbackTarget(66)).unwrap();
    f.objects.get_mut(&11).unwrap().slot18 = true;
    assert!(matches!(owner.on_key_down(&mut f, 0x1B, 0), Ok(Progress::Partial(_))));
    assert_eq!(
        f.operations,
        [
            Operation::CallbackPlan {
                target: CallbackTarget(66),
                message: 0x0BB3_24BD,
                tool: 99,
                values: [RawValue::Known(1), RawValue::Unpublished]
            },
            Operation::State13 { automaton: A, value: 0 },
            Operation::Callback40(A),
            Operation::DispatchTrack(None),
            Operation::VehicleManagerTrack(None),
            Operation::ZoomLimits(0, 5),
            Operation::EngineEvent { event: 0, start: false },
            Operation::OccupantSound {
                id: 0x1234,
                start: false
            },
            Operation::ReleaseDriven(A),
            Operation::ClearCursorText(0xAB73_510C),
            Operation::DisasterLayer(true),
            Operation::RemoveTopControl(false)
        ]
    );
    f.clear();
    owner.end_input(&mut f).unwrap();
    assert_eq!(f.operations.len(), 4); // Still callbacks, cursor, disaster, stack removal.
    assert!(matches!(f.operations[0], Operation::CallbackPlan { .. }));
    assert_eq!(f.operations[3], Operation::RemoveTopControl(false));
}

#[test]
fn removed_and_notification_identity_matching_is_mandatory() {
    for via_message in [false, true] {
        let (mut owner, mut f) = driving();
        assert_eq!(owner.automaton_removed(&mut f, B), Ok(Progress::Ignored));
        for code in [3, 0xB, 0xD] {
            assert_eq!(
                owner.do_message(&mut f, 0x09D9_2FC3, Some(B), code),
                Ok(Progress::Ignored)
            );
        }
        assert!(f.operations.is_empty());
        assert!(f.queries.is_empty());
        if via_message {
            owner.do_message(&mut f, 0x09D9_2FC3, Some(A), 3).unwrap();
        } else {
            owner.automaton_removed(&mut f, A).unwrap();
        }
        assert_eq!(f.operations[0], Operation::Callback40(A));
        assert_eq!(f.operations[7], Operation::ReleaseDriven(A));
        assert_eq!(
            &f.operations[8..],
            [
                Operation::ClearCursorText(0xAB73_510C),
                Operation::DisasterLayer(true),
                Operation::RemoveTopControl(false)
            ]
        );
        assert!(!f.queries.contains(&"slot18")); // StopTracking precedes EndInput here.
    }
}

#[test]
fn callback_plans_preserve_values_and_insertion_order_without_invented_refs() {
    let mut owner = PlayerDrive::new(0x1234);
    let mut f = Foreign::default();
    owner.add_callback_target(CallbackTarget(6)).unwrap();
    owner.add_callback_target(CallbackTarget(5)).unwrap();
    assert_eq!(
        owner.add_callback_target(CallbackTarget(6)),
        Err(Unsupported::CallbackListPolicy)
    );
    assert_eq!(
        owner.send_callback_message(&mut f, 0xFFFF_FFFF, 9),
        Ok(Progress::Partial(vec![Unsupported::CallbackDelivery]))
    );
    assert_eq!(
        f.operations,
        [
            Operation::CallbackPlan {
                target: CallbackTarget(6),
                message: 0x0BB3_24BD,
                tool: 0x1234,
                values: [RawValue::Known(0xFFFF_FFFF), RawValue::Known(9)]
            },
            Operation::CallbackPlan {
                target: CallbackTarget(5),
                message: 0x0BB3_24BD,
                tool: 0x1234,
                values: [RawValue::Known(0xFFFF_FFFF), RawValue::Known(9)]
            },
        ]
    );
}

#[test]
fn hover_terrain_and_valid_drive_mouse_paths_use_individual_foreign_queries() {
    let mut owner = PlayerDrive::new(99);
    let mut f = Foreign {
        picked: Some(B),
        terrain: Some(World([8, 9, 10])),
        ..Foreign::default()
    };
    assert_eq!(
        owner.on_mouse_move(&mut f, P),
        Ok(Progress::Partial(vec![Unsupported::HoverOwnershipAndText]))
    );
    assert_eq!(owner.hover(), Some(B));
    assert_eq!(owner.hover_cell(), Some(Cell(3, 4)));
    assert_eq!(f.operations, [Operation::JunctionHover(Some(Cell(3, 4)))]);
    f.picked = None;
    f.terrain = None;
    f.clear();
    owner.on_mouse_move(&mut f, P).unwrap();
    assert_eq!(
        f.operations,
        [Operation::JunctionHover(None), Operation::ClearCursorText(0xAB73_510C)]
    );
    assert_eq!(owner.hover(), None);
    let (mut owner, mut f) = driving();
    f.terrain = Some(World([1, 2, 3]));
    owner.on_mouse_move(&mut f, P).unwrap();
    assert_eq!(
        f.operations,
        [
            Operation::JunctionHover(Some(Cell(3, 4))),
            Operation::DriveMouse { automaton: A, at: P }
        ]
    );
    f.clear();
    f.objects.get_mut(&11).unwrap().present = false;
    assert_eq!(owner.on_mouse_move(&mut f, P), Err(Unsupported::MissingDriveState));
    assert_eq!(f.operations, [Operation::JunctionHover(Some(Cell(3, 4)))]);
}

#[test]
fn raw_buttons_messages_and_unsupported_dashboard_model_policies_are_visible() {
    let (mut owner, mut f) = driving();
    for (button, command) in [
        (0x4C1A_5CC9, 0xCC1F_0457),
        (0x2C16_C9CB, 0xAC1F_0449),
        (0x4992_046A, 0x6A93_5CB7),
        (0xE992_0494, 0x6A93_5CBA),
    ] {
        f.clear();
        owner.do_win_proc_message(&mut f, button).unwrap();
        assert_eq!(f.operations, [Operation::PostCommand(command)]);
    }
    f.clear();
    owner.do_message(&mut f, 0xA6B7_9602, None, 0).unwrap();
    assert_eq!(
        f.operations,
        [
            Operation::WindowEnabled {
                window: W,
                id: 0x4992_046A,
                enabled: true
            },
            Operation::WindowEnabled {
                window: W,
                id: 0xE992_0494,
                enabled: true
            }
        ]
    );
    f.clear();
    assert_eq!(
        owner.do_message(&mut f, 0x86AD_10EE, None, 0),
        Err(Unsupported::UpdateGauges)
    );
    for id in [0xAC1F_0449, 0xAC1F_045D, 0xCC1F_0457] {
        assert_eq!(owner.do_message(&mut f, id, None, 0), Err(Unsupported::PanelPolicy));
    }
    assert_eq!(
        owner.do_message(&mut f, 0x09D9_2FC3, Some(A), 0xB),
        Err(Unsupported::AlternateModel)
    );
    assert_eq!(f.operations, [Operation::EngineEvent { event: 0, start: false }]);
    assert_eq!(owner.timer(), TimerState::RestartBlocked);
    f.clear();
    owner.do_message(&mut f, 0x09D9_2FC3, Some(A), 0xD).unwrap();
    assert!(f.operations.is_empty());
    f.objects.get_mut(&11).unwrap().present = false;
    owner.do_message(&mut f, 0x09D9_2FC3, Some(A), 0xD).unwrap();
    assert_eq!(f.operations, [Operation::EngineEvent { event: 0, start: false }]);
    assert_eq!(owner.do_win_proc_message(&mut f, 0), Err(Unsupported::UnknownButton));
    assert_eq!(owner.do_message(&mut f, 0, None, 0), Err(Unsupported::UnknownMessage));
    assert_eq!(owner.init(), Err(Unsupported::NativeInit));
    assert_eq!(owner.shutdown(), Err(Unsupported::NativeShutdown));
}

#[test]
fn failures_preserve_emitted_prefix_and_never_invent_reference_cleanup() {
    let (mut owner, mut f) = driving();
    f.fail = Some("property");
    assert_eq!(owner.stop_tracking(&mut f), Err(Unsupported::ForeignPrimitive));
    assert_eq!(owner.current(), Some(A));
    assert_eq!(
        f.operations,
        [
            Operation::Callback40(A),
            Operation::DispatchTrack(None),
            Operation::VehicleManagerTrack(None),
            Operation::ZoomLimits(0, 5),
            Operation::EngineEvent { event: 0, start: false }
        ]
    );
    // No guessed release or rollback after a failed foreign primitive.
    f.fail = None;
    f.objects.get_mut(&11).unwrap().sound = None;
    f.clear();
    assert_eq!(owner.stop_tracking(&mut f), Err(Unsupported::MissingSoundProperty));
    assert_eq!(owner.current(), Some(A));
}

#[test]
fn raw_camera_values_keep_bits_and_reject_undefined_subtraction_domains() {
    for zoom in [0, -1, i32::MIN, i32::MAX, 7] {
        let mut owner = PlayerDrive::new(99);
        let mut f = Foreign::default();
        assert_eq!(
            owner.enter_driving_mode(
                &mut f,
                A,
                CameraFields {
                    zoom: Some(zoom),
                    ..CameraFields::default()
                }
            ),
            Err(Unsupported::NumericBoundary)
        );
        assert_eq!(f.operations, [Operation::DisasterLayer(false)]);
        assert_eq!(owner.current(), None);
    }
    let mut owner = PlayerDrive::new(99);
    let mut f = Foreign::default();
    owner
        .enter_driving_mode(
            &mut f,
            A,
            CameraFields {
                zoom: Some(2),
                level: Some(3),
                speed_bits: Some(0x7FC0_1234),
                preference_f10: 1,
            },
        )
        .unwrap();
    assert!(f.operations.contains(&Operation::ZoomLimits(1, 5)));
    assert!(f.operations.contains(&Operation::Zoom(2)));
    assert_eq!(owner.camera_speed_bits(), 0x7FC0_1234); // No float normalization.
}

#[test]
fn command_and_window_domains_do_not_invent_missing_resources_or_meanings() {
    let (mut owner, mut f) = driving();
    owner.attach_dashboard_window(None);
    for command in [2, 3] {
        assert_eq!(
            owner.apply_numeric_command(&mut f, command, true),
            Err(Unsupported::DashboardWindowMissing)
        );
    }
    assert!(f.operations.is_empty());
    assert_eq!(owner.apply_numeric_command(&mut f, 13, true), Ok(Progress::Ignored));
    assert_eq!(
        owner.apply_numeric_command(&mut f, 14, false),
        Err(Unsupported::UnknownCommand)
    );
    assert_eq!(
        owner.do_message(&mut f, 0xA6B7_9602, None, 0),
        Err(Unsupported::DashboardWindowMissing)
    );
    assert!(f.operations.is_empty());
    f.objects.get_mut(&11).unwrap().group = 0x4003;
    assert_eq!(
        owner.on_mouse_down_l(&mut f, P, CameraFields::default()),
        Ok(Progress::Ignored)
    );
    assert!(f.operations.is_empty());
}

#[test]
fn choosing_after_driving_finishes_tracking_before_pause_and_drops_hover_identity() {
    let (mut owner, mut f) = driving();
    assert!(matches!(owner.enter_choosing_mode(&mut f), Ok(Progress::Partial(_))));
    assert_eq!(f.operations[7], Operation::ReleaseDriven(A));
    assert_eq!(
        &f.operations[8..],
        [
            Operation::ClearCursorText(0xAB73_510C),
            Operation::SimulatorPause,
            Operation::ManagerPause(43),
            Operation::ManagerPause(44),
            Operation::ManagerPause(45),
            Operation::ManagerPause(46)
        ]
    );
    assert_eq!(owner.current(), None);
    assert_eq!(owner.hover(), None);
}
