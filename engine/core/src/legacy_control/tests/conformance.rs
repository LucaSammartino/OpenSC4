// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordered recording fixtures for legacy-control §14, LCV01–LCV33.
use super::*;

fn passes(kind: Kind, props: &[Property]) -> Fake {
    let mut f=Fake::default();
    f.queries.insert((10,kind.iid()),11);
    f.raw.insert((11,0x140),n(0));
    interpret(kind,10,Properties(props),true,&mut f).unwrap();
    interpret(kind,10,Properties(props),false,&mut f).unwrap();
    f
}
fn widgets(f: &Fake)->Vec<Event> {
    f.events.iter().filter(|e|matches!(e,Event::Widget(..))).cloned().collect()
}
fn has(f: &Fake,slot:u32,args:Vec<Value>) {assert!(f.events.contains(&call(11,slot,args)),"missing slot {slot:x}: {:?}",f.events);}

#[test]
fn lcv01_03_layout_and_com() {
    assert_eq!(OBJECT_SIZE,32);assert_eq!(FIELD_OFFSETS,[12,16,20,24,28]);
    assert_eq!(VTABLES,[(0,0xad5360),(4,0xad5344),(8,0xad5324)]);
    let (mut c,mut g,mut f)=initialized();
    assert_eq!((c.ref_count(),c.control(),c.pending_parent(),c.procedure(),c.owns_control()),(0,None,None,None,false));
    for (iid,view) in [(1,Interface::Primary),(0x1a356083,Interface::Primary),(0x5a356159,Interface::Area),(LEGACY_IID,Interface::Legacy)] {
        let mut out=None;assert!(c.query_interface(iid,&mut out));assert_eq!(out,Some(view));
    }
    let mut out=Some(Interface::Area);assert!(!c.query_interface(0xdead,&mut out));assert_eq!(out,Some(Interface::Area));assert_eq!(c.ref_count(),4);
    assert_eq!(c.release(&mut g,&mut f),3);assert_eq!(c.release(&mut g,&mut f),2);assert_eq!(c.release(&mut g,&mut f),1);assert_eq!(c.release(&mut g,&mut f),0);assert!(c.destroyed());
}

#[test]
fn lcv09_10_text_fit_presence_and_opacity() {
    let f=passes(Kind::Text,&[u(0x304,0),u(0xf015,0xf008),u(0x301,0),s(0xf004,"bad")]);
    assert_eq!(widgets(&f),vec![call(11,0x1c,vec![n(4),n(0)]),call(11,0x5c,vec![i(0),i(0)]),call(11,0x14,vec![n(0),n(0)]),call(11,0x54,vec![n(4)]),call(11,0x14,vec![n(0),n(0)])]);
    assert!(f.events.contains(&win(10,Write::Flag(0x20000,true))));
    assert_eq!(f.events.iter().filter(|e|**e==win(10,Write::Param(0xdacf8f9c,n(1)))).count(),2);
    let f=passes(Kind::Text,&[]);assert!(!widgets(&f).iter().any(|e|matches!(e,Event::Widget(_,0x14,_))));
}

#[test]
fn lcv11_12_textedit_flags_and_failed_images() {
    let mut f=Fake::default();f.queries.insert((10,Kind::TextEdit.iid()),11);f.raw.insert((11,0x140),n(0x80000003));
    let p=[u(0x400,2),u(0x300,1),u(0x419,1),u(0x40c,1),s(0x413,"{1,2}"),s(0x411,"{3,4}")];
    interpret(Kind::TextEdit,10,Properties(&p),true,&mut f).unwrap();interpret(Kind::TextEdit,10,Properties(&p),false,&mut f).unwrap();
    has(&f,0x134,vec![n(0x80002002)]);has(&f,0x100,vec![n(0)]);
    for (id,g,inst,index) in [(0x8b4bfefc,1,2,0),(0x8b4bfefd,3,4,1)] {
        assert!(f.events.contains(&win(10,Write::Param(id,Value::Key(ResourceKey{kind:0x856ddbac,group:g,instance:inst})))));
        assert!(f.events.contains(&win(10,Write::Param(0xdaa3b860+index,Value::Pair(g,inst)))));
    }
    assert!(!widgets(&f).iter().any(|e|matches!(e,Event::Widget(_,0x160|0x15c,_))));
    assert_eq!(f.events.iter().filter(|e|**e==Event::Release(11)).count(),2);
}

#[test]
fn lcv13_15_list_and_combo_order_and_exact_booleans() {
    let f=passes(Kind::ListBox,&[s(0x502,"B"),s(0x502,"A"),u(0x503,1),u(0x501,1),u(0x304,0)]);
    assert_eq!(widgets(&f),vec![call(11,0x8c,vec![n(1),n(0)]),call(11,0x1c,vec![text("B"),n(0xfffffffe),n(0)]),call(11,0x1c,vec![text("A"),n(0xfffffffe),n(0)]),call(11,0x38,vec![]),call(11,0x40,vec![]),call(11,0x28,vec![n(1),n(1)]),call(11,0x24,vec![]),call(11,0xbc,vec![]),call(11,0xc0,vec![])]);
    let f=passes(Kind::ListBox,&[u(0x503,2),u(0xf019,2),u(0x504,2)]);
    has(&f,0xcc,vec![n(0)]);has(&f,0x44,vec![n(0)]);assert!(!widgets(&f).iter().any(|e|matches!(e,Event::Widget(_,0x38|0x40,_))));
    let mut f=Fake::default();f.queries.insert((10,Kind::Combo.iid()),11);f.images.insert((1,2),20);
    let p=[s(0x800,"{1,2}"),u(0x400,2),s(0x502,"X"),u(0x502,42),s(0x502,"X"),u(0x501,3)];
    interpret(Kind::Combo,10,Properties(&p),true,&mut f).unwrap();interpret(Kind::Combo,10,Properties(&p),false,&mut f).unwrap();
    assert_eq!(widgets(&f),vec![call(11,0x84,vec![Value::Handle(20),Value::None]),call(11,0x54,vec![n(0)]),call(11,0x6c,vec![text("X"),n(0xfffffffe)]),call(11,0x6c,vec![text("X"),n(0xfffffffe)]),call(11,0x70,vec![n(3),n(1)])]);
    assert!(f.events.contains(&Event::Release(20)));
}

#[test]
fn lcv16_17_button_defaults_boolean_timing_and_failed_tip() {
    let f=passes(Kind::Btn,&[u(0xa07,30),u(0xa08,0),u(0xf005,0xa0c),s(0xa10,"literal"),s(0xa12,"{1,2}"),u(0xa02,0)]);
    has(&f,0x14,vec![n(0x200),n(0)]);has(&f,0x14,vec![n(0x400),n(1)]);
    has(&f,0x30,vec![n(3),Value::None,Value::None]);has(&f,0x98,vec![n(0x01000000)]);
    has(&f,0xb4,vec![n(0),n(0)]);has(&f,0xb4,vec![n(1),n(0)]);has(&f,0x8c,vec![text("literal")]);
    assert_eq!(f.events.iter().filter(|e|**e==call(11,0x14,vec![n(4),n(0)])).count(),2);
    assert!(f.events.contains(&win(10,Write::Param(0x2b340ffe,Value::Key(ResourceKey{kind:0x2026960b,group:1,instance:2})))));
}

#[test]
fn lcv18_19_bmp_alpha_and_common_override() {
    for (v,bits) in [(0,0),(128,0x3f008081),(255,0x3f800000),(256,0x3f800000),(u32::MAX,0x3f800000)] {let f=passes(Kind::Bmp,&[u(0xb00,v)]);has(&f,0x18,vec![n(1),n(bits)]);has(&f,0x2c,vec![n(0x10),n(0)]);}
    let (mut c,g,mut f)=initialized();f.creates.push_back(Some(10));f.queries.insert((10,Kind::Bmp.iid()),11);f.images.insert((1,2),20);
    let p=[u(0xffffffff,0x82fe68c4),s(0xf016,"{1,2}"),s(0x103,"(90,30)")];
    c.set_parameters(Properties(&p),&g,&mut f).unwrap();
    let sizes:Vec<_>=f.events.iter().filter(|e|matches!(e,Event::Write(_,Write::Size(..)))).cloned().collect();
    assert_eq!(sizes,vec![win(10,Write::Size(20,10)),win(10,Write::Size(90,30))]);
}

#[test]
fn lcv20_22_scrollbar_and_line() {
    let f=passes(Kind::Scrollbar,&[s(0xc00,"(8,-2)"),u(0xc01,0xc03),u(0x40a,u32::MAX)]);
    assert_eq!(widgets(&f),vec![call(11,0x1c,vec![i(-2),i(8)]),call(11,0x24,vec![n(2)]),call(11,0x14,vec![i(-1)])]);
    let mut f=Fake::default();f.queries.insert((10,Kind::Scrollbar.iid()),11);f.images.insert((1,2),20);
    interpret(Kind::Scrollbar,10,Properties(&[s(0xf016,"{1,2}"),s(0xf018,"(1,2,3,4)")]),true,&mut f).unwrap();
    has(&f,0x44,vec![Value::Rect([1,2,3,4]),Value::Handle(20),n(0)]);
    let f=passes(Kind::LineInput,&[s(0xf004,"(2,3)"),u(0xe08,128),s(0xe0b,"(9,4)")]);
    has(&f,0x84,vec![i(2),i(3),i(2),i(3)]);has(&f,0x4c,vec![n(0x3f008081)]);has(&f,0x7c,vec![i(9),i(-5)]);
}

#[test]
fn lcv23_24_spinner_and_ordered_ticker() {
    let f=passes(Kind::Spinner,&[u(0x1000,0xf007),u(0xf015,0xf008),s(0x1008,"(1,2,3)"),s(0x1004,"(8,-2)"),u(0x1005,(-3i32) as u32)]);
    has(&f,0x10,vec![n(2)]);has(&f,0x6c,vec![n(1)]);has(&f,0x7c,vec![n(0),n(0x030201),n(0),n(0)]);
    let w=widgets(&f);assert!(w.iter().position(|e|*e==call(11,0x28,vec![i(8),i(-2)])).unwrap()<w.iter().position(|e|*e==call(11,0x30,vec![i(-3)])).unwrap());
    let f=passes(Kind::TextTicker,&[s(0x1202,"A"),s(0xf002,"(1,2,3)"),s(0x1202,"B"),s(0xf001,"(4,5,6)"),s(0x1202,"C")]);
    let messages:Vec<_>=widgets(&f).into_iter().filter(|e|matches!(e,Event::Widget(_,0x0c,_))).collect();
    assert_eq!(messages,vec![call(11,0x0c,vec![text("A"),n(u32::MAX),n(0),n(0),n(0)]),call(11,0x0c,vec![text("B"),n(0x030201),n(0),n(0),n(0)]),call(11,0x0c,vec![text("C"),n(0x030201),n(0x060504),n(0),n(0)])]);
}

#[test]
fn lcv25_27_generator_options_and_procedure_order() {
    let (mut c,g,mut f)=initialized();f.creates.push_back(Some(10));f.queries.insert((10,GEN_IID),11);f.images.insert((1,2),20);c.use_win_proc(Some(77),&mut f);
    let p=[u(0xffffffff,0x4386d516),u(0x10000,7),u(0x10001,9),s(0xf016,"{1,2}"),s(0x10002,"(12,-4)"),s(0x10003,"bad")];
    c.set_parameters(Properties(&p),&g,&mut f).unwrap();has(&f,0x4c,vec![n(7)]);has(&f,0x34,vec![n(1),Value::Handle(20)]);has(&f,0x78,vec![n(12),i(-4)]);
    let default=f.events.iter().position(|e|*e==call(11,0x1c,vec![Value::Handle(900)])).unwrap();
    let caller=f.events.iter().position(|e|*e==call(11,0x1c,vec![Value::Handle(77)])).unwrap();
    let init=f.events.iter().position(|e|*e==win(10,Write::Init)).unwrap();assert!(default<caller&&caller<init);
    let f=passes(Kind::OptGrp,&[u(0xf00,2),u(0xf0c,0xf0e)]);has(&f,0x14,vec![n(1),n(0)]);assert!(!widgets(&f).iter().any(|e|matches!(e,Event::Widget(_,0x70,_))));
    let f=passes(Kind::OptGrp,&[s(0xf11,"4:(8,9)"),s(0xf0d,"4:Four:2:{1,2}:(0,0,5,6)"),u(0x501,4),u(0x304,0)]);
    assert_eq!(widgets(&f),vec![call(11,0x3c,vec![i(4),text("Four"),i(2),Value::None,Value::Rect([0,0,5,6])]),call(11,0x4c,vec![i(4)]),call(11,0x44,vec![i(4),i(8),i(9)]),call(11,0x38,vec![n(4)]),call(11,0x60,vec![])]);
}

#[test]
fn lcv28_32_33_failure_boundaries() {
    for table in vocabulary::TABLES {
        if matches!(table.kind,Kind::Common|Kind::Custom) {continue;}
        let mut f=Fake::default();interpret(table.kind,10,Properties(&[s(0xf016,"{1,2}")]),true,&mut f).unwrap();
        assert_eq!(f.events,vec![Event::Query(10,table.iid)]);
    }
    let f=passes(Kind::Combo,&[s(0x800,"bad"),s(0xf004,"(4,5)")]);assert_eq!(widgets(&f),vec![call(11,0x9c,vec![i(4),i(5)])]);
    let f=passes(Kind::Scrollbar,&[s(0xf016,"{1}"),s(0xc00,"(8,-2)")]);assert_eq!(widgets(&f),vec![call(11,0x1c,vec![i(-2),i(8)])]);
    for (kind,p) in [(Kind::Bmp,vec![s(0xf016,"{1}")]),(Kind::Btn,vec![s(0xf004,"(1,bad)")]),(Kind::TextEdit,vec![s(0x40b,"(1,2,3)")])] {
        let mut f=Fake::default();f.queries.insert((10,kind.iid()),11);f.raw.insert((11,0x140),n(0));assert!(interpret(kind,10,Properties(&p),true,&mut f).is_err());assert_eq!(f.events.last(),Some(&Event::Release(11)));
    }
}
