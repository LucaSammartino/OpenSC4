// SPDX-License-Identifier: GPL-3.0-or-later
//! Manager queue/bootstrap functions, not independent COM owners (effects §6.2).

use crate::effects::collection::Collection;
use std::collections::BTreeSet;

pub const COMPLETE: u32 = 0xC9F7_75BB;
pub const RESOURCE_IID: u32 = 0xCA51_189B;
pub type Key = [u32; 3];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub path: String,
    pub directory: String,
    pub flags: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileResult {
    Parsed(u32),
    Rejected(String),
    Exception { code: u32, diagnostic: String },
}

/// Each call is one foreign primitive. Readers and file commands may publish
/// into the supplied collection; no host supplies an aggregate bootstrap result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    RetainParser(u64),
    ReleaseParser(u64),
    InitializeDictionary(u64),
    Watch(String),
    Ready(u64),
    BeginParse(u64),
    Parse(Input),
    FinishParse(u64),
    Diagnostic(String),
    UiError(Option<String>),
    Send(u32, u32),
    ReleaseMessage,
    PackedOption,
    MainDirectory,
    DataDirectory,
    ResourceService,
    UiService,
    MessageService,
    Exists(String),
    Request(Key, u32),
    Read(u64),
    ReleaseResource(u64),
    Enumerate(u32),
    ReleaseEnumeration,
    Attach,
    Install,
    RenderPropertiesChanged,
    Progress(u32, String),
    Detach,
    BeginCollection,
    EndCollection,
    Subscribe(u32),
    Unsubscribe(u32),
    ShutdownFileParser,
    ReleaseFileParser,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Void,
    Bool(bool),
    Text(String),
    Option(Option<String>),
    Resource(Option<u64>),
    Keys(Vec<Key>),
    File(FileResult),
}
pub trait Host {
    fn call(&mut self, call: Call, collection: &mut Collection) -> Reply;
}
fn boolean(host: &mut impl Host, collection: &mut Collection, call: Call) -> bool {
    let Reply::Bool(value) = host.call(call, collection) else {
        panic!("boolean host reply")
    };
    value
}
fn text(host: &mut impl Host, collection: &mut Collection, call: Call) -> String {
    let Reply::Text(value) = host.call(call, collection) else {
        panic!("text host reply")
    };
    value
}

#[derive(Default)]
pub struct EffectsParser {
    pub inputs: Vec<Input>,
    pub files_modified: u8,
    pub diagnostic: String,
    parser: Option<u64>,
    watchers: BTreeSet<String>,
}
impl EffectsParser {
    pub fn install(&mut self, parser: u64, host: &mut impl Host, collection: &mut Collection) {
        host.call(Call::RetainParser(parser), collection);
        if let Some(old) = self.parser.replace(parser) {
            host.call(Call::ReleaseParser(old), collection);
        }
        host.call(Call::InitializeDictionary(parser), collection);
    }
    pub fn append(&mut self, path: &str, flags: u32, host: &mut impl Host, collection: &mut Collection) {
        let directory = path
            .rsplit_once(['/', '\\'])
            .map_or("", |(parent, _)| parent)
            .to_owned();
        if flags & 2 != 0 && self.watchers.insert(directory.clone()) {
            host.call(Call::Watch(directory.clone()), collection);
        }
        self.inputs.push(Input {
            path: path.to_owned(),
            directory,
            flags,
        });
        self.files_modified = 1;
    }
    pub fn num_input_files(&self) -> usize {
        self.inputs.len()
    }
    pub fn input_file_path(&self, index: usize) -> Option<&str> {
        self.inputs.get(index).map(|i| i.path.as_str())
    }
    pub fn set_files_modified(&mut self, value: u8) {
        self.files_modified = value;
    }
    pub fn files_modified(&self) -> u8 {
        self.files_modified
    }

    pub fn parse_all(&mut self, host: &mut impl Host, collection: &mut Collection) -> bool {
        let Some(parser) = self.parser.filter(|p| boolean(host, collection, Call::Ready(*p))) else {
            self.diagnostic = "Not ready to parse".into();
            return true;
        };
        self.files_modified = 0;
        host.call(Call::BeginParse(parser), collection);
        let mut failed = false;
        let mut lines = 0u32;
        for input in &self.inputs {
            let Reply::File(result) = host.call(Call::Parse(input.clone()), collection) else {
                panic!("file host reply")
            };
            match result {
                FileResult::Parsed(count) => lines = lines.wrapping_add(count),
                FileResult::Rejected(diagnostic) => {
                    self.diagnostic = diagnostic;
                    failed = true;
                }
                FileResult::Exception { code, diagnostic } => {
                    if input.flags & 1 != 0 || code != 2 {
                        self.diagnostic = diagnostic;
                    }
                    failed |= input.flags & 1 != 0;
                }
            }
        }
        host.call(Call::FinishParse(parser), collection);
        if !failed {
            self.diagnostic = format!("Successfully parsed {lines} lines");
        }
        !failed
    }

    /// Manager wrapper has no public boolean and reports success every time.
    pub fn parse_queued_files(&mut self, host: &mut impl Host, collection: &mut Collection) {
        let success = self.parse_all(host, collection);
        let diagnostic = if success {
            None
        } else {
            host.call(Call::Diagnostic(self.diagnostic.clone()), collection);
            Some(self.diagnostic.clone())
        };
        if boolean(host, collection, Call::UiService) {
            host.call(Call::UiError(diagnostic), collection);
        }
        if success && boolean(host, collection, Call::MessageService) {
            host.call(Call::Send(COMPLETE, 0), collection);
            host.call(Call::ReleaseMessage, collection);
        }
    }

    pub fn shutdown(&mut self, host: &mut impl Host, collection: &mut Collection) {
        if let Some(parser) = self.parser.take() {
            host.call(Call::ReleaseParser(parser), collection);
        }
        host.call(Call::ShutdownFileParser, collection);
        host.call(Call::ReleaseFileParser, collection);
    }
}

pub struct EffectsBootstrap;
impl EffectsBootstrap {
    pub fn initialize(
        &mut self,
        parser: &mut EffectsParser,
        command_parser: u64,
        collection: &mut Collection,
        host: &mut impl Host,
    ) {
        let Reply::Option(option) = host.call(Call::PackedOption, collection) else {
            panic!("option host reply")
        };
        let packed = option.map_or(true, |s| !s.eq_ignore_ascii_case("off"));
        let main = join(&text(host, collection, Call::MainDirectory), "main.fx");
        host.call(Call::BeginCollection, collection);
        let messages = boolean(host, collection, Call::MessageService);
        if messages {
            for (id, _) in collection.triggers.clone() {
                host.call(Call::Unsubscribe(id), collection);
            }
        }
        collection.begin(None);
        let resources = boolean(host, collection, Call::ResourceService);
        let mut held = None;
        if packed && resources {
            request(host, collection, [0xEA51_18B0, 0xEA51_18B1, 1], &mut held);
            if boolean(host, collection, Call::Exists(main.clone())) {
                host.call(Call::Attach, collection);
                host.call(Call::Install, collection);
                parser.install(command_parser, host, collection);
                parser.append(&main, 3, host, collection);
                parser.parse_queued_files(host, collection);
            } else {
                host.call(Call::RenderPropertiesChanged, collection);
            }
        }
        if resources {
            let Reply::Keys(keys) = host.call(Call::Enumerate(0xEA51_18B0), collection) else {
                panic!("keys host reply")
            };
            for key in keys {
                if key[2] != 1 {
                    request(host, collection, key, &mut held);
                }
            }
        }
        if !packed {
            host.call(Call::Attach, collection);
            host.call(Call::Install, collection);
            parser.install(command_parser, host, collection);
            let config = join(&text(host, collection, Call::DataDirectory), "Effects\\config.fx");
            parser.append(&config, 3, host, collection);
            parser.append(&main, 2, host, collection);
            parser.parse_queued_files(host, collection);
            host.call(Call::Progress(2, String::new()), collection);
            host.call(Call::Detach, collection);
        }
        collection.end(None);
        if messages {
            for (id, _) in collection.triggers.clone() {
                host.call(Call::Subscribe(id), collection);
            }
        }
        host.call(Call::EndCollection, collection);
        if resources {
            host.call(Call::ReleaseEnumeration, collection);
        }
        if let Some(resource) = held {
            host.call(Call::ReleaseResource(resource), collection);
        }
    }
}
fn join(parent: &str, child: &str) -> String {
    format!("{}\\{}", parent.trim_end_matches(['/', '\\']), child)
}
fn request(host: &mut impl Host, collection: &mut Collection, key: Key, held: &mut Option<u64>) {
    if let Some(old) = held.take() {
        host.call(Call::ReleaseResource(old), collection);
    }
    let Reply::Resource(resource) = host.call(Call::Request(key, RESOURCE_IID), collection) else {
        panic!("resource host reply")
    };
    *held = resource;
    if let Some(resource) = resource {
        host.call(Call::Read(resource), collection);
    }
}

#[cfg(test)]
mod tests;
