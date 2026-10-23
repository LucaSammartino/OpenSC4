// SPDX-License-Identifier: GPL-3.0-or-later
use super::{numeric::X, *};

const IDENTITY: [f32; 9] = [1., 0., 0., 0., 1., 0., 0., 0., 1.];

impl Default for InstanceState {
    fn default() -> Self {
        // The base leaves bytes 2..3 unwritten. Zero is a portable choice for
        // those bytes only; supplied records preserve them (model-instances §1.1).
        let mut draw_attributes = [0; 20];
        for offset in [4, 8, 12, 16] {
            draw_attributes[offset..offset + 4].copy_from_slice(&1_f32.to_le_bytes());
        }
        Self {
            playback: Playback {
                frame: 0,
                position: 0.,
                rate: f32::from_bits(0x3a83126f),
                direction: 1,
                play_mode: 3,
                frame_skipping: false,
            },
            zoom: 255,
            rotation: 255,
            flags: 0,
            alpha: 1.,
            highlight: 0,
            custom_data: 0,
            properties: 1,
            draw_attributes,
            transform: Transform {
                kind: 0,
                rotation: IDENTITY,
                translation: [0.; 3],
                scale: 1.,
                change: 0,
            },
            bounds: Bounds {
                min: [0.; 3],
                max: [1.; 3],
            },
        }
    }
}

impl<O> Default for AnimatedSprite<O> {
    fn default() -> Self {
        let mut sprite = Self::from_snapshot(InstanceState::default(), None);
        sprite.cache = Some(SpriteCache {
            metric: StoredMetric {
                page: 0,
                width: 0.,
                height: 0.,
                offset: [0.; 2],
            },
            // AS3: global corner initializer is unspecified. These are never
            // consumed before initial view refresh; pre-refresh UpdateView is open.
            local: [[0.; 3]; 4],
            vertices: [DrawVertex {
                position: [0.; 3],
                colour: u32::MAX,
                uv: [0.; 2],
            }; 4],
            view_dirty: true,
            counter: 255,
        });
        sprite
    }
}

impl<O> AnimatedSprite<O> {
    pub fn set_position(&mut self, position: Vec3) -> u32 {
        self.state.transform.translation = position;
        self.state.transform.kind |= 1;
        self.state.transform.change = self.state.transform.change.wrapping_add(1);
        1
    }
    pub fn set_transform(&mut self, input: Transform) {
        let change = self.state.transform.change.wrapping_add(1);
        self.state.transform = Transform { change, ..input };
    }
    pub fn get_transform(&self, output: &mut Transform) {
        *output = Transform {
            change: output.change.wrapping_add(1),
            ..self.state.transform
        };
    }
    pub fn bounds(&self) -> Bounds {
        self.state.bounds
    }
    pub fn world_bounds(&self) -> Result<Bounds, Unsupported> {
        let p = self.state.transform.translation;
        let bounds = Bounds {
            min: std::array::from_fn(|i| X::f(self.state.bounds.min[i]).add(X::f(p[i])).store()),
            max: std::array::from_fn(|i| X::f(self.state.bounds.max[i]).add(X::f(p[i])).store()),
        };
        if bounds.min.into_iter().chain(bounds.max).any(|x| !x.is_finite()) {
            return Err(Unsupported::Arithmetic);
        }
        Ok(bounds)
    }
    pub fn resource_model(&self) -> Option<()> {
        None
    }
    pub fn sort_value(&self) -> Result<u32, Unsupported> {
        match (&self.loaded.binding, &self.cache) {
            (Some(binding), Some(cache)) => binding
                .table
                .get(cache.metric.page as usize)
                .map(|t| t.0 as u32)
                .ok_or(Unsupported::Texture),
            _ => Ok(0),
        }
    }

    /// Â§Â§3.1.1â€“3.1.3: exact stage/spill schedule on the specified hardware.
    /// Safe API takes disjoint angle values; native pointer-alias ABI is not exposed.
    pub fn set_orientation(&mut self, angles: Vec3) -> Result<u32, Unsupported> {
        let mask = angles
            .iter()
            .enumerate()
            .fold(0_u8, |m, (i, a)| m | if a.to_bits() == 0xc2c80000 { 0 } else { 1 << i });
        if mask == 0 {
            return Ok(0);
        }
        if angles
            .iter()
            .any(|a| !a.is_finite() || a.abs() >= 9_223_372_036_854_775_808.0)
        {
            return Err(Unsupported::Arithmetic);
        }
        // AS6 explicitly leaves hardware approximation/native qualification
        // open. Without unsafe or host-trig substitution only exact zero-angle
        // primitives are currently supported by the safe owner entry.
        if angles.iter().any(|a| a.to_bits() != 0xc2c80000 && *a != 0.) {
            return Err(Unsupported::Arithmetic);
        }
        let mut next = self.state.transform;
        let t = &mut next;
        if mask == 7 {
            let ca = X::f(angles[0]).cos().b();
            let sa = X::f(angles[0]).sin().b();
            let cp = X::f(angles[1]).cos().b();
            let sp = X::f(angles[1]).sin();
            let cr = X::f(angles[2]).cos();
            let sr = X::f(angles[2]).sin();
            let e = sp.mul(ca);
            let f = sp.mul(sa);
            t.rotation = [
                cr.mul(cp).store(),
                sr.mul(cp).neg().store(),
                sp.store(),
                f.mul(cr).add(sr.mul(ca)).store(),
                cr.mul(ca).sub(f.mul(sr)).store(),
                cp.mul(sa).neg().store(),
                sr.mul(sa).sub(e.mul(cr)).store(),
                e.mul(sr).add(cr.mul(sa)).store(),
                cp.mul(ca).store(),
            ];
            t.kind |= 4;
            t.change = t.change.wrapping_add(1);
        } else {
            if t.kind & 4 != 0 {
                t.rotation = IDENTITY;
                t.kind &= !4;
                t.change = t.change.wrapping_add(1);
            }
            for (axis, angle) in angles.into_iter().enumerate() {
                if mask & (1 << axis) != 0 {
                    let c = X::f(angle).cos().b();
                    let s = X::f(angle).sin();
                    stage(&mut t.rotation, axis, c, s);
                    t.kind |= 4;
                    t.change = t.change.wrapping_add(1);
                }
            }
        }
        if next.rotation.iter().any(|x| !x.is_finite()) {
            return Err(Unsupported::Arithmetic);
        }
        self.state.transform = next;
        Ok(0)
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) -> Result<u32, Unsupported> {
        fn normalize(v: Vec3) -> Result<Vec3, Unsupported> {
            let x = v.map(X::f);
            let length = x[0].mul(x[0]).add(x[1].mul(x[1])).add(x[2].mul(x[2])).sqrt();
            let reciprocal = X::f(1.).div(length);
            let out = x.map(|a| a.mul(reciprocal).store());
            if out.iter().any(|a| !a.is_finite()) {
                return Err(Unsupported::Arithmetic);
            }
            Ok(out)
        }
        fn cross(a: Vec3, b: Vec3) -> Vec3 {
            std::array::from_fn(|i| {
                let j = (i + 1) % 3;
                let k = (i + 2) % 3;
                X::f(a[j]).mul(X::f(b[k])).sub(X::f(a[k]).mul(X::f(b[j]))).store()
            })
        }
        // Literal interpretation of Â§3.1's binary32 difference/normalization
        // intermediates; exact omitted spill details are tracked by AS10.
        let back = normalize(std::array::from_fn(|i| {
            X::f(self.state.transform.translation[i]).sub(X::f(target[i])).store()
        }))?;
        let right = normalize(cross(up, back))?;
        let true_up = cross(back, right);
        if true_up.iter().any(|a| !a.is_finite()) {
            return Err(Unsupported::Arithmetic);
        }
        self.state.transform.rotation = std::array::from_fn(|i| [right, true_up, back][i % 3][i / 3]);
        self.state.transform.kind |= 4;
        self.state.transform.change = self.state.transform.change.wrapping_add(2);
        Ok(2)
    }
}

// Snapshot columns before any in-place writes, with the axis-specific B stores.
pub(crate) fn stage(m: &mut [f32; 9], axis: usize, c: X, s: X) {
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let u: [X; 3] = std::array::from_fn(|i| X::f(m[i * 3 + a]));
    let v: [X; 3] = std::array::from_fn(|i| X::f(m[i * 3 + b]));
    let t = [v[0].mul(s), v[1].mul(s).b(), v[2].mul(s).b()];
    let q = [u[0].mul(c), u[1].mul(c).b(), u[2].mul(c).b()];
    for i in 0..3 {
        m[i * 3 + a] = if axis == 1 { q[i].sub(t[i]) } else { t[i].add(q[i]) }.store();
    }
    let p = v.map(|a| a.mul(c).b());
    let h = if axis == 1 {
        [u[0].mul(s).b(), u[1].mul(s).b(), u[2].mul(s)]
    } else {
        let n = s.neg().b();
        [u[0].mul(n), u[1].mul(n).b(), u[2].mul(n).b()]
    };
    for i in 0..3 {
        m[i * 3 + b] = h[i].add(p[i]).store();
    }
}
