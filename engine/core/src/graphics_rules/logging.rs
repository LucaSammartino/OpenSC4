// SPDX-License-Identifier: GPL-3.0-or-later
//! Configuration logging (§6); byte-exact unspecified layout is tracked in GR6.

use super::{Environment, Properties, PropertyValue, FLAGS, NUMBERS, STRINGS};

/// File acquisition/writes, not preformatted manager output.
pub trait LogFile {
    fn open(&mut self, path: &str) -> bool;
    fn write(&mut self, text: &str);
    fn close(&mut self);
}

#[derive(Default)]
pub struct NoLog;
impl LogFile for NoLog {
    fn open(&mut self, _: &str) -> bool {
        false
    }
    fn write(&mut self, _: &str) {}
    fn close(&mut self) {}
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
pub fn general(value: f32) -> String {
    // GR6: nonfinite MSVC spellings have not been published; these are portable
    // spellings, not a claim of native byte-exact logging on injected NaNs/Infs.
    if !value.is_finite() {
        return value.to_string();
    }
    let rounded = format!("{value:.5e}");
    let (mantissa, exponent) = rounded.split_once('e').expect("scientific format");
    let exponent: i32 = exponent.parse().expect("formatted exponent");
    if !(-4..6).contains(&exponent) {
        let mantissa = trim_fraction(mantissa.to_owned());
        format!("{mantissa}e{exponent:+03}")
    } else {
        let decimals = (5 - exponent).max(0) as usize;
        trim_fraction(format!("{value:.decimals$}"))
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

pub(super) fn configuration(log: &mut impl LogFile, env: &Environment, stamp: &LogStamp) {
    log.write(&format!(
        "Log generated on {}, {:02}:{:02}\n\nFlags\n-----\n",
        stamp.date, stamp.hour, stamp.minute
    ));
    for (i, name) in FLAGS.iter().enumerate() {
        log.write(&format!("{name} = {}\n", env.flags & (1 << i) != 0));
    }
    log.write("\nStrings\n-------\n");
    for (name, value) in STRINGS.iter().zip(&env.strings) {
        log.write(&format!("{name} = {value}\n"));
    }
    log.write("\nNumbers\n-------\n");
    for (name, value) in NUMBERS.iter().zip(env.numbers) {
        log.write(&format!("{name} = {}\n", general(value)));
    }
    log.write("\n");
}

pub(super) fn options(
    log: &mut impl LogFile,
    properties: &impl Properties,
    preferences: Option<&[u8; 12]>,
    recommended: impl Fn(u32) -> u32,
) {
    // GR6: the spec gives the data and ordering, not exact dump separators.
    for (name, value) in properties.dump() {
        let text = match value {
            PropertyValue::Bool(v) => v.to_string(),
            PropertyValue::Int(v) => v.to_string(),
            PropertyValue::Float(v) => general(v),
        };
        log.write(&format!("{name} = {text}\n"));
    }
    if let Some(preferences) = preferences {
        for ((label, group, _), current) in SUMMARY.iter().zip(preferences) {
            // GR6: padded label width is unspecified; literal example uses 20.
            log.write(&format!(
                "{label:<20} current: {current:3} recommended: {:3}\n",
                recommended(*group) as i32
            ));
        }
    }
}

pub(super) fn in_directory(directory: &str, file: &str) -> String {
    if directory.is_empty() || directory.ends_with(['/', '\\']) {
        format!("{directory}{file}")
    } else {
        format!("{directory}/{file}")
    }
}
