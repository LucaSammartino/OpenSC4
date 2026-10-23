// SPDX-License-Identifier: GPL-3.0-or-later
use super::{References, Result, Token, Unsupported};
use crate::variant::{Elem, Variant};
pub const COMMAND_EXTENSION: u32 = 0x1A2BDEB9;
pub const COLLECTION_EXTENSION: u32 = 0x1A2BDEBA;
pub const STREAM_IID: u32 = 0xC56B8F08;
pub trait Host: References {
    fn translate(&mut self, vocabulary: Token, text: &[u8]) -> Option<Variant>;
    /// GZF9: native numeric syntax is injected, never guessed as Rust syntax.
    fn numeric(&mut self, text: &[u8]) -> Option<Variant>;
    fn unassigned_parameter(&mut self) -> Result<Variant>;
    fn start(&mut self, target: Token, interpreter: &mut Script);
    fn text(&mut self, target: Token, interpreter: &mut Script, text: &[u8]);
    fn command_start(&mut self, extension: Token, interpreter: &mut Script, command: u32);
    fn parameter(&mut self, extension: Token, interpreter: &mut Script, id: u32, value: &Variant);
    fn command_end(&mut self, extension: Token, interpreter: &mut Script, command: u32);
    fn parameters(&mut self, extension: Token, interpreter: &mut Script, command: u32, values: &[(u32, Variant)]);
    /// IO adapter resolves GZF9 short-read/NUL policies, returning actual bytes.
    fn read_file(&mut self, path: &str) -> Result<Option<Vec<u8>>>;
    fn acquire_resource(&mut self, id: u64) -> Option<Token>;
    fn read_stream(&mut self, stream: Token) -> Result<Vec<u8>>;
    fn return_resource(&mut self, resource: Token);
}
#[derive(Default)]
pub struct Script {
    target: Option<Token>,
    command_extension: Option<Token>,
    collection_extension: Option<Token>,
    context: Option<Token>,
    vocabularies: Vec<Token>,
    active: bool,
    pub error: bool,
    pub cursor: usize,
    pub current_command: Option<u32>,
    pub current_parameter: Option<u32>,
}
impl Script {
    /// Empty core fixture, not a claim about the native default vocabulary (GZF9).
    pub fn new_fixture() -> Self {
        Self::default()
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn set_target(&mut self, host: &mut impl Host, target: Option<Token>) -> bool {
        for t in [
            self.target.take(),
            self.command_extension.take(),
            self.collection_extension.take(),
        ]
        .into_iter()
        .flatten()
        {
            host.release(t);
        }
        let target = target.filter(|&t| t != 0);
        self.target = target;
        if let Some(t) = target {
            host.add_ref(t);
            self.command_extension = host.query(t, COMMAND_EXTENSION);
            self.collection_extension = host.query(t, COLLECTION_EXTENSION);
        }
        true
    }
    pub fn set_context(&mut self, host: &mut impl References, context: Option<Token>) {
        let context = context.filter(|&t| t != 0);
        if let Some(t) = self.context.take() {
            host.release(t);
        }
        self.context = context;
        if let Some(t) = context {
            host.add_ref(t);
        }
    }
    pub fn context(&self) -> Option<Token> {
        self.context
    }
    pub fn push_vocabulary(&mut self, host: &mut impl References, vocabulary: Option<Token>) -> bool {
        let Some(v) = vocabulary.filter(|&v| v != 0) else {
            return false;
        };
        host.add_ref(v);
        self.vocabularies.push(v);
        true
    }
    pub fn pop_vocabulary(&mut self, host: &mut impl References) -> bool {
        if self.vocabularies.len() <= 1 {
            return false;
        }
        host.release(self.vocabularies.pop().unwrap());
        true
    }
    pub fn lookup(&self, host: &mut impl Host, text: &[u8]) -> Option<Variant> {
        self.vocabularies.iter().rev().find_map(|&v| host.translate(v, text))
    }
    fn identifier(&self, host: &mut impl Host, word: &[u8]) -> Option<u32> {
        let v = self.lookup(host, word).or_else(|| host.numeric(word))?;
        if let Variant::Scalar(Elem::U32(id)) = v {
            Some(id)
        } else {
            None
        }
    }
    pub fn execute(&mut self, host: &mut impl Host, input: &[u8]) -> Result<bool> {
        if self.active {
            return Ok(false);
        }
        self.active = true;
        self.error = false;
        self.cursor = 0;
        self.current_command = None;
        self.current_parameter = None;
        if let Some(t) = self.target {
            host.start(t, self);
        }
        let result = self.parse(host, input);
        self.active = false;
        result
    }
    fn parse(&mut self, host: &mut impl Host, input: &[u8]) -> Result<bool> {
        while self.cursor < input.len() {
            if input[self.cursor] != b'<' {
                let start = self.cursor;
                while self.cursor < input.len() && input[self.cursor] != b'<' {
                    self.cursor += 1;
                }
                if let Some(t) = self.target {
                    host.text(t, self, &input[start..self.cursor]);
                }
                continue;
            }
            self.cursor += 1;
            let token = self.next(input)?;
            let Lex::Word(command) = token else {
                self.error = true;
                return Ok(false);
            };
            let Some(command) = self.identifier(host, &command) else {
                while self.cursor < input.len() && input[self.cursor] != b'>' {
                    self.cursor += 1;
                }
                if self.cursor < input.len() {
                    self.cursor += 1;
                }
                continue;
            };
            self.current_command = Some(command);
            if let Some(e) = self.command_extension {
                host.command_start(e, self, command);
            }
            let mut values = Vec::new();
            loop {
                let token = self.next(input)?;
                match token {
                    Lex::Eof => return Ok(!self.error),
                    Lex::Close => {
                        if let Some(e) = self.command_extension {
                            host.command_end(e, self, command);
                        }
                        if let Some(e) = self.collection_extension {
                            host.parameters(e, self, command, &values);
                        }
                        break;
                    }
                    Lex::Word(word) => {
                        let id = self.identifier(host, &word);
                        self.current_parameter = id;
                        let saved = self.cursor;
                        let next = self.next(input)?;
                        let value = if next == Lex::Equals {
                            match self.next(input)? {
                                Lex::Quoted(s) => Some(Variant::string(&s)),
                                Lex::Word(w) => Some(
                                    self.lookup(host, &w)
                                        .or_else(|| host.numeric(&w))
                                        .unwrap_or_else(|| Variant::string(&w)),
                                ),
                                _ => {
                                    self.error = true;
                                    return Ok(false);
                                }
                            }
                        } else {
                            self.cursor = saved;
                            if id.is_some() {
                                Some(host.unassigned_parameter()?)
                            } else {
                                None
                            }
                        };
                        if let (Some(id), Some(value)) = (id, value) {
                            if let Some(e) = self.command_extension {
                                host.parameter(e, self, id, &value);
                            }
                            values.push((id, value));
                        }
                    }
                    _ => {
                        self.error = true;
                        return Ok(false);
                    }
                }
            }
        }
        Ok(!self.error)
    }
    fn next(&mut self, input: &[u8]) -> Result<Lex> {
        while self.cursor < input.len() && matches!(input[self.cursor], b' ' | b'\t' | b'\n' | b'\r') {
            self.cursor += 1;
        }
        let Some(&byte) = input.get(self.cursor) else {
            return Ok(Lex::Eof);
        };
        self.cursor += 1;
        Ok(match byte {
            b'<' => Lex::Open,
            b'>' => Lex::Close,
            b'=' => Lex::Equals,
            b'"' => {
                let start = self.cursor;
                while self.cursor < input.len() && input[self.cursor] != b'"' {
                    self.cursor += 1;
                }
                if self.cursor == input.len() {
                    return Err(Unsupported("GZF9"));
                }
                let word = input[start..self.cursor].to_vec();
                self.cursor += 1;
                Lex::Quoted(word)
            }
            _ => {
                let start = self.cursor - 1;
                while self.cursor < input.len()
                    && !matches!(
                        input[self.cursor],
                        b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'<' | b'>' | b'='
                    )
                {
                    self.cursor += 1;
                }
                Lex::Word(input[start..self.cursor].to_vec())
            }
        })
    }
    pub fn execute_file(&mut self, host: &mut impl Host, path: &str) -> Result<bool> {
        let Some(mut bytes) = host.read_file(path)? else {
            return Ok(false);
        };
        bytes.push(0);
        self.execute(host, &bytes[..bytes.len() - 1])
    }
    pub fn execute_resource(&mut self, host: &mut impl Host, id: u64) -> Result<bool> {
        let Some(resource) = host.acquire_resource(id) else {
            return Ok(false);
        };
        let result = if let Some(stream) = host.query(resource, STREAM_IID) {
            let bytes = host.read_stream(stream);
            host.release(stream);
            bytes.and_then(|bytes| self.execute(host, &bytes))
        } else {
            Ok(false)
        };
        host.return_resource(resource);
        result
    }
    pub fn dispose(&mut self, host: &mut impl Host) {
        self.set_target(host, None);
        self.set_context(host, None);
        for v in self.vocabularies.drain(..).rev() {
            host.release(v);
        }
    }
    pub fn token_mutation(&self) -> Result<()> {
        Err(Unsupported("GZF9"))
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Lex {
    Word(Vec<u8>),
    Quoted(Vec<u8>),
    Open,
    Close,
    Equals,
    Eof,
}
