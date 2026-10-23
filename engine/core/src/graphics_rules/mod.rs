// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable graphics-rule manager and parser, written from graphics-rules.md.
//!
//! GR4/GR6 mark remaining shared-parser/failure/exceptional-format contracts. This module owns
//! all rule parsing/evaluation decisions; its services supply measurements,
//! file bytes, lowercase bytes, render properties and log I/O only. No original
//! parser/manager output is accepted as a foreign service. No complete-owner,
//! native acquisition, shadow or replacement credit is claimed with GR4 open.

mod environment;
mod expression;
mod logging;
mod model;
mod parser;

pub use environment::{
    forced_fsaa_from_samples, Environment, InformationProvider, Measurements, FLAGS, NUMBERS, STRINGS,
};
pub use logging::{general, LogContext, LogFile, LogStamp, NoLog, SUMMARY};
pub use model::{
    threshold_from_real, wildcard_match, AsciiLowercase, Lowercase, MatchError, OptionGroup, Properties, PropertySet,
    PropertyValue, Rule,
};
pub use parser::{card_ids, real, ParseError, ParsedRules, RuleParser, ScriptFiles};

use std::sync::{Arc, Mutex};

pub const CLASS_ID: u32 = 0x0a1d8285;
pub const INTERFACE_ID: u32 = 0x0a1d8284;

#[derive(Clone, Debug)]
pub struct InitContext {
    pub rules_path: String,
    pub options_path: String,
    pub prefs_first_time: bool,
}
impl InitContext {
    pub fn from_directories(data: &str, user: &str, prefs_first_time: bool) -> Self {
        Self {
            rules_path: logging::in_directory(data, "Graphics Rules.sgr"),
            options_path: logging::in_directory(user, "Graphics Options.sgr"),
            prefs_first_time,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagerError {
    NotInitialized,
    StringService(MatchError),
    Allocation(String),
    Unsupported(&'static str),
}

/// Pure state. The held renderer uses Arc/Mutex, never RefCell: getters may run
/// on another thread. Init retains once, replaces/releases once, and leaves an
/// identical Arc alone. Rust Drop releases renderer, lists and log resources.
pub struct GraphicsRuleManager<P: Properties, L: LogFile = NoLog> {
    properties: Option<Arc<Mutex<P>>>,
    pub environment: Environment,
    rules: Vec<Rule>,
    groups: Vec<OptionGroup>,
    log: L,
    log_open: bool,
    pub last_parse_error: Option<ParseError>,
    pub last_init_error: Option<ManagerError>,
    pub diagnostics: Vec<String>,
}

impl<P: Properties> Default for GraphicsRuleManager<P, NoLog> {
    fn default() -> Self {
        Self::new(NoLog)
    }
}

impl<P: Properties, L: LogFile> GraphicsRuleManager<P, L> {
    pub fn new(log: L) -> Self {
        Self {
            properties: None,
            environment: Environment::default(),
            rules: Vec::new(),
            groups: Vec::new(),
            log,
            log_open: false,
            last_parse_error: None,
            last_init_error: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
    pub fn groups(&self) -> &[OptionGroup] {
        &self.groups
    }
    pub fn logger(&self) -> &L {
        &self.log
    }

    /// Native Init has no success result and does not evaluate any rule (§3).
    pub fn init(
        &mut self,
        properties: &Arc<Mutex<P>>,
        provider: &mut impl InformationProvider,
        files: &mut impl ScriptFiles,
        context: &InitContext,
    ) {
        if !self.properties.as_ref().is_some_and(|old| Arc::ptr_eq(old, properties)) {
            self.properties = Some(Arc::clone(properties));
        }
        self.last_init_error = None;
        // Presence is guaranteed above. Poison recovery preserves owned state;
        // there is no RefCell or panic across a future foreign binding.
        if let Err(e) = self.gather_information(provider) {
            self.last_init_error = Some(e);
        }
        if !self.parse_config_file(files, context) {
            if let Err(e) = self.set_fallback_rules() {
                self.last_init_error = Some(e);
            }
        }
    }

    pub fn gather_information(&mut self, provider: &mut impl InformationProvider) -> Result<(), ManagerError> {
        let properties = self.properties.as_ref().ok_or(ManagerError::NotInitialized)?;
        let force = properties
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_bool(0)
            .unwrap_or(false);
        self.environment.gather(provider, force);
        Ok(())
    }

    /// Failure retains existing lists; only Init installs fallback (§3).
    pub fn parse_config_file(&mut self, files: &mut impl ScriptFiles, context: &InitContext) -> bool {
        self.environment.strings[12] = self.environment.strings[10].clone();
        let parsed = (|| {
            let (vendor, device) = card_ids(&self.environment.strings[11]).map_err(|message| ParseError {
                path: context.rules_path.clone(),
                line: 0,
                message,
            })?;
            let properties = self.properties.as_ref().ok_or_else(|| ParseError {
                path: context.rules_path.clone(),
                line: 0,
                message: "Missing render properties".into(),
            })?;
            let properties = properties.lock().unwrap_or_else(|p| p.into_inner());
            RuleParser::new(&*properties, vendor, device, context.prefs_first_time).parse_files(
                files,
                &context.rules_path,
                &context.options_path,
            )
        })();
        match parsed {
            Ok(parsed) => {
                self.diagnostics
                    .extend(parsed.diagnostics.iter().map(ToString::to_string));
                self.rules = parsed.rules;
                self.groups = parsed.groups;
                if let Some(identity) = parsed.card_identity {
                    self.environment.strings[12] = identity;
                }
                self.last_parse_error = None;
                true
            }
            Err(e) => {
                self.last_parse_error = Some(e);
                false
            }
        }
    }

    pub fn apply_option_group_setting(&mut self, group: u32, option: u32) -> Result<(), ManagerError> {
        let Some(set) = self
            .groups
            .get(group as usize)
            .and_then(|g| g.options.get(option as usize))
        else {
            return Ok(());
        };
        let properties = self.properties.as_ref().ok_or(ManagerError::NotInitialized)?;
        set.apply(&mut *properties.lock().unwrap_or_else(|p| p.into_inner()));
        Ok(())
    }

    pub fn get_option_group_auto_setting(&self, group: u32) -> u32 {
        self.groups.get(group as usize).map_or(0, |g| g.recommendation)
    }

    pub fn apply_rules(&mut self, strings: &impl Lowercase) -> Result<Vec<String>, ManagerError> {
        let properties = Arc::clone(self.properties.as_ref().ok_or(ManagerError::NotInitialized)?);
        let mut properties = properties.lock().unwrap_or_else(|p| p.into_inner());
        let mut stack = Vec::new();
        let mut applied = Vec::new();
        let mut i = 0;
        while i < self.rules.len() {
            let rule = &self.rules[i];
            if let Some(&top) = stack.last() {
                if rule.depth <= top {
                    while i < self.rules.len() && self.rules[i].depth >= top {
                        i += 1;
                    }
                    stack.pop();
                    continue;
                }
            }
            if !rule
                .matches(&self.environment, strings)
                .map_err(ManagerError::StringService)?
            {
                let depth = rule.depth;
                i += 1;
                while i < self.rules.len() && self.rules[i].depth > depth {
                    i += 1;
                }
                continue;
            }
            if self.log.is_open() {
                logging::line(
                    &mut self.log,
                    format!("applying {}", rule.name.split('\0').next().unwrap_or("")).as_bytes(),
                );
            }
            applied.push(rule.name.clone());
            rule.properties.apply(&mut *properties);
            for &(group, option) in &rule.recommendations {
                model::grow(&mut self.groups, group).map_err(ManagerError::Allocation)?;
                self.groups[group as usize].recommendation = option;
                if self.log.is_open() {
                    logging::line(&mut self.log, format!("  option {group} = {option}").as_bytes());
                }
            }
            if properties.get_bool(0) == Some(true) {
                self.environment.flags &= !1;
            }
            if !rule.partial {
                stack.push(rule.depth);
            }
            i += 1;
        }
        Ok(applied)
    }

    pub fn set_fallback_rules(&mut self) -> Result<(), ManagerError> {
        let properties = self.properties.as_ref().ok_or(ManagerError::NotInitialized)?;
        let properties = properties.lock().unwrap_or_else(|p| p.into_inner());
        let names = ["RenderCloudsAndFog", "RenderShadows", "RenderDecals", "UseSecondStage"];
        let mut indices = Vec::new();
        for name in names {
            // §12.1: retain the unknown-index sentinel as an ordinary action;
            // the renderer setter determines its effect, not the parser.
            indices.push(properties.find_bool(name).unwrap_or(u32::MAX));
        }
        let software = Rule {
            name: "standard software".into(),
            flag_mask: 1,
            properties: PropertySet {
                bools: indices.iter().map(|&i| (i, false)).collect(),
                ..PropertySet::default()
            },
            ..Rule::default()
        };
        let hardware = Rule {
            name: "standard hardware".into(),
            properties: PropertySet {
                bools: indices.iter().enumerate().map(|(n, &i)| (i, n != 3)).collect(),
                ..PropertySet::default()
            },
            ..Rule::default()
        };
        let stages = Rule {
            name: "two stages".into(),
            depth: 1,
            lower: vec![(8, 2)],
            properties: PropertySet {
                bools: vec![(indices[3], true)],
                ..PropertySet::default()
            },
            ..Rule::default()
        };
        self.rules = vec![software, hardware, stages];
        // §7 does not clear or construct groups, or apply any property.
        self.diagnostics.push("Using fallback rules.".into());
        if self.log.is_open() {
            logging::text(&mut self.log, "Using fallback rules.\n");
        }
        Ok(())
    }

    pub fn set_options_from_configuration(
        &mut self,
        context: &LogContext,
        strings: &impl Lowercase,
    ) -> Result<Vec<String>, ManagerError> {
        if !context
            .write_log
            .as_ref()
            .is_some_and(|v| v.eq_ignore_ascii_case("off"))
        {
            let file = format!("{}-config-log.txt", self.environment.strings[4]);
            self.log.open(&logging::in_directory(&context.app_directory, &file));
            self.log_open = self.log.is_open();
        }
        let mut log_result = Ok(());
        if self.log_open {
            log_result = logging::configuration(&mut self.log, &self.environment, &context.stamp);
            logging::text(&mut self.log, "Rules\n-----\n");
        }
        let result = self.apply_rules(strings);
        if self.log_open {
            if let Some(properties) = &self.properties {
                let properties = properties.lock().unwrap_or_else(|p| p.into_inner());
                let groups = &self.groups;
                let options_result =
                    logging::options(&mut self.log, &*properties, context.preferences.as_ref(), |group| {
                        groups.get(group as usize).map_or(0, |g| g.recommendation)
                    });
                log_result = log_result.and(options_result);
            }
            self.log.close();
            self.log_open = false;
        }
        // Unsupported logging profiles still evaluate the rules and close the file.
        result.and_then(|applied| log_result.map(|()| applied))
    }

    pub fn log_configuration_to_file(
        &mut self,
        path: &str,
        stamp: &LogStamp,
        provider: &mut impl InformationProvider,
    ) -> Result<bool, ManagerError> {
        self.gather_information(provider)?;
        self.log.open(path);
        if !self.log.is_open() {
            return Ok(false);
        }
        let result = logging::configuration(&mut self.log, &self.environment, stamp);
        self.log.close();
        result.map(|()| true)
    }
}

#[cfg(test)]
mod tests;
