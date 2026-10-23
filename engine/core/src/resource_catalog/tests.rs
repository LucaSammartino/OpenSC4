// SPDX-License-Identifier: GPL-3.0-or-later
// V01–V24 refer to the independent, pre-implementation ledger.md, not outputs
// captured from this implementation. These are portable cases, not game runs.
use super::key_input::*;
use super::*;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
enum Trace {
    Effect(Effect),
    Input(InputEffect),
    Enum(u32),
    Count(usize),
    Entry(usize),
    Name(u64),
    Order,
    Coordinates,
    Data((i32, i32)),
    Parser,
    Qi,
    NameBytes,
    Close(u32),
    Delegate(u32),
}

struct PrimitiveHost<'a> {
    entries: Vec<&'a RawKey>,
    counts: VecDeque<usize>,
    enumeration_ok: bool,
    permutation: Option<Vec<usize>>,
    selected: Option<&'a RawKey>,
    name: Option<String>,
    parser: u32,
    qi: bool,
    name_bytes: Vec<u8>,
    close_al: bool,
    trace: Vec<Trace>,
}

impl<'a> PrimitiveHost<'a> {
    fn new(entries: &'a [RawKey]) -> Self {
        Self {
            entries: entries.iter().collect(),
            counts: VecDeque::new(),
            enumeration_ok: true,
            permutation: None,
            selected: None,
            name: None,
            parser: 0,
            qi: true,
            name_bytes: Vec::new(),
            close_al: false,
            trace: Vec::new(),
        }
    }
    fn effects(&self) -> Vec<Effect> {
        self.trace
            .iter()
            .filter_map(|e| match e {
                Trace::Effect(e) => Some(e.clone()),
                _ => None,
            })
            .collect()
    }
    fn inputs(&self) -> Vec<InputEffect> {
        self.trace
            .iter()
            .filter_map(|e| match e {
                Trace::Input(e) => Some(e.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Effects for PrimitiveHost<'_> {
    fn emit(&mut self, effect: Effect) {
        self.trace.push(Trace::Effect(effect));
    }
}

impl<'a> RowPrimitives<'a> for PrimitiveHost<'a> {
    fn enumerate(&mut self, t: u32) -> bool {
        self.trace.push(Trace::Enum(t));
        self.enumeration_ok
    }
    fn enumeration_count(&mut self) -> usize {
        let n = self.counts.pop_front().unwrap_or(self.entries.len());
        self.trace.push(Trace::Count(n));
        n
    }
    fn entry(&mut self, index: usize) -> Option<&'a RawKey> {
        self.trace.push(Trace::Entry(index));
        self.entries.get(index).copied()
    }
    fn name(&mut self, key: &'a RawKey) -> Option<String> {
        self.trace.push(Trace::Name(key.identity));
        self.name.clone()
    }
    fn post_order(&mut self, rows: &[Row<'a>]) -> Vec<usize> {
        self.trace.push(Trace::Order);
        self.permutation.clone().unwrap_or_else(|| (0..rows.len()).collect())
    }
    fn selected_coordinates(&mut self) -> (i32, i32) {
        self.trace.push(Trace::Coordinates);
        (7, -4)
    }
    fn row_data(&mut self, coords: (i32, i32)) -> Option<&'a RawKey> {
        self.trace.push(Trace::Data(coords));
        self.selected
    }
}

impl InputPrimitives for PrimitiveHost<'_> {
    fn input_effect(&mut self, effect: InputEffect) {
        self.trace.push(Trace::Input(effect));
    }
    fn parse_uint32(&mut self) -> u32 {
        self.trace.push(Trace::Parser);
        self.parser
    }
    fn spinner_qi(&mut self) -> bool {
        self.trace.push(Trace::Qi);
        self.qi
    }
    fn name_bytes(&mut self) -> Vec<u8> {
        self.trace.push(Trace::NameBytes);
        self.name_bytes.clone()
    }
    fn close(&mut self, result: u32) -> bool {
        self.trace.push(Trace::Close(result));
        self.close_al
    }
    fn delegate_message(&mut self, id: u32) -> bool {
        self.trace.push(Trace::Delegate(id));
        false
    }
}

fn raw(identity: u64, t: u32, g: u32, i: u32) -> RawKey {
    RawKey {
        identity,
        words: Key::new(t, g, i),
    }
}
fn descriptor(t: u32, g: u32, preview: Handle) -> Descriptor {
    Descriptor {
        t,
        g,
        preview: Some(preview),
        name: format!("descriptor {preview}"),
    }
}
fn catalog<'a>(group: u32) -> Catalog<'a> {
    let mut c = Catalog::new(vec![descriptor(0x10, group, 80)], Some(90)).unwrap();
    c.descriptor_index = 0;
    c
}
fn selected_rows(host: &PrimitiveHost<'_>) -> Vec<usize> {
    host.effects()
        .iter()
        .filter_map(|e| match e {
            Effect::Select(i) => Some(*i),
            _ => None,
        })
        .collect()
}

#[test]
fn v01_group_duplicates_raw_identity_and_count_call_order() {
    let entries = [
        raw(1, 0x10, 0x20, 0x2a),
        raw(2, 0x10, 0x21, 1),
        raw(3, 0x10, 0x20, 0x2a),
    ];
    let mut h = PrimitiveHost::new(&entries);
    let mut c = catalog(0x20);
    assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Ordered(2)));
    assert!(std::ptr::eq(c.rows[0].data, &entries[0]));
    assert!(std::ptr::eq(c.rows[1].data, &entries[2]));
    assert_eq!(
        h.trace,
        vec![
            Trace::Effect(Effect::ClearRows),
            Trace::Enum(0x10),
            Trace::Count(3),
            Trace::Entry(0),
            Trace::Name(1),
            Trace::Effect(Effect::Append {
                row: 0,
                text: "0x0000002A".into(),
                identity: 1
            }),
            Trace::Count(3),
            Trace::Entry(1),
            Trace::Count(3),
            Trace::Entry(2),
            Trace::Name(3),
            Trace::Effect(Effect::Append {
                row: 1,
                text: "0x0000002A".into(),
                identity: 3
            }),
            Trace::Count(3),
            Trace::Effect(Effect::Order([0, 0, 1, 0])),
            Trace::Order,
            Trace::Effect(Effect::SelectionRange {
                start: 0,
                count: 2,
                flags: 15
            }),
            Trace::Effect(Effect::Select(0)),
            Trace::Effect(Effect::Show(80)),
            Trace::Effect(Effect::PreviewKey(80, Key::new(0x10, 0x20, 0x2a))),
            Trace::Effect(Effect::FocusList(90)),
        ]
    );
}

#[test]
fn v02_literal_zero_group_and_returned_type_is_not_rechecked() {
    let entries = [raw(1, 0x99, 0, 1), raw(2, 0x10, 1, 2)];
    let mut h = PrimitiveHost::new(&entries);
    h.name = Some("raw helper name".into());
    let mut c = catalog(0);
    assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Ordered(1)));
    assert_eq!(c.rows[0].data.identity, 1);
    assert_eq!(c.rows[0].text, "raw helper name");
}

#[test]
fn v03_v05_missing_key_uses_post_order_first_without_numeric_sort() {
    let entries = [raw(1, 0x10, 0x20, 9), raw(2, 0x10, 0x20, 3)];
    for (permutation, expected_i) in [(vec![0, 1], 9), (vec![1, 0], 3)] {
        let mut h = PrimitiveHost::new(&entries);
        h.permutation = Some(permutation);
        let mut c = catalog(0x20);
        c.update_key_list(Key::new(0x10, 0x20, 0xff), &mut h).unwrap();
        assert_eq!(selected_rows(&h), vec![0]);
        assert_eq!(
            &h.effects()[h.effects().len() - 2..],
            &[
                Effect::PreviewKey(80, Key::new(0x10, 0x20, expected_i)),
                Effect::FocusList(90)
            ]
        );
    }
}

#[test]
fn v04_last_full_key_duplicate_selected_after_primitive_reorder() {
    let entries = [
        raw(1, 0x10, 0x20, 0x2a),
        raw(2, 0x10, 0x20, 0x2a),
        raw(3, 0x10, 0x20, 3),
    ];
    let mut h = PrimitiveHost::new(&entries);
    h.permutation = Some(vec![1, 2, 0]);
    let mut c = catalog(0x20);
    c.update_key_list(Key::new(0x10, 0x20, 0x2a), &mut h).unwrap();
    assert_eq!(selected_rows(&h), vec![0, 2]);
    assert_eq!(
        c.rows.iter().map(|r| r.data.identity).collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert_eq!(
        h.effects()
            .iter()
            .filter(|e| matches!(e, Effect::PreviewKey(..)))
            .count(),
        1
    );
    // Full T/G/I equality, not instance-only equality.
    h.trace.clear();
    c.update_key_list(Key::new(0x11, 0x20, 0x2a), &mut h).unwrap();
    assert_eq!(selected_rows(&h), vec![0]);
}

#[test]
fn v06_invalid_descriptor_and_null_list_retain_before_clear() {
    let entries = [raw(1, 0x10, 0x20, 9)];
    let mut h = PrimitiveHost::new(&entries);
    let mut c = catalog(0x20);
    c.rows.push(Row {
        data: &entries[0],
        text: "prior".into(),
    });
    for index in [u32::MAX, 1] {
        c.descriptor_index = index;
        assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Retained));
        assert_eq!(c.rows[0].text, "prior");
        assert!(h.trace.is_empty());
    }
    c.descriptor_index = 0;
    c.list = None;
    assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Retained));
    assert_eq!(c.rows[0].text, "prior");
    assert!(h.trace.is_empty());
}

#[test]
fn v07_failure_happens_after_clear_and_before_ordering() {
    let entries = [raw(1, 0x10, 0x20, 9)];
    let mut h = PrimitiveHost::new(&entries);
    h.enumeration_ok = false;
    let mut c = catalog(0x20);
    c.rows.push(Row {
        data: &entries[0],
        text: "prior".into(),
    });
    c.current_preview = Some(80);
    assert_eq!(
        c.update_key_list(Key::default(), &mut h),
        Ok(Rebuild::EnumerationFailedAfterClear)
    );
    assert!(c.rows.is_empty());
    assert_eq!(c.current_preview, Some(80));
    assert_eq!(h.trace, vec![Trace::Effect(Effect::ClearRows), Trace::Enum(0x10)]);
}

#[test]
fn v08_zero_passes_still_order_and_leave_preview() {
    let miss = [raw(1, 0x10, 0x21, 9)];
    for entries in [&miss[..], &[][..]] {
        let mut h = PrimitiveHost::new(entries);
        let mut c = catalog(0x20);
        c.current_preview = Some(80);
        assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Ordered(0)));
        assert_eq!(h.effects(), vec![Effect::ClearRows, Effect::Order([0, 0, 1, 0])]);
        assert_eq!(c.current_preview, Some(80));
    }
}

#[test]
fn v09_null_output_unchanged_and_refresh_conditional() {
    let entries = [raw(1, 0x10, 0x20, 9)];
    let mut h = PrimitiveHost::new(&entries);
    let mut c = catalog(0x20);
    let mut output = Key::new(0xaaaaaaaa, 0xbbbbbbbb, 0xcccccccc);
    assert!(!Catalog::get_selected_key(&mut output, &mut h));
    assert_eq!(output, Key::new(0xaaaaaaaa, 0xbbbbbbbb, 0xcccccccc));
    h.trace.clear();
    assert_eq!(c.refresh(&mut h), Ok(None));
    assert_eq!(h.trace, vec![Trace::Coordinates, Trace::Data((7, -4))]);
    h.selected = Some(&entries[0]);
    h.trace.clear();
    assert_eq!(c.refresh(&mut h), Ok(Some(Rebuild::Ordered(1))));
    assert_eq!(
        &h.trace[..3],
        &[
            Trace::Coordinates,
            Trace::Data((7, -4)),
            Trace::Effect(Effect::ClearRows)
        ]
    );
}

#[test]
fn v10_v11_picker_permutation_and_zero_on_failure() {
    let entries = [raw(1, 0x11223344, 0x55667788, 0x99aabbcc)];
    let mut h = PrimitiveHost::new(&entries);
    let mut p = Picker { raw: [4, 5, 6] };
    h.selected = Some(&entries[0]);
    assert_eq!(p.output(&mut h), [0x55667788, 0x11223344, 0x99aabbcc]);
    h.selected = None;
    h.trace.clear();
    assert_eq!(p.output(&mut h), [0, 0, 0]);
    assert_eq!(
        h.trace,
        vec![
            Trace::Coordinates,
            Trace::Data((7, -4)),
            Trace::Effect(Effect::WriteVariant([0, 0, 0])),
            Trace::Effect(Effect::AssignOutputVariant)
        ]
    );
    let mut c = catalog(0x20);
    c.read_only = 7;
    h.trace.clear();
    p.input([0x20, 0x10, 123], &mut c, &mut h);
    assert_eq!(
        (c.type_filter, c.group_filter, c.read_only, c.byte_dd),
        (0x10, 0x20, 0, 0)
    );
    assert_eq!(h.effects(), vec![Effect::AssignInputVariant, Effect::ReadVariantData]);
}

#[test]
fn v12_output_available_for_both_modal_results_but_acceptance_separate() {
    let entries = [raw(1, 0x10, 0x20, 0x2a)];
    for (modal, accepted) in [(0, Some(Key::new(0x10, 0x20, 0x2a))), (1, None)] {
        let mut h = PrimitiveHost::new(&entries);
        h.selected = Some(&entries[0]);
        let mut p = Picker { raw: [0; 3] };
        let output = p.output(&mut h);
        assert_eq!(output, [0x20, 0x10, 0x2a]);
        assert_eq!(Picker::accepted_key(output, modal), accepted);
        assert!(h.effects().contains(&Effect::AssignOutputVariant));
    }
}

#[test]
fn v13_qi_effect_order_is_owner_specific_only() {
    let mut h = PrimitiveHost::new(&[]);
    assert_eq!(catalog_qi(0x7a368ee3, 10, &mut h), Ok(true));
    assert_eq!(h.effects(), vec![Effect::WriteQiPointer(10), Effect::AddRef(10)]);
    h.trace.clear();
    assert_eq!(picker_qi(0x06c5ff7b, 11, &mut h), Ok(true));
    assert_eq!(h.effects(), vec![Effect::AddRef(11), Effect::WriteQiPointer(11)]);
    h.trace.clear();
    assert_eq!(catalog_qi(0, 10, &mut h), Err(Boundary::UnknownMethod));
    assert!(h.trace.is_empty());
}

#[test]
fn v14_v15_descriptor_admission_mapping_and_store_only_setters() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = Catalog::new(
        vec![
            descriptor(0x10, 0x20, 80),
            descriptor(0x10, 0x21, 81),
            descriptor(0x11, 0x20, 82),
        ],
        Some(90),
    )
    .unwrap();
    for (t, g, expected) in [
        (0x10, 0, vec![0, 1]),
        (0, 0x20, vec![0, 2]),
        (0, 0, vec![0, 1, 2]),
        (0x11, 0x20, vec![2]),
    ] {
        h.trace.clear();
        c.set_type_filter(t);
        c.set_group_filter(g);
        c.set_read_only(0xa5);
        c.set_byte_dd(0x17);
        assert!(h.trace.is_empty());
        c.init_descriptor_map(&mut h).unwrap();
        assert_eq!(c.visible_to_original, expected);
        assert_eq!(c.descriptor_index, expected[0] as u32);
        assert_eq!(
            &h.effects()[..4],
            &[
                Effect::PreviewReadOnly(80 + expected[0] as u64, 0xa5),
                Effect::AttachPreview(80 + expected[0] as u64),
                Effect::Hide(80 + expected[0] as u64),
                Effect::AppendDescriptor(format!("descriptor {}", 80 + expected[0]), -2)
            ]
        );
        assert_eq!(h.effects().last(), Some(&Effect::SelectDescriptor(0)));
    }
    h.trace.clear();
    c.set_type_filter(0x99);
    assert_eq!(
        c.init_descriptor_map(&mut h),
        Err(Boundary::RegistrationOrEmptyAdmission)
    );
    assert!(h.trace.is_empty());
    assert!(matches!(
        Catalog::new(vec![], Some(90)),
        Err(Boundary::RegistrationOrEmptyAdmission)
    ));
}

#[test]
fn v16_v17_byte_sanitation_and_lower_hex_display_only() {
    assert_eq!(filename(b"A9-b c", 0x1234), "A__b_c_00001234");
    assert_eq!(filename(b"", 0xabcdef01), "abcdef01");
    assert_eq!(filename(&[b'Z', 0xc3, 0xa9, b'0', b'_', b'z'], 0), "Z____z_00000000");
}

#[test]
fn v18_initial_signed_normalization_and_unsigned_direction() {
    for (current, requested, probe, direction) in [
        (0, 0x8000, 1, 1),
        (0, 0xffffffff, 0x7fff, 1),
        (3, 0, 0x7fff, -1),
        (0xffffffff, 0x7fff, 0x7fff, -1),
    ] {
        let mut s = Search::new(Key::new(0x10, 0x20, current), requested);
        assert_eq!(s.direction(), direction);
        assert_eq!(s.query(), Key::new(0x10, 0x20, probe));
        let SearchProgress::Complete(done) = s.reply(false) else {
            panic!("free must complete")
        };
        assert_eq!(done.key(), Key::new(0x10, 0x20, probe));
    }
}

#[test]
fn v19_v20_occupancy_wrap_has_different_initial_and_later_normalization() {
    for (current, requested, first, second) in [(0x7ffe, 0x7fff, 0x7fff, 0), (2, 1, 1, 0x7fff)] {
        let mut s = Search::new(Key::new(0x10, 0x20, current), requested);
        assert_eq!(s.query().i, first);
        assert_eq!(s.reply(true), SearchProgress::Query(Key::new(0x10, 0x20, second)));
        let SearchProgress::Complete(done) = s.reply(false) else {
            panic!("free must complete")
        };
        assert_eq!(done.key().i, second);
    }
}

#[test]
fn v21_parser_raw_word_and_text_qi_no_release_order() {
    let mut h = PrimitiveHost::new(&[]);
    h.parser = 0xffffffff;
    let mut input = KeyInput::new(Key::new(0x10, 0x20, 4));
    assert!(input.text_notification(0xe83b22ee, &mut h));
    assert_eq!(input.key.i, 0xffffffff);
    assert_eq!(input.reentrancy, 0);
    assert_eq!(input.display_filename, "ffffffff");
    assert_eq!(
        h.trace,
        vec![
            Trace::Input(InputEffect::ReadInstanceText),
            Trace::Input(InputEffect::ParseUint32),
            Trace::Parser,
            Trace::Input(InputEffect::FindSpinner),
            Trace::Input(InputEffect::Reentrancy(1)),
            Trace::Input(InputEffect::QuerySpinner612ce0c3),
            Trace::Qi,
            Trace::Input(InputEffect::SpinnerWrite(0xffffffff)),
            Trace::Input(InputEffect::Reentrancy(0)),
            Trace::NameBytes,
            Trace::Input(InputEffect::Filename("ffffffff".into()))
        ]
    );
}

#[test]
fn v22_reentrant_spinner_returns_true_before_search_or_effects() {
    let mut h = PrimitiveHost::new(&[]);
    let mut input = KeyInput::new(Key::new(0x10, 0x20, 4));
    input.reentrancy = 1;
    assert_eq!(
        input.message(
            Message::Spinner {
                control: SPINNER_ID,
                value: 5
            },
            &mut h
        ),
        MessageProgress::Return(true)
    );
    assert!(h.trace.is_empty());
    assert_eq!(input.key.i, 4);
    assert_eq!(input.spinner_start(0, 5), SpinnerStart::Return(false));
}

#[test]
fn v23_shutdown_aliases_release_each_descriptor_not_current_preview() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = Catalog::new(vec![descriptor(0x10, 0x20, 80), descriptor(0x10, 0x21, 80)], Some(90)).unwrap();
    c.current_preview = Some(80);
    c.shutdown(&mut h);
    assert_eq!(
        h.effects(),
        vec![
            Effect::HideOwner,
            Effect::Window158Zero,
            Effect::Release(80),
            Effect::DestroyDescriptorName,
            Effect::Release(80),
            Effect::DestroyDescriptorName,
            Effect::ClearCurrentPreview,
            Effect::Release(90),
            Effect::BaseShutdown
        ]
    );
    assert!(c.descriptors.is_empty());
    assert_eq!(c.list, None);
    assert_eq!(c.current_preview, None);
    h.trace.clear();
    c.destroy(1, &mut h);
    assert_eq!(
        h.effects(),
        vec![
            Effect::InstallCatalogTables,
            Effect::DestroyVectorMap,
            Effect::DestroyBaseWindow,
            Effect::HeapFree
        ]
    );
}

#[test]
fn v24_base_creation_failure_still_success_and_success_release_once() {
    let mut h = PrimitiveHost::new(&[]);
    assert!(base_init(false, None, &mut h));
    assert_eq!(h.effects(), vec![Effect::ConfigureBase, Effect::CreateBaseChild]);
    h.trace.clear();
    assert!(base_init(true, Some(7), &mut h));
    assert!(h.trace.is_empty());
    assert!(base_init(false, Some(7), &mut h));
    assert_eq!(
        h.effects(),
        vec![
            Effect::ConfigureBase,
            Effect::CreateBaseChild,
            Effect::ConfigureAddChild(7),
            Effect::Release(7)
        ]
    );
}

#[test]
fn preview_switch_identity_order_no_refs_and_same_preview_still_updates() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = catalog(0x20);
    c.current_preview = Some(70);
    let key = Key::new(0x10, 0x20, 9);
    c.set_view_key(key, &mut h).unwrap();
    assert_eq!(
        h.effects(),
        vec![
            Effect::PreviewClear(70),
            Effect::Hide(70),
            Effect::Show(80),
            Effect::PreviewKey(80, key),
            Effect::FocusList(90)
        ]
    );
    h.trace.clear();
    c.set_view_key(key, &mut h).unwrap();
    assert_eq!(h.effects(), vec![Effect::PreviewKey(80, key), Effect::FocusList(90)]);
}

#[test]
fn selector_missing_child_qi_failure_success_and_checked_index() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = Catalog::new(vec![descriptor(0x10, 0x20, 80), descriptor(0x10, 0x21, 81)], Some(90)).unwrap();
    c.visible_to_original = vec![1];
    c.descriptor_index = 0;
    assert_eq!(
        c.selector_command(0xe8711255, 0x04c9303e, SelectorReply::MissingChild, &mut h),
        Ok(true)
    );
    assert!(h.trace.is_empty());
    assert_eq!(
        c.selector_command(0xe8711255, 0x04c9303e, SelectorReply::QiFailed, &mut h),
        Ok(false)
    );
    assert_eq!(c.descriptor_index, 0);
    assert_eq!(h.effects().last(), Some(&Effect::FocusList(90)));
    h.trace.clear();
    assert_eq!(
        c.selector_command(0xe8711255, 0x04c9303e, SelectorReply::VisibleIndex(0), &mut h),
        Ok(false)
    );
    assert_eq!(c.descriptor_index, 1);
    assert_eq!(h.effects()[0], Effect::ReleaseSelector);
    h.trace.clear();
    assert_eq!(
        c.selector_command(0xe8711255, 0x04c9303e, SelectorReply::VisibleIndex(99), &mut h),
        Err(Boundary::CheckedIndex)
    );
    assert_eq!(h.effects(), vec![Effect::ReleaseSelector]);
}

#[test]
fn button_maps_and_foreign_al_are_distinct_and_add_resource_unsupported() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = catalog(0x20);
    c.current_preview = Some(80);
    for (id, modal, expected) in [
        (0x24c93018, true, vec![Effect::PreviewClear(80), Effect::ModalClose(1)]),
        (0x24c9301a, true, vec![Effect::ModalClose(0)]),
        (
            0x24c93018,
            false,
            vec![Effect::PreviewClear(80), Effect::RemoveFromParent],
        ),
    ] {
        h.trace.clear();
        assert_eq!(c.button(id, modal, &mut h), Ok(false));
        assert_eq!(h.effects(), expected);
    }
    h.trace.clear();
    assert_eq!(c.button(0x24c93019, true, &mut h), Err(Boundary::AddResource));
    assert!(h.trace.is_empty());
    assert_eq!(c.button(0x24c9301b, true, &mut h), Ok(true));
    assert!(h.trace.is_empty());
    assert_eq!(c.button(0, true, &mut h), Ok(false));
    let input = KeyInput::new(Key::default());
    for (id, result, al) in [(0x24c93018, 0, false), (0x24c93019, 1, true)] {
        h.trace.clear();
        h.close_al = al;
        assert_eq!(input.command(0x287259f6, id, &mut h), al);
        assert_eq!(
            h.trace,
            vec![Trace::Input(InputEffect::ModalClose(result)), Trace::Close(result)]
        );
    }
    h.trace.clear();
    assert!(!input.command(0x287259f6, 0x24c9301a, &mut h));
    assert!(h.trace.is_empty());
}

#[test]
fn spinner_continuation_releases_qi_only_when_adjusting_and_qi_succeeds() {
    for (requested, occupied, qi, expect_write) in [
        (0x7fff, true, true, true),
        (5, false, true, false),
        (0x8000, false, false, false),
    ] {
        let mut h = PrimitiveHost::new(&[]);
        h.qi = qi;
        let mut input = KeyInput::new(Key::new(0x10, 0x20, 4));
        let SpinnerStart::Search(mut search) = input.spinner_start(SPINNER_ID, requested) else {
            panic!("search")
        };
        if occupied {
            assert!(matches!(search.reply(true), SearchProgress::Query(_)));
        }
        let SearchProgress::Complete(done) = search.reply(false) else {
            panic!("free")
        };
        assert!(!input.spinner_finish(done, &mut h));
        let events = h.inputs();
        assert_eq!(
            events.iter().any(|e| matches!(e, InputEffect::SpinnerWrite(_))),
            expect_write
        );
        assert_eq!(events.contains(&InputEffect::ReleaseSpinner), expect_write);
        assert_eq!(input.reentrancy, 0);
        assert!(events.ends_with(&[
            InputEffect::SelectInstanceText,
            InputEffect::FocusInstanceText,
            InputEffect::Filename(
                if occupied {
                    "00000000"
                } else if requested == 5 {
                    "00000005"
                } else {
                    "00000001"
                }
                .into()
            )
        ]));
    }
}

#[test]
fn all_occupied_cycle_is_noncompletion_with_no_probe_limit_or_failure() {
    let mut search = Search::new(Key::new(0x10, 0x20, 0), 1);
    for _ in 0..32768 {
        assert!(matches!(search.reply(true), SearchProgress::Query(_)));
    }
    assert_eq!(search.query(), Key::new(0x10, 0x20, 1));
    // Another probe still progresses. This is a suspended domain, not native failure.
    assert_eq!(search.reply(true), SearchProgress::Query(Key::new(0x10, 0x20, 2)));
}

#[test]
fn count_requery_can_end_traversal_and_malformed_primitive_is_checked_extension() {
    let entries = [raw(1, 0x10, 0x20, 1), raw(2, 0x10, 0x20, 2)];
    let mut h = PrimitiveHost::new(&entries);
    h.counts = VecDeque::from([2, 1]);
    let mut c = catalog(0x20);
    assert_eq!(c.update_key_list(Key::default(), &mut h), Ok(Rebuild::Ordered(1)));
    assert!(!h.trace.contains(&Trace::Entry(1)));
    h.counts.clear();
    h.permutation = Some(vec![0, 0]);
    assert_eq!(
        c.update_key_list(Key::default(), &mut h),
        Err(Boundary::CheckedPermutation)
    );
    assert!(!h
        .effects()
        .iter()
        .any(|e| matches!(e, Effect::Release(_) | Effect::AddRef(_))));
}

#[test]
fn notification_dispatch_and_key_input_shutdown_explicit_intent_order() {
    let mut h = PrimitiveHost::new(&[]);
    let mut input = KeyInput::new(Key::default());
    assert_eq!(
        input.message(Message::Forward4348b111, &mut h),
        MessageProgress::Return(true)
    );
    assert_eq!(
        input.message(Message::Other(0x0d), &mut h),
        MessageProgress::Return(false)
    );
    assert_eq!(
        h.trace,
        vec![
            Trace::Input(InputEffect::ForwardMessage {
                message: 0x4348b111,
                argument: 0
            }),
            Trace::Input(InputEffect::DelegateMessage(0x0d)),
            Trace::Delegate(0x0d)
        ]
    );
    h.trace.clear();
    assert!(input.text_notification(0, &mut h));
    assert!(h.trace.is_empty());
    input.held = [Some(10), Some(10), Some(12)];
    input.shutdown(&mut h);
    assert_eq!(
        h.trace,
        vec![
            Trace::Effect(Effect::HideOwner),
            Trace::Effect(Effect::Window158Zero),
            Trace::Input(InputEffect::HeldSlot10(10)),
            Trace::Effect(Effect::Release(10)),
            Trace::Input(InputEffect::HeldSlot10(10)),
            Trace::Effect(Effect::Release(10)),
            Trace::Input(InputEffect::HeldWindowSlot14(12)),
            Trace::Effect(Effect::Release(12)),
            Trace::Effect(Effect::BaseShutdown)
        ]
    );
    h.trace.clear();
    input.shutdown(&mut h);
    assert_eq!(
        h.effects(),
        vec![Effect::HideOwner, Effect::Window158Zero, Effect::BaseShutdown]
    );
    assert!(h.inputs().is_empty());
}

#[test]
fn scoped_destructors_keep_picker_catalog_and_key_input_orders_distinct() {
    let mut h = PrimitiveHost::new(&[]);
    let mut c = catalog(0x20);
    let mut p = Picker { raw: [1, 2, 3] };
    p.destroy(&mut c, 2, &mut h);
    assert_eq!(
        h.effects(),
        vec![
            Effect::InstallPickerTables,
            Effect::DestroyVariant,
            Effect::InstallCatalogTables,
            Effect::Release(80),
            Effect::DestroyDescriptorName,
            Effect::DestroyVectorMap,
            Effect::DestroyBaseWindow,
        ]
    );
    // Catalog destruction does not imply Shutdown or a list Release.
    assert_eq!(c.list, Some(90));
    h.trace.clear();
    let mut input = KeyInput::new(Key::default());
    input.held[0] = Some(10);
    input.destroy(1, &mut h);
    assert_eq!(
        h.trace,
        vec![
            Trace::Input(InputEffect::InstallTables),
            Trace::Effect(Effect::HideOwner),
            Trace::Effect(Effect::Window158Zero),
            Trace::Input(InputEffect::HeldSlot10(10)),
            Trace::Effect(Effect::Release(10)),
            Trace::Effect(Effect::BaseShutdown),
            Trace::Input(InputEffect::DestroyFilename),
            Trace::Effect(Effect::DestroyBaseWindow),
            Trace::Effect(Effect::HeapFree),
        ]
    );
}

#[test]
fn checked_extensions_refuse_invalid_storage_without_native_false_claims() {
    let entries = [raw(1, 0x10, 0x20, 1)];
    let mut h = PrimitiveHost::new(&entries);
    let mut c = catalog(0x20);
    c.list = None;
    assert_eq!(c.set_view_key(Key::default(), &mut h), Err(Boundary::CheckedControls));
    assert!(h.trace.is_empty());
    c.list = Some(90);
    c.descriptor_index = u32::MAX;
    assert_eq!(c.set_view_key(Key::default(), &mut h), Err(Boundary::CheckedIndex));
    assert!(h.trace.is_empty());
    c.descriptor_index = 0;
    c.descriptors[0].preview = None;
    assert_eq!(
        c.init_descriptor_map(&mut h),
        Err(Boundary::RegistrationOrEmptyAdmission)
    );
    assert!(h.trace.is_empty());
    h.entries.clear();
    h.counts.push_back(1);
    assert_eq!(
        c.update_key_list(Key::default(), &mut h),
        Err(Boundary::InvalidEnumerationEntry)
    );
    assert_eq!(
        h.trace,
        vec![
            Trace::Effect(Effect::ClearRows),
            Trace::Enum(0x10),
            Trace::Count(1),
            Trace::Entry(0)
        ]
    );
}
