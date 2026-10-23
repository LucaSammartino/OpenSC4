use super::power_pole::*;
use super::xform::Xform;
use crate::automata::OccupantBase;
use crate::stream::{RefValue, ResKey};
use super::{pylon::{References,RenderStateArgs}, tests_pylon::{reference,token,writer,Reader}};
use crate::stream::{InStream,OutStream,ByteReader};

fn fields() -> PoleFields {
    PoleFields {
        transform: Xform::default(),
        position: [2.0, 3.0, 4.0],
        cell: (-7, 9),
        model_y: 3.5,
        distressed: 2,
        ignored: 3,
        field_118: 123,
        connections: (0..2)
            .map(|i| PoleConnection {
                pole: RefValue::Host {
                    token: i + 1,
                    class_id: 0x09C0_5C6A,
                },
                wire_points: vec![RefValue::Null],
                flags: [i as u8, 4],
                words: [5, 6],
                vertices: [vec![vec![[1.0, 2.0, 3.0]]], vec![]],
            })
            .collect(),
    }
}

#[derive(Default)]
struct Host(Vec<&'static str>);

impl PoleInitHost for Host {
    fn occupant_init(&mut self, base: &mut OccupantBase) {
        assert_eq!(base.occupant_type, 0x2890_D4DE);
        self.0.push("base");
    }

    fn set_default_exemplar(&mut self, _: &mut OccupantBase, key: ResKey) {
        assert_eq!(key.instance, 77);
        self.0.push("exemplar");
    }

    fn model_property(&mut self, _: &OccupantBase, property: u32) -> u32 {
        assert_eq!(property, 0x2781_2821);
        self.0.push("property");
        0x1234
    }

    fn model_resource(&mut self, resource_type: u32, model_id: u32) -> RefValue {
        assert_eq!((resource_type, model_id), (0x9AD0_E7E3, 0x1234));
        self.0.push("resource");
        RefValue::Host {
            token: 42,
            class_id: resource_type,
        }
    }
}

#[test]
fn published_init_copies_models_and_clears_geometry_in_order() {
    let mut statics = PoleStatics::new(0x89AB_CDEF, 0xFEDC_BA98);
    statics.init_once([16.0, 32.0], [10, 20], |_| None, |_| None);
    let mut pole = PowerPole::new(fields());
    pole.geometry = std::array::from_fn(|_| vec![super::record::Vertex::default()]);
    pole.bytes_54_55 = [4, 5];
    pole.scale_64 = 6.0;
    let mut host = Host::default();
    pole.init_published_state(
        &statics,
        ResKey {
            instance: 77,
            ..ResKey::default()
        },
        99,
        3,
        &mut host,
    );
    assert_eq!(host.0, ["base", "exemplar", "property", "resource"]);
    assert_eq!(pole.model_ids, [10, 20]);
    assert_eq!(pole.wire_exemplar_index, 99);
    assert!(matches!(pole.model, RefValue::Host { token: 42, .. }));
    assert_eq!(pole.connection_point_table, Some(ConnectionPointTable::Second));
    assert!(pole.geometry.iter().all(Vec::is_empty));
    assert_eq!(pole.bytes_54_55, [0, 0]);
    assert_eq!(pole.scale_64, 1.0);
    // Init does not clear the separately specified connection vector.
    assert_eq!(pole.connection_count(), 2);
    assert!(pole.is_distressed());
}

#[test]
fn static_controls_load_once_and_cost_preserves_both_words() {
    let mut statics = PoleStatics::new(0x89AB_CDEF, 0xFEDC_BA98);
    statics.init_once(
        [16.0, 32.0],
        [10, 20],
        |id| (id == PROP_E9A18D3F).then_some(31.0),
        |_| None,
    );
    statics.init_once([1.0; 2], [1; 2], |_| panic!("second read"), |_| panic!("second read"));
    assert_eq!(statics.control.v_e9a18d3f, 31.0);
    assert_eq!(statics.control.v_49a1e8c4, 10.0);
    assert_eq!(statics.cell_size, [16.0, 32.0]);
    assert_eq!(statics.model_ids, [10, 20]);
    assert_eq!(statics.demolition_cost() as u64, 0xFEDC_BA98_89AB_CDEF);
    assert_eq!(PoleStatics::new(7, 0).demolition_cost(), 7);
}

#[test]
fn connection_views_and_published_state() {
    let mut pole = PowerPole::new(fields());
    // §13 test 2.
    assert_eq!(pole.base.occupant_type, 0x2890_D4DE);
    // Metadata only; the native pool is not implemented by this Rust type.
    assert_eq!(POWER_POLE_POOL_OBJECT_SIZE, 0x124);
    assert_eq!(pole.get_cell(), (-7, 9));
    assert!(pole.is_distressed());
    assert!(pole.ignored());
    pole.set_field_118(456);
    assert_eq!(pole.fields.field_118, 456);
    assert_eq!(pole.connection(0).unwrap().vertices[0], [vec![[1.0, 2.0, 3.0]]]);
    assert!(matches!(pole.connected_pole(1), Some(RefValue::Host { token: 2, .. })));
    assert!(pole.connection(2).is_none());
    for (nibble, expected) in [
        (1, ConnectionPointTable::First),
        (2, ConnectionPointTable::First),
        (3, ConnectionPointTable::Second),
        (4, ConnectionPointTable::Second),
    ] {
        pole.select_connection_point_table(nibble);
        assert_eq!(pole.connection_point_table, Some(expected));
    }
}

#[derive(Default)]
struct PoleFixture { log:Vec<String>, quote:Option<i64>, max_distance:Option<f32>, exemplar:bool }
impl References for PoleFixture {
    fn add_ref(&mut self,v:&RefValue) { self.log.push(format!("add{}",token(v))); }
    fn release(&mut self,v:&RefValue) { self.log.push(format!("release{}",token(v))); }
}
impl PoleStaticHost for PoleFixture {
    fn acquire_resource_manager(&mut self)->RefValue { self.log.push("resource".into()); reference(88) }
    fn acquire_texture_manager(&mut self,class:u32,iid:u32)->RefValue { assert_eq!((class,iid),(0x1AC0_E11A,0xFAC0_E219)); reference(89) }
    fn register_shutdown(&mut self) { self.log.push("callback".into()); }
    fn control_exemplar(&mut self,key:ResKey)->bool { assert_eq!(key,CONTROL_EXEMPLAR); self.log.push("exemplar".into()); self.exemplar }
    fn float_property(&self,id:u32)->Option<f32> { if id==PROP_E9A18D3F { self.max_distance } else { None } }
    fn uint_property(&self,_:u32)->Option<u32> { None }
    fn width_property(&self,id:u32)->Option<[f32;5]> { assert_eq!(id,0xE9AC_43E7); None }
    fn sint64_property(&self,id:u32)->Option<i64> { assert_eq!(id,0x0A2E_CED8); self.quote }
    fn pack(&mut self,args:RenderStateArgs)->u64 { assert_eq!(args,RenderStateArgs([1,1,1,1,0,1,0,3,4,5,4],0.1,[1,-1])); 42 }
}
impl PoleRecordHost for PoleFixture {
    fn exemplar_model_instance(&mut self,_:&OccupantBase)->u32 { 0x100 }
}
impl PoleHost for PoleFixture {
    fn insert_line(&mut self,line:&RefValue,arg:u32) { assert_eq!(arg,0); self.log.push(format!("insert{}",token(line))); }
    fn remove_line(&mut self,line:&RefValue,destroy:bool,arg:u32) { assert!(destroy); assert_eq!(arg,0); self.log.push(format!("remove{}",token(line))); }
    fn reciprocal_remove(&mut self,other:&RefValue,this:&RefValue,quiet:bool) { assert!(!quiet); assert_eq!(token(this),9); self.log.push(format!("reciprocal{}",token(other))); }
    fn post_message(&mut self,message:u32,data:u32) { assert_eq!((message,data),(0x99EF_1145,0)); self.log.push("post".into()); }
    fn lines_to(&mut self,_:&RefValue,_:&RefValue,out:&mut Vec<(i32,i32)>)->bool { out.push((77,88)); true }
    fn create_effect(&mut self,name:&str)->RefValue { assert_eq!(name,"powerpolezap"); self.log.push("effect".into()); reference(55) }
    fn place_effect(&mut self,_:&RefValue,position:[f32;3],rotation:[f32;9],scale:f32) { assert_eq!(rotation,super::xform::IDENTITY_MATRIX); assert_eq!(scale,1.0); self.log.push(format!("place{position:?}")); }
    fn start_effect(&mut self,_:&RefValue) { self.log.push("start".into()); }
    fn stop_effect(&mut self,_:&RefValue) { self.log.push("stop".into()); }
    fn finish_effect(&mut self,_:&RefValue) { self.log.push("finish".into()); }
    fn subscription(&mut self,message:u32,subscribe:bool) { assert_eq!(message,0x99EF_1142); self.log.push(format!("subscription{subscribe}")); }
    fn occupant_shutdown(&mut self,_:&mut OccupantBase) { self.log.push("shutdown".into()); }
}

#[test]
fn pole_constructor_shared_tables_and_geometry_conformance_43_44() {
    let mut p=PowerPole::fresh(); let mut tables=AttachmentTables::default();
    assert!(p.stale && !p.subscribed && !p.is_distressed() && !p.is_preview());
    assert_eq!(p.model_ids,[u32::MAX;2]); assert_eq!(p.fields.transform,Xform::default());
    assert!(p.connection_point_table.is_none()); p.select_connection_point_table(0); assert!(p.connection_point_table.is_none());
    p.select_connection_point_table(1); p.select_connection_point_table(0); assert_eq!(p.connection_point_table,Some(ConnectionPointTable::First));
    assert_eq!(tables.0[0][0][0].map(f32::to_bits),[0,0x41FD_999A,0x4099_999A]);
    p.select_connection_point_table(3); p.select_connection_point_table(15); assert_eq!(p.connection_point_table,Some(ConnectionPointTable::Second));
    assert_eq!(tables.0[1][0][0].map(f32::to_bits),[0xC053_3333,0x41FD_999A,0x4053_3333]);
    tables.0[1][0][0]=[1.0,2.0,3.0]; p.fields.position=[10.0,20.0,30.0];
    assert_eq!(p.attachment_points(4,&tables).unwrap()[0],[11.0,22.0,33.0]);
    p.fields.model_y=4.0;
    for r in 0..4 {
        for _ in 0..2 { p.create_floor(false).unwrap(); p.create_walls(r); }
        assert_eq!(p.geometry.each_ref().map(Vec::len),[4;3]);
        let pairs=[[(2,3),(1,2)],[(1,2),(0,1)],[(0,1),(3,0)],[(3,0),(2,3)]][r];
        for (i,(a,b)) in pairs.into_iter().enumerate() {
            assert_eq!(p.geometry[i+1][0].pos,p.geometry[0][a].pos); assert_eq!(p.geometry[i+1][3].pos,p.geometry[0][b].pos);
        }
    }
    assert_eq!(p.geometry[0].iter().map(|v|v.pos).collect::<Vec<_>>(),[[5.0,20.0,25.0],[5.0,20.0,35.0],[15.0,20.0,35.0],[15.0,20.0,25.0]]);
    let a=super::record::Vertex {pos:[1.0,36.0,3.0],color:0x4000_0000,..Default::default()};
    let b=super::record::Vertex {pos:[4.0,20.0,6.0],color:0x8000_0000,..Default::default()};
    let wall=PowerPole::wall_quad(a,b,4.0); assert_eq!(wall.iter().map(|v|v.alpha()).collect::<Vec<_>>(),[64,64,128,128]);
    assert_eq!(wall.iter().map(|v|v.v).collect::<Vec<_>>(),[0.0,2.0,1.0,0.0]);
    let mut a=a; a.pos[1]=-12.0; assert_eq!(PowerPole::wall_quad(a,b,4.0)[1].v,-1.0);
    assert_eq!(p.create_floor(true),Err(PoleUnsupported::DiagonalFloorNO43));
}

#[test]
fn pole_quote_conversion_once_and_independent_controls_conformance_17() {
    for (exists,quote) in [(false,None),(true,None),(true,Some(0x0000_0001_0000_0002)),(true,Some(-7))] {
        let mut h=PoleFixture {exemplar:exists,quote,max_distance:Some(32.0),..Default::default()};
        let mut s=PoleStatics::new(0,0); s.initialize(&mut h);
        assert_eq!(s.demolition_cost(),quote.unwrap_or(0));
        h.quote=Some(999); let calls=h.log.len(); s.initialize(&mut h); assert_eq!(h.log.len(),calls);
        assert_eq!(s.demolition_cost(),quote.unwrap_or(0));
        s.control=PowerLineControl::load(|_|None,|_|None); assert_eq!(s.demolition_cost(),quote.unwrap_or(0));
        assert_eq!(s.widths,[0.3;5]); assert_eq!(s.packed_state,42);
    }
}

#[test]
fn pole_nested_record_iids_shared_overwrite_and_failure_cleanup_conformance_16() {
    let mut p=PowerPole::fresh(); p.select_connection_point_table(1);
    p.fields.connections=vec![PoleConnection {pole:reference(1),wire_points:vec![reference(2),reference(3)],flags:[0x80,0x80],words:[(-7i32) as u32,19],vertices:[vec![vec![[1.0,2.0,3.0]],vec![]],vec![vec![]]]},PoleConnection {pole:reference(4),..Default::default()}];
    p.wire_exemplar_index=7; p.model_ids=[11,12]; p.fields.cell=(-3,4);
    p.geometry[0]=vec![super::record::Vertex {pos:[1.0,2.0,3.0],u:0.25,v:0.5,color:0x4411_2233}];
    let mut tables=AttachmentTables::default(); tables.0[0][0]=[[1.0;3],[2.0;3],[3.0;3],[4.0;3]];
    let mut w=writer(); assert_eq!(p.write(&tables,&mut w),Ok(true)); let bytes=w.into_bytes();
    let mut input=Reader::new(&bytes); let mut back=PowerPole::fresh(); back.model=reference(9);
    let mut s=PoleStatics::new(0,0); let mut h=PoleFixture::default(); let mut shared=AttachmentTables::default();
    assert_eq!(back.read_into(&mut shared,&mut s,&mut input,&mut h),Ok(true)); assert_eq!(input.inner.remaining(),0);
    assert_eq!(input.iids,[0x09C0_5C6A,0xC9C0_5C5D,0xC9C0_5C5D,0x09C0_5C6A]);
    assert_eq!(back.fields.connections[0].flags,[1,1]); assert_eq!(back.fields.connections[0].words,[(-7i32) as u32,19]);
    assert_eq!(back.fields.connections[0].vertices,p.fields.connections[0].vertices);
    assert_eq!(back.fields.connections[1].wire_points.len(),0); assert_eq!(back.model_ids,[11,12]); assert_eq!(back.fields.cell,(-3,4));
    assert_eq!(shared.0[0][0],tables.0[0][0]); assert_eq!(back.geometry,p.geometry); assert!(matches!(back.model,RefValue::Null));
    assert_eq!(h.log.last().unwrap(),"release9");
    let mut peek=ByteReader::new(&bytes); peek.get_u16(); OccupantBase::read(&mut peek).unwrap(); let body=peek.position();
    for end in [body+1,bytes.len()-1] {
        back.model=reference(9); h.log.clear();
        assert_eq!(back.read_into(&mut shared,&mut s,&mut Reader::new(&bytes[..end]),&mut h),Ok(false));
        assert!(matches!(back.model,RefValue::Null)); assert_eq!(h.log.iter().filter(|s|*s=="release9").count(),1);
    }
    for bad in [&[3,0][..],&bytes[..3]] { back.model=reference(9); h.log.clear(); assert_eq!(back.read_into(&mut shared,&mut s,&mut Reader::new(bad),&mut h),Ok(false)); assert_eq!(token(&back.model),9); assert!(h.log.is_empty()); }
}

#[test]
fn pole_connections_effects_messages_and_model_attributes_conformance_38_39() {
    let mut p=PowerPole::fresh(); let mut h=PoleFixture::default();
    p.fields.connections=vec![PoleConnection {pole:reference(1),wire_points:vec![reference(2)],flags:[0,1],..Default::default()},PoleConnection {pole:reference(3),wire_points:vec![reference(4)],..Default::default()}];
    p.insert_lines(&mut h); p.insert_lines(&mut h); assert_eq!(h.log,["insert2","insert4"]);
    h.log.clear(); p.break_all_connections(&reference(9),true,&mut h);
    assert_eq!(p.connection_count(),2); assert!(p.fields.connections.iter().all(|r|matches!(r.pole,RefValue::Null)));
    assert_eq!(h.log,["reciprocal1","release1","remove2","release2","reciprocal3","release3","remove4","release4"]);
    p.fields.connections[0].pole=reference(1); h.log.clear(); p.remove_connection(&reference(1),false,&mut h); assert_eq!(p.connection_count(),1); assert_eq!(h.log,["release1","post"]);
    p.fields.connections[0].pole=reference(3); let mut out=vec![];
    assert_eq!(p.line_cells_to(&reference(3),&reference(9),&mut out,&mut h),Ok(true)); assert_eq!(out,[(77,88)]);
    assert_eq!(p.line_cells(99,&reference(9),&mut out,&mut h),Ok(false));
    h.log.clear(); p.set_distressed(true,&mut h); p.set_distressed(true,&mut h); p.set_distressed(false,&mut h);
    assert_eq!(h.log,["effect","place[0.0, 0.0, 0.0]","start","stop","finish","release55"]);
    p.set_preview(true); assert!(p.is_preview()); p.set_highlight(7); assert_eq!(p.bytes_54_55,[1,7]); p.set_highlight(0); assert_eq!(p.bytes_54_55,[0,0]);
    p.fields.position=[1.0,2.0,3.0]; p.fields.transform.counter=255; h.log.clear();
    p.model_instances(&reference(99),&mut h); assert_eq!(p.fields.transform.translation,[1.0,2.0,3.0]); assert_eq!((p.fields.transform.counter,p.fields.transform.mask),(0,1)); assert_eq!(h.log,["add99"]);
    p.subscribed=true; h.log.clear(); p.on_inserted_message(0x99EF_1142,false,&mut h); assert!(h.log.is_empty()); p.on_inserted_message(0x99EF_1142,true,&mut h); assert!(!p.subscribed);
    p.shutdown(&reference(9),&mut h); assert!(!p.stale); assert_eq!(h.log.last().unwrap(),"shutdown");
}
