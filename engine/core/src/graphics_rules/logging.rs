// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte-mode configuration logging (§6); exceptional numeric spellings remain GR6.

use super::{Environment, ManagerError, Properties, PropertyValue, FLAGS, NUMBERS, STRINGS};

/// File acquisition/writes, not preformatted manager output.
pub trait LogFile {
    fn open(&mut self, path: &str) -> bool;
    fn is_open(&self) -> bool;
    fn write_bytes(&mut self, bytes: &[u8]) -> bool;
    fn close(&mut self) -> bool;
}

#[derive(Default)]
pub struct NoLog;
impl LogFile for NoLog {
    fn open(&mut self, _: &str) -> bool {
        false
    }
    fn is_open(&self) -> bool {
        false
    }
    fn write_bytes(&mut self, _: &[u8]) -> bool {
        false
    }
    fn close(&mut self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Default)]
pub struct LogStamp {
    /// Preformatted date from the shared clock service.
    pub date: String,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Clone, Debug, Default)]
pub struct LogContext {
    pub app_directory: String,
    /// Value of -writeLog, not the entire command line. Only "off" disables it.
    pub write_log: Option<String>,
    pub stamp: LogStamp,
    /// Unsigned application preference bytes, in SUMMARY order below, if present.
    pub preferences: Option<[u8; 12]>,
}

pub const SUMMARY: [(&str, u32, u32); 12] = [
    ("Visual Effects", 0, 0xf0d),
    ("Cars/Sims", 1, 0xf0e),
    ("Shadows", 2, 0xf0f),
    ("Atmospheric effects", 4, 0xf11),
    ("Water effects", 5, 0xf12),
    ("Software/Hardware", 6, 0xf13),
    ("Cursor", 7, 0xf14),
    ("VariableSpeedAutomata", 10, 0xf18),
    ("Translucent UI", 8, 0xf15),
    ("Graphic Detail", 9, 0xf16),
    ("Building Load Speed", 15, 0xf28),
    ("Texture Quality", 3, 0xf10),
];

/// C's default six-significant-digit %g for finite binary32 values.
pub fn general(value: f32) -> Result<String, ManagerError> {
    // GR6: do not silently invent native exceptional-value spellings.
    if !value.is_finite() {
        return Err(ManagerError::Unsupported("GR6 nonfinite %g spelling"));
    }
    let rounded = format!("{value:.5e}");
    let (mantissa, exponent) = rounded.split_once('e').expect("scientific format");
    let exponent: i32 = exponent.parse().expect("formatted exponent");
    if !(-4..6).contains(&exponent) {
        let mantissa = trim_fraction(mantissa.to_owned());
        Ok(format!("{mantissa}e{exponent:+03}"))
    } else {
        let decimals = (5 - exponent).max(0) as usize;
        Ok(trim_fraction(format!("{value:.decimals$}")))
    }
}

fn trim_fraction(mut text: String) -> String {
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// §6.1 WriteLine ignores failures at its callers, but skips CRLF on payload failure.
pub(super) fn line(log: &mut impl LogFile, payload: &[u8]) -> bool {
    log.is_open() && log.write_bytes(payload) && log.write_bytes(b"\r\n")
}

pub(super) fn text(log: &mut impl LogFile, payload: &str) {
    // Keep each native line as a distinct two-write operation, including empty lines.
    for payload in payload.strip_suffix('\n').unwrap_or(payload).split('\n') {
        line(log, payload.as_bytes());
    }
}

pub(super) fn configuration(log: &mut impl LogFile, env: &Environment, stamp: &LogStamp) -> Result<(), ManagerError> {
    if log.is_open() {
        line(
            log,
            format!("Log generated on {}, {:02}:{:02}", stamp.date, stamp.hour, stamp.minute).as_bytes(),
        );
    }
    text(log, "\nFlags\n-----\n");
    for (i, name) in FLAGS.iter().enumerate() {
        if log.is_open() {
            line(log, format!("{name} = {}", env.flags & (1 << i) != 0).as_bytes());
        }
    }
    text(log, "\nStrings\n-------\n");
    for (name, value) in STRINGS.iter().zip(&env.strings) {
        if log.is_open() {
            line(
                log,
                format!("{name} = {}", value.split('\0').next().unwrap_or("")).as_bytes(),
            );
        }
    }
    text(log, "\nNumbers\n-------\n");
    for (name, value) in NUMBERS.iter().zip(env.numbers) {
        if log.is_open() {
            line(log, format!("{name} = {}", general(value)?).as_bytes());
        }
    }
    text(log, "\n");
    Ok(())
}

pub(super) fn options(
    log: &mut impl LogFile,
    properties: &impl Properties,
    preferences: Option<&[u8; 12]>,
    recommended: impl Fn(u32) -> u32,
) -> Result<(), ManagerError> {
    text(log, "Options\n-------\n");
    if let Some(preferences) = preferences {
        for ((label, group, _), current) in SUMMARY.iter().zip(preferences) {
            if !log.is_open() {
                continue;
            }
            text(
                log,
                &format!(
                    // GR8: the listed VariableSpeedAutomata is 21 bytes. Follow the
                    // explicit 20-byte field/current-at-21 requirement literally.
                    "{label:<20.20} current: {current:3} recommended: {:3}\n",
                    recommended(*group) as i32
                ),
            );
        }
    }
    text(log, "\nProperties\n-------\n");
    let dump = properties.dump();
    for kind in 0..3 {
        for (name, value) in &dump {
            if !log.is_open() {
                continue;
            }
            let formatted = match (kind, value) {
                (0, PropertyValue::Bool(v)) => v.to_string(),
                (1, PropertyValue::Int(v)) => v.to_string(),
                (2, PropertyValue::Float(v)) => general(*v)?,
                _ => continue,
            };
            let mut payload = name[..name.iter().position(|b| *b == 0).unwrap_or(name.len())].to_vec();
            payload.extend_from_slice(b" = ");
            payload.extend_from_slice(formatted.as_bytes());
            line(log, &payload);
        }
        text(log, "\n");
    }
    Ok(())
}

pub(super) fn in_directory(directory: &str, file: &str) -> String {
    // GR10: bases are opaque; even an absent trailing separator is preserved.
    format!("{directory}{file}")
}
