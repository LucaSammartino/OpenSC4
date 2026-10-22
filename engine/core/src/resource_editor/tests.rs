// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::variant::{mem, Elem, Variant};

struct Numbers;
impl TextPrimitives for Numbers {
    fn english_float32(&mut self, text: &str) -> f32 { text.parse().unwrap_or(0.0) }
    fn atol32(&mut self, text: &str) -> i32 { decimal64(text) as i32 }
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
