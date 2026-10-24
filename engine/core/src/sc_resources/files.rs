// SPDX-License-Identifier: GPL-3.0-or-later
use super::{command::CommandParser, lexical, parse_error, Error, Result};
use std::sync::{Arc, Mutex};

/// ReadLine false is EOF to this owner, including foreign read failures.
pub trait Lines {
    fn read_line(&mut self) -> Option<Vec<u8>>;
}
pub trait Watcher: Send {
    fn shutdown(&mut self);
}
pub trait Files {
    fn open(&mut self, path: &str) -> Option<Box<dyn Lines>>;
    fn watch(&mut self, path: &str, class: u32, iid: u32) -> Option<Box<dyn Watcher>>;
    fn finished(&mut self, _path: &str, _physical: u32, _result: &Result<u32>) {}
}
pub trait ParseHooks {
    fn begin(&mut self, parser: &mut CommandParser) -> Result<()>;
    fn end(&mut self, parser: &mut CommandParser) -> Result<()>;
    fn finish(&mut self, parser: &mut CommandParser);
}
pub type ParserHandle = Arc<Mutex<CommandParser>>;
#[derive(Default)]
pub struct FileParser {
    references: u32,
    parser: Option<ParserHandle>,
    inputs: Vec<(String, u32)>,
    watchers: Vec<Box<dyn Watcher>>,
    modified: bool,
    errors: Vec<Error>,
}
impl FileParser {
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut bool>) -> bool {
        let Some(out) = out else { return false };
        if !matches!(iid, 1 | 0x1cf9_4583) {
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
            self.set_parser(None);
        }
        self.references
    }
    pub fn init(&self) {}
    pub fn set_parser(&mut self, parser: Option<ParserHandle>) {
        if let Some(old) = self.parser.take() {
            let mut old = old.lock().unwrap_or_else(|e| e.into_inner());
            old.shutdown();
            old.release();
        }
        if let Some(new) = &parser {
            new.lock().unwrap_or_else(|e| e.into_inner()).add_ref();
        }
        self.parser = parser;
    }
    /// Borrowed/raw interface: no logical AddRef.
    pub fn parser(&self) -> Option<&ParserHandle> {
        self.parser.as_ref()
    }
    pub fn add_input(&mut self, path: &str, flags: u32, files: &mut impl Files) {
        self.inputs.push((path.into(), flags));
        self.modified = true;
        if flags & 2 != 0 {
            if let Some(w) = files.watch(path, 0xe953_da68, 0xe953_da64) {
                self.watchers.push(w);
            }
        }
    }
    pub fn shutdown(&mut self) {
        for mut w in self.watchers.drain(..) {
            w.shutdown();
        }
    }
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    pub fn input_path(&self, index: usize) -> Option<&str> {
        self.inputs.get(index).map(|i| i.0.as_str())
    }
    pub fn modified(&self) -> bool {
        self.modified
    }
    pub fn set_modified(&mut self, value: bool) {
        self.modified = value;
    }
    pub fn message(&mut self, ty: u32) -> bool {
        if ty == 0x6ae7_6406 {
            self.modified = true;
            true
        } else {
            false
        }
    }
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }
    pub fn error_string(&self) -> String {
        self.errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn parse_all(&mut self, files: &mut impl Files, hooks: &mut impl ParseHooks) -> Result<bool> {
        self.parse_range(0, self.inputs.len(), files, hooks)
    }
    pub fn parse_range(
        &mut self,
        start: usize,
        count: usize,
        files: &mut impl Files,
        hooks: &mut impl ParseHooks,
    ) -> Result<bool> {
        let parser = self.parser.clone().ok_or(Error::InvalidContext)?;
        let mut parser = parser.lock().unwrap_or_else(|e| e.into_inner());
        let inputs = self
            .inputs
            .get(start..start.checked_add(count).ok_or(Error::InvalidContext)?)
            .ok_or(Error::InvalidContext)?;
        self.errors.clear();
        let mut success = true;
        if let Err(e) = hooks.begin(&mut parser) {
            self.errors.push(e);
            success = false;
        }
        for (path, flags) in inputs {
            if let Err(e) = parse_file(files, &mut parser, path) {
                if *flags & 1 != 0 {
                    success = false;
                }
                if *flags & 1 != 0 || !matches!(e, Error::Parse { code: 2, .. }) {
                    self.errors.push(e);
                }
            }
        }
        if success {
            if let Err(e) = hooks.end(&mut parser) {
                self.errors.push(e);
                success = false;
            }
        }
        hooks.finish(&mut parser);
        Ok(success)
    }
}
impl Drop for FileParser {
    fn drop(&mut self) {
        self.shutdown();
        self.set_parser(None);
    }
}
fn parse_file(files: &mut impl Files, parser: &mut CommandParser, path: &str) -> Result<()> {
    parse_file_with(files, path, &mut |_, _, _, tokens| parser.dispatch(tokens)).map(|_| ())
}

/// The production file loop with command execution supplied by the caller.
/// A passive replay supplies the observed command boundary, not a second
/// graphics-rule interpreter. Positions are physical, local to this file;
/// the returned count also includes successfully completed child files.
pub fn parse_file_with(
    files: &mut impl Files,
    path: &str,
    dispatch: &mut impl FnMut(&str, u32, &[u8], Vec<super::ScriptToken>) -> Result<()>,
) -> Result<u32> {
    let Some(mut lines) = files.open(path) else {
        let result = Err(parse_error(2, format!("Cannot open {path}")));
        files.finished(path, 0, &result);
        return result;
    };
    let mut line_number = 0;
    let mut inclusive = 0;
    let result = (|| {
        while let Some(line) = lines.read_line() {
            line_number += 1;
            inclusive += 1;
            let tokens = lexical::split(&line)?;
            if tokens
                .first()
                .is_some_and(|t| matches!(t.bytes.as_slice(), b"include" | b"sinclude"))
            {
                if tokens.len() != 2 {
                    return Err(parse_error(0, "Include requires one argument"));
                }
                let include = std::str::from_utf8(&tokens[1].bytes).map_err(|_| parse_error(0, "Invalid path"))?;
                let joined = if include.starts_with(['/', '\\']) || include.as_bytes().get(1) == Some(&b':') {
                    include.to_owned()
                } else {
                    let end = path.rfind(['/', '\\']).map_or(0, |n| n + 1);
                    format!("{}{include}", &path[..end])
                };
                match parse_file_with(files, &joined, dispatch) {
                    Ok(count) => inclusive += count,
                    Err(e) => match e {
                        Error::Parse { code: 2, .. } if tokens[0].bytes == b"sinclude" => {}
                        Error::Parse { code: 2, message } => return Err(parse_error(3, message)),
                        e => return Err(e),
                    },
                }
            } else {
                dispatch(path, line_number, &line, tokens)?;
            }
        }
        Ok(inclusive)
    })();
    files.finished(path, line_number, &result);
    result.map_err(|error| match error {
        Error::Parse { code, message } => parse_error(code, format!("{message}\n{path}:{line_number}")),
        error => error,
    })
}
