// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent, literal animation coordinator vectors. The fixture supplies
//! foreign data and primitive traces, never the coordinator's eligibility result.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use fc4_core::animation_coordinator::{
    AnimationCoordinator, Block, BlockRect, Error, Host, Message, Occupant, OccupantId, WorldRect,
};

use crate::json::Json;

type R<T> = Result<T, String>;
type Log = Arc<Mutex<Vec<String>>>;

struct Lease {
    name: String,
    log: Log,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(format!("release:{}", self.name));
    }
}

struct Foreign {
    log: Log,
    occupants: BTreeMap<OccupantId, Occupant>,
    powered: BTreeSet<OccupantId>,
    ticks: BTreeSet<u64>,
    cells: u32,
    zoom: u32,
    visible: WorldRect,
    detail: u8,
    missing_target: bool,
    fail_add: bool,
    fail_remove: bool,
    fail_subscription: Option<u32>,
}

impl Foreign {
    fn record(&self, message: String) {
        self.log.lock().unwrap().push(message);
    }

    fn lease(&self, name: String) -> Lease {
        self.record(format!("acquire:{name}"));
        Lease {
            name,
            log: self.log.clone(),
        }
    }
}

impl Host for Foreign {
    type Services = Lease;
    type Subscription = Lease;
    type Target = (u64, Lease);

    fn acquire_services(&mut self) -> Result<Lease, Error> {
        Ok(self.lease("services".into()))
    }

    fn subscribe(&mut self, message: u32) -> Result<Lease, Error> {
        if self.fail_subscription == Some(message) {
            return Err(Error::ForeignUnavailable);
        }
        Ok(self.lease(format!("subscription:{message:#010X}")))
    }

    fn city_cells_x(&mut self) -> Result<u32, Error> {
        Ok(self.cells)
    }
    fn view_zoom(&mut self) -> Result<u32, Error> {
        Ok(self.zoom)
    }

    fn visible_world_rect(&mut self) -> Result<WorldRect, Error> {
        self.record("read:rect".into());
        Ok(self.visible)
    }

    fn animation_detail_byte(&mut self) -> Result<u8, Error> {
        self.record("read:detail".into());
        Ok(self.detail)
    }

    fn occupant(&mut self, id: OccupantId) -> Result<Occupant, Error> {
        self.record(format!("read:occupant:{}", id.0));
        self.occupants.get(&id).cloned().ok_or(Error::ForeignUnavailable)
    }

    fn powered(&mut self, id: OccupantId) -> Result<bool, Error> {
        self.record(format!("power:{}", id.0));
        Ok(self.powered.contains(&id))
    }

    fn message_target(&mut self, id: OccupantId) -> Result<Option<Self::Target>, Error> {
        Ok((!self.missing_target).then(|| (id.0, self.lease(format!("target:{}", id.0)))))
    }

    fn tick_add(&mut self, target: &Self::Target) -> Result<(), Error> {
        self.record(format!("add:{}", target.0));
        if self.fail_add {
            return Err(Error::ForeignUnavailable);
        }
        self.ticks.insert(target.0);
        Ok(())
    }

    fn tick_remove(&mut self, target: &Self::Target, flags: u32) -> Result<(), Error> {
        self.record(format!("remove:{}:{flags:#04X}", target.0));
        if self.fail_remove {
            return Err(Error::ForeignUnavailable);
        }
        self.ticks.remove(&target.0);
        Ok(())
    }

    fn set_animating_flag(&mut self, id: OccupantId, on: bool) {
        self.record(format!("flag:{}:{on}", id.0));
        let occupant = self.occupants.get_mut(&id).unwrap();
        if on {
            occupant.flags |= 0x20;
        } else {
            occupant.flags &= !0x20;
        }
    }
}

fn field<'a>(v: &'a Json, name: &str) -> R<&'a Json> {
    v.get(name).ok_or_else(|| format!("missing field {name}"))
}

fn array(v: &Json) -> R<&[Json]> {
    v.as_array().ok_or_else(|| "expected array".into())
}

fn integer(v: &Json) -> R<u64> {
    v.as_i64()
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| "expected nonnegative integer".into())
}

fn u32_value(v: &Json) -> R<u32> {
    u32::try_from(integer(v)?).map_err(|_| "integer exceeds u32".into())
}

fn boolean(v: &Json) -> R<bool> {
    v.as_bool().ok_or_else(|| "expected boolean".into())
}

fn four(v: &Json) -> R<[f64; 4]> {
    let a = array(v)?;
    if a.len() != 4 {
        return Err("expected rectangle of length 4".into());
    }
    let mut out = [0.0; 4];
    for (i, value) in a.iter().enumerate() {
        out[i] = value.as_f64().ok_or_else(|| "expected rectangle number".to_owned())?;
    }
    Ok(out)
}

fn world_rect(v: &Json) -> R<WorldRect> {
    let [min_x, min_z, max_x, max_z] = four(v)?;
    Ok(WorldRect {
        min_x,
        min_z,
        max_x,
        max_z,
    })
}

fn block_rect(v: &Json) -> R<BlockRect> {
    let a = array(v)?;
    if a.len() != 4 {
        return Err("expected rectangle of length 4".into());
    }
    let mut out = [0; 4];
    for (i, value) in a.iter().enumerate() {
        out[i] = usize::try_from(integer(value)?).map_err(|_| "block coordinate exceeds usize")?;
    }
    Ok(BlockRect {
        min_x: out[0],
        min_z: out[1],
        max_x: out[2],
        max_z: out[3],
    })
}

fn ji(n: u64) -> Json {
    Json::Int(n as i64)
}

fn snapshot(owner: &AnimationCoordinator<Foreign>, name: &str) -> R<Json> {
    Ok(match name {
        "initialized" => Json::Bool(owner.initialized()),
        "enabled" => Json::Bool(owner.enabled()),
        "zoom" => ji(u64::from(owner.zoom())),
        "side" => ji(owner.blocks_per_side() as u64),
        "registered" => ji(owner.registered_count() as u64),
        "ticks" => Json::Array(owner.host().ticks.iter().copied().map(ji).collect()),
        "flags" => Json::Array(
            owner
                .host()
                .occupants
                .iter()
                .map(|(id, o)| Json::Array(vec![ji(id.0), ji(u64::from(o.flags))]))
                .collect(),
        ),
        "events" => Json::Array(
            owner
                .host()
                .log
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .map(Json::Str)
                .collect(),
        ),
        "blocks" => {
            let mut blocks = Vec::new();
            for z in 0..owner.blocks_per_side() {
                for x in 0..owner.blocks_per_side() {
                    let count = owner.block_occupant_count(Block { x, z });
                    if count != 0 {
                        blocks.push(Json::Array(vec![ji(x as u64), ji(z as u64), ji(count as u64)]));
                    }
                }
            }
            Json::Array(blocks)
        }
        other => return Err(format!("unknown state field {other}")),
    })
}

fn expect(owner: &AnimationCoordinator<Foreign>, step: &Json) -> R<()> {
    for (name, expected) in field(step, "state")?.as_object().ok_or("expected state object")? {
        let actual = snapshot(owner, name)?;
        if &actual != expected {
            return Err(format!("{name}: expected {expected:?}, got {actual:?}"));
        }
    }
    Ok(())
}

fn execute(owner: &mut AnimationCoordinator<Foreign>, step: &Json) -> R<()> {
    let op = field(step, "op")?.as_str().ok_or("expected op string")?;
    let result = match op {
        // The workspace turbo-off audit injects this global fixture step into
        // every vector. This owner has no clock; accept explicit off only.
        "set_turbo" => {
            if boolean(field(step, "on")?)? {
                return Err("animation coordinator vectors have no turbo clock".into());
            }
            Ok(())
        }
        "init" => owner.init().map(|_| ()),
        "shutdown" => {
            owner.shutdown();
            Ok(())
        }
        "clear" => {
            owner.host().log.lock().unwrap().clear();
            Ok(())
        }
        "set" => owner.set_occupant_animation(OccupantId(integer(field(step, "id")?)?), boolean(field(step, "on")?)?),
        "refresh_enabled" => owner.refresh_animation_enabled(),
        "view" => {
            owner.host_mut().visible = world_rect(field(step, "rect")?)?;
            Ok(())
        }
        "power" => {
            let id = OccupantId(integer(field(step, "id")?)?);
            if boolean(field(step, "on")?)? {
                owner.host_mut().powered.insert(id);
            } else {
                owner.host_mut().powered.remove(&id);
            }
            Ok(())
        }
        "detail" => {
            owner.host_mut().detail = u8::try_from(integer(field(step, "byte")?)?).map_err(|_| "byte exceeds u8")?;
            Ok(())
        }
        "fail" => {
            for (key, value) in step.as_object().unwrap() {
                match key.as_str() {
                    "op" => {}
                    "target" => owner.host_mut().missing_target = boolean(value)?,
                    "add" => owner.host_mut().fail_add = boolean(value)?,
                    "remove" => owner.host_mut().fail_remove = boolean(value)?,
                    "subscription" => {
                        owner.host_mut().fail_subscription = if value == &Json::Null {
                            None
                        } else {
                            Some(u32_value(value)?)
                        }
                    }
                    other => return Err(format!("unknown failure field {other}")),
                }
            }
            Ok(())
        }
        "message" => {
            let message = match u32_value(field(step, "id")?)? {
                0x8A07_AE00 => Message::PowerChanged(block_rect(field(step, "blocks")?)?),
                0x6A5F_D3FD => Message::BudgetChanged(OccupantId(integer(field(step, "occupant")?)?)),
                0x231B_BF91 => Message::OpaqueRefresh,
                0xA6B7_9612 => Message::Translation,
                0xA6B7_9602 => Message::ZoomRotation {
                    zoom: u32_value(field(step, "zoom")?)?,
                    old_rotation: u32_value(field(step, "old_rotation")?)?,
                    new_rotation: u32_value(field(step, "new_rotation")?)?,
                },
                other => Message::Other(other),
            };
            owner.do_message(message).map(|_| ())
        }
        "expect" => return expect(owner, step),
        other => return Err(format!("unknown animation coordinator op {other}")),
    };
    let expected_error = step.get("error").and_then(Json::as_str);
    match (result, expected_error) {
        (Ok(()), None) => Ok(()),
        (Err(error), Some(expected)) if format!("{error:?}") == expected => Ok(()),
        (result, expected) => Err(format!("expected error {expected:?}, got {result:?}")),
    }
}

pub(crate) fn run(vector: &Json) -> R<usize> {
    let mut foreign = Foreign {
        log: Arc::default(),
        occupants: BTreeMap::new(),
        powered: BTreeSet::new(),
        ticks: BTreeSet::new(),
        cells: u32_value(field(vector, "city_cells_x")?)?,
        zoom: u32_value(field(vector, "zoom")?)?,
        visible: world_rect(field(vector, "visible")?)?,
        detail: 2,
        missing_target: false,
        fail_add: false,
        fail_remove: false,
        fail_subscription: None,
    };
    for entry in array(field(vector, "occupants")?)? {
        let id = OccupantId(integer(field(entry, "id")?)?);
        let position = array(field(entry, "position")?)?;
        if position.len() != 2 {
            return Err("expected two position coordinates".into());
        }
        let occupant = Occupant {
            x_metres: position[0].as_f64().ok_or("expected x number")?,
            z_metres: position[1].as_f64().ok_or("expected z number")?,
            flags: u32_value(field(entry, "flags")?)?,
            model_classes: array(field(entry, "models")?)?
                .iter()
                .map(u32_value)
                .collect::<R<_>>()?,
            exemplar_properties: array(field(entry, "properties")?)?
                .iter()
                .map(u32_value)
                .collect::<R<_>>()?,
        };
        if occupant.flags & 0x20 != 0 {
            foreign.ticks.insert(id.0);
        }
        if entry.get("powered").map(boolean).transpose()?.unwrap_or(false) {
            foreign.powered.insert(id);
        }
        if foreign.occupants.insert(id, occupant).is_some() {
            return Err("duplicate occupant identity".into());
        }
    }
    let mut owner = AnimationCoordinator::new(foreign);
    let steps = array(field(vector, "steps")?)?;
    for (i, step) in steps.iter().enumerate() {
        execute(&mut owner, step).map_err(|error| format!("animation coordinator step {i}: {error}"))?;
    }
    Ok(steps.len())
}
