// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::property::PropertyHolder;
use crate::stream::{RefValue, ResKey};
use crate::variant::{mem, Elem, Variant};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

struct Numbers;

#[test]
fn published_editor_class_ids_dispatch_only_to_supported_models() {
    for (class, ty) in [
        (0x671E972A, mem::RZ_CHAR),
        (0x490E01A7, mem::SINT64),
        (0xC71E972F, mem::UINT32),
        (0x471E9733, mem::SINT32),
        (0x471E9737, mem::SINT8),
        (0xC71E973C, mem::UINT8),
        (0xC71E9740, mem::FLOAT32),
        (0x47C35AFC, mem::BOOL),
    ] {
        assert_eq!(editor_kind(class), Some(EditorKind::Text(ty)));
    }
    assert_eq!(editor_kind(0x8739DFC1), Some(EditorKind::Combo(mem::UINT32)));
    assert_eq!(editor_kind(0x87C34EDA), Some(EditorKind::Combo(mem::UINT8)));
    assert_eq!(editor_kind(0x871E9744), Some(EditorKind::Spinner));
    assert_eq!(editor_kind(0x0A09AC46), Some(EditorKind::Grid));
    assert_eq!(editor_kind(0xCA0CB10F), Some(EditorKind::Colors));
    assert_eq!(editor_kind(0xA7847D6A), Some(EditorKind::ResourceKeyTable));
    for identity_only in [0x071E971B, 0xE8DDA013, 0x1A368ED8, 0x671E9725, 0xA73B81F0, 0x67336285] {
        assert_eq!(editor_kind(identity_only), None);
    }
}
impl TextPrimitives for Numbers {
    fn english_float32(&mut self, text: &str) -> f32 {
        text.parse().unwrap_or(0.0)
    }
    fn atol32(&mut self, text: &str) -> i32 {
        decimal64(text) as i32
    }
}

#[test]
fn ingredient_sections_are_processed_in_dependency_order() {
    let ini = Ingredients::parse("[Properties]\n16=UINT32,2,\"1 2\",Mask,\"Name, comma\",\"help\",Editor,Editor_7,Mask\n[Aliases]\n1=Mask\n42=Editor\n[ExemplarFilters]\n3=Mask 4", &mut Numbers);
    let p = &ini.properties[&16];
    assert_eq!(p.name, "Name, comma");
    assert_eq!(p.filter, 1);
    assert_eq!(p.editor_class, 42);
    assert_eq!(p.parameter.kind, 42);
    assert_eq!(p.parameter.argument, 7);
    assert_eq!(ini.exemplar_filters[&3], 5);
    assert_eq!(p.default, Variant::array(mem::UINT32, vec![Elem::U32(1), Elem::U32(2)]));
}

fn words(values: &[u32]) -> Variant {
    Variant::array(mem::UINT32, values.iter().copied().map(Elem::U32).collect())
}

fn ingredients() -> Ingredients {
    Ingredients::parse("[Properties]\n16=UINT32,0,\"\",1,\"A\",\"help A\",42,0,0\n32=UINT32,1,\"9\",2,\"B\",\"help B\",43,0,0\n48=UINT32,1,\"7\",1,\"\",\"help C\",44,0,0", &mut Numbers)
}

struct Formatting;
impl UnpublishedFormatting for Formatting {
    fn format(&mut self, _: u32, value: &Variant) -> Result<String, EditorError> {
        match value {
            Variant::Scalar(Elem::I64(n)) => Ok(n.to_string()),
            Variant::Scalar(Elem::F32(n)) => Ok(n.to_string()),
            Variant::Array { elem: mem::RZ_CHAR, .. } => Ok(String::from_utf8(value.bytes().unwrap()).unwrap()),
            _ => Err(EditorError::UnspecifiedFormatting(value.mem_type())),
        }
    }
}

#[test]
fn aliases_filters_duplicates_case_and_literal_space_separator() {
    let ini = Ingredients::parse("[Aliases]\n1= Mask \n2=mask\n4=Mask\n[ExemplarFilters]\n8=Mask mask 0x10 Unknown\n8=Mask\n[Properties]\n5=UINT8,1,\"1\",Mask,\"old\",\"h\",0,0,0\n5=UINT8,1,\"2\",mask,\"new\",\"h\",0,0,0", &mut Numbers);
    assert_eq!(ini.aliases["Mask"], 4);
    assert_eq!(ini.aliases["mask"], 2);
    assert_eq!(ini.filter("Mask mask 0x10 Unknown"), 22);
    assert_eq!(ini.filter("Mask\tmask"), 0);
    assert_eq!(ini.filter_word("-1"), 0);
    assert_eq!(ini.filter_word("16"), 16);
    assert_eq!(ini.exemplar_filters[&8], 4);
    assert_eq!(ini.properties[&5].name, "new");
    assert_eq!(ini.properties[&5].default, Variant::Scalar(Elem::U8(2)));
}

#[test]
fn every_type_name_and_default_kind_count_zero_and_parse_errors() {
    let cases = [
        (" BOOL ", mem::BOOL, "true", Elem::Bool(true)),
        ("uint8", mem::UINT8, "0xFF", Elem::U8(255)),
        ("SINT8", mem::SINT8, "-128", Elem::I8(-128)),
        ("UINT16", mem::UINT16, "65535", Elem::U16(65535)),
        ("SINT16", mem::SINT16, "-32768", Elem::I16(-32768)),
        ("UINT32", mem::UINT32, "0xFFFFFFFF", Elem::U32(u32::MAX)),
        ("SINT32", mem::SINT32, "-2147483648", Elem::I32(i32::MIN)),
        ("SINT64", mem::SINT64, "9223372036854775807", Elem::I64(i64::MAX)),
        ("FLOAT32", mem::FLOAT32, "1.5", Elem::F32(1.5)),
    ];
    for (name, ty, input, elem) in cases {
        assert_eq!(variant_type(name), ty);
        assert_eq!(
            default_value(ty, 1, input, &mut Numbers).unwrap(),
            Variant::Scalar(elem)
        );
        assert_eq!(
            default_value(ty, 0, "garbage ignored", &mut Numbers).unwrap(),
            words(&[])
        );
    }
    assert_eq!(variant_type("STRING"), 0x800E);
    for name in ["", "UINT64", "FLOAT64", "unknown"] {
        assert_eq!(variant_type(name), 0);
    }
    assert_eq!(
        default_value(0x800E, 1, "hello, world", &mut Numbers).unwrap(),
        Variant::string(b"hello, world")
    );
    for (ty, count, text) in [
        (mem::UINT8, 1, "256"),
        (mem::SINT8, 1, "128"),
        (mem::BOOL, 1, "maybe"),
        (mem::UINT32, 2, "1"),
        (mem::UINT32, 1, "1 2"),
        (mem::FLOAT32, 1, "no"),
        (0x800E, 2, "s"),
    ] {
        assert_eq!(
            default_value(ty, count, text, &mut Numbers),
            Err(EditorError::InvalidValue)
        );
    }
}

#[test]
fn property_drop_rules_parameters_and_swapped_table_defaults() {
    let ini = Ingredients::parse("[Aliases]\n42=Editor\n5=Mode\n[Properties]\n1=STRING,0,\"\",1,\"n\",\"h\",Editor,0,0\n2=BAD,1,\"1\",1,\"n\",\"h\",Editor,0,0\n3=UINT8,1,\"256\",1,\"n\",\"h\",Editor,0,0\n4=FLOAT32,0,\"ignored\",1,\"n\",\"h\",Editor,\"literal, comma\", Mode \n5=UINT32,1,\"1\",1,\"n\",\"h\",Editor,Mode,0\n6=UINT32,1,\"1\",1,\"n\",\"h\",Editor,Mode_123,0\n7=UINT32,1,\"1\",1,\"n\",\"h\",Editor,Mode_123_bad,0\n[ResKeyTableEditorDefaults]\n0x12345678=0x1 0x2 0x3 0x4\n8=1 2\n9=", &mut Numbers);
    assert_eq!(ini.properties.keys().copied().collect::<Vec<_>>(), vec![4, 5, 6, 7]);
    assert_eq!(ini.properties[&4].default, words(&[]));
    assert_eq!(
        ini.properties[&4].parameter,
        EditorParameter {
            kind: STRING_PARAMETER,
            text: "literal, comma".into(),
            argument: 0
        }
    );
    assert_eq!(ini.properties[&4].last, 5);
    assert_eq!(ini.properties[&5].parameter.argument, 5);
    assert_eq!(ini.properties[&6].parameter.argument, 123);
    assert_eq!(ini.properties[&7].parameter.kind, 0);
    assert_eq!(
        ini.table_defaults[&0x12345678],
        TableDefaults {
            parameter: 1,
            key: ResKey {
                type_id: 3,
                group: 2,
                instance: 4
            }
        }
    );
    assert_eq!(ini.table_defaults.len(), 1);
    let mut fields = Fields("\"a,b\",  tail");
    assert_eq!(fields.read_field().unwrap(), ("a,b".into(), true));
    assert_eq!(fields.read_field().unwrap(), ("tail".into(), false));
    assert_eq!(Fields("\"broken").read_field(), Err(EditorError::InvalidField));
}

#[test]
fn ingredient_cache_is_process_explicit_and_loads_only_once() {
    let mut cache = IngredientCache::default();
    assert_eq!(cache.load_once("[Aliases]\n1=A", &mut Numbers).aliases["A"], 1);
    assert_eq!(cache.load_once("[Aliases]\n2=A", &mut Numbers).aliases["A"], 1);
}

#[test]
fn list_formatting_names_and_explicit_formatter_gaps() {
    let ini = ingredients();
    assert_eq!(ini.name(16), "A");
    assert_eq!(ini.name(48), "0x00000030");
    assert_eq!(ini.name(0xDEAD), "0x0000DEAD");
    assert_eq!(
        property_value_text(&ini, 16, &words(&[]), &mut Formatting).unwrap(),
        "<empty list>"
    );
    assert_eq!(
        property_value_text(
            &ini,
            16,
            &Variant::array(mem::BOOL, vec![Elem::Bool(true), Elem::Bool(false)]),
            &mut Formatting
        )
        .unwrap(),
        "True, False"
    );
    assert_eq!(
        property_value_text(&ini, 16, &words(&[1, u32::MAX]), &mut Formatting).unwrap(),
        "0x00000001, 0xFFFFFFFF"
    );
    for (elem, expected) in [
        (Elem::U8(3), "0x03"),
        (Elem::I8(-1), "0xFF"),
        (Elem::U16(3), "0x0003"),
        (Elem::I16(-1), "0xFFFF"),
        (Elem::I32(-1), "0xFFFFFFFF"),
    ] {
        assert_eq!(
            property_value_text(&ini, 16, &Variant::Scalar(elem), &mut Formatting).unwrap(),
            expected
        );
    }
    assert!(property_value_text(&ini, 16, &Variant::Object(RefValue::Null), &mut Formatting).is_err());
}

#[test]
fn typed_text_layout_lowercase_bool_init_once_and_sint64_scalar_list_difference() {
    let ini = ingredients();
    let cases = [
        (mem::UINT8, Elem::U8(1), 0x5A293904),
        (mem::SINT8, Elem::I8(1), 0x5A293904),
        (mem::UINT32, Elem::U32(1), 0x5A293904),
        (mem::SINT32, Elem::I32(1), 0x5A293904),
        (mem::BOOL, Elem::Bool(true), 0x67C35A67),
        (mem::SINT64, Elem::I64(1), 0x490E039A),
    ];
    for (ty, elem, base) in cases {
        let mut editor = TypedTextEditor::new(ty, Variant::array(ty, vec![elem, elem]));
        editor.flags = 0x2800;
        editor.init(&ini, 16, &mut Formatting).unwrap();
        assert_eq!((editor.inputs[0].child_id, editor.inputs[1].child_id), (base, base + 1));
        assert_eq!((editor.inputs[1].x, editor.inputs[1].y), (5, 25));
        assert_eq!(editor.flags, 0x12000);
        assert!(editor.background_fill);
        if ty == mem::BOOL {
            assert_eq!(editor.inputs[0].text, "true");
        }
        editor.inputs[0].text = "changed".into();
        editor.init(&ini, 16, &mut Formatting).unwrap();
        assert_eq!(editor.inputs[0].text, "changed");
        if ty != mem::SINT64 {
            assert_eq!(editor.read(&mut Numbers), Err(EditorError::UnsupportedType(ty)));
        }
    }
    let mut scalar = TypedTextEditor::new(mem::SINT64, Variant::Scalar(Elem::I64(0)));
    scalar.init(&ini, 16, &mut Formatting).unwrap();
    scalar.inputs[0].text = "4294967297suffix".into();
    assert_eq!(scalar.read(&mut Numbers).unwrap(), Variant::Scalar(Elem::I64(1)));
    let mut list = TypedTextEditor::new(mem::SINT64, Variant::array(mem::SINT64, vec![Elem::I64(0)]));
    list.init(&ini, 16, &mut Formatting).unwrap();
    list.inputs[0].text = "4294967297suffix".into();
    assert_eq!(
        list.read(&mut Numbers).unwrap(),
        Variant::array(mem::SINT64, vec![Elem::I64(4294967297)])
    );
    assert_eq!(decimal64(" -9223372036854775808 trailing"), i64::MIN);
    assert_eq!(decimal64("0x10"), 0);
    assert_eq!(decimal64("18446744073709551617"), 1);
    let mut string = TypedTextEditor::new(mem::RZ_CHAR, Variant::string(b"abc"));
    string.init(&ini, 16, &mut Formatting).unwrap();
    assert_eq!(string.inputs.len(), 1);
    assert_eq!(string.inputs[0].child_id, 0xF9E2E9E4);
    let mut float = TypedTextEditor::new(mem::FLOAT32, Variant::Scalar(Elem::F32(1.5)));
    float.init(&ini, 16, &mut Formatting).unwrap();
    assert_eq!(float.inputs[0].child_id, 0xDA4FB6F6);
    let mut empty = TypedTextEditor::new(mem::UINT8, Variant::array(mem::UINT8, vec![]));
    empty.init(&ini, 16, &mut Formatting).unwrap();
    assert!(empty.inputs.is_empty());
}

#[test]
fn both_combos_sort_map_match_fallback_and_preserve_scalar_or_list() {
    let mut ini = ingredients();
    ini.uint32_maps
        .insert(16, [(5, "High".into()), (1, "Low".into()), (2, "Mid".into())].into());
    let mut combo = ComboEditor::new(mem::UINT32, Variant::Scalar(Elem::U32(5))).unwrap();
    combo.init(&ini, 16, 200, &mut Formatting).unwrap();
    assert_eq!(combo.index_values, vec![1, 2, 5]);
    let c = combo.controls[0].as_mut().unwrap();
    assert_eq!(c.items, vec!["Low", "Mid", "High"]);
    assert_eq!(c.selected, 2);
    assert_eq!(c.child_id, 0x46C60DBF);
    assert_eq!(
        c.area,
        Area {
            left: 5,
            top: 5,
            right: 205,
            bottom: 23
        }
    );
    assert_eq!((c.text_color, c.second_color, c.background), (0, 0x80, Some(0xE0E0E0)));
    c.selected = 0;
    assert_eq!(combo.read().unwrap(), Variant::Scalar(Elem::U32(1)));
    combo.init(&Ingredients::default(), 0, 1, &mut Formatting).unwrap();
    assert_eq!(combo.index_values, vec![1, 2, 5]);
    ini.uint8_maps
        .insert(16, [(2, "Two".into()), (8, "Eight".into())].into());
    let mut byte = ComboEditor::new(mem::UINT8, Variant::array(mem::UINT8, vec![Elem::U8(7), Elem::U8(8)])).unwrap();
    byte.init(&ini, 16, 100, &mut Formatting).unwrap();
    assert_eq!(byte.controls[0].as_ref().unwrap().selected, 0);
    assert_eq!(byte.controls[1].as_ref().unwrap().child_id, 0xC7C34B1B);
    assert_eq!(byte.controls[1].as_ref().unwrap().background, None);
    assert_eq!(
        byte.read().unwrap(),
        Variant::array(mem::UINT8, vec![Elem::U8(2), Elem::U8(8)])
    );
    byte.controls[1] = None;
    assert_eq!(byte.read(), Err(EditorError::MissingControl(1)));
}

#[test]
fn uint8_combo_wraps_indices_and_reports_native_nontermination_and_invalid_indices() {
    let mut ini = Ingredients::default();
    // Test item-index width independently of the unsettled map-key narrowing.
    assert_eq!(combo_selection_index(mem::UINT8, 256), 0);
    assert_eq!(combo_selection_index(mem::UINT8, 511), 255);
    assert_eq!(combo_selection_index(mem::UINT32, 511), 511);
    ini.uint8_maps
        .insert(16, (0..257).map(|n| (n, n.to_string())).collect());
    let mut combo = ComboEditor::new(mem::UINT8, Variant::Scalar(Elem::U8(0))).unwrap();
    combo.init(&ini, 16, 10, &mut Formatting).unwrap();
    assert_eq!(combo.controls[0].as_ref().unwrap().selected, 0);
    combo.controls[0].as_mut().unwrap().selected = 999;
    assert_eq!(combo.read(), Err(EditorError::InvalidSelection(999)));
    let mut large = ComboEditor::new(mem::UINT8, Variant::array(mem::UINT8, vec![Elem::U8(0); 256])).unwrap();
    large.init(&ini, 16, 10, &mut Formatting).unwrap();
    assert_eq!(large.read(), Err(EditorError::NonterminatingUint8List));
    let mut no_map = ComboEditor::new(mem::UINT32, Variant::Scalar(Elem::U32(1))).unwrap();
    no_map.init(&Ingredients::default(), 16, 10, &mut Formatting).unwrap();
    assert!(no_map.controls[0].as_ref().unwrap().items.is_empty());
    assert_eq!(no_map.read(), Err(EditorError::InvalidSelection(0)));
}

struct SpinnerHost(Vec<(u32, u32, u32)>);
impl SpinnerPrimitives for SpinnerHost {
    fn initial_value(&mut self, requested: u32, min: u32, max: u32) -> u32 {
        self.0.push((requested, min, max));
        requested.clamp(min, max)
    }
}

#[test]
fn spinner_widget_receives_original_uint32_and_range_user_clamps_missing_scalar_preserved() {
    let mut host = SpinnerHost(vec![]);
    let mut editor = SpinnerEditor::new(Variant::Scalar(Elem::U32(1000)));
    editor.init(&mut host);
    assert_eq!(host.0, vec![(1000, 0, 255)]);
    assert_eq!(editor.read().unwrap(), Variant::Scalar(Elem::U32(255)));
    let c = editor.controls[0].as_mut().unwrap();
    assert_eq!(c.child_id, 0xBA293DA4);
    assert_eq!(
        c.area,
        Area {
            left: 5,
            top: 5,
            right: 53,
            bottom: 25
        }
    );
    c.set_user_value(u32::MAX);
    assert_eq!(c.value, 255);
    c.set_user_value(0);
    assert_eq!(editor.read().unwrap(), Variant::Scalar(Elem::U32(0)));
    editor.controls[0] = None;
    assert_eq!(editor.read().unwrap(), Variant::Scalar(Elem::U32(1000)));
    editor.init(&mut host);
    assert_eq!(host.0.len(), 1);
    let mut list = SpinnerEditor::new(words(&[1, 2]));
    list.init(&mut host);
    assert_eq!(list.read().unwrap(), words(&[1, 2]));
    list.controls[1] = None;
    assert_eq!(list.read(), Err(EditorError::MissingControl(1)));
}

struct ThreeDigits;
impl GridFormatting for ThreeDigits {
    fn three_significant_digits(&mut self, value: f32) -> String {
        // Independent published examples; this double is a runtime primitive,
        // not a second implementation of the grid's row/readback algorithm.
        match value.to_bits() {
            n if n == 1234.5f32.to_bits() => "1.23e+03".into(),
            n if n == 0.33333f32.to_bits() => "0.333".into(),
            _ => value.to_string(),
        }
    }
}

#[test]
fn grid_floors_rows_truncates_integer_widths_and_retains_tail() {
    let value = Variant::array(mem::UINT8, (0..35).map(Elem::U8).collect());
    let mut grid = GridEditor::new(value);
    grid.init(320, 240, &mut ThreeDigits);
    assert_eq!(grid.rows, 2);
    assert_eq!(grid.cells.len(), 32);
    assert_eq!(grid.row_headers, vec!["0", "1"]);
    assert_eq!(grid.column_headers[15], "15");
    assert_eq!(
        grid.area,
        Area {
            left: 5,
            top: 5,
            right: 315,
            bottom: 217
        }
    );
    grid.cells[16] = Some("300".into());
    let result = grid.read(&mut Numbers).unwrap().items();
    assert_eq!(result[16], Elem::U8(44));
    assert_eq!(&result[32..], &[Elem::U8(32), Elem::U8(33), Elem::U8(34)]);
    let mut signed = GridEditor::new(Variant::array(mem::SINT16, vec![Elem::I16(-1); 16]));
    signed.init(10, 10, &mut ThreeDigits);
    assert_eq!(signed.cells[0].as_deref(), Some("-1"));
    signed.cells[0] = Some("65535suffix".into());
    signed.cells[1] = Some("32768".into());
    let result = signed.read(&mut Numbers).unwrap().items();
    assert_eq!(&result[..2], &[Elem::I16(-1), Elem::I16(-32768)]);
    grid.cells[0] = None;
    assert_eq!(grid.read(&mut Numbers), Err(EditorError::MissingControl(0)));
}

#[test]
fn grid_float_confirmation_rounds_every_visible_cell_only_and_other_types_are_untouched() {
    let mut values = vec![Elem::F32(1234.5); 17];
    values[1] = Elem::F32(0.33333);
    let mut grid = GridEditor::new(Variant::array(mem::FLOAT32, values));
    grid.init(100, 100, &mut ThreeDigits);
    assert_eq!(grid.cells[0].as_deref(), Some("1.23e+03"));
    let result = grid.read(&mut Numbers).unwrap().items();
    assert_eq!(result[0], Elem::F32(1230.0));
    assert_eq!(result[1], Elem::F32(0.333));
    assert_eq!(result[16], Elem::F32(1234.5));
    let original = Variant::array(mem::BOOL, vec![Elem::Bool(true); 16]);
    let mut grid = GridEditor::new(original.clone());
    grid.init(100, 100, &mut ThreeDigits);
    assert_eq!(grid.cells, vec![None; 16]);
    assert_eq!(grid.read(&mut Numbers).unwrap(), original);
}

#[derive(Default)]
struct FakeEditors {
    values: BTreeMap<usize, Variant>,
    areas: BTreeMap<usize, Area>,
    events: Vec<String>,
    fail_classes: BTreeSet<u32>,
    next: usize,
    end_result: bool,
}
impl EditorHost for FakeEditors {
    type Editor = usize;
    fn create_editor(&mut self, class: u32) -> Option<usize> {
        self.events.push(format!("create:{class}"));
        if self.fail_classes.contains(&class) {
            return None;
        }
        self.next += 1;
        Some(self.next)
    }
    fn resolve_reference(&mut self, value: &RefValue) -> Option<RefValue> {
        self.events.push("resolve".into());
        match value {
            RefValue::Null => None,
            _ => Some(value.clone()),
        }
    }
    fn table_defaults_interface(&mut self, _: usize) -> bool {
        self.events.push("query-table".into());
        true
    }
    fn table_parameter(&mut self, _: usize, value: u32) {
        self.events.push(format!("parameter:{value}"));
    }
    fn table_default(&mut self, _: usize, key: ResKey) {
        self.events.push(format!("key:{:?}", key_words(key)));
    }
    fn initialize(&mut self, editor: usize, property: u32, value: &Variant) {
        self.events.push(format!("init:{editor}:{property}"));
        self.values.insert(editor, value.clone());
    }
    fn value(&mut self, editor: usize) -> Variant {
        self.events.push(format!("read:{editor}"));
        self.values[&editor].clone()
    }
    fn show(&mut self, editor: usize, visible: bool) {
        self.events.push(format!("show:{editor}:{visible}"));
    }
    fn area(&mut self, editor: usize) -> Area {
        self.areas[&editor]
    }
    fn set_area(&mut self, editor: usize, area: Area) {
        self.areas.insert(editor, area);
    }
    fn child_add(&mut self, editor: usize) {
        self.events.push(format!("add:{editor}"));
    }
    fn child_remove(&mut self, editor: usize) {
        self.events.push(format!("remove:{editor}"));
    }
    fn end_dialog(&mut self, modal: bool, result: i32) -> bool {
        self.events.push(format!("end:{modal}:{result}"));
        self.end_result
    }
}

#[derive(Default)]
struct Target {
    id: u32,
    value: Option<Variant>,
    writes: Vec<&'static str>,
}
impl PropertyTarget for Target {
    fn set_id(&mut self, id: u32) {
        self.id = id;
        self.writes.push("id");
    }
    fn set_value(&mut self, value: Variant) {
        self.value = Some(value);
        self.writes.push("value");
    }
}

#[test]
fn property_view_filters_factory_failures_selection_help_done_order_cancel_and_invalid_rows() {
    let ini = ingredients();
    let mut source = PropertyHolder::new();
    source.add(16, words(&[77]));
    let mut host = FakeEditors {
        end_result: true,
        ..Default::default()
    };
    host.fail_classes.insert(44);
    let mut view = PropertyView::new(1);
    view.flags = 0x1800;
    view.init(
        &ini,
        &source,
        Area {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        },
        &mut host,
    );
    assert_eq!(
        view.area,
        Area {
            left: 200,
            top: 75,
            right: 600,
            bottom: 525
        }
    );
    assert_eq!(view.flags, 0x11000);
    assert_eq!(view.editors.keys().copied().collect::<Vec<_>>(), vec![16]);
    assert_eq!(matching_properties(&ini, 1), vec![16, 48]);
    let editor = view.editors[&16];
    // Literal §6 fallback uses default for non-reference exemplar values (RE5).
    assert_eq!(host.values[&editor], words(&[]));
    host.values.insert(editor, words(&[99]));
    view.rows = vec![16, 48];
    let mut target = Target::default();
    assert!(!view.command(SELECTION_MESSAGE, PROPERTY_LIST, 0, &ini, &mut target, &mut host));
    assert_eq!(view.help, "help A");
    assert!(target.writes.is_empty());
    view.command(SELECTION_MESSAGE, PROPERTY_LIST, 1, &ini, &mut target, &mut host);
    assert_eq!(view.current, None);
    assert_eq!(view.help, "help C");
    assert!(host.events.contains(&format!("show:{editor}:false")));
    let events = host.events.len();
    view.command(SELECTION_MESSAGE, PROPERTY_LIST, 20, &ini, &mut target, &mut host);
    assert_eq!(host.events.len(), events);
    assert_eq!(view.help, "help C");
    assert!(view.command(BUTTON_MESSAGE, PROPERTY_DONE, 0, &ini, &mut target, &mut host));
    assert_eq!(target.id, 16);
    assert_eq!(target.value, Some(words(&[99])));
    assert_eq!(target.writes, vec!["id", "value"]);
    assert_eq!(host.events.last().unwrap(), "end:true:0");
    let writes = target.writes.len();
    view.command(BUTTON_MESSAGE, PROPERTY_CANCEL, 0, &ini, &mut target, &mut host);
    assert_eq!(target.writes.len(), writes);
    assert_eq!(host.events.last().unwrap(), "end:true:1");
    view.command(BUTTON_MESSAGE, PROPERTY_DONE, 1, &ini, &mut target, &mut host);
    assert_eq!(target.writes.len(), writes);
    assert!(!view.command(0, PROPERTY_DONE, 0, &ini, &mut target, &mut host));
}

#[test]
fn editor_factory_value_resolution_and_table_configuration_order() {
    let mut ini = ingredients();
    let definition = ini.properties.get_mut(&16).unwrap();
    definition.editor_class = RESOURCE_TABLE_CLASS;
    ini.table_defaults.insert(
        16,
        TableDefaults {
            parameter: 1,
            key: words_key([2, 3, 4]),
        },
    );
    let mut host = FakeEditors::default();
    let editor = get_value_editor(&ini, 16, Some(&words(&[7, 8, 9])), None, &mut host).unwrap();
    assert_eq!(host.values[&editor], words(&[7, 8, 9]));
    assert_eq!(
        host.events,
        vec![
            format!("create:{RESOURCE_TABLE_CLASS}"),
            "query-table".into(),
            "parameter:1".into(),
            "key:[2, 3, 4]".into(),
            "init:1:16".into()
        ]
    );
    let value = Variant::Object(RefValue::Host { token: 7, class_id: 8 });
    let editor = get_value_editor(&ini, 16, None, Some(&value), &mut host).unwrap();
    assert_eq!(host.values[&editor], value);
    assert!(host.events.contains(&"resolve".into()));
    assert!(get_value_editor(&ini, 16, Some(&Variant::Object(RefValue::Null)), None, &mut host).is_none());
    let calls = host.events.len();
    assert!(get_value_editor(&ini, 0xDEAD, None, None, &mut host).is_none());
    assert_eq!(host.events.len(), calls);
}

struct Metrics;
impl ButtonMetrics for Metrics {
    fn button_size(&mut self, _: &str, _: u32) -> (i32, i32) {
        (60, 20)
    }
}

#[test]
fn edit_dialog_resize_rebuild_retains_area_and_only_okay_writes_value() {
    let ini = ingredients();
    let mut host = FakeEditors {
        end_result: true,
        ..Default::default()
    };
    let mut dialog = EditValueDialog::new(
        16,
        Area {
            left: 0,
            top: 0,
            right: 400,
            bottom: 200,
        },
    );
    dialog.init(&ini, &words(&[0x10, 0x20]), &mut Metrics, &mut host);
    assert_eq!(
        dialog.buttons.iter().map(|b| b.id).collect::<Vec<_>>(),
        vec![EDIT_OKAY, EDIT_CANCEL, EDIT_ADD, EDIT_REMOVE]
    );
    assert_eq!(
        dialog.buttons[0].area,
        Area {
            left: 335,
            top: 175,
            right: 395,
            bottom: 195
        }
    );
    assert_eq!(dialog.help_area.right, 140);
    let area = host.areas[&dialog.editor.unwrap()];
    assert_eq!(
        area,
        Area {
            left: 5,
            top: 5,
            right: 395,
            bottom: 175
        }
    );
    let mut target = Target::default();
    dialog.command(BUTTON_MESSAGE, EDIT_ADD, &ini, &mut target, &mut host);
    assert_eq!(host.values[&dialog.editor.unwrap()], words(&[0x10, 0x20, 0]));
    assert_eq!(host.areas[&dialog.editor.unwrap()], area);
    assert!(host.events.contains(&"remove:1".into()));
    for _ in 0..2 {
        dialog.command(BUTTON_MESSAGE, EDIT_REMOVE, &ini, &mut target, &mut host);
    }
    assert_eq!(host.values[&dialog.editor.unwrap()], words(&[0x10]));
    let created = host.next;
    dialog.command(BUTTON_MESSAGE, EDIT_REMOVE, &ini, &mut target, &mut host);
    assert_eq!(host.next, created);
    assert!(target.writes.is_empty());
    assert!(dialog.command(BUTTON_MESSAGE, EDIT_OKAY, &ini, &mut target, &mut host));
    assert_eq!(target.writes, vec!["value"]);
    assert_eq!(target.value, Some(words(&[0x10])));
    assert_eq!(host.events.last().unwrap(), "end:true:1");
    dialog.command(BUTTON_MESSAGE, EDIT_CANCEL, &ini, &mut target, &mut host);
    assert_eq!(target.writes.len(), 1);
    assert_eq!(host.events.last().unwrap(), "end:true:0");
    assert!(!dialog.command(3, EDIT_ADD, &ini, &mut target, &mut host));
    assert!(dialog.command(BUTTON_MESSAGE, 0xDEAD, &ini, &mut target, &mut host));
}

#[test]
fn dialog_all_supported_resize_types_unsupported_empty_variant_and_no_editor_paths() {
    for elem in [
        Elem::Bool(true),
        Elem::U8(9),
        Elem::I8(-9),
        Elem::U16(9),
        Elem::I16(-9),
        Elem::U32(9),
        Elem::I32(-9),
        Elem::F32(9.0),
    ] {
        let value = Variant::Scalar(elem);
        let resized = resize_dialog_value(&value, true);
        assert_eq!(resized.count(), 2);
        assert_eq!(resized.items()[0], elem);
        assert_eq!(resized.items()[1].as_i64(), 0);
        assert_eq!(
            resize_dialog_value(&resized, false),
            Variant::array(elem.mem_type(), vec![elem])
        );
    }
    assert_eq!(
        resize_dialog_value(&Variant::array(mem::SINT64, vec![Elem::I64(7), Elem::I64(8)]), true),
        Variant::Empty
    );
    assert_eq!(resize_dialog_value(&Variant::string(b"a"), true), Variant::Empty);
    let mut ini = ingredients();
    assert!(!ini.variable_length(32));
    ini.properties.get_mut(&16).unwrap().editor_class = RESPONSE_CURVE_CLASS;
    assert!(!ini.variable_length(16));
    let mut host = FakeEditors::default();
    let mut dialog = EditValueDialog::new(0xDEAD, Area::default());
    dialog.init(&ini, &words(&[]), &mut Metrics, &mut host);
    assert_eq!(dialog.buttons.len(), 2);
    let mut target = Target::default();
    assert!(!dialog.command(BUTTON_MESSAGE, EDIT_OKAY, &ini, &mut target, &mut host));
    assert!(target.writes.is_empty());
    assert_eq!(host.events.last().unwrap(), "end:true:0");
}

struct Picker(u32);
impl ColorPicker for Picker {
    fn pick_color(&mut self, current: u32) -> u32 {
        self.0 ^= current;
        self.0
    }
}

#[test]
fn colors_drop_odd_word_ignore_unrefreshed_text_sort_signed_positions_and_hex_colors() {
    let mut colors = ColorSequenceEditor::default();
    colors.set_value(&words(&[0, 0xAAFF0000, 100, 0x00FF00, 777])).unwrap();
    colors.init();
    assert_eq!(colors.pairs.len(), 2);
    assert_eq!(colors.rows[0].value, "aaff0000");
    assert_eq!(colors.rows[0].swatch, 0xFF0000);
    colors.rows[1].position = "-5 suffix".into();
    colors.rows[1].value = "-1".into();
    assert_eq!(colors.read(), words(&[0, 0xAAFF0000, 100, 0x00FF00]));
    colors.refresh(&mut Numbers);
    assert_eq!(colors.read(), words(&[u32::MAX - 4, u32::MAX, 0, 0xAAFF0000]));
    assert_eq!(colors.rows[0].position, "-5");
    assert_eq!(colors.rows[0].value, "ffffffff");
    colors.rows[0].position = "50".into();
    colors.rows[0].value = "0x123456789".into();
    colors.refresh(&mut Numbers);
    assert_eq!(colors.pairs[1].position, 50);
    assert_eq!(colors.pairs[1].color, u32::MAX);
}

#[test]
fn colors_add_clone_no_selection_delete_floor_activation_and_inset_paint() {
    let mut colors = ColorSequenceEditor::default();
    assert_eq!(colors.add(), Err(EditorError::OutOfBoundsNativeRead));
    colors.set_value(&words(&[10, 0x112233, 20, 0x445566])).unwrap();
    colors.init();
    colors.add().unwrap();
    assert_eq!(colors.read(), words(&[10, 0x112233, 20, 0x445566, 20, 0x445566]));
    colors.selected = Some(0);
    colors.rows[0].position = "uncommitted".into();
    colors.add().unwrap();
    assert_eq!(colors.rows[0].position, "uncommitted");
    assert_eq!(colors.pairs[1], colors.pairs[0]);
    colors.delete();
    colors.delete();
    colors.delete();
    colors.delete();
    assert_eq!(colors.pairs.len(), 1);
    let original = colors.read();
    colors.activate(0, 2, &mut Picker(0));
    assert_eq!(colors.read(), original);
    let area = Area {
        left: 10,
        top: 20,
        right: 40,
        bottom: 50,
    };
    assert_eq!(
        colors.paint(0, 2, area),
        Some((
            Area {
                left: 11,
                top: 21,
                right: 39,
                bottom: 49
            },
            0x445566
        ))
    );
    assert_eq!(colors.paint(0, 1, area), None);
    colors.add().unwrap();
    colors.activate(0, 2, &mut Picker(0xFFFFFF));
    assert_eq!(colors.pairs[0].color, 0xBBAA99);
}

#[derive(Default)]
struct TableHost {
    picks: VecDeque<KeyPickerResult>,
    requests: Vec<KeyPickerRequest>,
    row_calls: Vec<(u32, Variant)>,
}
impl ResourceTableHost for TableHost {
    fn pick_key(&mut self, request: KeyPickerRequest) -> KeyPickerResult {
        self.requests.push(request);
        self.picks.pop_front().unwrap()
    }
    fn property_name(&mut self, property: u32, value: &Variant) -> String {
        self.row_calls.push((property, value.clone()));
        "Name".into()
    }
    fn property_value_text(&mut self, _: u32, value: &Variant) -> String {
        value
            .items()
            .iter()
            .map(|v| format!("0x{:08X}", v.as_i64() as u32))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
fn table() -> ResourceKeyTableEditor {
    ResourceKeyTableEditor::new(
        16,
        TableDefaults {
            parameter: 1,
            key: words_key([2, 3, 4]),
        },
    )
}

#[test]
fn table_swaps_words_preserves_order_duplicates_readonly_and_rejects_incomplete_triples() {
    let mut table = table();
    let value = words(&[0x1000, 0x6534284A, 0x2000, 0x1000, 0x6534284A, 0x2000]);
    table.set_value(&value).unwrap();
    assert_eq!(
        table.keys,
        vec![
            ResKey {
                group: 0x1000,
                type_id: 0x6534284A,
                instance: 0x2000
            };
            2
        ]
    );
    table.read_only = true;
    let mut host = TableHost::default();
    table.init(&mut host);
    assert!(table.buttons.is_empty());
    assert_eq!(table.rows[0].debug_name, "Name (L)");
    assert_eq!(table.rows[0].resource_key, "0x00001000, 0x6534284A, 0x00002000");
    assert_eq!(host.row_calls[0], (16, words(&[0x1000, 0x6534284A, 0x2000])));
    assert_eq!(table.read(), value);
    assert_eq!(
        table.set_value(&words(&[1, 2])),
        Err(EditorError::OutOfBoundsNativeRead)
    );
    assert_eq!(table.read(), value);
    assert_eq!(
        table.set_value(&Variant::Scalar(Elem::U32(1))),
        Err(EditorError::UnsupportedType(mem::UINT32))
    );
}

#[test]
fn table_picker_accept_cancel_noop_edit_invalid_delete_save_getvalue_and_revert() {
    let mut table = table();
    table.set_value(&words(&[10, 20, 30])).unwrap();
    let mut host = TableHost::default();
    table.init(&mut host);
    assert!(!table.dirty);
    assert_eq!(table.buttons, vec![TABLE_SAVE, TABLE_ADD, TABLE_EDIT, TABLE_DELETE]);
    host.picks.push_back(KeyPickerResult {
        modal_result: 1,
        words: [90, 80, 70],
    });
    table.add_key(&mut host);
    assert_eq!(table.keys.len(), 1);
    assert!(!table.dirty);
    assert_eq!(
        host.requests[0],
        KeyPickerRequest {
            initial: [2, 3, 4],
            width: 575,
            height: 475,
            centered_on_main_window: true
        }
    );
    host.picks.push_back(KeyPickerResult {
        modal_result: 0,
        words: [40, 50, 60],
    });
    table.add_key(&mut host);
    assert!(table.dirty && table.save_enabled);
    assert_eq!(table.held, words(&[10, 20, 30]));
    table.revert().unwrap();
    assert_eq!(table.keys.len(), 1);
    assert!(!table.dirty && !table.save_enabled);
    host.picks.push_back(KeyPickerResult {
        modal_result: 0,
        words: [10, 20, 30],
    });
    table.edit_key(Some(0), &mut host);
    assert!(!table.dirty);
    assert_eq!(host.requests.last().unwrap().initial, [10, 20, 30]);
    let rows_before = host.row_calls.len();
    table.edit_key(Some(99), &mut host);
    table.delete_key(None, &mut host);
    assert_eq!(host.row_calls.len(), rows_before);
    table.delete_key(Some(99), &mut host);
    assert!(table.dirty && table.save_enabled);
    assert_eq!(table.keys.len(), 1);
    table.save_table(&mut host);
    assert!(!table.dirty);
    host.picks.push_back(KeyPickerResult {
        modal_result: 0,
        words: [4, 5, 6],
    });
    table.add_key(&mut host);
    // GetValue, without Save Table, commits current keys to held value too.
    assert_eq!(table.read(), words(&[10, 20, 30, 4, 5, 6]));
    assert!(table.dirty);
    table.delete_key(Some(0), &mut host);
    table.revert().unwrap();
    assert_eq!(table.keys.len(), 2);
    assert!(!table.dirty);
    assert!(!table.command(3, TABLE_SAVE, None, &mut host));
    assert!(!table.command(BUTTON_MESSAGE, 0xDEAD, None, &mut host));
}
