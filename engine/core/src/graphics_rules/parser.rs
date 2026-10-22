// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned command parser for the explicitly published literal subset (§§3–4/12).
//!
//! GR4 is open: this is not a claimed reconstruction of the unspecified nSCRes
//! expression/lexical grammar. Unsupported expressions fail rather than being
//! evaluated by an OS provider or an original rule-manager callback.

use super::environment::{FLAGS, NUMBERS, STRINGS};
use super::model::{grow, threshold_from_real, OptionGroup, Properties, PropertySet, Rule};
use std::collections::BTreeMap;

/// Filesystem acquisition only. None means absent; Err means acquisition failed.
pub trait ScriptFiles {
    fn read(&mut self, path: &str) -> Result<Option<String>, String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub path: String,
    pub line: usize,
    pub message: String,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.path, self.line, self.message)
    }
}
impl std::error::Error for ParseError {}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedRules {
    pub rules: Vec<Rule>,
    pub groups: Vec<OptionGroup>,
    pub card_identity: Option<String>,
}

enum Block {
    Rule(usize),
    Group { group: u32, option: Option<u32> },
    Vendor,
}

/// All parser-owned lists are discarded on failure and moved on success.
pub struct RuleParser<'a, P> {
    properties: &'a P,
    output: ParsedRules,
    blocks: Vec<Block>,
    symbols: BTreeMap<String, String>,
    vendor_id: u32,
    device_id: u32,
    active_vendor: Option<String>,
    include_stack: Vec<String>,
}

impl<'a, P: Properties> RuleParser<'a, P> {
    pub fn new(properties: &'a P, vendor_id: u32, device_id: u32, prefs_first_time: bool) -> Self {
        let mut symbols = BTreeMap::new();
        symbols.insert("prefsFirstTime".into(), prefs_first_time.to_string());
        Self {
            properties,
            output: ParsedRules::default(),
            blocks: Vec::new(),
            symbols,
            vendor_id,
            device_id,
            active_vendor: None,
            include_stack: Vec::new(),
        }
    }

    /// Both root paths are acquired before command parsing starts (§3).
    pub fn parse_files(
        mut self,
        files: &mut impl ScriptFiles,
        rules_path: &str,
        options_path: &str,
    ) -> Result<ParsedRules, ParseError> {
        let rules = files.read(rules_path);
        let options = files.read(options_path);
        let rules = rules
            .map_err(|message| error(rules_path, 0, message))?
            .ok_or_else(|| error(rules_path, 0, "Missing rules file"))?;
        let options = options.map_err(|message| error(options_path, 0, message))?;
        // GR4: registration 1/0 semantics are not specified. The literal optional
        // user file is skipped when absent; the primary rules file is required.
        self.source(files, rules_path, &rules)?;
        if let Some(options) = options {
            self.source(files, options_path, &options)?;
        }
        self.finish(options_path)
    }

    /// In-memory root script, with the same internal parser and include handling.
    pub fn parse_text(
        mut self,
        files: &mut impl ScriptFiles,
        path: &str,
        text: &str,
    ) -> Result<ParsedRules, ParseError> {
        self.source(files, path, text)?;
        self.finish(path)
    }

    fn finish(self, path: &str) -> Result<ParsedRules, ParseError> {
        if !self.blocks.is_empty() {
            return Err(error(path, 0, "Unclosed block"));
        }
        Ok(self.output)
    }

    fn source(&mut self, files: &mut impl ScriptFiles, path: &str, text: &str) -> Result<(), ParseError> {
        // GR4: cycles are reported, not guessed to mean an empty include.
        if self.include_stack.iter().any(|p| p == path) {
            return Err(error(path, 0, "Include cycle"));
        }
        self.include_stack.push(path.to_owned());
        let result = self.lines(files, path, text);
        self.include_stack.pop();
        result
    }

    fn lines(&mut self, files: &mut impl ScriptFiles, path: &str, text: &str) -> Result<(), ParseError> {
        for (line, text) in text.split('\n').enumerate() {
            let line = line + 1;
            let mut args = tokenize(text).map_err(|m| error(path, line, m))?;
            if args.is_empty() {
                continue;
            }
            // GR4: only the explicitly described whole-token $name form.
            for arg in &mut args[1..] {
                if let Some(name) = arg.strip_prefix('$') {
                    *arg = self
                        .symbols
                        .get(name)
                        .cloned()
                        .ok_or_else(|| error(path, line, "Unknown symbol"))?;
                }
            }
            if args[0] == "sinclude" {
                arity(&args, 2).map_err(|m| error(path, line, m))?;
                let include_path = relative_path(path, &args[1]);
                let included = files
                    .read(&include_path)
                    .map_err(|m| error(path, line, m))?
                    .ok_or_else(|| error(path, line, "Missing included file"))?;
                self.source(files, &include_path, &included)?;
            } else {
                self.command(&args).map_err(|m| error(path, line, m))?;
            }
        }
        Ok(())
    }

    fn command(&mut self, a: &[String]) -> Result<(), String> {
        match a[0].as_str() {
            "set" => {
                arity(a, 3)?;
                self.symbols.insert(a[1].clone(), a[2].clone());
            }
            "end" => {
                arity(a, 1)?;
                self.blocks.pop().ok_or("Unexpected end")?;
            }
            "rule" | "partialRule" => self.begin_rule(a)?,
            "optionGroup" => {
                arity(a, 2)?;
                if !self.blocks.is_empty() {
                    return Err("Unexpected optionGroup".into());
                }
                let group = unsigned(&a[1])?;
                grow(&mut self.output.groups, group)?;
                self.blocks.push(Block::Group { group, option: None });
            }
            "vendor" => self.begin_vendor(a)?,
            "card" => {
                arity(a, 3)?;
                if !matches!(self.blocks.last(), Some(Block::Vendor)) {
                    return Err("Unexpected card".into());
                }
                let device = unsigned(&a[1])?;
                if self.output.card_identity.is_none() && device == self.device_id {
                    if let Some(vendor) = &self.active_vendor {
                        self.output.card_identity = Some(format!("{vendor}, {}", a[2]));
                    }
                }
            }
            "option" => match self.blocks.last_mut() {
                Some(Block::Group { group, option }) => {
                    arity(a, 2)?;
                    let index = unsigned(&a[1])?;
                    grow(&mut self.output.groups[*group as usize].options, index)?;
                    *option = Some(index);
                }
                Some(Block::Rule(index)) => {
                    arity(a, 3)?;
                    self.output.rules[*index]
                        .recommendations
                        .push((unsigned(&a[1])?, unsigned(&a[2])?));
                }
                _ => return Err("Unexpected option".into()),
            },
            "property" => {
                arity(a, 3)?;
                let properties = self.properties;
                let set = match self.blocks.last() {
                    Some(Block::Rule(index)) => &mut self.output.rules[*index].properties,
                    Some(Block::Group {
                        group,
                        option: Some(option),
                    }) => &mut self.output.groups[*group as usize].options[*option as usize],
                    // §4.1 gives a structural precondition, no implicit option.
                    _ => return Err("Property requires a rule or selected option".into()),
                };
                add_property(set, properties, &a[1], &a[2])?;
            }
            "has" | "hasNo" => {
                arity(a, 2)?;
                let bit = 1 << enum_index(&FLAGS, &a[1])?;
                let rule = self.current_rule()?;
                rule.flag_mask |= bit;
                if a[0] == "has" {
                    rule.flag_expected |= bit;
                } else {
                    rule.flag_expected &= !bit;
                }
            }
            "atLeast" | "atMost" => {
                arity(a, 3)?;
                let index = enum_index(&NUMBERS, &a[1])?;
                let threshold = threshold_from_real(real(&a[2])?);
                let rule = self.current_rule()?;
                if a[0] == "atLeast" {
                    rule.lower.push((index, threshold));
                } else {
                    rule.upper.push((index, threshold));
                }
            }
            "stringMatch" | "stringNotMatch" => {
                arity(a, 3)?;
                let index = enum_index(&STRINGS, &a[1])?;
                let rule = self.current_rule()?;
                if a[0] == "stringMatch" {
                    rule.positive.push((index, a[2].clone()));
                } else {
                    rule.negative.push((index, a[2].clone()));
                }
            }
            _ => return Err("Unknown command".into()),
        }
        Ok(())
    }

    fn current_rule(&mut self) -> Result<&mut Rule, String> {
        match self.blocks.last() {
            Some(Block::Rule(index)) => Ok(&mut self.output.rules[*index]),
            _ => Err("Condition requires a rule".into()),
        }
    }

    fn begin_rule(&mut self, a: &[String]) -> Result<(), String> {
        if !self.blocks.iter().all(|b| matches!(b, Block::Rule(_))) {
            return Err("Unexpected rule".into());
        }
        let mut rule = Rule {
            depth: self.blocks.len() as u32,
            partial: a[0] == "partialRule",
            name: self
                .output
                .rules
                .last()
                .map_or_else(|| "unknown".into(), |r| format!("{}>", r.name)),
            ..Rule::default()
        };
        let mut named = false;
        for arg in &a[1..] {
            match arg.as_str() {
                "-partial" => rule.partial = true,
                "-any" => rule.any = true,
                _ if !named && !arg.starts_with('-') => {
                    rule.name.clone_from(arg);
                    named = true;
                }
                _ => return Err("Bad rule arguments".into()),
            }
        }
        self.blocks.push(Block::Rule(self.output.rules.len()));
        self.output.rules.push(rule);
        Ok(())
    }

    fn begin_vendor(&mut self, a: &[String]) -> Result<(), String> {
        if a.len() < 3 {
            return Err("Bad argument count".into());
        }
        if !self.blocks.is_empty() {
            return Err("Unexpected vendor".into());
        }
        if self.output.card_identity.is_none() {
            self.active_vendor = None;
            for id in &a[2..] {
                if unsigned(id)? == self.vendor_id {
                    self.active_vendor = Some(a[1].clone());
                }
            }
        } else {
            for id in &a[2..] {
                unsigned(id)?;
            }
        }
        self.blocks.push(Block::Vendor);
        Ok(())
    }
}

fn error(path: &str, line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        path: path.to_owned(),
        line,
        message: message.into(),
    }
}

fn arity(a: &[String], expected: usize) -> Result<(), String> {
    if a.len() == expected {
        Ok(())
    } else {
        Err("Bad argument count".into())
    }
}

fn enum_index(names: &[&str], name: &str) -> Result<usize, String> {
    names
        .iter()
        .position(|&n| n == name)
        .ok_or_else(|| format!("Unknown enum: {name}"))
}

fn unsigned(s: &str) -> Result<u32, String> {
    // GR4: decimal plus the hexadecimal literal form in published V9.
    if let Some(hex) = s.strip_prefix("0x") {
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Bad integer".into());
        }
        u32::from_str_radix(hex, 16).map_err(|_| "Bad integer".into())
    } else {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Bad integer".into());
        }
        s.parse().map_err(|_| "Bad integer".into())
    }
}

fn signed(s: &str) -> Result<i32, String> {
    // GR4: literal signed decimals, and unsigned hex bit patterns, only.
    if s.starts_with("0x") {
        return unsigned(s).map(|n| n as i32);
    }
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Bad integer".into());
    }
    s.parse().map_err(|_| "Bad integer".into())
}

/// Literal real reader. Deliberately does not invent nSCRes expression operators.
pub fn real(s: &str) -> Result<f64, String> {
    let b = s.as_bytes();
    let mut p = usize::from(b.first().is_some_and(|c| *c == b'+' || *c == b'-'));
    let mut digits = 0;
    while p < b.len() && b[p].is_ascii_digit() {
        p += 1;
        digits += 1;
    }
    if p < b.len() && b[p] == b'.' {
        p += 1;
        while p < b.len() && b[p].is_ascii_digit() {
            p += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err("Bad real number".into());
    }
    if p < b.len() && (b[p] == b'e' || b[p] == b'E') {
        p += 1;
        if p < b.len() && (b[p] == b'+' || b[p] == b'-') {
            p += 1;
        }
        let start = p;
        while p < b.len() && b[p].is_ascii_digit() {
            p += 1;
        }
        if start == p {
            return Err("Bad real number".into());
        }
    }
    if p != b.len() {
        return Err("Bad real number".into());
    }
    s.parse().map_err(|_| "Bad real number".into())
}

fn add_property(set: &mut PropertySet, properties: &impl Properties, name: &str, value: &str) -> Result<(), String> {
    if let Some(index) = properties.find_bool(name) {
        let value = match value {
            "true" => true,
            "false" => false,
            _ => return Err("Bad boolean".into()),
        };
        set.bools.push((index, value));
    } else if let Some(index) = properties.find_int(name) {
        set.ints.push((index, signed(value)?));
    } else if let Some(index) = properties.find_float(name) {
        set.floats.push((index, real(value)? as f32));
    } else {
        return Err("Unknown render property".into());
    }
    Ok(())
}

fn tokenize(line: &str) -> Result<Vec<String>, String> {
    // GR4: literal spaces/tabs/CR, double-quoted strings and unquoted # comments.
    // Backslashes are literal; no undocumented escape language is added.
    let mut chars = line.chars().peekable();
    let mut out = Vec::new();
    while let Some(&c) = chars.peek() {
        if matches!(c, ' ' | '\t' | '\r') {
            chars.next();
            continue;
        }
        if c == '#' {
            break;
        }
        let mut word = String::new();
        if c == '"' {
            chars.next();
            loop {
                match chars.next() {
                    None => return Err("Unclosed quote".into()),
                    Some('"') => break,
                    Some(c) => word.push(c),
                }
            }
            if chars.peek().is_some_and(|c| !matches!(c, ' ' | '\t' | '\r' | '#')) {
                return Err("Missing argument separator".into());
            }
        } else {
            while let Some(&c) = chars.peek() {
                if matches!(c, ' ' | '\t' | '\r' | '#') {
                    break;
                }
                if c == '"' {
                    return Err("Missing argument separator".into());
                }
                word.push(c);
                chars.next();
            }
        }
        out.push(word);
    }
    Ok(out)
}

pub(super) fn relative_path(current: &str, file: &str) -> String {
    // GR4: parent-relative literal paths; normalization/search policy is unspecified.
    if file.starts_with(['/', '\\']) || file.as_bytes().get(1) == Some(&b':') {
        return file.to_owned();
    }
    match current.rfind(['/', '\\']) {
        Some(p) => format!("{}{file}", &current[..=p]),
        None => file.to_owned(),
    }
}

pub fn card_ids(version: &str) -> Result<(u32, u32), String> {
    let bytes = version.as_bytes();
    if bytes.len() <= 26 {
        return Ok((0, 0));
    }
    let parse = |part| {
        // GR6: malformed hexadecimal returns an error, not a guessed PCI ID.
        std::str::from_utf8(part)
            .ok()
            .filter(|s| s.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .ok_or_else(|| "Bad card ID".to_owned())
    };
    Ok((parse(&bytes[8..12])?, parse(&bytes[22..26])?))
}
