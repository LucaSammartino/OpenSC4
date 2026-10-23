// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned command parser for the published subset (§§3–4/12).
//!
//! GR4 is open: this is not a claimed reconstruction of the unspecified nSCRes
//! complete lexical grammar. Unsupported functions fail rather than being
//! evaluated by an OS provider or an original rule-manager callback.

use super::environment::{FLAGS, NUMBERS, STRINGS};
use super::model::{grow, threshold_from_real, OptionGroup, Properties, PropertySet, Rule};
use std::collections::BTreeMap;

/// Filesystem acquisition only. None means open failure; Err means another acquisition error.
/// ReadLine false (including a read failure) ends the returned prefix, not a new error (§3).
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
    /// Nonfatal optional-root errors; executed callbacks are retained (§3).
    pub diagnostics: Vec<ParseError>,
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

    /// Visit roots in registration order; only required-root errors fail (§3).
    pub fn parse_files(
        mut self,
        files: &mut impl ScriptFiles,
        rules_path: &str,
        options_path: &str,
    ) -> Result<ParsedRules, ParseError> {
        let required = match files.read(rules_path) {
            Ok(Some(text)) => self.source(files, rules_path, &text),
            Ok(None) => Err(error(rules_path, 0, "Missing rules file")),
            Err(message) => Err(error(rules_path, 0, message)),
        };
        let optional = match files.read(options_path) {
            Ok(Some(text)) => self.source(files, options_path, &text),
            Ok(None) => Ok(()),
            Err(message) => Err(error(options_path, 0, message)),
        };
        if let Err(e) = optional {
            self.output.diagnostics.push(e);
        }
        required?;
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
            for arg in &mut args {
                *arg = substitute(arg, &self.symbols).map_err(|m| error(path, line, m))?;
            }
            self.dispatch(files, path, line, &args)?;
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        files: &mut impl ScriptFiles,
        path: &str,
        line: usize,
        args: &[String],
    ) -> Result<(), ParseError> {
        if args[0] == "eval" {
            arity(args, 2).map_err(|m| error(path, line, m))?;
            let mut tokens = tokenize(&args[1]).map_err(|m| error(path, line, m))?;
            for token in &mut tokens {
                *token = substitute(token, &self.symbols).map_err(|m| error(path, line, m))?;
            }
            if tokens.is_empty() {
                return Err(error(path, line, "Unknown command"));
            }
            return self.dispatch(files, path, line, &tokens);
        } else if args[0] == "sinclude" {
            arity(&args, 2).map_err(|m| error(path, line, m))?;
            let include_path = relative_path(path, &args[1]);
            let included = files
                .read(&include_path)
                .map_err(|m| error(path, line, m))?
                .ok_or_else(|| error(path, line, "Missing included file"))?;
            self.source(files, &include_path, &included)?;
        } else {
            self.command(args).map_err(|m| error(path, line, m))?;
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
            "define" | "enddef" | "create" | "arrayCreate" | "namespace" => {
                return Err("Unsupported GR4 shared command body".into());
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
    Ok(threshold_from_real(real(s)?) as u32)
}

fn signed(s: &str) -> Result<i32, String> {
    // §4.5: portable round-to-nearest/even profile of the native bias/bit trick.
    // GR6: an embedding with another x87 rounding mode is unsupported.
    let biased = (real(s)? + f64::from(12582912.0f32)) as f32;
    Ok(biased.to_bits().wrapping_sub(0x4b400000) as i32)
}

/// Published arithmetic subset; function/primary tables remain open GR4.
pub fn real(s: &str) -> Result<f64, String> {
    super::expression::evaluate(s)
}

fn substitute(input: &str, symbols: &BTreeMap<String, String>) -> Result<String, String> {
    let mut output = String::new();
    let mut rest = input;
    while let Some(at) = rest.find('$') {
        output.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let name;
        if let Some(braced) = rest.strip_prefix('{') {
            let end = braced.find('}').ok_or("Unknown variable")?;
            name = &braced[..end];
            rest = &braced[end + 1..];
        } else {
            let end = rest
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
                .count();
            name = &rest[..end];
            rest = &rest[end..];
        }
        // GR4: literal insertion; boundary-dependent parenthesization remains open.
        output.push_str(symbols.get(name).ok_or("Unknown variable")?);
    }
    output.push_str(rest);
    Ok(output)
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
    let parse = |mut part: &[u8]| {
        while part.first().is_some_and(u8::is_ascii_whitespace) {
            part = &part[1..];
        }
        if part.starts_with(b"0x") || part.starts_with(b"0X") {
            part = &part[2..];
        }
        part.iter().take_while(|b| b.is_ascii_hexdigit()).fold(0, |value, b| {
            value * 16
                + u32::from(if b.is_ascii_digit() {
                    b - b'0'
                } else {
                    b.to_ascii_lowercase() - b'a' + 10
                })
        })
    };
    Ok((parse(&bytes[8..12]), parse(&bytes[22..26])))
}
