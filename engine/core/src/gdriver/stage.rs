// SPDX-License-Identifier: GPL-3.0-or-later
//! Deferred texture/sampler state and separate environment cache (§10).
use super::{backend::*, texture::TextureManager};
#[derive(Clone, Debug)]
pub struct Stage {
    pub dirty: bool,
    pub enabled: bool,
    pub alpha: bool,
    pub name: u32,
    pub coordinates: u32,
    pub parameters: [u32; 4],
    pub environment: u32,
    pub applied: Option<u32>,
    pub function: [u32; 2],
    pub sources: [[u32; 3]; 2],
    pub operands: [[u32; 3]; 2],
    pub scale: [u32; 2],
}
impl Default for Stage {
    fn default() -> Self {
        Self {
            dirty: false, enabled: false, alpha: true, name: 0, coordinates: 0,
            parameters: [0, 0, 3, 3], environment: 0, applied: None,
            function: [0; 2], sources: [[3; 3]; 2], operands: [[2; 3], [4; 3]], scale: [0; 2],
        }
    }
}
impl Stage {
    pub fn reset(&mut self, _host: &mut impl Device) -> Result<()> {
        // §10.1: device defaults invalidate dirtiness, not the cached record.
        self.dirty = true;
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.applied = None;
    }
    pub fn packed(&mut self, d: &[u8; 10]) -> Result<()> {
        self.environment = 4;
        self.dirty = true;
        // Deliberately retain `applied`, even when mode 4 was already sent.
        if d[0] == 255 {
            return Ok(());
        }
        self.function = [d[0] as u32, d[4] as u32];
        self.scale = [d[8] as u32, d[9] as u32];
        for i in 0..3 {
            self.sources[0][i] = (d[1 + i] & 15) as u32;
            self.sources[1][i] = (d[5 + i] & 15) as u32;
            self.operands[0][i] = *[2, 3, 4, 5].get((d[1 + i] >> 4) as usize).ok_or(Error::InvalidInput)?;
            self.operands[1][i] = *[4, 5, 4, 5].get((d[5 + i] >> 4) as usize).ok_or(Error::InvalidInput)?;
        }
        Ok(())
    }
    pub fn apply(&mut self, i: u32, device: u32, textures: &TextureManager, host: &mut impl Device) -> Result<()> {
        self.dirty = false;
        let surface = textures.surface(self.name);
        if !self.enabled || surface == 0 {
            self.applied = None;
            self.send(i, device, 1, 1, host);
            self.send(i, device, 4, 1, host);
            return Ok(());
        }
        dx(host, device, Call::Texture, &[i, surface]);
        match self.parameters[0] {
            0 => self.send(i, device, 16, 1, host),
            1 => self.send(i, device, 16, 2, host),
            _ => (),
        }
        let min = match self.parameters[1] {
            0 => Some((1, 1)),
            1 => Some((2, 1)),
            4 => Some((1, 2)),
            5 => {
                self.send(i, device, 17, 2, host);
                self.send(i, device, 18, 2, host);
                Some((1, 3))
            }
            6 => Some((1, 3)),
            7 => Some((2, 3)),
            _ => None,
        };
        if let Some((a, b)) = min {
            self.send(i, device, 17, a, host);
            self.send(i, device, 18, b, host);
        }
        for (k, state) in [(2, 13), (3, 14)] {
            match self.parameters[k] {
                2 => self.send(i, device, state, 3, host),
                3 => self.send(i, device, state, 1, host),
                _ => (),
            }
        }
        if self.applied == Some(self.environment) {
            return Ok(());
        }
        self.applied = Some(self.environment);
        match self.environment {
            0 => self.env(i, device, [2, 2, 0, 2, 2, 0], host),
            1 => self.env(
                i,
                device,
                [
                    4,
                    2,
                    1,
                    if self.alpha { 4 } else { 2 },
                    if self.alpha { 2 } else { 1 },
                    1,
                ],
                host,
            ),
            2 => self.env(
                i,
                device,
                [
                    if self.alpha { 13 } else { 2 },
                    2,
                    1,
                    if self.alpha { 3 } else { 2 },
                    1,
                    1,
                ],
                host,
            ),
            3 => {
                self.send(i, device, 1, 1, host);
                self.send(i, device, 4, 1, host);
            }
            4 | 5 => {
                for channel in 0..2 {
                    let mut args = [0; 2];
                    for (j, arg) in args.iter_mut().enumerate() {
                        *arg = *[2, 1, 3, 0]
                            .get(self.sources[channel][j] as usize)
                            .ok_or(Error::InvalidInput)?;
                        let operand = self.operands[channel][j];
                        if (channel == 0 && matches!(operand, 3 | 5)) || (channel == 1 && operand == 5) {
                            *arg |= 0x10;
                        }
                        if channel == 0 && matches!(operand, 4 | 5) {
                            *arg |= 0x20;
                        }
                    }
                    let state = if channel == 0 { 1 } else { 4 };
                    self.send(i, device, state + 1, args[0], host);
                    self.send(i, device, state + 2, args[1], host);
                    let op = match self.function[channel] {
                        0 => Some(2),
                        1 => Some(*[4, 5, 6].get(self.scale[channel] as usize).ok_or(Error::InvalidInput)?),
                        2 => Some(7),
                        3 => Some(if self.scale[channel] == 0 { 8 } else { 9 }),
                        4 => match self.sources[channel][2] {
                            0 => Some(13),
                            1 => Some(16),
                            2 => Some(14),
                            3 => Some(12),
                            _ if channel == 0 => None,
                            _ => return Err(Error::InvalidInput),
                        },
                        5 => Some(24),
                        _ => Some(1),
                    };
                    if let Some(op) = op {
                        self.send(i, device, state, op, host);
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
    fn send(&self, i: u32, device: u32, state: u32, value: u32, host: &mut impl Device) {
        dx(host, device, Call::StageState, &[i, state, value]);
    }
    fn env(&self, i: u32, device: u32, v: [u32; 6], host: &mut impl Device) {
        for (state, value) in [1, 2, 3, 4, 5, 6].into_iter().zip(v) {
            self.send(i, device, state, value, host);
        }
    }
}
