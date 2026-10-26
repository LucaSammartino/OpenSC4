// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled driver preference and UI contracts from gz-graphics §5.4.
use super::*;
use std::sync::{Arc, OnceLock};

pub trait DriverSettings {
    fn exists(&self, path: &str) -> bool;
    /// None means absent; Some(empty) deliberately suppresses built-in defaults.
    fn entries(&self, path: &str, section: &str) -> Option<Vec<String>>;
    fn value(&self, path: &str, section: &str, key: &str) -> Option<String>;
    // SPEC-GAP: parsing/plugin-failure ownership stays at the §5.4 observation boundary.
    fn load(&mut self, entry: &str) -> Option<Arc<dyn Driver>>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiArgument {
    None,
    Word(u32),
    Words(Vec<u32>),
    Floats(Vec<f32>),
    Projection([f32; 16]),
}

static UI_PROJECTION: OnceLock<[f32; 16]> = OnceLock::new();

impl GraphicSystem {
    pub fn load_all_drivers(&mut self, ini: &mut dyn DriverSettings, app_directory: &str) -> bool {
        let path = if !self.driver_ini.is_empty() && ini.exists(&self.driver_ini) {
            self.driver_ini.clone()
        } else {
            format!("{}/GZGraphic2.ini", app_directory.trim_end_matches(['/', '\\']))
        };
        let entries = ini.entries(&path, "Graphics Drivers").unwrap_or_else(|| {
            [
                "SimGLRef.dll, C4554841",
                "SimGLSoft.dll, 7ACA35C6",
                "SimGLDx7.dll, BADB6906",
            ]
            .map(str::to_owned)
            .to_vec()
        });
        for entry in entries {
            if let Some(driver) = ini.load(&entry) {
                self.drivers.push(driver);
            }
        }
        let explicit = (self.default_driver_class != 0)
            .then(|| self.driver_by_id(self.default_driver_class).cloned())
            .flatten();
        let chosen = explicit.or_else(|| {
            let preferred = ini
                .value(&path, "Current Graphics Driver", "Driver")
                .and_then(|s| scan_hex(&s))
                .unwrap_or(0xbadb6906);
            self.driver_by_id(preferred)
                .or_else(|| self.driver_by_id(0x7aca35c6))
                .cloned()
        });
        let Some(driver) = chosen else { return false };
        self.set_driver(Some(&driver));
        true
    }

    pub fn control_slot17(&mut self, _word: u32) {}

    pub fn control_slot20(&self) -> f32 {
        self.control_float_f0
    }

    pub(super) fn apply_ui_rendering_mode(&self) -> Result<()> {
        let driver = self.driver().ok_or(Error::Detached)?;
        if !driver.ui_available() {
            return Ok(());
        }
        use UiArgument::*;
        for (slot, arg) in [
            (0xcc, Word(1)),
            (0xb4, Word(5)),
            (0xcc, Word(0)),
            (0xb0, Word(5)),
            (0xd4, Words(vec![0, 3, 2, 1])),
            (0xd0, Word(0)),
            (0xb4, Word(2)),
            (0xb0, Word(1)),
            (0xb4, Word(1)),
            (0xb4, Word(0)),
            (0xb4, Word(4)),
            (0xb4, Word(3)),
            (0xb4, Word(6)),
            (0x84, Floats(vec![1.])),
            (0x80, Floats(vec![1., 1., 1.])),
            (0x44, Word(0)),
            (0x3c, Word(1)),
            (0x88, Words(vec![1, 1])),
            (0x10c, Word(0)),
            (0x12c, None),
        ] {
            driver.ui_call(slot, arg)?;
        }
        let viewport = driver.ui_viewport()?; // +0x130, even when the static matrix already exists.
        let projection = *UI_PROJECTION.get_or_init(|| {
            let mut p = [0.; 16];
            p[0] = (2. / f64::from(viewport.width())) as f32;
            p[5] = (-2. / f64::from(viewport.height())) as f32;
            p[12..].copy_from_slice(&[-1., 1., 0.5, 1.]);
            p
        });
        for (slot, arg) in [
            (0xa0, Word(1)),
            (0xa4, Projection(projection)),
            (0xa0, Word(0)),
            (0xa8, None),
        ] {
            driver.ui_call(slot, arg)?;
        }
        Ok(())
    }
}

fn scan_hex(s: &str) -> Option<u32> {
    let s = s.trim_start();
    let (negative, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let digits: String = s.chars().take_while(char::is_ascii_hexdigit).collect();
    let value = u32::from_str_radix(&digits, 16).ok()?;
    Some(if negative { value.wrapping_neg() } else { value })
}
