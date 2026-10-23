// SPDX-License-Identifier: GPL-3.0-or-later
//! Particle warp/source/emit/force, effects §§19.43–51. Store boundaries are
//! explicit; numerical qualification of extended arithmetic remains EFP7.
use super::*;
use particle::bounded;
use resources::vector2;

fn extended(reply: Reply) -> Result<f64> {
    match reply {
        Reply::Float(x) => Ok(x),
        Reply::Float32(x) => Ok(f64::from(f32::from_bits(x))),
        _ => Err(Error::Unsupported("extended reply")),
    }
}
fn wide(v: [f32; 3]) -> [f64; 3] {
    v.map(f64::from)
}
fn square(v: [f64; 3], order: [usize; 3]) -> f64 {
    (v[order[0]] * v[order[0]] + v[order[1]] * v[order[1]]) + v[order[2]] * v[order[2]]
}
fn rounded(x: f64) -> f64 {
    f64::from(x as f32)
}

impl Commands {
    fn pv(&mut self, at: u32, v: [f32; 3]) {
        for (i, x) in v.into_iter().enumerate() {
            self.p_set(at + 4 * i as u32, x);
        }
    }
    fn read_pv(&mut self, at: u32) -> Result<[f32; 3]> {
        let mut v = [0.; 3];
        for (i, x) in v.iter_mut().enumerate() {
            *x = self
                .particle_desc()
                .get_scalar(at + 4 * i as u32)
                .ok_or(Error::Unsupported("indeterminate particle scalar"))?;
        }
        Ok(v)
    }
    fn opt(
        &mut self,
        tokens: &mut [ScriptToken],
        key: &[u8],
        count: usize,
        locale: &mut impl CharacterRules,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        if self.switch(tokens, key, count, locale)? {
            Ok(Some(self.selected_bytes(tokens)?))
        } else {
            Ok(None)
        }
    }
    pub fn particle_warp(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if let Some(a) = self.opt(tokens, b"screw", 1, locale)? {
            let x = raw_float(host, &a[0])?;
            self.p_set(0x104, (x * 0.02) as f32);
        }
        if let Some(a) = self.opt(tokens, b"wiggle", 4, locale)? {
            let w2 = float(host, &a[0])?;
            let w0 = float(host, &a[1])?;
            let amp = float(host, &a[2])?;
            let phase = bounded(host, &a[3], 0., 1.)? as f32;
            let angle = f64::from(phase) * f64::from(f32::from_bits(0x40c90fdb));
            let c = extended(host.call(Call::Cos(angle))?)?;
            let s = extended(host.call(Call::Sin(angle))?)?;
            self.particle_desc().wiggles.push([
                w0,
                0.,
                w2,
                0.,
                (c * f64::from(amp)) as f32,
                0.,
                (-s * f64::from(amp)) as f32,
            ]);
        }
        if let Some(a) = self.opt(tokens, b"wiggleDir", 2, locale)? {
            let w0 = float(host, &a[0])?;
            let v = vector(host, &a[1])?;
            let u = if a.len() > 2 { vector(host, &a[2])? } else { [0.; 3] };
            self.particle_desc()
                .wiggles
                .push([w0, u[0], u[1], u[2], v[0], v[1], v[2]]);
        }
        if self.switch(tokens, b"wiggleVerts", 0, locale)? {
            self.p_or(8, 0x100);
        }
        if let Some(a) = self.opt(tokens, b"uv", 1, locale)? {
            let x = float(host, &a[0])?;
            self.p_set(0x114, x);
            // Literal minimum-one plus unchecked second access. No invented
            // second-operand parser exception or default (EC219).
            let v = vector2(
                host,
                a.get(1).ok_or(Error::Unsupported("unchecked uv second index"))?.clone(),
            )?;
            self.p_set(0x118, v[0]);
            self.p_set(0x11c, v[1]);
            self.p_or(8, 0x100);
        }
        if let Some(a) = self.opt(tokens, b"alpha", 2, locale)? {
            let v = wide(vector(host, &a[0])?);
            let r = 1. / square(v, [0, 1, 2]).sqrt();
            for i in [2, 0, 1] {
                self.p_set(0x120 + 4 * i as u32, (r * v[i]) as f32);
            }
            self.particle_desc().floats.entry(0x12c).or_default().clear();
            for arg in &a[1..] {
                let x = bounded(host, arg, 0., 1.)?;
                self.particle_desc().floats.get_mut(&0x12c).unwrap().push(x as f32);
            }
            self.p_or(8, 0x100);
        }
        Ok(())
    }
    pub fn particle_source(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        let mut lo = [-1., 0., -1.];
        let mut hi = [1., 0., 1.];
        for (key, min) in [
            (b"point".as_slice(), 0),
            (b"square", 1),
            (b"quad", 1),
            (b"cube", 1),
            (b"box", 1),
            (b"dice", 1),
            (b"model", 0),
            (b"modelBase", 0),
            (b"city", 0),
            (b"cityWindySide", 0),
        ] {
            let a = match self.opt(tokens, key, min, locale)? {
                Some(a) => a,
                None => continue,
            };
            match key {
                b"point" => {
                    lo = if a.is_empty() { [0.; 3] } else { vector(host, &a[0])? };
                    hi = lo;
                }
                b"square" | b"quad" | b"cube" | b"box" => {
                    hi = match key {
                        b"quad" => {
                            let v = vector2(host, a[0].clone())?;
                            [v[0], 0., v[1]]
                        }
                        b"box" => vector(host, &a[0])?,
                        _ => {
                            let r = float(host, &a[0])?;
                            [r, if key == b"cube" { r } else { 0. }, r]
                        }
                    };
                    lo = hi.map(|x| -x);
                    // The square/quad Y coordinate is an explicit positive zero.
                    if matches!(key, b"square" | b"quad") {
                        lo[1] = 0.;
                    }
                    if a.len() > 1 {
                        let center = vector(host, &a[1])?;
                        for i in 0..3 {
                            lo[i] = (f64::from(lo[i]) + f64::from(center[i])) as f32;
                            hi[i] = (f64::from(hi[i]) + f64::from(center[i])) as f32;
                        }
                    }
                }
                b"dice" => {
                    let x = float(host, &a[0])?;
                    self.p_set(0x1d8, x);
                    lo = [0.; 3];
                    hi = lo;
                }
                b"model" => self.p_or(0, 0x40),
                b"modelBase" => self.p_or(0, 0x80),
                _ => {
                    self.p_or(0, 0x1000);
                    if key == b"cityWindySide" {
                        self.p_or(0, 0x2000);
                    }
                    if !a.is_empty() {
                        let [a, b] = self.pair(tokens, false, host)?;
                        lo[1] = a;
                        hi[1] = b;
                    }
                }
            }
            break;
        }
        self.pv(0x50, lo);
        self.pv(0x5c, hi);
        for (key, bit) in [
            (b"scaleParticles".as_slice(), 0x400000),
            (b"pinToTerrain", 0x4000),
            (b"pinToWater", 0x8000),
        ] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, bit);
            }
        }
        for (key, at, bit) in [
            (b"terrainOnly".as_slice(), 0x158, 0),
            (b"waterOnly", 0x15c, 0),
            (b"seaOnly", 0x15c, 0x20000),
            (b"lakeOnly", 0x15c, 0x40000),
        ] {
            if self.switch(tokens, key, 0, locale)? {
                self.p_or(0, 0x10000);
                self.p_set(at, 0.);
                self.p_or(0, bit);
            }
        }
        for (key, at) in [
            (b"belowHeight".as_slice(), 0x15c),
            (b"aboveHeight", 0x158),
            (b"heightRange", 0x158),
        ] {
            if let Some(a) = self.opt(tokens, key, if key == b"heightRange" { 2 } else { 1 }, locale)? {
                self.p_or(0, 0x10000);
                let x = float(host, &a[0])?;
                self.p_set(at, x);
                if key == b"heightRange" {
                    let x = float(host, &a[1])?;
                    self.p_set(at + 4, x);
                }
            }
        }
        if self.switch(tokens, b"killOutsideCity", 0, locale)? {
            self.p_or(0, 0x800);
        }
        if self.switch(tokens, b"resetIncoming", 0, locale)? {
            self.p_or(8, 0x200);
        }
        Ok(())
    }
    fn rate_values(&mut self, a: &[Vec<u8>], inject: bool, host: &mut impl Services) -> Result<()> {
        if inject {
            self.particle_desc().floats.entry(0x88).or_default().resize(1, 0.);
            let x = float(host, &a[0])?;
            self.particle_desc().floats.get_mut(&0x88).unwrap()[0] = x;
            self.p_or(0, 2);
        } else {
            self.particle_desc().floats.entry(0x88).or_default().clear();
            for arg in a {
                let x = float(host, arg)?;
                self.particle_desc().floats.get_mut(&0x88).unwrap().push(x);
            }
        }
        Ok(())
    }
    pub fn particle_emit(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if let Some(a) = self.opt(tokens, b"inject", 1, locale)? {
            self.rate_values(&a, true, host)?;
        } else if let Some(a) = self.opt(tokens, b"rate", 1, locale)? {
            self.rate_values(&a, false, host)?;
        }
        self.particle_rate_options(tokens, locale, host)?;
        if let Some(a) = self.opt(tokens, b"velocity", 1, locale)? {
            let v = wide(vector(host, &a[0])?);
            let m = if a.len() > 1 { float(host, &a[1])? } else { 1. };
            let t = if a.len() > 2 { raw_float(host, &a[2])? } else { 0. };
            let f = 1. / square(v, [0, 2, 1]).sqrt();
            let n = [v[0] * f, rounded(v[1] * f), rounded(v[2] * f)];
            self.pv(0x30, n.map(|x| (x - t) as f32));
            self.pv(0x3c, n.map(|x| (x + t) as f32));
            self.p_set(0x48, ((1. - t) * f64::from(m)) as f32);
            self.p_set(0x4c, ((1. + t) * f64::from(m)) as f32);
        }
        if let Some(a) = self.opt(tokens, b"speed", 1, locale)? {
            let m = f64::from(float(host, &a[0])?);
            let r = if a.len() > 1 {
                f64::from(float(host, &a[1])?)
            } else {
                0.
            };
            self.p_set(0x48, (m - r) as f32);
            self.p_set(0x4c, (m + r) as f32);
        }
        if let Some(a) = self.opt(tokens, b"dir", 1, locale)? {
            let mut v = wide(vector(host, &a[0])?);
            let mut r = [0.; 3];
            if a.len() > 1 {
                let split = match host.call(Call::SplitArguments(a[1].clone()))? {
                    Reply::Arguments(v) => v,
                    _ => return Err(Error::Unsupported("split arguments reply")),
                };
                match split.len() {
                    1 => {
                        r = [float(host, &split[0])?; 3];
                    }
                    3 => {
                        for i in [2, 1, 0] {
                            r[i] = float(host, &split[i])?;
                        }
                    }
                    _ => {
                        return Err(Error::Parse(
                            "Expecting float or vector3 for second -dir argument".into(),
                        ))
                    }
                }
            }
            let q = square(v, [2, 1, 0]);
            if q > 0. {
                let f = 1. / q.sqrt();
                v = [v[0] * f, rounded(v[1] * f), rounded(v[2] * f)];
            }
            self.pv(0x30, std::array::from_fn(|i| (v[i] - f64::from(r[i])) as f32));
            self.pv(0x3c, std::array::from_fn(|i| (v[i] + f64::from(r[i])) as f32));
        }
        if let Some(a) = self.opt(tokens, b"vel", 1, locale)? {
            let mut v = wide(vector(host, &a[0])?);
            let q = square(v, [2, 1, 0]);
            let mut m = 0.;
            if q > 0. {
                m = rounded(q).sqrt();
                let f = 1. / m;
                v = v.map(|x| rounded(x * f));
            }
            self.p_set(0x48, m as f32);
            self.p_set(0x4c, m as f32);
            self.pv(0x30, v.map(|x| x as f32));
            self.pv(0x3c, v.map(|x| x as f32));
        }
        if self.switch(tokens, b"base", 0, locale)? {
            self.p_or(0, 0x20);
        }
        Ok(())
    }
    pub fn particle_force(
        &mut self,
        tokens: &mut [ScriptToken],
        locale: &mut impl CharacterRules,
        host: &mut impl Services,
    ) -> Result<()> {
        if self.switch(tokens, b"reset", 0, locale)? {
            self.pv(0xe0, [0.; 3]);
            self.particle_desc().tractors.clear();
        }
        if let Some(a) = self.opt(tokens, b"gravity", 1, locale)? {
            let g = raw_float(host, &a[0])?;
            let y = self.read_pv(0xe0)?[1];
            self.p_set(0xe4, (f64::from(y) - g) as f32);
        }
        if let Some(a) = self.opt(tokens, b"wind", 1, locale)? {
            let v = wide(vector(host, &a[0])?);
            let f = 1. / square(v, [0, 1, 2]).sqrt();
            let mut n = [rounded(v[0] * f), rounded(v[1] * f), v[2] * f];
            if a.len() > 1 {
                let m = raw_float(host, &a[1])?;
                n = [rounded(n[0] * m), rounded(n[1] * m), rounded(n[2]) * m];
            }
            let old = self.read_pv(0xe0)?;
            self.pv(0xe0, std::array::from_fn(|i| (f64::from(old[i]) + n[i]) as f32));
        }
        for (key, at) in [
            (b"global_wind".as_slice(), 0xec),
            (b"screw", 0x104),
            (b"bomb", 0xf0),
            (b"drag", 0x100),
        ] {
            if let Some(a) = self.opt(tokens, key, 1, locale)? {
                let x = raw_float(host, &a[0])?;
                self.p_set(at, (if key == b"screw" { x * 0.02 } else { x }) as f32);
                if key == b"bomb" && a.len() > 1 {
                    let v = vector(host, &a[1])?;
                    self.pv(0xf4, v);
                }
            }
        }
        for (key, min) in [
            (b"motherDuck".as_slice(), 1),
            (b"attractor", 1),
            (b"alphaAttractor", 1),
            (b"automata", 2),
        ] {
            let a = match self.opt(tokens, key, min, locale)? {
                Some(a) => a,
                None => continue,
            };
            if key == b"automata" {
                if self.particle_desc().floats.entry(0x1a4).or_default().is_empty()
                    || self.particle_desc().words.get(&0x1b4).copied().unwrap_or(0) != 0
                {
                    self.p_set(0x1b0, 1.);
                    let id = uint(host, &a[0])?;
                    self.particle_desc().words.insert(0x1b4, id);
                    self.particle_desc().floats.entry(0x1a4).or_default().clear();
                    let x = float(host, &a[1])?;
                    self.particle_desc().floats.get_mut(&0x1a4).unwrap().push(x);
                }
            } else {
                self.force_list(&a, 0x1b0, 0x1a4, host)?;
                if key == b"motherDuck" {
                    self.p_or(0, 0x8000000);
                } else if key == b"alphaAttractor" {
                    self.p_or(0, 0x4000000);
                    self.p_or(8, 0x10);
                }
            }
            break;
        }
        let relative = self.switch(tokens, b"tractorRel", 2, locale)?;
        if relative || self.switch(tokens, b"tractor", 2, locale)? {
            let a = self.selected_bytes(tokens)?;
            self.tractor(relative, &a, host)?;
        }
        if let Some(a) = self.opt(tokens, b"tractorResetSpeed", 1, locale)? {
            let x = float(host, &a[0])?;
            self.p_set(0x1c4, x);
        }
        if let Some(a) = self.opt(tokens, b"tractorTimeScale", 1, locale)? {
            let t = float(host, &a[0])?;
            self.scalar_498 = t;
            self.word_494 = (f32::from_bits(self.word_494) * t).to_bits();
            for at in [0xc, 0x10] {
                let old = self
                    .particle_desc()
                    .get_scalar(at)
                    .ok_or(Error::Unsupported("indeterminate lifetime"))?;
                self.p_set(at, (f64::from(old) * f64::from(t)) as f32);
            }
            let r = 1. / f64::from(t);
            for record in &mut self.particle_desc().tractors {
                for x in &mut record[3..6] {
                    *x = (f64::from(*x) * r) as f32;
                }
                for x in &mut record[6..8] {
                    *x = (f64::from(*x) * f64::from(t)) as f32;
                }
            }
        }
        if let Some(a) = self.opt(tokens, b"terrainRepel", 2, locale)? {
            for (i, arg) in a.iter().take(5).enumerate() {
                let x = float(host, arg)?;
                self.p_set(0x13c + i as u32 * 4, x);
                if i == 1 {
                    self.p_or(0, 0x400);
                }
            }
        }
        if let Some(a) = self.opt(tokens, b"explosion", 2, locale)? {
            self.force_list(&a, 0x1ec, 0x1f4, host)?;
        }
        if let Some(a) = self.opt(tokens, b"explosionFront", 2, locale)? {
            let x = float(host, &a[0])?;
            self.p_set(0x200, x);
            let x = float(host, &a[1])?;
            self.p_set(0x1f0, x);
        }
        Ok(())
    }
    fn force_list(&mut self, a: &[Vec<u8>], at: u32, list: u32, host: &mut impl Services) -> Result<()> {
        let x = float(host, &a[0])?;
        self.p_set(at, x);
        self.particle_desc().floats.entry(list).or_default().clear();
        for arg in &a[1..] {
            let x = float(host, arg)?;
            self.particle_desc().floats.get_mut(&list).unwrap().push(x);
        }
        Ok(())
    }
    fn tractor(&mut self, relative: bool, a: &[Vec<u8>], host: &mut impl Services) -> Result<()> {
        let mut p = vector(host, &a[0])?;
        let count = match host.call(Call::CountArguments(a[1].clone()))? {
            Reply::Int(n) => n,
            Reply::Uint(n) => n as i32,
            _ => return Err(Error::Unsupported("argument count reply")),
        };
        let s = f64::from(self.scalar_498);
        let previous = self.particle_desc().tractors.last().copied();
        let tangent = if count == 1 {
            let magnitude = rounded(raw_float(host, &a[1])? / s);
            let center = if let Some(old) = previous {
                [old[0], old[1], old[2]]
            } else {
                let lo = wide(self.read_pv(0x50)?);
                let hi = wide(self.read_pv(0x5c)?);
                [
                    ((lo[0] + hi[0]) * 0.5) as f32,
                    ((lo[1] + hi[1]) * 0.5) as f32,
                    (rounded(lo[2] + hi[2]) * 0.5) as f32,
                ]
            };
            let d = wide(std::array::from_fn(|i| (f64::from(p[i]) - f64::from(center[i])) as f32));
            let q = square(d, [1, 2, 0]);
            if q > 0. {
                d.map(|x| (x * (magnitude / q.sqrt())) as f32)
            } else {
                d.map(|x| x as f32)
            }
        } else {
            let v = wide(vector(host, &a[1])?);
            let r = 1. / s;
            v.map(|x| (x * r) as f32)
        };
        if relative {
            if let Some(old) = previous {
                for i in 0..3 {
                    p[i] = (f64::from(p[i]) + f64::from(old[i])) as f32;
                }
            }
        }
        let start = f32::from_bits(self.word_494);
        let mut end = 0.;
        if a.len() > 2 {
            let duration = raw_float(host, &a[2])?;
            end = (duration * s + f64::from(start)) as f32;
            self.word_494 = end.to_bits();
        }
        self.particle_desc()
            .tractors
            .push([p[0], p[1], p[2], tangent[0], tangent[1], tangent[2], start, end]);
        self.p_or(0, 0x10000000);
        Ok(())
    }
}
#[cfg(test)]
mod tests;
