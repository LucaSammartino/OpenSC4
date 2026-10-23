// SPDX-License-Identifier: GPL-3.0-or-later
//! Stateless, independently expected `lm_*` public vectors. These fixtures use
//! only published lot-model decisions; no original renderer supplies outputs.
use crate::json::Json;
use fc4_core::lot_models::*;

type R<T> = Result<T, String>;

fn field<'a>(op: &'a Json, key: &str) -> R<&'a Json> {
    op.get(key).ok_or_else(|| format!("missing {key}"))
}
fn integer(op: &Json, key: &str) -> R<u32> {
    let value = field(op, key)?
        .as_i64()
        .ok_or_else(|| format!("{key}: expected integer"))?;
    u32::try_from(value).map_err(|_| format!("{key}: outside u32"))
}
fn number(op: &Json, key: &str) -> R<f32> {
    let value = field(op, key)?
        .as_f64()
        .ok_or_else(|| format!("{key}: expected number"))? as f32;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{key}: nonfinite"))
    }
}
fn boolean(op: &Json, key: &str) -> R<bool> {
    field(op, key)?
        .as_bool()
        .ok_or_else(|| format!("{key}: expected boolean"))
}
fn byte(op: &Json, key: &str) -> R<u8> {
    u8::try_from(integer(op, key)?).map_err(|_| format!("{key}: outside u8"))
}
fn edge(op: &Json, key: &str) -> R<Edge> {
    Edge::ALL
        .get(integer(op, key)? as usize)
        .copied()
        .ok_or_else(|| format!("{key}: outside four edges"))
}
fn level(op: &Json) -> R<Zoom> {
    Zoom::new(byte(op, "zoom")?).map_err(|e| format!("zoom: {e:?}"))
}
fn equal<T: std::fmt::Debug + PartialEq>(name: &str, actual: T, expected: T) -> R<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name}: expected {expected:?}, got {actual:?}"))
    }
}
fn corners(value: &Json) -> R<[Vec2; 4]> {
    let list = value.as_array().ok_or("corners: expected array")?;
    let mut out = [[0.0; 2]; 4];
    if list.len() != 4 {
        return Err("corners: expected four".into());
    }
    for (out, input) in out.iter_mut().zip(list) {
        let pair = input.as_array().ok_or("corner: expected array")?;
        if pair.len() != 2 {
            return Err("corner: expected pair".into());
        }
        for (out, input) in out.iter_mut().zip(pair) {
            *out = input.as_f64().ok_or("corner: expected number")? as f32;
        }
    }
    Ok(out)
}

struct Binding;
impl Texture for Binding {
    fn id(&self) -> u32 {
        0
    }
    fn has_alpha(&self) -> bool {
        false
    }
}
#[derive(Default)]
struct Factory(Vec<TextureKey>);
impl TextureFactory for Factory {
    type Binding = Binding;
    fn load(&mut self, key: TextureKey) -> Option<Binding> {
        self.0.push(key);
        Some(Binding)
    }
}
struct UnusedLighting;
impl Lighting for UnusedLighting {
    fn for_normal(&mut self, _: Vec3) -> Vec3 {
        panic!("height/fallback vector unexpectedly sampled light")
    }
    fn at_corner(&mut self, _: Vec3, _: Vec3) -> Vec3 {
        panic!("height/fallback vector unexpectedly sampled light")
    }
}

fn foundation(top: f32, base: f32) -> R<FoundationModel<Binding>> {
    // Explicit vector fixture: a 16 x 16 m box, top scale 16. These are test
    // inputs, never inferred native constructor defaults.
    FoundationModel::new(
        Footprint {
            min: [0.0, 0.0],
            max: [16.0, 16.0],
        },
        base,
        top,
        16.0,
        FoundationTextureKeys {
            sides: std::array::from_fn(|i| Some(0x69f20000 + i as u32)),
            tops: std::array::from_fn(|i| Some(0x0912220f + i as u32)),
        },
    )
    .map_err(|e| format!("foundation: {e:?}"))
}

pub fn exec(name: &str, op: &Json) -> Option<R<()>> {
    match name {
        "lm_foundation_steps"
        | "lm_foundation_fallback"
        | "lm_wall_texture"
        | "lm_wall_facing"
        | "lm_cell_mode"
        | "lm_cell_corners" => Some(execute(name, op)),
        _ => None,
    }
}

fn execute(name: &str, op: &Json) -> R<()> {
    match name {
        "lm_foundation_steps" => {
            let mut model = foundation(number(op, "top")?, number(op, "base")?)?;
            let water = match field(op, "water")? {
                Json::Null => None,
                _ => Some(number(op, "water")?),
            };
            let result = model
                .adjust_height_to_terrain(
                    &[BorderHeight {
                        ground: number(op, "ground")?,
                        water,
                    }],
                    boolean(op, "force")?,
                    &mut UnusedLighting,
                )
                .map_err(|e| format!("height: {e:?}"))?;
            equal("base", result.base, number(op, "expect_base")?)?;
            equal("rebuilt", result.rebuilt, boolean(op, "expect_rebuilt")?)
        }
        "lm_foundation_fallback" => {
            let mut model = foundation(40.0, 40.0)?;
            let mut factory = Factory::default();
            model.load_textures(level(op)?, &mut factory);
            equal(
                "texture requests",
                factory.0,
                vec![
                    TextureKey::FoundationSide(integer(op, "expect_side")?),
                    TextureKey::FoundationTop(integer(op, "expect_top")?),
                ],
            )
        }
        "lm_wall_texture" => {
            let samples = [
                WallSample {
                    position: [0.0, 0.0],
                    terrain_height: 0.0,
                },
                WallSample {
                    position: [16.0, 0.0],
                    terrain_height: 0.0,
                },
            ];
            let mut wall = RetainingWallModel::init(
                Edge::North,
                1.0,
                &samples,
                1.0,
                std::array::from_fn(|i| Some(0x695c00c6 + i as u32)),
                true,
            )
            .map_err(|e| format!("wall: {e:?}"))?;
            let mut factory = Factory::default();
            wall.load_texture(level(op)?, &mut factory);
            equal(
                "texture requests",
                factory.0,
                vec![TextureKey::Wall {
                    group: 0x891B_0E1A,
                    instance: integer(op, "expect_instance")?,
                }],
            )
        }
        "lm_wall_facing" => equal(
            "facing",
            edge(op, "side")?.wall_faces(edge(op, "rotation")?),
            boolean(op, "expect_draw")?,
        ),
        "lm_cell_mode" => {
            let expected = match field(op, "expect_mode")?.as_str() {
                Some("opaque") => CellMode::Opaque,
                Some("alpha") => CellMode::Alpha,
                Some("tinted") => CellMode::Tinted,
                _ => return Err("expect_mode: unknown mode".into()),
            };
            equal(
                "cell mode",
                cell_mode(byte(op, "mode")?, byte(op, "alpha")?, boolean(op, "texture_alpha")?),
                expected,
            )
        }
        "lm_cell_corners" => {
            let orientation = Orientation::new(byte(op, "orientation")?).map_err(|e| format!("orientation: {e:?}"))?;
            equal(
                "texture corners",
                CellCornerLayout::native_texture_corners(orientation, boolean(op, "mirror")?),
                corners(field(op, "expect_corners")?)?,
            )
        }
        _ => Err(format!("unknown lot-model op {name}")),
    }
}
