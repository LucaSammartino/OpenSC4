// SPDX-License-Identifier: GPL-3.0-or-later
//! How a model names its textures and how an exemplar names its model (`docs/specs/graphics-formats.md` §4).
//!
//! - [`resolve_texture`]: a material texture ID → FSH key, in the §4.1 order: the shared texture group
//!   `0x1ABE787D`, then the model's own group, then the fallback texture `0x07F94200` (and the slot's ID becomes
//!   0 so it is not looked up again); no texture when the fallback does not load either (FS13).
//! - [`model_key`]: the RKT0–RKT3 instance arithmetic of §4.2, with the zoom remap by model detail
//!   ([`remap_zoom`]) and the view rotation ([`view_rotation`]).

use crate::tgi::{Tgi, GROUP_TEXTURES, TYPE_FSH};

/// The instance of the fallback texture (§4.1; shipped in `SimCity_3.dat`, 32 × 32 DXT1).
pub const FALLBACK_TEXTURE_INSTANCE: u32 = 0x07F9_4200;
/// The fallback texture's key.
pub const FALLBACK_TEXTURE: Tgi = Tgi::new(TYPE_FSH, GROUP_TEXTURES, FALLBACK_TEXTURE_INSTANCE);

/// Which step of §4.1 found the texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureStep {
    /// (`0x7AB50E44`, `0x1ABE787D`, ID).
    SharedGroup = 1,
    /// (`0x7AB50E44`, the model's group, ID).
    ModelGroup = 2,
    /// The fallback texture.
    Fallback = 3,
}

/// The result of [`resolve_texture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureBinding {
    /// The FSH to bind.
    pub key: Tgi,
    pub step: TextureStep,
    /// The slot's texture ID afterwards: unchanged, or 0 after the fallback.
    pub slot_id: u32,
}

/// Resolves a material texture slot (§4.1). `loads` says whether an FSH key loads (exists and passes the
/// factory). `None` means the slot draws without a texture and its ID is (or becomes) 0: a slot with ID 0 is not
/// looked up, and when neither key nor the fallback loads, the original leaves the slot with no binding and its
/// ID set to 0, so it is never retried (FS13; FreeCity4 resolves it to no texture *\[choice\]*).
pub fn resolve_texture(
    model_group: u32,
    texture_id: u32,
    mut loads: impl FnMut(&Tgi) -> bool,
) -> Option<TextureBinding> {
    if texture_id == 0 {
        return None;
    }
    let shared = Tgi::new(TYPE_FSH, GROUP_TEXTURES, texture_id);
    if loads(&shared) {
        return Some(TextureBinding {
            key: shared,
            step: TextureStep::SharedGroup,
            slot_id: texture_id,
        });
    }
    let own = Tgi::new(TYPE_FSH, model_group, texture_id);
    if loads(&own) {
        return Some(TextureBinding {
            key: own,
            step: TextureStep::ModelGroup,
            slot_id: texture_id,
        });
    }
    // FS13: the fallback lookup's result is not tested by the original; a fallback that does not load leaves
    // the slot untextured.
    if !loads(&FALLBACK_TEXTURE) {
        return None;
    }
    Some(TextureBinding {
        key: FALLBACK_TEXTURE,
        step: TextureStep::Fallback,
        slot_id: 0,
    })
}

/// The texture binding cache key (§2.10): (group, instance); the type is ignored.
pub fn binding_cache_key(key: &Tgi) -> (u32, u32) {
    (key.group, key.instance)
}

/// Resource-key-type properties that name an occupant's model (§4.2).
pub const RKT0: u32 = 0x2781_2820;
pub const RKT1: u32 = 0x2781_2821;
pub const RKT2: u32 = 0x2781_2822;
pub const RKT3: u32 = 0x2781_2823;
/// Not handled by [`model_key`] (GF7).
pub const RKT4: u32 = 0x2781_2824;
/// Not handled by [`model_key`] (GF7).
pub const RKT5: u32 = 0x2781_2825;

/// The zoom remap table, indexed by zoom + 5 × detail (§4.2): detail 0 → 0, 1, 2, 3, 4; detail 1 → 0, 1, 2, 3, 3;
/// detail 2 → 0, 0, 1, 2, 3.
pub const ZOOM_REMAP: [u8; 15] = [0, 1, 2, 3, 4, 0, 1, 2, 3, 3, 0, 0, 1, 2, 3];

/// z = table[zoom + 5 × detail] for a zoom index 0–4; a detail above 2 is clamped to 2 and a zoom above 4 gives
/// `None` *\[choice\]* (§4.2, FS14: the original checks neither, and reads the next table row or past the table).
///
/// The detail is the original's model detail global: the integer render property `kModelLODOffset` (22),
/// clamped to 0–2 when it is read, which `Graphics Rules.sgr` sets from the Texture Quality option (High 0,
/// Medium 1, Low 2; registered default 0) (§4.2, GF8). FreeCity4 has no such option yet; callers pass the
/// detail they want.
pub fn remap_zoom(zoom: u32, detail: u32) -> Option<u32> {
    if zoom > 4 {
        return None;
    }
    // FS14 [choice]: the detail is clamped to 0-2, as the original clamps its global when it is set (GF8).
    let detail = detail.min(2);
    Some(u32::from(ZOOM_REMAP[(zoom + 5 * detail) as usize]))
}

/// The rotation r, 0–3, from the model function's two rotation arguments: (3a + b) mod 4, or 3a mod 4 when its
/// flag argument is set (§4.2).
pub fn view_rotation(a: u32, b: u32, flag: bool) -> u32 {
    let three_a = a.wrapping_mul(3);
    if flag {
        three_a % 4
    } else {
        three_a.wrapping_add(b) % 4
    }
}

/// The model key named by an RKT property with values `v` (type, group, instance…) at a zoom index (0–4),
/// detail (0–2) and rotation (0–3) (§4.2):
///
/// | Property | Key |
/// |---|---|
/// | RKT0 | (v[0], v[1], v[2]) for every zoom and rotation |
/// | RKT1 | (v[0], v[1], v[2] + z × `0x100` + r × `0x10`); v[2] = 0 means no model |
/// | RKT2 | (v[0], v[1], v[2 + 4z + r]) |
/// | RKT3 | (v[0], v[1], v[2 + z]) |
///
/// `None` for no model, for RKT4/RKT5 and other properties, and *\[choice\]* for too few values or a zoom above 4.
/// *\[choice\]* A detail above 2 is clamped to 2 and the rotation is taken mod 4 (FS14; the original checks
/// nothing).
pub fn model_key(property: u32, v: &[u32], zoom: u32, detail: u32, rotation: u32) -> Option<Tgi> {
    let key = |i: usize| v.get(i).map(|&inst| Tgi::new(v[0], v[1], inst));
    if property == RKT0 {
        return key(2);
    }
    let z = remap_zoom(zoom, detail)?;
    let rotation = rotation % 4;
    match property {
        RKT1 => {
            let base = *v.get(2)?;
            if base == 0 {
                return None;
            }
            let inst = base.wrapping_add(z * 0x100).wrapping_add(rotation * 0x10);
            Some(Tgi::new(v[0], v[1], inst))
        }
        RKT2 => key(2 + 4 * z as usize + rotation as usize),
        RKT3 => key(2 + z as usize),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §8.3 test 38.
    #[test]
    fn t38_texture_lookup() {
        let g = 0xBADB_57F1;
        let x = 0x1234_5678;
        let shared = Tgi::new(TYPE_FSH, GROUP_TEXTURES, x);
        let own = Tgi::new(TYPE_FSH, g, x);
        let b = resolve_texture(g, x, |k| *k == shared || *k == own).unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (shared, TextureStep::SharedGroup, x));
        let b = resolve_texture(g, x, |k| *k == own).unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (own, TextureStep::ModelGroup, x));
        let mut asked = Vec::new();
        let b = resolve_texture(g, x, |k| {
            asked.push(*k);
            *k == FALLBACK_TEXTURE
        })
        .unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (FALLBACK_TEXTURE, TextureStep::Fallback, 0));
        assert_eq!(asked, [shared, own, FALLBACK_TEXTURE]);
        assert_eq!(FALLBACK_TEXTURE, Tgi::new(0x7AB5_0E44, 0x1ABE_787D, 0x07F9_4200));
        // FS13: when the fallback does not load either, the slot has no texture.
        let mut asked = Vec::new();
        let b = resolve_texture(g, x, |k| {
            asked.push(*k);
            false
        });
        assert_eq!(b, None);
        assert_eq!(asked, [shared, own, FALLBACK_TEXTURE]);
        // ID 0 is never looked up.
        assert_eq!(resolve_texture(g, 0, |_| panic!("looked up")), None);
        // The binding cache ignores the type.
        assert_eq!(
            binding_cache_key(&Tgi::new(1, 2, 3)),
            binding_cache_key(&Tgi::new(9, 2, 3))
        );
    }

    /// §8.3 test 39.
    #[test]
    fn t39_rkt1() {
        let v = [0x5AD0_E817, 0xBADB_57F1, 0x1234_0000];
        let inst = |detail| model_key(RKT1, &v, 4, detail, 2).map(|k| k.instance);
        assert_eq!(inst(0), Some(0x1234_0420));
        assert_eq!(inst(1), Some(0x1234_0320));
        assert_eq!(inst(2), Some(0x1234_0320));
        assert_eq!(
            model_key(RKT1, &v, 4, 0, 2).unwrap(),
            Tgi::new(0x5AD0_E817, 0xBADB_57F1, 0x1234_0420)
        );
        assert_eq!(model_key(RKT1, &[0x5AD0_E817, 0xBADB_57F1, 0], 4, 0, 2), None);
    }

    /// §8.3 test 40.
    #[test]
    fn t40_rkt2() {
        let mut v = vec![0x5AD0_E817, 0xBADB_57F1];
        v.extend((0..20).map(|i| 0x1000 + i));
        assert_eq!(model_key(RKT2, &v, 1, 0, 3).map(|k| k.instance), Some(0x1000 + 7));
        assert_eq!(model_key(RKT2, &v, 4, 0, 3).map(|k| k.instance), Some(0x1000 + 19));
        assert_eq!(model_key(RKT2, &v[..21], 4, 0, 3), None);
    }

    /// §8.3 test 41.
    #[test]
    fn t41_rkt3() {
        let mut v = vec![0x5AD0_E817, 0xBADB_57F1];
        v.extend((0..5).map(|i| 0x2000 + i));
        assert_eq!(model_key(RKT3, &v, 2, 2, 0).map(|k| k.instance), Some(0x2000 + 1));
        assert_eq!(model_key(RKT3, &v, 2, 0, 0).map(|k| k.instance), Some(0x2000 + 2));
    }

    #[test]
    fn rkt_helpers() {
        let v = [1, 2, 3];
        for zoom in 0..5 {
            for r in 0..4 {
                assert_eq!(model_key(RKT0, &v, zoom, 0, r), Some(Tgi::new(1, 2, 3)));
            }
        }
        assert_eq!(model_key(RKT4, &v, 0, 0, 0), None);
        assert_eq!(model_key(RKT5, &v, 0, 0, 0), None);
        assert_eq!(model_key(RKT1, &v, 5, 0, 0), None);
        assert_eq!(model_key(RKT1, &v[..2], 0, 0, 0), None);
        // FS14 [choice]: a detail above 2 is clamped, a rotation taken mod 4.
        let big = [1, 2, 0x1000];
        assert_eq!(model_key(RKT1, &big, 4, 3, 1), model_key(RKT1, &big, 4, 2, 1));
        assert_eq!(model_key(RKT1, &big, 4, u32::MAX, 1).map(|k| k.instance), Some(0x1310));
        assert_eq!(model_key(RKT1, &big, 4, 0, 6).map(|k| k.instance), Some(0x1420));
        assert_eq!(remap_zoom(4, 7), Some(3));
        assert_eq!(remap_zoom(5, 0), None);
        let table: Vec<Vec<u32>> = (0..3)
            .map(|d| (0..5).map(|z| remap_zoom(z, d).unwrap()).collect())
            .collect();
        assert_eq!(table, [vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3, 3], vec![0, 0, 1, 2, 3]]);
        assert_eq!(view_rotation(1, 2, false), 1);
        assert_eq!(view_rotation(1, 2, true), 3);
        assert_eq!(view_rotation(u32::MAX, 0, false), 1); // -1: 3 × -1 = -3 ≡ 1 (mod 4)
    }
}
