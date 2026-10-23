// SPDX-License-Identifier: GPL-3.0-or-later
//! legacy-control §§9.14–9.19. Float arguments travel as exact binary32 bits.
use super::controls::*;
use super::*;

pub(super) fn unchecked<const N: usize>(s: &str, pattern: &str) -> Result<[u32;N]> {
    let values=scan(s,pattern)?;
    // LC11: native uninitialised/stale arguments have no portable value.
    values.try_into().map_err(|_| Unsupported::Malformed("LC11 unchecked scan"))
}
fn alpha(v: u32)->Value {
    n(((v.min(255) as f64 * f32::from_bits(0x3b808081) as f64) as f32).to_bits())
}
fn gutters(t: Handle,p: Properties<'_>,pair_slot: u32,quad_slot: u32,repeat: bool,f: &mut impl Foreign)->Result<()> {
    if let Some(s)=p.text(0xf004) {
        match s.bytes().filter(|b| *b==b',').count() {
            1 => {let [a,b]=unchecked::<2>(&s,"(%d,%d)")?;
                let args=if repeat {vec![i(a as i32),i(b as i32),i(a as i32),i(b as i32)]} else {vec![i(a as i32),i(b as i32)]};
                call(f,t,pair_slot,args);
            }
            3 => {let v=unchecked::<4>(&s,"(%d,%d,%d,%d)")?; call(f,t,quad_slot,v.into_iter().map(|x|i(x as i32)).collect());}
            _=>{}
        }
    }
    Ok(())
}
pub(super) fn run(kind: Kind,h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    match kind {
        Kind::Btn=>button(h,t,p,first,f),
        Kind::Bmp=>bmp(h,t,p,first,f),
        Kind::Scrollbar=>scrollbar(h,t,p,first,f),
        Kind::LineInput=>line(h,t,p,first,f),
        Kind::Spinner=>spinner(h,t,p,first,f),
        Kind::TextTicker=>ticker(h,t,p,first,f),
        _=>unreachable!(),
    }
}
fn button(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first {
        if let Some(v)=p.number(0xa02) {call(f,t,0x14,vec![n(4),n(u32::from(v!=0))]);}
        return Ok(());
    }
    for (key,mask,invert) in [(0xa00,1,false),(0xa01,2,false),(0xa02,4,false),(0xf00d,0x10,true),
        (0xa03,0x20,false),(0xa04,0x40,false),(0xa05,0x80,false),(0xa06,0x100,false),
        (0xa07,0x200,true),(0xa08,0x400,true)] {
        if let Some(v)=p.number(key) {call(f,t,0x14,vec![n(mask),n(u32::from((v!=0)^invert))]);}
    }
    let style=match p.number(0xf005) {
        Some(0xa09)=>1,Some(0xa0a|0xa0b)=>2,Some(0xa00|0xa0c|0xa0d)=>3,
        Some(0xa0e|0xa0f|0xa1b)=>4,_=>0,
    };
    font(h,t,p,0xf000,0,0x50,false,Some(0x4c),f)?;
    colours(t,p,0x5c,&[(0xf00f,0),(0xf010,4),(0xf011,2),(0xf012,1),(0xf013,5),(0xf014,3)],f)?;
    gutters(t,p,0x78,0x74,false,f)?;
    if let Some(s)=p.text(0xf016) {
        let [g,inst]=unchecked::<2>(&s,"{%x,%x}")?;
        let b=f.load_image(g,inst);
        let rect=b.map(|b|f.image_rect(b));
        call(f,t,0x30,vec![n(style),b.map_or(Value::None,Value::Handle),rect.map_or(Value::None,Value::Rect)]);
        resource(h,0x2b343bec,0x856ddbac,g,inst,f);image_key(h,g,inst,0,f);
        if let Some(b)=b {f.release(b);}
    } else {call(f,t,0x30,vec![n(style),Value::None,Value::None]);}
    if let Some(v)=p.number(0xf015).and_then(|v|match v {0xf006=>Some(1),0xf007=>Some(2),0xf008=>Some(0),_=>None}) {call(f,t,0x6c,vec![n(v)]);}
    if let Some(s)=p.text(0xa10) {call(f,t,0x8c,vec![text(s)]);}
    if let Some(s)=p.text(0xa12) {
        let [g,inst]=unchecked::<2>(&s,"{%x,%x}")?;
        if let Some(s)=f.load_string(g,inst) {call(f,t,0x8c,vec![text(s)]);}
        resource(h,0x2b340ffe,0x2026960b,g,inst,f);
    }
    pair(t,p,0xa11,0xa0,f)?;
    call(f,t,0x98,vec![n(p.number(0xa18).unwrap_or(0x01000000))]);
    for (key,which,param_id) in [(0xa19,0,0x0a5be39d),(0xa1a,1,0x4a5be2f8)] {
        let mut instance=0;
        if let Some(s)=p.text(key) {let [g,inst]=unchecked::<2>(&s,"{%x,%x}")?; instance=inst;param(h,param_id,Value::Pair(g,inst),f);}
        call(f,t,0xb4,vec![n(which),n(instance)]);
    }
    Ok(())
}
fn bmp(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if first {
        if let Some(v)=p.number(0xb00) {call(f,t,0x18,vec![n(1),alpha(v)]);}
        for (key,mask) in [(0xf003,1),(0xb01,2),(0xb02,8)] {
            if let Some(v)=p.number(key) {call(f,t,0x2c,vec![n(mask),n(u32::from(v!=0))]);}
        }
        if let Some(s)=p.text(0xf016) {
            let [g,inst]=unchecked::<2>(&s,"{%x,%x}")?;
            if let Some(b)=f.load_image(g,inst) {
                call(f,t,0x10,vec![Value::Handle(b)]);
                let [l,top,r,bottom]=f.image_rect(b);
                f.write(h,Write::Size(r.wrapping_sub(l),bottom.wrapping_sub(top))); f.release(b);
            }
            resource(h,0xab428c5d,0x856ddbac,g,inst,f);image_key(h,g,inst,0,f);
        }
    } else {
        let rect=tuple::<4>(p,0xf018)?;
        if let Some(rect)=rect {call(f,t,0x20,vec![Value::Rect(rect)]);}
        call(f,t,0x2c,vec![n(0x10),n(u32::from(rect.is_some()))]);
    }
    Ok(())
}
fn scrollbar(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first {if let Some(v)=p.number(0x40a) {call(f,t,0x14,vec![i(v as i32)]);} return Ok(());}
    if let Some([a,b])=tuple::<2>(p,0xc00)? {call(f,t,0x1c,vec![i(a.min(b)),i(a.max(b))]);}
    if let Some(v)=p.number(0xc01) {call(f,t,0x24,vec![n(if v==0xc03 {2} else {1})]);}
    numbers(t,p,&[(0xd00,0x2c),(0xd01,0x34)],f);pair(t,p,0xd02,0x3c,f)?;
    if let Some((g,inst))=key(p,0xf016)? {
        let rect=tuple::<4>(p,0xf018)?;
        if let Some(b)=f.load_image(g,inst) {let rect=rect.unwrap_or_else(||f.image_rect(b));call(f,t,0x44,vec![Value::Rect(rect),Value::Handle(b),n(0)]);f.release(b);}
        resource(h,0x6b4bcbd4,0x856ddbac,g,inst,f);image_key(h,g,inst,0,f);
    }
    Ok(())
}
fn line(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first {
        numbers(t,p,&[(0xe09,0x64)],f);
        if let Some(v)=p.number(0x400) {call(f,t,0x70,vec![n(u32::from(v!=0))]);}
        numbers(t,p,&[(0xe0a,0x74)],f);
        if let Some([a,b])=tuple::<2>(p,0xe0b)? {call(f,t,0x7c,vec![i(a),i(b.wrapping_sub(a))]);}
        return Ok(());
    }
    font(h,t,p,0xf000,0,0x10,true,Some(0x10),f)?;
    colours(t,p,0x14,&[(0xf00f,0),(0xf010,1),(0xf011,2)],f)?;
    colours(t,p,0x38,&[(0xe03,0),(0xe04,1),(0xe05,2),(0xe06,3)],f)?;
    singles(t,p,&[(0xe07,0x44)],f)?;
    if let Some(v)=p.number(0xe08) {call(f,t,0x4c,vec![alpha(v)]);}
    if let Some(v)=p.number(0xe00) {call(f,t,0x24,vec![n(u32::from(v!=0))]);}
    if let Some(v)=p.number(0xf005).and_then(|v|match v {0xe01=>Some(2),0xe02=>Some(3),0xf00a=>Some(1),_=>None}) {call(f,t,0x2c,vec![n(v)]);}
    gutters(t,p,0x84,0x84,true,f)
}
fn spinner(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first {
        pair(t,p,0x1004,0x28,f)?;
        for (key,slot) in [(0x1005,0x30),(0x40a,0x24)] {if let Some(v)=p.number(key) {call(f,t,slot,vec![i(v as i32)]);}}
        return Ok(());
    }
    if let Some(v)=p.number(0x1000).and_then(|v|match v {0=>Some(0),0xf006=>Some(1),0xf007=>Some(2),_=>None}) {call(f,t,0x10,vec![n(v)]);}
    for (key,mask) in [(0xf019,1),(0xf00d,2),(0x1001,0x1000),(0x1002,0x2000),(0x1003,0x4000)] {if let Some(v)=p.number(key) {call(f,t,0x18,vec![n(mask),n(u32::from(v!=0))]);}}
    font(h,t,p,0xf000,0,0x4c,true,Some(0x4c),f)?;
    colours(t,p,0x5c,&[(0xf00f,0),(0xf010,2),(0xf011,1)],f)?;
    if let Some(s)=p.text(0xf016) {
        let [g,inst]=unchecked::<2>(&s,"{%x,%x}")?;
        if let Some(b)=f.load_image(g,inst) {call(f,t,0x38,vec![Value::Handle(b),n(0)]);f.release(b);}
        resource(h,0x8bd1333a,0x856ddbac,g,inst,f);image_key(h,g,inst,0,f);
    }
    if let Some(v)=p.number(0xf015).and_then(|v|match v {0xf006=>Some(0),0xf007=>Some(2),0xf008=>Some(1),_=>None}) {call(f,t,0x6c,vec![n(v)]);}
    pair(t,p,0xf004,0x74,f)?;
    let mut edges=[0;4];let mut found=false;
    if let Some(c)=colour(p,0x1006)? {edges.fill(f.colour(c));found=true;}
    for (k,edge) in edges.iter_mut().enumerate() {if let Some(c)=colour(p,0x1007+k as u32)? {*edge=f.colour(c);found=true;}}
    if found {call(f,t,0x7c,edges.into_iter().map(n).collect());}
    Ok(())
}
fn ticker(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first {return Ok(());}
    numbers(t,p,&[(0x1200,0x20),(0x1201,0x24)],f);
    font(h,t,p,0xf000,0,0x1c,true,Some(0x1c),f)?;
    let (mut fore,mut back)=(0xffffffff,0);
    for prop in p.0 {
        let Some(s)=Properties(std::slice::from_ref(prop)).text(prop.key) else {continue;};
        match prop.key {
            0xf002|0xf001=> {let c=unchecked::<3>(&s,"(%d,%d,%d)")?;let c=f.colour(c.map(|x|x as u8));if prop.key==0xf002 {fore=c;}else {back=c;}}
            0x1202=>{call(f,t,0x0c,vec![text(s),n(fore),n(back),n(0),n(0)]);}
            _=>{}
        }
    }
    Ok(())
}
