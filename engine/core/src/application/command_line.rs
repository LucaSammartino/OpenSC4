// SPDX-License-Identifier: GPL-3.0-or-later
//! Published command-line effects and resolution selection (application.md section 8).

use crate::cmdline::CmdLine;

use super::{Application, Boundary, Effect, Host};

#[derive(Clone, Debug, PartialEq)]
pub struct Preferences {
    pub ime: bool,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub fullscreen: bool,
    pub pause_on_focus_loss: bool,
    pub audio_04: bool,
    pub audio_69: bool,
    pub radio: u32,
    pub language: Vec<u8>,
    pub simulation: f32,
    pub rdd: bool,
    pub driver_f13: u8,
    pub driver_f0c: u8,
}

/// Named switches use the existing cRZCmdLine parser, preserving its first-match behavior.
pub(crate) fn option(line: &CmdLine, name: &[u8]) -> Option<Vec<u8>> {
    let mut value = Vec::new();
    line.is_switch_present_value(name, &mut value, false).then_some(value)
}

fn character(line: &CmdLine, name: u8) -> Option<Vec<u8>> {
    let mut value = Vec::new();
    line.is_switch_present_char(name, &mut value, false).then_some(value)
}

pub(crate) fn on_off(value: &[u8]) -> Option<bool> {
    match value.to_ascii_lowercase().as_slice() {
        b"on" | b"enabled" => Some(true),
        b"off" | b"disabled" => Some(false),
        _ => None,
    }
}

pub fn is_startup_resolution_ok(width: u32, height: u32, custom: bool) -> bool {
    custom
        || [(800, 600), (1024, 768), (1152, 864), (1280, 1024), (1600, 1200), (1800, 1440), (2048, 1536)]
            .iter()
            .any(|&(w, h)| height == h && width >= w)
}

pub fn fix_startup_resolution(width: &mut u32, height: &mut u32, custom: bool) {
    if is_startup_resolution_ok(*width, *height, custom) {
        return;
    }
    // Unsigned area, including native 32-bit overflow (section 8).
    let area = width.wrapping_mul(*height);
    (*width, *height) = match area {
        0..480_000 => (800, 600),
        480_000..786_432 => (1024, 768),
        786_432..995_328 => (1152, 864),
        995_328..1_310_720 => (1280, 1024),
        1_310_720..1_920_000 => (1600, 1200),
        1_920_000..2_592_000 => (1800, 1440),
        2_592_000..3_145_728 => (2048, 1536),
        _ => (1024, 768),
    };
}

impl Application {
    /// Applies every fully specified option. Returns exact unresolved validation
    /// boundaries instead of asking the host for this owner's final decisions.
    /// AP8: cross-option order is not published; use the table's literal order.
    pub fn read_command_line(&mut self, line: &CmdLine, host: &mut impl Host) -> Vec<Boundary> {
        let mut unresolved = Vec::new();
        if let Some(value) = option(line, b"IME").as_deref().and_then(on_off) {
            self.preferences.ime = value;
        }
        if let Some(value) = character(line, b'r') {
            // AP8: scanf accepts partial assignments; final acceptance checks
            // are unpublished. Do not commit even a well-formed resolution.
            let mut values = [self.preferences.width, self.preferences.height, self.preferences.depth];
            let mut rest = value.as_slice();
            for (index, target) in values.iter_mut().enumerate() {
                let Some((number, used)) = unsigned_prefix(rest, 10) else { break };
                *target = number;
                rest = &rest[used..];
                if index < 2 {
                    if rest.first() != Some(&b'x') { break; }
                    rest = &rest[1..];
                }
            }
            let custom = option(line, b"CustomResolution").as_deref().and_then(on_off) == Some(true);
            let (mut width, mut height) = (values[0], values[1]);
            fix_startup_resolution(&mut width, &mut height, custom);
            unresolved.push(Boundary::ResolutionAcceptance { width, height, depth: values[2] });
        }
        if character(line, b'f').is_some() { self.preferences.fullscreen = true; }
        if character(line, b'w').is_some() { self.preferences.fullscreen = false; }
        if character(line, b'g').is_some_and(|v| matches!(v.first(), Some(b'p' | b'P'))) {
            self.preferences.pause_on_focus_loss = true;
        }
        if let Some(value) = character(line, b'd') {
            self.driver_id = match value.to_ascii_lowercase().as_slice() {
                b"opengl" | b"gl" => 0xC455_4841,
                b"soft" => 0x7ACA_35C6,
                b"dx" | b"directx" => 0xBADB_6906,
                other => unsigned_prefix(other, 16).map_or(0, |(v, _)| v),
            };
        }
        if let Some(value) = option(line, b"audio").or_else(|| option(line, b"sound")).as_deref().and_then(on_off) {
            self.preferences.audio_04 = value;
            self.preferences.audio_69 = value;
        }
        if let Some(value) = option(line, b"radio").or_else(|| option(line, b"music")).as_deref().and_then(on_off) {
            self.preferences.radio = u32::from(value);
        }
        if let Some(value) = option(line, b"ExceptionHandling") {
            let lower = value.to_ascii_lowercase();
            if let Some(enabled) = on_off(&lower) { self.set_exception_handling(enabled, host); }
            if lower == b"verbose" {
                self.set_exception_handling(true, host);
                self.exception_verbose = true;
                host.effect(Effect::ExceptionOption { index: 6, value: true });
            }
        }
        if let Some(value) = character(line, b'l') {
            let mut text = value.as_slice();
            if text.first() == Some(&b':') { text = &text[1..]; }
            if text.first() == Some(&b'"') { text = &text[1..]; }
            if text.last() == Some(&b'"') { text = &text[..text.len() - 1]; }
            self.preferences.language = text[..text.len().min(63)].to_vec();
        }
        if let Some(value) = option(line, b"UserDir") {
            if on_off(&value) == Some(false) {
                self.directories.user_base_11c.clone_from(&self.directories.plugin_a4);
            } else {
                // The further path acceptance checks are expressly untraced.
                unresolved.push(Boundary::UserDirectoryAcceptance(value));
            }
        }
        if option(line, b"CPUPriority").is_some_and(|v| v.eq_ignore_ascii_case(b"low")) {
            host.effect(Effect::ProcessPriority(0x40));
        }
        if let Some(value) = option(line, b"AutoZoomChange").as_deref().and_then(on_off) {
            host.effect(Effect::RenderProperty { key: 0x25, value: u32::from(value) });
        }
        if let Some(value) = option(line, b"sim") {
            // AP8: English numeric prefix, preserving prefs on no conversion.
            if let Some(value) = float_prefix(&value) { self.preferences.simulation = value; }
        }
        if let Some(value) = option(line, b"RDD").as_deref().and_then(on_off) {
            self.preferences.rdd = value;
        }
        if let Some(value) = option(line, b"CPUCount") {
            let value = signed_decimal(&value);
            host.effect(Effect::CpuCount(value));
        }
        unresolved
    }

    fn set_exception_handling(&mut self, enabled: bool, host: &mut impl Host) {
        if enabled != self.exception_enabled {
            self.exception_enabled = enabled;
            if enabled {
                self.previous_exception_filter = Some(host.install_exception_filter());
            } else if let Some(previous) = self.previous_exception_filter.take() {
                host.effect(Effect::RestoreExceptionFilter(previous));
            }
        }
    }
}

fn unsigned_prefix(text: &[u8], radix: u32) -> Option<(u32, usize)> {
    let mut index = text.iter().position(|b| !b.is_ascii_whitespace())?;
    let negative = text.get(index) == Some(&b'-');
    if matches!(text.get(index), Some(b'+' | b'-')) { index += 1; }
    if radix == 16 && text.get(index..index + 2).is_some_and(|s| s.eq_ignore_ascii_case(b"0x")) { index += 2; }
    let start = index;
    let mut value = 0u32;
    while let Some(digit) = text.get(index).and_then(|&b| (b as char).to_digit(radix)) {
        value = value.wrapping_mul(radix).wrapping_add(digit);
        index += 1;
    }
    (index > start).then_some((if negative { value.wrapping_neg() } else { value }, index))
}

fn signed_decimal(text: &[u8]) -> i32 {
    // AP8: native atol overflow is not specified; portable two's-complement wrap.
    unsigned_prefix(text, 10).map_or(0, |(v, _)| v as i32)
}

fn float_prefix(text: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(text).ok()?.trim_start();
    (1..=text.len()).rev().filter(|&n| text.is_char_boundary(n)).find_map(|n| text[..n].parse().ok())
}
