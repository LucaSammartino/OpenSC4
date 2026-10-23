// SPDX-License-Identifier: GPL-3.0-or-later
//! Hardware/OS acquisition boundary and manager-owned conversions (§§2–2.1).

pub const FLAGS: [&str; 7] = [
    "hardwareDriver",
    "DXT",
    "fullscreen",
    "MMX",
    "FPU",
    "backingStore",
    "forcedFSAA",
];
pub const NUMBERS: [&str; 11] = [
    "cpuSpeed",
    "memory",
    "freeMemory",
    "screenWidth",
    "screenHeight",
    "screenBPP",
    "videoMemory",
    "textureMemory",
    "textureStages",
    "bogoFPS",
    "bogoMPPS",
];
pub const STRINGS: [&str; 16] = [
    "userName",
    "appName",
    "version",
    "osVersion",
    "computerName",
    "CPU",
    "sglDriverName",
    "sglDriverVersion",
    "driverName",
    "driverVersion",
    "cardName",
    "cardVersion",
    "cardIdentity",
    "soundDriverName",
    "soundCardName",
    "buildType",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    pub flags: u32,
    pub strings: [String; 16],
    pub numbers: [f32; 11],
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            flags: 0,
            strings: std::array::from_fn(|_| String::new()),
            numbers: [0.0; 11],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Measurements {
    pub cpu_features: u32,
    pub cpu_mhz: u32,
    pub physical_bytes: u32,
    pub available_bytes: u32,
    pub screen: [u32; 3],
    pub video_bytes: u32,
    pub texture_bytes: u32,
    pub os_strings: [String; 6],
    pub driver_record: [u8; 0x34],
    pub driver_description: String,
    pub sound_names: Option<[String; 2]>,
    pub backing_store: bool,
}

pub trait InformationProvider {
    fn measure(&mut self) -> Measurements;
    /// Called on every gather, regardless of forceSoftwareDriver.
    /// The provider implements the renderer/readback operation, not rule decisions.
    fn probe_forced_fsaa(&mut self, driver_record: &[u8; 0x34]) -> bool;
}

fn memory_mib(bytes: u32) -> f32 {
    (((bytes >> 20).wrapping_sub(1) | 3).wrapping_add(1)) as f32
}

impl Environment {
    pub fn gather(&mut self, provider: &mut impl InformationProvider, force_software: bool) {
        let m = provider.measure();
        let fsaa = provider.probe_forced_fsaa(&m.driver_record);
        self.flags = u32::from(m.driver_record[0x30] != 0 && !force_software)
            | (u32::from(m.driver_record[0x2c] != 0) << 1)
            | (u32::from(m.driver_record[0x20] != 0) << 2)
            | (u32::from(m.cpu_features & (1 << 23) != 0) << 3)
            | (u32::from(m.cpu_features & 1 != 0) << 4)
            | (u32::from(m.backing_store) << 5)
            | (u32::from(fsaa) << 6);
        self.numbers[0] = m.cpu_mhz as f32;
        self.numbers[1] = memory_mib(m.physical_bytes);
        self.numbers[2] = memory_mib(m.available_bytes);
        for (out, value) in self.numbers[3..6].iter_mut().zip(m.screen) {
            *out = value as f32;
        }
        self.numbers[6] = (m.video_bytes >> 20) as f32;
        self.numbers[7] = (m.texture_bytes >> 20) as f32;
        self.numbers[8] = u32::from_le_bytes(m.driver_record[0x24..0x28].try_into().expect("four-byte field")) as f32;
        // bogoFPS and bogoMPPS are intentionally untouched, including on regather.
        self.strings[..6].clone_from_slice(&m.os_strings);
        let mut fields = m.driver_description.split('\n').skip(1);
        for out in &mut self.strings[6..12] {
            // §2.2: exhausted LF-only remainder reads as empty; preserve CR.
            *out = fields.next().unwrap_or("").to_owned();
        }
        // §2.2: only ParseConfigFile resets cardIdentity; gather retains index 12.
        let sound = m.sound_names.unwrap_or_else(|| ["Unknown".into(), "Unknown".into()]);
        self.strings[13..15].clone_from_slice(&sound);
        self.strings[15] = "Release".into();
    }
}

/// Optional provider helper: alpha is ignored, exactly two colored samples suffice.
pub fn forced_fsaa_from_samples(hardware: bool, samples: Option<&[[u8; 4]; 64]>) -> bool {
    hardware
        && samples.is_some_and(|pixels| pixels.iter().filter(|p| p[..3].iter().any(|&c| c != 0)).take(2).count() > 1)
}
