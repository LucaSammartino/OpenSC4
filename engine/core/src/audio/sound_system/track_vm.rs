// SPDX-License-Identifier: GPL-3.0-or-later
//! §3.2–3.3 confirmed subset. Remaining opcodes require the specified runtime traces.
use super::{track::HitTrackPlayer, Unsupported};

#[derive(Debug, PartialEq)]
pub enum TrackEffect {
    Kill,
    Start,
    Unpause,
    StartDependent(i32),
    Pitch,
    Patch,
    VolPan,
    Primary26(i32),
    ForwardFloat(u32, f32),
    ForwardInteger(u32, i32),
    PauseCurrent(bool),
    RetainTarget(u32),
    NoteOn(i32),
    NoteOff,
}
pub trait TrackIo {
    fn effect(&mut self, effect: TrackEffect) -> bool;
    fn report(&mut self, message: &str) -> i32;
    fn entry(&mut self, id: u32) -> Option<usize>;
    // SPEC-GAP: alias-table words are not enumerated; the register adapter supplies them.
    fn alias(&mut self, _id: u32) -> u32 {
        0
    }
}
pub struct RejectTrackIo;
impl TrackIo for RejectTrackIo {
    fn effect(&mut self, _: TrackEffect) -> bool {
        false
    }
    fn report(&mut self, _: &str) -> i32 {
        1
    }
    fn entry(&mut self, _: u32) -> Option<usize> {
        None
    }
}

impl HitTrackPlayer {
    pub fn play_with(&mut self, payload: [u32; 4], io: &mut dyn TrackIo) -> bool {
        if self.is_playing() {
            if self.properties.get(&0xFF30_3B1B) == Some(&0) {
                return true;
            }
            io.effect(TrackEffect::Kill);
        }
        self.write_play_payload(payload);
        let started = io.effect(TrackEffect::Start);
        let dependent = self.properties.get(&0xFF59_FDD8).copied().unwrap_or(0);
        if dependent != 0 {
            io.effect(TrackEffect::StartDependent(dependent));
        }
        started && io.effect(TrackEffect::Unpause)
    }
    pub fn set_register(&mut self, id: u32, value: i32, suppress: u32, io: &mut dyn TrackIo) -> bool {
        let alias = if id < 0x50 { io.alias(id) } else { 0 };
        let id = if alias == 0 { id } else { alias };
        self.flags = [value == 0, value > 0, value < 0];
        self.properties.insert(id, value);
        if suppress as u8 != 0 {
            true
        } else {
            self.property_notifications.push((id, value));
            self.notify_register(id, io)
        }
    }
    pub fn notify_register(&mut self, id: u32, io: &mut dyn TrackIo) -> bool {
        let value = self.properties.get(&id).copied().unwrap_or(0);
        match id {
            0xFF09_9DB2 => {
                io.effect(TrackEffect::Pitch);
            }
            0xFFAE_C905 => {
                io.effect(TrackEffect::Patch);
            }
            0xFF2D_16F2 => {
                if value != 0 {
                    self.completion_after_ramp = false;
                }
            }
            0xFF5F_15E1 => return io.effect(TrackEffect::Primary26(value)),
            0xFF90_AAC8 | 0xFF46_9820 | 0xFF4A_01D6 | 0xFF4C_6660 | 0xFF53_DBF3 | 0xFF3E_C880 | 0xFF40_FF96
            | 0xFF44_4C9A | 0xFFA6_A614 | 0xFFAA_B623 | 0xFFC2_0061 | 0xFFC6_B36D | 0xFFCC_3B29 | 0xFFCC_4D2D
            | 0xFFCE_9997 => {
                io.effect(TrackEffect::VolPan);
            }
            0xFF30_71B9 | 0xFF01_50F2 | 0xFF73_4343 | 0xFF86_FE79 | 0xFFD3_D39B => {
                io.effect(TrackEffect::ForwardFloat(
                    id,
                    self.float_properties.get(&id).copied().unwrap_or(0.0),
                ));
            }
            0xFF05_0A94 | 0xFF59_5FFD | 0xFF7E_5922 | 0xFFA4_2951 | 0xFFA9_B525 | 0xFFBD_B196 | 0xFFF3_EDF2
            | 0xFFF7_B8DF | 0xFFFF_4E79 => {
                io.effect(TrackEffect::ForwardInteger(id, value));
            }
            0xFFB3_6659 => {
                io.effect(TrackEffect::PauseCurrent(value != 0));
            }
            0xFFC6_E666 => {
                io.effect(TrackEffect::RetainTarget(value as u32));
            }
            _ => (),
        }
        true
    }
    fn operand(&mut self, data: &[u8], size: usize) -> Result<u32, Unsupported> {
        // SPEC-GAP: malformed bytecode has unchecked reads; safely reject a short buffer.
        let bytes = data
            .get(self.pc..self.pc + size)
            .ok_or(Unsupported("SS1-short-stream"))?;
        self.pc += size;
        Ok(bytes
            .iter()
            .enumerate()
            .fold(0, |word, (i, byte)| word | (u32::from(*byte) << (8 * i))))
    }
    fn reg_operand(&mut self, data: &[u8], io: &mut dyn TrackIo) -> Result<u32, Unsupported> {
        let id = self.operand(data, 1)?;
        if id > 74 {
            self.error = io.report("Illegal Register Id");
            Ok(1)
        } else {
            Ok(id)
        }
    }
    fn reg_value(&self, id: u32, io: &mut dyn TrackIo) -> i32 {
        let alias = if id < 0x50 { io.alias(id) } else { 0 };
        self.properties
            .get(&if alias == 0 { id } else { alias })
            .copied()
            .unwrap_or(0)
    }
    pub fn execute_next(&mut self, data: &[u8], io: &mut dyn TrackIo) -> Result<bool, Unsupported> {
        let opcode = self.operand(data, 1)?;
        match opcode {
            1 => {
                let note = self.reg_operand(data, io)?;
                let count = self.reg_operand(data, io)?;
                self.note_countdown = self.reg_value(count, io);
                let value = self.reg_value(note, io);
                if !io.effect(TrackEffect::NoteOn(value)) {
                    self.error = io.report("Could not set patch - file or resource not found");
                }
            }
            2 => {
                let note = self.operand(data, 1)?;
                io.effect(TrackEffect::NoteOn(note as i32));
            }
            3 => {
                self.operand(data, 1)?;
                return Ok(self.current.as_ref().map_or(true, |sound| sound.stop()));
            }
            4 | 5 | 6 | 0x4A => {
                let destination = self.reg_operand(data, io)?;
                let value = match opcode {
                    4 => self.operand(data, 1)? as i32,
                    5 => self.operand(data, 4)? as i32,
                    _ => {
                        let source = self.reg_operand(data, io)?;
                        self.reg_value(source, io)
                    }
                };
                if !self.set_register(destination, value, 0, io) {
                    self.error = io.report("Could not set register");
                }
            }
            7 | 0x0A => {
                let target = self.operand(data, 4)?;
                self.return_stack.push(self.pc);
                self.pc = if opcode == 7 {
                    target as usize
                } else {
                    io.entry(target).ok_or(Unsupported("SS1-null-entry"))?
                };
            }
            8 if !self.return_stack.is_empty() => {
                self.pc = self.return_stack.pop().expect("nonempty");
            }
            8 | 0x0C => {
                self.executing = false;
                self.playing_reference = false;
            }
            9 => {
                let register = self.reg_operand(data, io)?;
                self.wait = self.reg_value(register, io);
            }
            0x0D => self.pc = self.operand(data, 4)? as usize,
            0x0F | 0x1D | 0x1E | 0x5D | 0x5E => (),
            0 | 0x1A | 0x34..=0x3D | 0x62..=0xFF => self.error = io.report("Illegal Instruction"),
            _ => return Err(Unsupported("SS1")),
        }
        Ok(true)
    }
    pub fn timer_callback(&mut self, elapsed: i32, data: &[u8], io: &mut dyn TrackIo) -> Result<(), Unsupported> {
        if self.error != 0 {
            return Ok(());
        }
        self.dependents.retain(|sound| sound.is_playing());
        if !self.executing && self.dependents.is_empty() {
            io.effect(TrackEffect::Kill);
            return Ok(());
        }
        if self.properties.get(&0xFF0A_D592).copied().unwrap_or(0) != 0 {
            let register = self.properties.get(&0xFF47_F685).copied().unwrap_or(0) as u32;
            let current = self.reg_value(register, io);
            let endpoint = self.properties.get(&0xFF2D_16F2).copied().unwrap_or(0);
            let mut speed = self.properties.get(&0xFFD1_C82F).copied().unwrap_or(0);
            if speed == 0 {
                self.properties.insert(0xFF0A_D592, 0);
            } else {
                if (current < endpoint) != (speed > 0) {
                    speed = speed.wrapping_neg();
                }
                let mut next = current.wrapping_add(speed.wrapping_mul(elapsed) / 1000);
                if (speed > 0 && next > endpoint) || (speed < 0 && next < endpoint) {
                    next = endpoint;
                    self.properties.insert(0xFF0A_D592, 0);
                }
                self.set_register(register, next, 0, io);
            }
        }
        if self.completion_after_ramp && self.properties.get(&0xFF0A_D592).copied().unwrap_or(0) == 0 {
            self.completion_after_ramp = false;
            io.effect(TrackEffect::Kill);
            return Ok(());
        }
        if self.wait != 0 {
            self.wait = self.wait.wrapping_sub(self.wait.min(elapsed));
        }
        let mut count = 0usize;
        while self.error == 0 && self.executing && self.wait == 0 {
            self.execute_next(data, io)?;
            if count > 100 {
                self.error = io.report("Too many instructions executed without wait (infinite loop?)");
            }
            count += 1;
        }
        if self.note_countdown != 0 {
            self.note_countdown = self.note_countdown.wrapping_sub(1);
            if self.note_countdown == 0 {
                io.effect(TrackEffect::NoteOff);
            }
        }
        Ok(())
    }
}
