// SPDX-License-Identifier: GPL-3.0-or-later
use super::{check_switches, lexical, parse_error, Error, Result, ScriptToken};
use crate::effects_parser::script::Ascii;
use std::{collections::BTreeMap, sync::Arc};

pub const BUILTINS: [&str; 15] = [
    "end",
    "version",
    "list",
    "trace",
    "set",
    "seti",
    "setf",
    "setc",
    "setv3",
    "eval",
    "define",
    "enddef",
    "create",
    "arrayCreate",
    "namespace",
];
pub trait Command: Send + Sync {
    fn execute(&self, parser: &mut CommandParser, tokens: &mut [ScriptToken]) -> Result<()>;
}
impl<F> Command for F
where
    F: Fn(&mut CommandParser, &mut [ScriptToken]) -> Result<()> + Send + Sync,
{
    fn execute(&self, parser: &mut CommandParser, tokens: &mut [ScriptToken]) -> Result<()> {
        self(parser, tokens)
    }
}
#[derive(Clone)]
struct Macro {
    name: Vec<u8>,
    formals: Vec<ScriptToken>,
    lines: Vec<Vec<ScriptToken>>,
}
#[derive(Default)]
pub struct CommandParser {
    pub references: u32,
    entries: Vec<Arc<dyn Command>>,
    names: BTreeMap<Vec<u8>, usize>,
    blocks: Vec<BTreeMap<Vec<u8>, Arc<dyn Command>>>,
    variables: BTreeMap<Vec<u8>, Vec<u8>>,
    namespaces: Vec<Vec<u8>>,
    macros: BTreeMap<Vec<u8>, Macro>,
    active: Option<Macro>,
}
impl CommandParser {
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut bool>) -> bool {
        let Some(out) = out else { return false };
        if !matches!(iid, 1 | 0x0950_35f4) {
            return false;
        }
        self.add_ref();
        *out = true;
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self) -> u32 {
        self.references = self.references.wrapping_sub(1);
        if self.references == 0 {
            self.shutdown();
        }
        self.references
    }
    pub fn init(&mut self) {
        self.register_commands();
    }
    pub fn register_commands(&mut self) {
        for name in BUILTINS {
            self.register(
                name.as_bytes(),
                Arc::new(move |p: &mut Self, t: &mut [ScriptToken]| p.builtin(name, t)),
            );
        }
    }
    pub fn register(&mut self, name: &[u8], command: Arc<dyn Command>) {
        self.names.insert(name.to_vec(), self.entries.len());
        self.entries.push(command);
    }
    pub fn registration_count(&self) -> usize {
        self.entries.len()
    }
    pub fn push_block(&mut self, commands: BTreeMap<Vec<u8>, Arc<dyn Command>>) {
        self.blocks.push(commands);
    }
    pub fn pop_block(&mut self) -> bool {
        self.blocks.pop().is_some()
    }
    pub fn set_variable(&mut self, name: &[u8], value: &[u8]) {
        self.variables.insert(name.to_vec(), value.to_vec());
    }
    pub fn push_namespace(&mut self, name: &[u8]) {
        self.namespaces.push(name.to_vec());
    }
    pub fn pop_namespace(&mut self) {
        self.namespaces.pop();
    }
    pub fn clear_active_macro(&mut self) {
        self.active = None;
    }
    pub fn reset(&mut self) {
        self.blocks.clear();
        self.variables.clear();
        self.namespaces.clear();
        self.macros.clear();
        self.active = None;
    }
    pub fn shutdown(&mut self) {
        self.entries.clear();
        self.names.clear();
    }
    pub fn no_op(&self) {}
    pub fn unsettled_slot(&self) -> Result<()> {
        Err(Error::Unsupported("SCR2: parser helper slot"))
    }
    pub fn substitute_variables(&self, input: &[u8]) -> Result<Vec<u8>> {
        if !input.contains(&b'$') {
            return Ok(input.to_vec());
        }
        let name = input
            .strip_prefix(b"${")
            .and_then(|s| s.strip_suffix(b"}"))
            .or_else(|| input.strip_prefix(b"$"));
        let Some(name) =
            name.filter(|n| !n.is_empty() && n.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':')))
        else {
            return Err(Error::Unsupported("SCR2: inserted-parenthesis boundary"));
        };
        for count in (0..=self.namespaces.len()).rev() {
            let mut qualified = Vec::new();
            for part in &self.namespaces[..count] {
                qualified.extend(part);
                qualified.push(b':');
            }
            qualified.extend(name);
            if let Some(value) = self.variables.get(&qualified) {
                return Ok(value.clone());
            }
        }
        Err(parse_error(0, "Unknown variable"))
    }
    pub fn parse_arguments(&self, input: &[u8]) -> Result<Vec<ScriptToken>> {
        let mut tokens = lexical::split(input)?;
        for token in &mut tokens {
            token.bytes = self.substitute_variables(&token.bytes)?;
        }
        Ok(tokens)
    }
    pub fn parse(&mut self, input: &[u8]) -> Result<()> {
        self.dispatch(lexical::split(input)?)
    }
    pub fn dispatch(&mut self, mut tokens: Vec<ScriptToken>) -> Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }
        if let Some(active) = self.active.as_mut() {
            if tokens[0].bytes != b"enddef" {
                if tokens[0].bytes == b"define" {
                    return Err(parse_error(0, "Nested macro definition"));
                }
                active.lines.push(tokens);
                return Ok(());
            }
        }
        for token in &mut tokens {
            token.bytes = self.substitute_variables(&token.bytes)?;
        }
        for block in self.blocks.iter().rev() {
            if let Some(handler) = block.get(&tokens[0].bytes).cloned() {
                handler.execute(self, &mut tokens)?;
                return check_switches(&tokens, &mut Ascii).map_err(|_| parse_error(0, "Unmarked switch"));
            }
        }
        let handler = self
            .names
            .get(&tokens[0].bytes)
            .map(|&i| self.entries[i].clone())
            .ok_or_else(|| parse_error(0, "Unknown command"))?;
        handler.execute(self, &mut tokens)
    }
    fn builtin(&mut self, name: &str, tokens: &[ScriptToken]) -> Result<()> {
        match name {
            "list" => Err(parse_error(0, "list is not implemented by the original owner")),
            "define" => {
                if self.active.is_some() || tokens.len() < 3 {
                    return Err(parse_error(0, "Invalid macro definition"));
                }
                self.active = Some(Macro {
                    name: tokens[1].bytes.clone(),
                    formals: lexical::split(&tokens[2].bytes)?,
                    lines: Vec::new(),
                });
                Ok(())
            }
            "enddef" => {
                let m = self
                    .active
                    .take()
                    .ok_or_else(|| parse_error(0, "No macro definition"))?;
                self.macros.insert(m.name.clone(), m);
                Ok(())
            }
            "create" => {
                if tokens.len() != 3 {
                    return Err(parse_error(0, "create requires two arguments"));
                }
                let m = self
                    .macros
                    .get(&tokens[1].bytes)
                    .cloned()
                    .ok_or_else(|| parse_error(0, "Unknown macro"))?;
                let actual = lexical::split(&tokens[2].bytes)?;
                if actual.len() != m.formals.len() {
                    return Err(parse_error(0, "Macro parameter count"));
                }
                for mut line in m.lines {
                    for token in &mut line {
                        if let Some(param) = token.bytes.strip_prefix(b"%") {
                            let i = m
                                .formals
                                .iter()
                                .position(|t| t.bytes == param)
                                .ok_or_else(|| parse_error(0, "Unknown macro parameter"))?;
                            token.bytes = actual[i].bytes.clone();
                        }
                    }
                    self.dispatch(line)?;
                }
                Ok(())
            }
            // SCR2 explicitly withholds built-in argument/failure contracts.
            _ => Err(Error::Unsupported("SCR2: builtin contract")),
        }
    }
}
