// SPDX-License-Identifier: GPL-3.0-or-later
use super::{cell_at, finite, Cell, Pixel, PrimitiveFailure, Resource, Unsupported, Vec3};
use crate::view_input_controls::{set_cursor, ControlHost};

pub const TORCH_ID: u32 = 0xE9C2_4AC4;
pub const TORCH_CURSOR: u32 = 0x4A08_C57E;
pub const TORCH_ACTIVE_CURSOR: u32 = 0x0A08_C54E;
pub const TORCH_TICK: u32 = 0x86AD_10EE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TorchAudioData {
    /// CT17: on/off payload not published; never synthesize three zeros.
    Unpublished,
    Active(bool),
    Mouse {
        mouse: Pixel,
        count: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TorchAudio {
    pub message: u32,
    pub data: TorchAudioData,
}

pub trait TorchHost: ControlHost {
    fn pick_terrain(&mut self, mouse: Pixel) -> Option<Vec3>;
    /// Returns a created/acquired reference; failure is a residual, not a
    /// guessed native Init/activation return or substitute effect (CT14/17).
    fn create_effect(&mut self, name: &'static str) -> Result<Resource, PrimitiveFailure>;
    fn effect_started(&mut self, effect: Resource) -> bool;
    fn start_effect(&mut self, effect: Resource, flags: u32);
    fn stop_effect(&mut self, effect: Resource, flags: u32);
    fn release_effect(&mut self, effect: Resource);
    fn transform_effect(&mut self, effect: Resource, position: Vec3, rotation: [f32; 9], scale: f32);
    fn start_timer(&mut self, timer: Resource);
    fn stop_timer(&mut self, timer: Resource);
    /// Supplied opaque count. Unit/width conversion is unclaimed (CT17).
    fn timer_count(&mut self, timer: Resource) -> u64;
    /// Intent boundary only. CT2 event creation/post/consumer is unsupported.
    fn torch_audio(&mut self, intent: TorchAudio);
    fn register_tick(&mut self, value: u32);
    fn unregister_tick(&mut self);
    fn ignite_cell(&mut self, fire: Resource, cell: Cell, probability: bool, teleport: bool, fire_id: u32);
    fn set_cause(&mut self, fire: Resource, cause: u32);
    fn release_fire_manager(&mut self, fire: Resource);
}

#[derive(Debug)]
pub struct Torch {
    fire: Option<Resource>,
    timer: Resource,
    effect: Option<Resource>,
    active: bool,
    position: Option<Vec3>,
    last_active_move: Cell,
}

impl Torch {
    /// Partial Init. fire transfers one explicitly held reference, timer is
    /// borrowed. Base Init, acquisition, native return and timer ownership
    /// are unclaimed. Fresh cell (0,0) is explicitly published in quirk 3.
    pub fn init(fire: Resource, timer: Resource, host: &mut impl TorchHost) -> Self {
        host.start_timer(timer);
        host.torch_audio(TorchAudio {
            message: 0x2A10_6113,
            data: TorchAudioData::Unpublished,
        });
        Self {
            fire: Some(fire),
            timer,
            effect: None,
            active: false,
            position: None,
            last_active_move: Cell { x: 0, z: 0 },
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn effect(&self) -> Option<Resource> {
        self.effect
    }

    pub fn position(&self) -> Option<Vec3> {
        self.position
    }

    pub fn last_active_move(&self) -> Cell {
        self.last_active_move
    }

    fn fire(&self) -> Result<Resource, Unsupported> {
        self.fire.ok_or(Unsupported::InactiveLifetime)
    }

    fn mouse_audio(&self, host: &mut impl TorchHost, message: u32, mouse: Pixel) {
        let count = host.timer_count(self.timer);
        host.torch_audio(TorchAudio {
            message,
            data: TorchAudioData::Mouse { mouse, count },
        });
    }

    fn active_audio(&self, host: &mut impl TorchHost) {
        host.torch_audio(TorchAudio {
            message: 0xAA96_FE8F,
            data: TorchAudioData::Active(self.active),
        });
    }

    pub fn activate(&mut self, host: &mut impl TorchHost, on: bool, mouse: Pixel) -> Result<(), Unsupported> {
        self.fire()?;
        if self.active == on {
            return Ok(());
        }
        if let Some(old) = self.effect.take() {
            host.stop_effect(old, 0);
            host.release_effect(old);
            // CT14/17: on foreign failure the released effect stays absent;
            // preserve the previous active flag and report the completed prefix.
            let effect = host.create_effect(if on { "torch_active" } else { "torch_inactive" })?;
            self.effect = Some(effect);
            host.start_effect(effect, 0);
            self.active = on;
            if !on {
                self.mouse_audio(host, 0x0A10_6122, mouse);
            }
            self.active_audio(host);
        }
        Ok(())
    }

    pub fn update_effect(&self, host: &mut impl TorchHost) -> Result<(), Unsupported> {
        self.fire()?;
        if let Some(effect) = self.effect {
            let position = self.position.ok_or(Unsupported::MissingPosition)?;
            host.transform_effect(effect, position, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1.0);
        }
        Ok(())
    }

    pub fn drop_effect(&mut self, host: &mut impl TorchHost) -> Result<(), Unsupported> {
        self.fire()?;
        if let Some(effect) = self.effect.take() {
            if host.effect_started(effect) {
                host.stop_effect(effect, 1);
            }
            host.release_effect(effect);
        }
        self.active = false;
        self.active_audio(host);
        Ok(())
    }

    pub fn mouse_exit(&mut self, host: &mut impl TorchHost) -> Result<(), Unsupported> {
        self.drop_effect(host)
    }

    pub fn mouse_move(&mut self, host: &mut impl TorchHost, mouse: Pixel, modifiers: u32) -> Result<bool, Unsupported> {
        self.fire()?;
        if let Some(p) = host.pick_terrain(mouse) {
            self.position = Some(finite(p)?);
            if self.effect.is_none() {
                let effect = host.create_effect("torch_inactive")?;
                self.effect = Some(effect);
                host.start_effect(effect, 0);
            }
            // CT1 deliberately preserves raw bit 0x10, with no key mapping.
            if !self.active && modifiers & 0x10 != 0 {
                self.activate(host, true, mouse)?;
            }
            self.update_effect(host)?;
            if self.active {
                self.last_active_move = cell_at(p)?;
            }
        } else {
            self.drop_effect(host)?;
        }
        self.mouse_audio(host, 0x4A10_6126, mouse);
        Ok(true)
    }

    pub fn press(&mut self, host: &mut impl TorchHost, mouse: Pixel) -> Result<bool, Unsupported> {
        let fire = self.fire()?;
        self.mouse_audio(host, 0x8A10_611D, mouse);
        if let Some(p) = host.pick_terrain(mouse) {
            self.position = Some(finite(p)?);
            // No effect creation here: ActivateTorch itself requires an effect.
            self.activate(host, true, mouse)?;
            self.update_effect(host)?;
            host.register_tick(0xC0);
            host.ignite_cell(fire, cell_at(p)?, false, true, 0);
            host.set_cause(fire, 1);
            set_cursor(host, TORCH_ACTIVE_CURSOR);
        }
        Ok(true)
    }

    pub fn animation_tick(&self, host: &mut impl TorchHost) -> Result<(), Unsupported> {
        let fire = self.fire()?;
        if self.active {
            host.ignite_cell(fire, self.last_active_move, true, true, 0);
            host.set_cause(fire, 1);
        }
        Ok(())
    }

    pub fn release(&mut self, host: &mut impl TorchHost, mouse: Pixel) -> Result<bool, Unsupported> {
        self.fire()?;
        self.mouse_audio(host, 0x0A10_6122, mouse);
        host.unregister_tick();
        self.activate(host, false, mouse)?;
        set_cursor(host, TORCH_CURSOR);
        Ok(true)
    }

    pub fn shutdown(&mut self, host: &mut impl TorchHost) -> Result<(), Unsupported> {
        let fire = self.fire()?;
        host.stop_timer(self.timer);
        self.drop_effect(host)?;
        host.torch_audio(TorchAudio {
            message: 0x8A10_611A,
            data: TorchAudioData::Unpublished,
        });
        host.release_fire_manager(fire);
        self.fire = None;
        host.unregister_tick();
        Ok(())
    }
}
