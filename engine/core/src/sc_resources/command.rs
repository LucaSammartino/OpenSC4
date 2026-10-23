// SPDX-License-Identifier: GPL-3.0-or-later
use super::{check_switches, lexical, parse_error, Result, ScriptToken};
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
    fn end_block(&self, _parser: &mut CommandParser) -> Result<()> {
        Ok(())
    }
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
pub struct CommandParser {
    pub references: u32,
    pub version: i32,
    entries: Vec<Arc<dyn Command>>,
    names: BTreeMap<Vec<u8>, usize>,
    blocks: Vec<BTreeMap<Vec<u8>, Arc<dyn Command>>>,
    endings: Vec<Option<Arc<dyn Command>>>,
    variables: BTreeMap<Vec<u8>, Vec<u8>>,
    namespaces: Vec<Vec<u8>>,
    macros: BTreeMap<Vec<u8>, Macro>,
    active: Option<Macro>,
}
impl Default for CommandParser {
    fn default() -> Self {
        Self {
            references: 0,
            version: 1,
            entries: Vec::new(),
            names: BTreeMap::new(),
            blocks: Vec::new(),
            endings: Vec::new(),
            variables: BTreeMap::new(),
            namespaces: Vec::new(),
            macros: BTreeMap::new(),
            active: None,
        }
    }
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
        self.endings.push(None);
    }
    pub fn push_block_with_end(&mut self, commands: BTreeMap<Vec<u8>, Arc<dyn Command>>, handler: Arc<dyn Command>) {
        self.blocks.push(commands);
        self.endings.push(Some(handler));
    }
    pub fn pop_block(&mut self) -> bool {
        self.endings.pop();
        self.blocks.pop().is_some()
    }
    pub fn set_variable(&mut self, name: &[u8], value: &[u8]) {
        let mut key = self.prefix(self.namespaces.len());
        key.extend(name);
        self.variables.insert(key, value.to_vec());
    }
    fn prefix(&self, count: usize) -> Vec<u8> {
        self.namespaces[..count]
            .iter()
            .flat_map(|n| n.iter().copied().chain(*b":"))
            .collect()
    }
    pub fn get_variable(&self, name: &[u8]) -> Option<&[u8]> {
        for count in (0..=self.namespaces.len()).rev() {
            let mut key = self.prefix(count);
            key.extend(name);
            if let Some(value) = self.variables.get(&key) {
                return Some(value);
            }
        }
        None
    }
    pub fn remove_variable(&mut self, key: &[u8]) -> bool {
        self.variables.remove(key).is_some()
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
        self.version = 0;
        self.blocks.clear();
        self.endings.clear();
        self.namespaces.clear();
        self.macros.clear();
        self.active = None;
    }
    pub fn shutdown(&mut self) {
        self.entries.clear();
        self.names.clear();
    }
    pub fn no_op(&self) {}
    pub fn substitute_variables(&self, input: &[u8]) -> Result<Vec<u8>> {
        substitute(input, b'$', |name| self.get_variable(name).map(<[u8]>::to_vec))
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
        let exact = |n| {
            if tokens.len() == n {
                Ok(())
            } else {
                Err(parse_error(0, "Argument count"))
            }
        };
        match name {
            "end" => {
                let ending = self.endings.last().cloned().ok_or_else(|| parse_error(0, "No block"))?;
                if let Some(handler) = ending {
                    handler.end_block(self)?;
                }
                self.pop_block();
                Ok(())
            }
            "version" => {
                exact(2)?;
                self.version = decimal_atol(&tokens[1].bytes);
                Ok(())
            }
            "trace" => exact(2),
            "eval" => {
                exact(2)?;
                self.dispatch(lexical::split(&tokens[1].bytes)?)
            }
            "namespace" => {
                exact(2)?;
                self.push_namespace(&tokens[1].bytes);
                struct Namespace;
                impl Command for Namespace {
                    fn execute(&self, _: &mut CommandParser, _: &mut [ScriptToken]) -> Result<()> {
                        Ok(())
                    }
                    fn end_block(&self, p: &mut CommandParser) -> Result<()> {
                        p.pop_namespace();
                        Ok(())
                    }
                }
                self.push_block_with_end(BTreeMap::new(), Arc::new(Namespace));
                Ok(())
            }
            "set" | "seti" | "setf" | "setv3" | "setc" => {
                if tokens.len() < 3 {
                    return Err(parse_error(0, "Assignment arguments"));
                }
                let value = match name {
                    "set" => tokens[2].bytes.clone(),
                    "seti" => lexical::parse_int(&tokens[2].bytes)?.to_string().into_bytes(),
                    "setf" => {
                        crate::rz_streams::format_g(f64::from(lexical::parse_float(&tokens[2].bytes)? as f32), b'.')
                            .into_bytes()
                    }
                    _ => transformed_value(name, tokens)?,
                };
                self.set_variable(&tokens[1].bytes, &value);
                Ok(())
            }
            "arrayCreate" => {
                exact(3)?;
                let count = lexical::parse_int(&tokens[2].bytes)? as u32;
                for index in 0..count {
                    let args = format!("{index} {count}");
                    self.dispatch(vec![
                        ScriptToken::new(0, b"create"),
                        tokens[1].clone(),
                        ScriptToken::new(2, args.as_bytes()),
                    ])?;
                }
                Ok(())
            }
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
                        token.bytes = substitute(&token.bytes, b'%', |param| {
                            m.formals
                                .iter()
                                .position(|t| t.bytes == param)
                                .map(|i| actual[i].bytes.clone())
                        })?;
                    }
                    self.dispatch(line)?;
                }
                Ok(())
            }
            _ => Err(parse_error(0, "Unknown builtin")),
        }
    }
}

fn substitute(input: &[u8], marker: u8, mut lookup: impl FnMut(&[u8]) -> Option<Vec<u8>>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < input.len() {
        if input[at] != marker {
            out.push(input[at]);
            at += 1;
            continue;
        }
        let left = at;
        at += 1;
        let braced = input.get(at) == Some(&b'{');
        at += usize::from(braced);
        let start = at;
        while input
            .get(at)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
        {
            at += 1;
        }
        let value = lookup(&input[start..at]).ok_or_else(|| parse_error(0, "Unknown substitution"))?;
        if braced && input.get(at) == Some(&b'}') {
            at += 1;
        }
        let white = |b: u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n');
        let wrap = !braced
            && (left == 0 || white(input[left - 1]))
            && (at == input.len() || white(input[at]))
            && !(left == 0 && at == input.len());
        if wrap {
            out.push(b'(');
        }
        out.extend(value);
        if wrap {
            out.push(b')');
        }
    }
    Ok(out)
}
fn decimal_atol(bytes: &[u8]) -> i32 {
    let bytes = &bytes[bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len())..];
    let negative = bytes.first() == Some(&b'-');
    let start = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let n = bytes[start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0u32, |n, b| n.wrapping_mul(10).wrapping_add(u32::from(*b - b'0')));
    if negative {
        n.wrapping_neg() as i32
    } else {
        n as i32
    }
}
fn transformed_value(name: &str, tokens: &[ScriptToken]) -> Result<Vec<u8>> {
    let color = name == "setc";
    let mut value = if color {
        lexical::color_alpha(&tokens[2].bytes)?
    } else {
        let v = lexical::vector3(&tokens[2].bytes)?;
        [v[0], v[1], v[2], 0.0]
    };
    for switch in [b"-scale".as_slice(), b"-mult", b"-add", b"-alpha"] {
        let Some(i) = tokens.iter().position(|t| t.bytes == switch) else {
            continue;
        };
        let arg = tokens
            .get(i + 1)
            .ok_or_else(|| parse_error(0, "Missing switch argument"))?;
        if switch == b"-scale" {
            let scale = lexical::parse_float(&arg.bytes)? as f32;
            for v in &mut value[usize::from(color)..if color { 4 } else { 3 }] {
                *v *= scale;
            }
        } else if switch == b"-alpha" {
            if color {
                value[1] = lexical::parse_float(&arg.bytes)? as f32;
            }
        } else {
            let other = if color {
                lexical::color_alpha(&arg.bytes)?
            } else {
                let v = lexical::vector3(&arg.bytes)?;
                [v[0], v[1], v[2], 0.0]
            };
            for i in 0..if color { 4 } else { 3 } {
                value[i] = if switch == b"-mult" {
                    value[i] * other[i]
                } else {
                    value[i] + other[i]
                };
            }
        }
    }
    let output = if color {
        let mut v = value[1..].to_vec();
        if value[0] < 1.0 {
            v.push(value[0]);
        }
        v
    } else {
        value[..3].to_vec()
    };
    Ok(output
        .iter()
        .map(|v| crate::rz_streams::format_g(f64::from(*v), b'.'))
        .collect::<Vec<_>>()
        .join(" ")
        .into_bytes())
}
