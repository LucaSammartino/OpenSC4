// SPDX-License-Identifier: GPL-3.0-or-later
//! Defined-input effects in legacy-control §§9.9–9.13.
use super::*;

pub(super) fn fit(h: Handle, t: Handle, p: Properties<'_>, slots: &[u32], f: &mut impl Foreign) {
    param(h, 0xdacf8f9c, n(u32::from(p.has(0x304))), f);
    if p.has(0x304) {
        for &slot in slots {
            call(f, t, slot, vec![]);
        }
    }
}

/// LC13: the named-font helper's payload is unsettled; never fabricate one.
#[allow(clippy::too_many_arguments)]
pub(super) fn font(h: Handle, t: Handle, p: Properties<'_>, key: u32, selector: u32,
    slot: u32, object: bool, default: Option<u32>, f: &mut impl Foreign) -> Result<()> {
    if let Some(id) = p.number(key) {
        let value = if object { Value::Handle(f.font(id)) } else { n(id) };
        call(f, t, slot, vec![value]);
        param(h, 0xfaa3be85u32.wrapping_add(selector), n(id), f);
    } else if p.text(key).is_some() {
        return Err(Unsupported::Malformed("LC13 named-font helper"));
    } else if let Some(default_slot) = default {
        let id = f.font_id("main9");
        let font = f.font(id);
        call(f, t, default_slot, vec![Value::Handle(font)]);
    }
    Ok(())
}

pub(super) fn colours(t: Handle, p: Properties<'_>, slot: u32, rows: &[(u32,u32)], f: &mut impl Foreign) -> Result<()> {
    for &(key, index) in rows {
        if let Some(c) = colour(p, key)? {
            let c = f.colour(c);
            call(f, t, slot, vec![n(index), n(c)]);
        }
    }
    Ok(())
}
pub(super) fn singles(t: Handle, p: Properties<'_>, rows: &[(u32,u32)], f: &mut impl Foreign) -> Result<()> {
    for &(key, slot) in rows {
        if let Some(c) = colour(p, key)? {
            let c = f.colour(c);
            call(f, t, slot, vec![n(c)]);
        }
    }
    Ok(())
}
pub(super) fn numbers(t: Handle, p: Properties<'_>, rows: &[(u32,u32)], f: &mut impl Foreign) {
    for &(key, slot) in rows {
        if let Some(v) = p.number(key) { call(f, t, slot, vec![n(v)]); }
    }
}
pub(super) fn pair(t: Handle, p: Properties<'_>, key: u32, slot: u32, f: &mut impl Foreign) -> Result<()> {
    if let Some([a,b]) = tuple::<2>(p,key)? { call(f,t,slot,vec![i(a),i(b)]); }
    Ok(())
}
pub(super) fn resource(h: Handle, id: u32, kind: u32, g: u32, inst: u32, f: &mut impl Foreign) {
    param(h,id,Value::Key(ResourceKey { kind, group:g, instance:inst }),f);
}
pub(super) fn run(kind: Kind, h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    match kind {
        Kind::Text => {
            if first {
                for (key,mask) in [(0xf003,1),(0x300,2),(0x301,4)] {
                    if let Some(v)=p.number(key) {
                        call(f,t,0x1c,vec![n(mask),n(u32::from(v!=0))]);
                        if key==0x301 && v==0 { f.write(h,Write::Flag(0x20000,true)); }
                    }
                }
                font(h,t,p,0xf000,0,0x4c,false,None,f)?;
                colours(t,p,0x24,&[(0xf002,0),(0xf001,1)],f)?;
                let [x,y]=tuple::<2>(p,0xf004)?.unwrap_or([0,0]);
                call(f,t,0x5c,vec![i(x),i(y)]);
                pair(t,p,0x305,0x64,f)?;
            } else if let Some(v)=p.number(0xf015) {
                call(f,t,0x54,vec![n(if v==0xf008 {4} else {v})]);
            }
            param(h,0xdacf8f9c,n(u32::from(p.has(0x304))),f);
            if p.has(0x304) { call(f,t,0x14,vec![n(0),n(0)]); }
        }
        Kind::TextEdit => text_edit(h,t,p,first,f)?,
        Kind::ListBox => list(h,t,p,first,f)?,
        Kind::Combo => combo(h,t,p,first,f)?,
        _ => unreachable!(),
    }
    Ok(())
}
fn text_edit(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if !first { fit(h,t,p,&[0xac],f); return Ok(()); }
    let mut flags=word(call(f,t,0x140,vec![]))?;
    for (key,mask) in [(0x400,1),(0x300,2),(0x401,4),(0x402,8),(0xf019,0x10),
        (0x301,0x20),(0x403,0x40),(0x404,0x80),(0x405,0x100),(0x406,0x200),
        (0x415,0x400),(0x416,0x800),(0x417,0x8000),(0x418,0x1000),(0x419,0x2000)] {
        if let Some(v)=p.number(key) { if v==1 { flags|=mask; } else { flags&=!mask; } }
    }
    call(f,t,0x134,vec![n(flags)]);
    font(h,t,p,0xf000,0,0x28,false,None,f)?;
    colours(t,p,0x30,&[(0xf00f,0),(0xf010,2),(0xf011,4),(0xf012,1),(0xf013,3),(0xf014,5)],f)?;
    singles(t,p,&[(0x407,0x48),(0x408,0x5c)],f)?;
    if let Some(c)=colour(p,0x40b)? {
        let outline=p.number(0xf019).ok_or(Unsupported::Malformed("LC11 outline argument"))?;
        let c=f.colour(c);
        call(f,t,0x94,vec![n(u32::from(outline!=0)),n(c)]);
    }
    numbers(t,p,&[(0x409,0x84)],f);
    if let Some([x,y])=tuple::<2>(p,0xf004)? { call(f,t,0xa0,vec![i(x)]); call(f,t,0xa8,vec![i(y)]); }
    if let Some(v)=p.number(0x40c) { call(f,t,0x100,vec![n(u32::from(v==0))]); }
    numbers(t,p,&[(0x40d,0x108)],f);
    pair(t,p,0x40e,0x110,f)?;
    numbers(t,p,&[(0x40f,0x120),(0x410,0x144)],f);
    for (key_id,rect_id,slot,param_id,index) in [(0x413,0x414,0x160,0x8b4bfefc,0),(0x411,0x412,0x15c,0x8b4bfefd,1)] {
        if let Some((g,inst))=key(p,key_id)? {
            let rect=tuple::<4>(p,rect_id)?.unwrap_or([0;4]);
            if let Some(b)=f.load_image(g,inst) { call(f,t,slot,vec![Value::Handle(b),Value::Rect(rect)]); f.release(b); }
            resource(h,param_id,0x856ddbac,g,inst,f); image_key(h,g,inst,index,f);
        }
    }
    Ok(())
}
fn list(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if first {
        call(f,t,0x8c,vec![n(u32::from(p.number(0x402).unwrap_or(1)==1)),n(u32::from(p.number(0x401).unwrap_or(0)==1))]);
        font(h,t,p,0xf000,0,0x98,true,None,f)?;
        colours(t,p,0x9c,&[(0xf00f,0),(0xf010,1),(0xf011,2)],f)?;
        singles(t,p,&[(0x408,0xa8)],f)?;
        if let Some(v)=p.number(0xf019) {call(f,t,0xcc,vec![n(u32::from(v==1))]);}
        if let Some(s)=p.text(0x40b) {
            let c=scan(&s,"(%d,%d,%d)(%d,%d,%d)")?;
            if c.len()==6 { let a=f.colour([c[0] as u8,c[1] as u8,c[2] as u8]); let b=f.colour([c[3] as u8,c[4] as u8,c[5] as u8]); call(f,t,0xdc,vec![n(a),n(b)]); }
        }
        if let Some(v)=p.number(0xf00b) {call(f,t,0xd4,vec![n(u32::from(v==1))]);}
        pair(t,p,0x500,0xe4,f)?; pair(t,p,0xf004,0xec,f)?;
        if let Some(v)=p.number(0x504) {call(f,t,0x44,vec![n(u32::from(v==1))]);}
    } else {
        for (_,s) in p.texts(0x502) {call(f,t,0x1c,vec![text(s),n(0xfffffffe),n(0)]);}
        if p.number(0x503)==Some(1) {call(f,t,0x38,vec![]); call(f,t,0x40,vec![]);}
        if let Some(v)=p.number(0x501) {call(f,t,0x28,vec![n(v),n(1)]);}
        numbers(t,p,&[(0x505,0x6c),(0x506,0x74)],f);
        fit(h,t,p,&[0x24,0xbc,0xc0],f);
    }
    Ok(())
}
fn combo(h: Handle,t: Handle,p: Properties<'_>,first: bool,f: &mut impl Foreign)->Result<()> {
    if first {
        font(h,t,p,0xf000,0,0x2c,true,None,f)?;
        if let Some((g,inst))=key(p,0x800)? {
            let rect=tuple::<4>(p,0x801)?;
            if let Some(b)=f.load_image(g,inst) {call(f,t,0x84,vec![Value::Handle(b),rect.map_or(Value::None,Value::Rect)]);f.release(b);}
            image_key(h,g,inst,0,f);
        }
        numbers(t,p,&[(0x803,0x94)],f); pair(t,p,0xf004,0x9c,f)?;
    } else {
        colours(t,p,0x30,&[(0xf00f,0),(0xf010,1),(0xf011,2)],f)?;
        singles(t,p,&[(0x408,0x40),(0x40b,0x58),(0x802,0x8c)],f)?;
        if let Some(v)=p.number(0x400) {call(f,t,0x54,vec![n(u32::from(v==1))]);}
        for (_,s) in p.texts(0x502) {call(f,t,0x6c,vec![text(s),n(0xfffffffe)]);}
        if let Some(v)=p.number(0x501) {call(f,t,0x70,vec![n(v),n(1)]);}
    }
    Ok(())
}
