// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

const STYLES:[u32;23]=[0,1,2,4,8,0x10,0x20,0x40,0x80,0x100,0x200,0x400,0x800,0x1000,0x2000,0x4000,0x8000,0x10000,0x20000,0x40000,0x80000,0x19bff,0xffffffff];

fn grid_colour(t:Handle,c:[u8;3],f:&mut impl Foreign)->Result<u32>{
    let w=handle(call(f,t,0x14,vec![]))?;Ok(f.grid_colour(w,c))
}
fn decimal(s:&str)->Result<u32>{scan(s,"%d")?.first().copied().ok_or(Unsupported::Malformed("Grid cell decimal"))}

pub(super) fn run(h:Handle,t:Handle,p:Properties<'_>,first:bool,f:&mut impl Foreign)->Result<()> {
    if !first {
        let col=grid_colour(t,colour(p,0x1623)?.unwrap_or([0;3]),f)?;
        let row=grid_colour(t,colour(p,0x1624)?.unwrap_or([0;3]),f)?;
        call(f,t,0xe8,vec![n(col),n(row)]);return Ok(());
    }
    if let Some(c)=colour(p,0x1601)?{let c=grid_colour(t,c,f)?;call(f,t,0x44,vec![n(c)]);}
    if !font_word(h,t,p,0xf000,0,0x3c,f){let font=f.font(0x68963c4c);call(f,t,0x3c,vec![Value::Handle(font)]);}
    if let Some(v)=p.number(0x1602){call(f,t,0xf0,vec![i(v as i32)]);}
    call(f,t,0x54,vec![n(flag(p,0x1603).unwrap_or(false) as u32)]);
    let gutters=tuple::<4>(p,0xf004)?.unwrap_or([0;4]);call(f,t,0x60,gutters.into_iter().map(i).collect());
    call(f,t,0x9c,vec![i(p.number(0x1605).unwrap_or(2) as i32),i(p.number(0x1606).unwrap_or(1) as i32)]);
    let outline=p.number(0x1616).unwrap_or(0)==1;let c=grid_colour(t,colour(p,0x1604)?.unwrap_or([0;3]),f)?;
    call(f,t,0x6c,vec![n(outline as u32),n(c)]);
    let width=p.number(0x1607).unwrap_or(100);
    call(f,t,0x100,vec![i(width as i32)]);call(f,t,0x108,vec![i(p.number(0x1608).unwrap_or(20) as i32)]);
    let hdr=p.number(0x1609).unwrap_or(width); // Q6: shares dcolwidth, not a fresh default.
    let (a,b)=match hdr{0=>(1,1),1=>(1,0),2=>(0,1),_=>(0,0)};
    call(f,t,0x128,vec![n(a),n(b)]);
    call(f,t,0x130,vec![i(p.number(0x160a).unwrap_or(0) as i32),i(p.number(0x160b).unwrap_or(0) as i32)]);
    let mut scratch=None;
    for (k,bit) in STYLES.into_iter().enumerate(){if let Some(value)=p.number(0x160c+k as u32){
        scratch=Some(value);call(f,t,if value!=0{0x28}else{0x30},vec![n(bit)]);
        if bit==4 && value==0{let w=handle(call(f,t,0x14,vec![]))?;call(f,w,0x170,vec![]);}
    }}
    let mut notified=false;let mut resource:[Option<u32>;2]=[None,None];
    for (_,s) in p.texts(0x1625){
        let raw:Vec<_>=s.splitn(11,',').collect();if raw.len()<4{continue;}
        let v=fields(&s,',',11);
        let col=decimal(v[0])? as i32;let row=decimal(v[1])? as i32;
        let flags=scan(v[2],"%x")?.first().copied().ok_or(Unsupported::Malformed("Grid cell flags"))?;
        let data=format!("{},{}",v[3],v[4]);let font_id=f.font_id(v[5]);
        if !notified{param(h,0x287a7136,Value::Notifier(Notifier::Cells),f);notified=true;}
        // Q10: use the LAST PRESENT style WORD VALUE, not the style bit or F.
        // Its initial stack value is unpublished; reject that domain honestly.
        let scratch=scratch.ok_or(Unsupported::Malformed("Grid style scratch uninitialized"))?;
        if scratch&0x40000==0 {
            for (k,value) in scan(&data,"%08x,%08x")?.into_iter().enumerate(){resource[k]=Some(value);}
        }
        let mut descriptor=CellDescriptor{flags,group:0,instance:0,text:String::new()};
        let wrap=n((flags&0x80000!=0) as u32);
        if flags&0x40000!=0 {
            call(f,t,0x1c0,vec![i(col),i(row),text(data.clone()),n(data.len() as u32),n(1),wrap]);
        }else{
            descriptor.group=resource[0].ok_or(Unsupported::Malformed("Grid cell uninitialized resource group"))?;
            descriptor.instance=resource[1].ok_or(Unsupported::Malformed("Grid cell uninitialized resource instance"))?;
            if flags&0x2000!=0 {
                if let Some(b)=f.load_image(descriptor.group,descriptor.instance){call(f,t,0x1c8,vec![i(col),i(row),Value::Handle(b),n(1)]);f.release(b);}
            }else if flags&0x10000!=0 {
                if let Some(s)=f.load_string(descriptor.group,descriptor.instance){let len=s.len();call(f,t,0x1c0,vec![i(col),i(row),text(s),n(len as u32),n(1),wrap]);}
            }
        }
        if flags&0x1000!=0 {
            // Other malformed/unsafe record domains are unsupported, not invented
            // defaults. Only explicitly published constructor defaults delegated.
            let mut values=[0;5];for k in 0..5{values[k]=decimal(v[6+k])?;}
            let font=if font_id!=0{Some(f.font(font_id))}else{None};
            let record=CellData{font,word04:values[0],alignment:if flags&0x100000!=0{1}else{255},bytes15_17:[values[1] as u8,values[2] as u8,values[3] as u8],word18:values[4]};
            call(f,t,0x154,vec![i(col),i(row),Value::CellData(record)]);
        }
        f.notify_cell(h,col,row,descriptor);
    }
    for (key,kind,param_key,slot) in [(0x1626,Notifier::Columns,0xc88ccb6d,0x104),(0x1627,Notifier::Rows,0xc88ccb6e,0x10c)]{
        let mut notified=false;
        for (_,s) in p.texts(key){let values=scan(&s,"%d,%d,%d")?;if values.len()!=3{continue;}
            let [a,b,size]=[values[0] as i32,values[1] as i32,values[2] as i32];
            if !notified{param(h,param_key,Value::Notifier(kind),f);notified=true;}
            f.notify_axis(h,kind,[a,b,size]);
            let count=if kind==Notifier::Columns{1}else{b.wrapping_sub(a).wrapping_add(1)};
            call(f,t,slot,vec![i(a),i(count),i(size)]);
        }
    }
    Ok(())
}
