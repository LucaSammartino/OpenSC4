// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded CRT scanf subset used by the published effect tables. Assignment
//! counts and partial writes matter; no general-purpose script parser lives here.
use super::*;

/// Return assigned fields, stopping at the first failed conversion or literal.
/// Overflow is outside the published safe domain (LC9), not wrapping garbage.
pub(super) fn scan(text:&str,pattern:&str)->Result<Vec<u32>> {
    let b=text.as_bytes(); let p=pattern.as_bytes(); let (mut t,mut k)=(0,0); let mut out=vec![];
    while k<p.len() {
        if p[k].is_ascii_whitespace() {
            while t<b.len() && b[t].is_ascii_whitespace(){t+=1;} k+=1;continue;
        }
        if p[k]!=b'%' {if b.get(t)!=Some(&p[k]){break;}t+=1;k+=1;continue;}
        k+=1;let mut width=0usize;
        while k<p.len() && p[k].is_ascii_digit(){width=width*10+(p[k]-b'0') as usize;k+=1;}
        if k==p.len(){return Err(Unsupported::Malformed("scan pattern"));}
        let radix=if p[k]==b'x'{16}else{10};k+=1;
        while t<b.len() && b[t].is_ascii_whitespace(){t+=1;}
        let start=t;let end=if width==0{b.len()}else{b.len().min(t+width)};
        let negative=if t<end && (b[t]==b'-'||b[t]==b'+'){let n=b[t]==b'-';t+=1;n}else{false};
        if radix==16 && t+2<=end && b[t]==b'0' && matches!(b[t+1],b'x'|b'X'){t+=2;}
        let digits=t;let mut value=0u64;
        while t<end {
            let d=match b[t]{b'0'..=b'9'=>(b[t]-b'0') as u32,b'a'..=b'f'=>(b[t]-b'a'+10) as u32,b'A'..=b'F'=>(b[t]-b'A'+10) as u32,_=>break};
            if d>=radix{break;}
            value=value.checked_mul(radix as u64).and_then(|v|v.checked_add(d as u64)).ok_or(Unsupported::Malformed("numeric overflow"))?;
            t+=1;
        }
        if t==digits{let _=start;break;}
        let limit=if radix==10{if negative{0x8000_0000}else{0x7fff_ffff}}else{0xffff_ffff};
        if value>limit{return Err(Unsupported::Malformed("numeric overflow"));}
        out.push(if negative{0u32.wrapping_sub(value as u32)}else{value as u32});
    }
    Ok(out)
}
pub(super) fn tuple<const N:usize>(p:Properties<'_>,key:u32)->Result<Option<[i32;N]>> {
    let Some(s)=p.text(key) else{return Ok(None);};
    let pattern=match N{2=>"(%d,%d)",3=>"(%d,%d,%d)",4=>"(%d,%d,%d,%d)",_=>return Err(Unsupported::Malformed("tuple size"))};
    let v=scan(&s,pattern)?;
    Ok((v.len()==N).then(||std::array::from_fn(|k|v[k] as i32)))
}
pub(super) fn colour(p:Properties<'_>,key:u32)->Result<Option<[u8;3]>> {
    Ok(tuple::<3>(p,key)?.map(|c|c.map(|v|v as u8)))
}
pub(super) fn key(p:Properties<'_>,id:u32)->Result<Option<(u32,u32)>> {
    let Some(s)=p.text(id) else{return Ok(None);}; let v=scan(&s,"{%x,%x}")?;
    Ok((v.len()==2).then(||(v[0],v[1])))
}
pub(super) fn zero_scan<const N:usize>(s:&str,pattern:&str)->Result<[u32;N]> {
    let v=scan(s,pattern)?;Ok(std::array::from_fn(|k|v.get(k).copied().unwrap_or(0)))
}
pub(super) fn fields(text:&str,separator:char,count:usize)->Vec<&str> {
    let mut v:Vec<_>=text.splitn(count,separator).collect();v.resize(count,"");v
}
