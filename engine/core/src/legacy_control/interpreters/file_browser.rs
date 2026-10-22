// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn mapping(value:Option<u32>,pairs:&[(u32,u32)])->Option<u32>{value.and_then(|v|pairs.iter().find(|(key,_)|*key==v).map(|(_,n)|*n))}
pub(super) fn run(h:Handle,t:Handle,p:Properties<'_>,first:bool,f:&mut impl Foreign)->Result<()> {
    if !first{return Ok(());}
    if let Some(s)=p.text(0x1301){call(f,t,0x3c,vec![text(s)]);}
    let name=p.text(0x1306).unwrap_or_default();let ext=p.text(0x1307).unwrap_or_default();
    if let Some(kind)=mapping(p.number(0x1305),&[(0x1302,0),(0x1303,1),(0x1304,2)]){call(f,t,0x70,vec![n(kind),text(name),text(ext)]);}
    let mut sort=words(call(f,t,0x88,vec![]),2)?;
    for k in 0..2{if let Some(v)=mapping(p.number(0x130c+k as u32),&[(0x1308,0),(0x1309,1),(0x130a,2),(0x130b,3)]){sort[k]=v;}}
    call(f,t,0x8c,sort.into_iter().map(n).collect());
    if let Some(v)=mapping(p.number(0x1312),&[(0x130e,0),(0x130f,1),(0x1310,2),(0x1311,3)]){call(f,t,0x94,vec![n(v)]);}
    let mut columns=words(call(f,t,0x98,vec![]),3)?;
    for k in 0..3{if let Some(v)=mapping(p.number(0x1319+k as u32),&[(0x1313,0),(0x1314,1),(0x1315,2),(0x1316,5)]){columns[k]=v;}}
    call(f,t,0xa0,columns.into_iter().map(n).collect());
    for (key,index,font_slot,colour_slot,align_slot) in [(0x131c,0,0xb4,0xb8,0xbc),(0x1324,1,0xcc,0xd0,0xd4),(0x132c,2,0xe4,0xe8,0xec)] {
        font_word(h,t,p,key,index,font_slot,f);
        for which in 0..6{if let Some(c)=colour(p,key+1+which)?{let c=f.colour(c);call(f,t,colour_slot,vec![n(which),n(c)]);}}
        if let Some(v)=mapping(p.number(key+7),&[(0x1317,0),(0x1318,1),(0xf006,2),(0xf008,3),(0xf007,4)]){call(f,t,align_slot,vec![n(v)]);}
    }
    for (key,slot) in [(0x1334,0xf4),(0x1335,0xfc),(0x1336,0x104)]{if let Some(v)=p.number(key){call(f,t,slot,vec![n(v)]);}}
    for (getter,setter,enabled,colour_key) in [(0x108,0x10c,0x1337,0x1338),(0x110,0x114,0x1339,0x133a)]{
        let mut values=words(call(f,t,getter,vec![]),2)?;
        if let Some(v)=flag(p,enabled){values[0]=v as u32;}
        if let Some(c)=colour(p,colour_key)?{values[1]=f.colour(c);}
        call(f,t,setter,values.into_iter().map(n).collect());
    }
    let mut highlight=words(call(f,t,0x118,vec![]),2)?;
    for k in 0..2{if let Some(c)=colour(p,0x133b+k as u32)?{highlight[k]=f.colour(c);}}
    call(f,t,0x11c,highlight.into_iter().map(n).collect());
    if let Some(s)=p.text(0x133d){call(f,t,0x134,vec![text(s)]);}
    if let Some(v)=p.number(0x133e){call(f,t,0x144,vec![n(v)]);}
    if let Some(c)=colour(p,0x133f)?{let c=f.colour(c);call(f,t,0x14c,vec![n(c)]);}
    for (key,slot) in [(0x1341,0x15c),(0xf019,0x13c)]{if let Some(v)=flag(p,key){call(f,t,slot,vec![n(v as u32)]);}}
    if let Some((g,inst))=key(p,0x1340)?{if let Some(b)=f.load_image(g,inst){call(f,t,0x154,vec![Value::Handle(b)]);image_key(h,g,inst,0,f);f.release(b);}}
    for (k,s) in p.texts(0x1342){
        let v=fields(&s,':',3);let number=scan(v[1],"%d")?.first().copied().unwrap_or(0);
        let key=scan(v[2],"{%x,%x}")?;let mut buffer=None;
        if key.len()==2{buffer=f.load_image(key[0],key[1]);image_key(h,key[0],key[1],k as u32+1,f);}
        call(f,t,0x54,vec![text(v[0]),buffer.map_or(Value::None,Value::Handle),i(number as i32),n(0)]);
        if let Some(b)=buffer{f.release(b);}
    }
    Ok(())
}
