// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent published examples from resource-exemplar-editor.md. Native
//! floating conversion is supplied by the existing formats implementation.
use fc4_core::resource_editor::*;
use fc4_core::variant::{mem, Elem, Variant};

struct Runtime;
impl TextPrimitives for Runtime {
    fn english_float32(&mut self, text: &str) -> f32 {
        fc4_formats::exemplar::parse_float(text.as_bytes()) as f32
    }
    fn atol32(&mut self, text: &str) -> i32 {
        // These vectors avoid CRT overflow (RE3); existing scanf-long primitive
        // has the same whitespace/sign/decimal-prefix behavior on this domain.
        fc4_core::rz_files::ini::scan_long(text.as_bytes()).unwrap_or(0)
    }
}

fn words(values: &[u32]) -> Variant {
    Variant::array(mem::UINT32, values.iter().copied().map(Elem::U32).collect())
}

struct Unknown;
impl UnpublishedFormatting for Unknown {
    fn format(&mut self, _: u32, value: &Variant) -> Result<String, EditorError> {
        Err(EditorError::UnspecifiedFormatting(value.mem_type()))
    }
}

#[test]
fn re_01_ingredient_callback_order_count_zero_and_defaults_word_swap() {
    let ini = Ingredients::parse("[ResKeyTableEditorDefaults]\n0x12345678=0x1 0x2 0x3 0x4\n[Properties]\n0x12345678=FLOAT32,0,\"ignored\",Editable,\"Example\",\"Help\",Table,0,0\n[Aliases]\n0x1=Editable\n0xA7847D6A=Table", &mut Runtime);
    let p = &ini.properties[&0x12345678];
    assert_eq!((p.filter, p.editor_class), (1, RESOURCE_TABLE_CLASS));
    assert_eq!(p.default, words(&[]));
    let defaults = ini.table_defaults[&0x12345678];
    assert_eq!(defaults.parameter, 1);
    assert_eq!(
        (defaults.key.type_id, defaults.key.group, defaults.key.instance),
        (3, 2, 4)
    );
}

#[test]
fn re_02_combo_maps_read_in_numeric_order_known_match_and_fallback() {
    let ini = Ingredients::parse("[Uint32ValueMap_0x00000010]\n5=High\n1=Low\n2=Mid\n[Properties]\n16=UINT32,1,\"5\",1,\"N\",\"H\",0x8739DFC1,0,0", &mut Runtime);
    let mut editor = ComboEditor::new(mem::UINT32, Variant::Scalar(Elem::U32(5))).unwrap();
    editor.init(&ini, 16, 200, &mut Unknown).unwrap();
    assert_eq!(editor.index_values, vec![1, 2, 5]);
    assert_eq!(editor.controls[0].as_ref().unwrap().selected, 2);
    editor.controls[0].as_mut().unwrap().selected = 0;
    assert_eq!(editor.read().unwrap(), Variant::Scalar(Elem::U32(1)));
    let mut editor = ComboEditor::new(mem::UINT32, Variant::Scalar(Elem::U32(7))).unwrap();
    editor.init(&ini, 16, 200, &mut Unknown).unwrap();
    assert_eq!(editor.controls[0].as_ref().unwrap().selected, 0);
    assert_eq!(combo_selection_index(mem::UINT8, 300), 44);
}

struct PrintThree;
impl GridFormatting for PrintThree {
    fn three_significant_digits(&mut self, value: f32) -> String {
        // The published primitive results, not the algorithm under test.
        match value.to_bits() {
            n if n == 1234.5f32.to_bits() => "1.23e+03".into(),
            n if n == 0.33333f32.to_bits() => "0.333".into(),
            _ => panic!("unexpected primitive fixture input"),
        }
    }
}

#[test]
fn re_03_grid_discards_partial_row_from_editing_and_truncates_uint8() {
    let mut editor = GridEditor::new(Variant::array(mem::UINT8, (0..35).map(Elem::U8).collect()));
    editor.init(300, 200, &mut PrintThree);
    assert_eq!(editor.rows, 2);
    editor.cells[16] = Some("300".into());
    let result = editor.read(&mut Runtime).unwrap().items();
    assert_eq!(result[16], Elem::U8(44));
    assert_eq!(&result[32..], &[Elem::U8(32), Elem::U8(33), Elem::U8(34)]);
}

#[test]
fn re_04_grid_float_open_accept_loses_precision_in_visible_rows_using_existing_parser() {
    let mut items = vec![Elem::F32(1234.5); 17];
    items[1] = Elem::F32(0.33333);
    let mut editor = GridEditor::new(Variant::array(mem::FLOAT32, items));
    editor.init(300, 200, &mut PrintThree);
    let result = editor.read(&mut Runtime).unwrap().items();
    assert_eq!(result[0], Elem::F32(1230.0));
    assert_eq!(
        result[1],
        Elem::F32(fc4_formats::exemplar::parse_float(b"0.333") as f32)
    );
    assert_eq!(result[16], Elem::F32(1234.5));
    assert_eq!(
        default_value(mem::FLOAT32, 1, "0.05", &mut Runtime).unwrap(),
        Variant::Scalar(Elem::F32(f32::from_bits(0x3D4CCCCC)))
    );
}

#[test]
fn re_05_color_typed_edits_reach_value_only_through_refresh() {
    let mut editor = ColorSequenceEditor::default();
    editor.set_value(&words(&[0, 0xFF0000, 100, 0x00FF00, 999])).unwrap();
    editor.init();
    editor.rows[1].position = "50".into();
    assert_eq!(editor.read(), words(&[0, 0xFF0000, 100, 0x00FF00]));
    assert!(editor.button_command(3, 0x2A0DBDEE, &mut Runtime).unwrap());
    assert_eq!(editor.read(), words(&[0, 0xFF0000, 50, 0x00FF00]));
    assert_eq!(editor.rows[0].value, "00ff0000");
}

struct TableRuntime;
impl ResourceTableHost for TableRuntime {
    fn pick_key(&mut self, request: KeyPickerRequest) -> KeyPickerResult {
        assert_eq!(request.initial, [0x1000, 0x6534284A, 0x2000]);
        KeyPickerResult {
            modal_result: 0,
            words: [9, 8, 7],
        }
    }
    fn property_name(&mut self, _: u32, _: &Variant) -> String {
        "Example".into()
    }
    fn property_value_text(&mut self, property: u32, value: &Variant) -> String {
        fc4_core::resource_editor::property_value_text(&Ingredients::default(), property, value, &mut Unknown).unwrap()
    }
}

#[test]
fn re_06_table_key_swap_and_getvalue_persists_without_save_table() {
    let mut editor = ResourceKeyTableEditor::new(
        16,
        TableDefaults {
            parameter: 1,
            key: words_key([0x1000, 0x6534284A, 0x2000]),
        },
    );
    editor.set_value(&words(&[0x1000, 0x6534284A, 0x2000])).unwrap();
    editor.init(&mut TableRuntime);
    assert_eq!(editor.keys[0].type_id, 0x6534284A);
    assert_eq!(editor.rows[0].debug_name, "Example (L)");
    editor.add_key(&mut TableRuntime);
    assert_eq!(editor.held.count(), 3);
    assert!(editor.dirty);
    assert_eq!(editor.read(), words(&[0x1000, 0x6534284A, 0x2000, 9, 8, 7]));
    assert!(editor.dirty);
    assert_eq!(editor.held.count(), 6);
}

#[test]
fn re_07_dialog_resize_supported_zero_append_and_unsupported_empty_result() {
    assert_eq!(
        resize_dialog_value(&words(&[0x10, 0x20]), true),
        words(&[0x10, 0x20, 0])
    );
    assert_eq!(
        resize_dialog_value(&words(&[0x10, 0x20, 0]), false),
        words(&[0x10, 0x20])
    );
    assert_eq!(
        resize_dialog_value(&Variant::array(mem::SINT64, vec![Elem::I64(7)]), true),
        Variant::Empty
    );
    assert_eq!(resize_dialog_value(&Variant::string(b"text"), true), Variant::Empty);
}
