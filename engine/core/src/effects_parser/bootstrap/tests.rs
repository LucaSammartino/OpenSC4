// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects::EffectDescription;
use std::collections::VecDeque;

struct Services {
    calls: Vec<Call>,
    option: Option<String>,
    resources: bool,
    ui: bool,
    messages: bool,
    exists: bool,
    ready: bool,
    keys: Vec<Key>,
    files: VecDeque<FileResult>,
    missing: Vec<u32>,
}
impl Default for Services {
    fn default() -> Self {
        Self {
            calls: vec![],
            option: None,
            resources: true,
            ui: true,
            messages: true,
            exists: false,
            ready: true,
            keys: vec![],
            files: VecDeque::new(),
            missing: vec![],
        }
    }
}
impl Host for Services {
    fn call(&mut self, call: Call, collection: &mut Collection) -> Reply {
        self.calls.push(call.clone());
        match call {
            Call::Ready(_) => Reply::Bool(self.ready),
            Call::UiService => Reply::Bool(self.ui),
            Call::MessageService => Reply::Bool(self.messages),
            Call::ResourceService => Reply::Bool(self.resources),
            Call::PackedOption => Reply::Option(self.option.clone()),
            Call::MainDirectory => Reply::Text("User".into()),
            Call::DataDirectory => Reply::Text("Data".into()),
            Call::Exists(_) => Reply::Bool(self.exists),
            Call::Enumerate(_) => Reply::Keys(self.keys.clone()),
            Call::Request(key, _) => Reply::Resource((!self.missing.contains(&key[2])).then_some(u64::from(key[2]))),
            Call::Read(id) => {
                collection.add_description(&format!("resource{id}"), &EffectDescription::new(1, vec![]));
                Reply::Void
            }
            Call::Parse(input) => {
                let result = self.files.pop_front().unwrap_or(FileResult::Parsed(3));
                if matches!(result, FileResult::Parsed(_)) {
                    collection.add_description(&input.path, &EffectDescription::new(1, vec![]));
                }
                Reply::File(result)
            }
            _ => Reply::Void,
        }
    }
}
fn parsed(host: &Services) -> Vec<&Input> {
    host.calls
        .iter()
        .filter_map(|c| if let Call::Parse(i) = c { Some(i) } else { None })
        .collect()
}
fn count(host: &Services, call: &Call) -> usize {
    host.calls.iter().filter(|c| *c == call).count()
}

#[test]
fn ef18_1_retained_queue_repeated_success_watchers_and_parser_references() {
    let mut parser = EffectsParser::default();
    let mut collection = Collection::default();
    let mut host = Services::default();
    parser.install(1, &mut host, &mut collection);
    parser.install(1, &mut host, &mut collection);
    parser.install(2, &mut host, &mut collection);
    assert_eq!(
        &host.calls[..8],
        &[
            Call::RetainParser(1),
            Call::InitializeDictionary(1),
            Call::RetainParser(1),
            Call::ReleaseParser(1),
            Call::InitializeDictionary(1),
            Call::RetainParser(2),
            Call::ReleaseParser(1),
            Call::InitializeDictionary(2)
        ]
    );
    for name in ["a/A", "a/B", "a/A"] {
        parser.append(name, 3, &mut host, &mut collection);
    }
    assert_eq!(count(&host, &Call::Watch("a".into())), 1);
    parser.set_files_modified(0xAB);
    assert_eq!(parser.files_modified(), 0xAB);
    parser.parse_queued_files(&mut host, &mut collection);
    parser.parse_queued_files(&mut host, &mut collection);
    assert_eq!(
        parsed(&host).iter().map(|i| i.path.as_str()).collect::<Vec<_>>(),
        ["a/A", "a/B", "a/A", "a/A", "a/B", "a/A"]
    );
    assert_eq!(parser.num_input_files(), 3);
    assert_eq!(parser.input_file_path(2), Some("a/A"));
    assert_eq!(parser.input_file_path(3), None);
    assert_eq!(parser.files_modified(), 0);
    assert_eq!(parser.diagnostic, "Successfully parsed 9 lines");
    assert_eq!(count(&host, &Call::UiError(None)), 2);
    assert_eq!(count(&host, &Call::Send(COMPLETE, 0)), 2);
    assert_eq!(count(&host, &Call::ReleaseMessage), 2);
    parser.shutdown(&mut host, &mut collection);
    assert_eq!(
        &host.calls[host.calls.len() - 3..],
        &[
            Call::ReleaseParser(2),
            Call::ShutdownFileParser,
            Call::ReleaseFileParser
        ]
    );
}

#[test]
fn ef18_2_6_failure_retains_publications_and_not_ready_reports_success() {
    let mut parser = EffectsParser::default();
    let mut collection = Collection::default();
    let mut host = Services {
        ui: false,
        ..Services::default()
    };
    parser.append("A", 1, &mut host, &mut collection);
    parser.parse_queued_files(&mut host, &mut collection);
    assert_eq!(parser.diagnostic, "Not ready to parse");
    assert_eq!(parser.files_modified(), 1);
    assert_eq!(count(&host, &Call::Send(COMPLETE, 0)), 1);
    parser.install(9, &mut host, &mut collection);
    parser.append("B", 1, &mut host, &mut collection);
    host.files = [FileResult::Parsed(1), FileResult::Rejected("D".into())].into();
    host.calls.clear();
    parser.parse_queued_files(&mut host, &mut collection);
    assert_eq!(count(&host, &Call::Diagnostic("D".into())), 1);
    assert_eq!(count(&host, &Call::Send(COMPLETE, 0)), 0);
    assert_eq!(count(&host, &Call::FinishParse(9)), 1);
    assert!(collection.get_description("a", None));
    host.ready = false;
    host.ui = true;
    host.messages = false;
    parser.parse_queued_files(&mut host, &mut collection);
    assert_eq!(parser.diagnostic, "Not ready to parse");
    assert_eq!(count(&host, &Call::UiError(None)), 1);
}

#[test]
fn ef18_3_packed_reader_order_skips_all_instance_one_keys() {
    let mut parser = EffectsParser::default();
    let mut collection = Collection::default();
    let mut host = Services {
        keys: vec![[0xEA5118B0, 55, 1], [0xEA5118B0, 55, 3], [0xEA5118B0, 55, 2]],
        ..Services::default()
    };
    EffectsBootstrap.initialize(&mut parser, 9, &mut collection, &mut host);
    let significant: Vec<_> = host
        .calls
        .iter()
        .filter(|c| {
            matches!(
                c,
                Call::BeginCollection
                    | Call::Read(_)
                    | Call::RenderPropertiesChanged
                    | Call::ReleaseResource(_)
                    | Call::EndCollection
                    | Call::ReleaseEnumeration
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        significant,
        [
            Call::BeginCollection,
            Call::Read(1),
            Call::RenderPropertiesChanged,
            Call::ReleaseResource(1),
            Call::Read(3),
            Call::ReleaseResource(3),
            Call::Read(2),
            Call::EndCollection,
            Call::ReleaseEnumeration,
            Call::ReleaseResource(2)
        ]
    );
    assert_eq!(parser.num_input_files(), 0);
    assert_eq!(collection.descriptions.len(), 3);
}

#[test]
fn ef18_4_unpacked_failure_continues_progress_detach_and_end() {
    let mut parser = EffectsParser::default();
    let mut collection = Collection::default();
    let mut host = Services {
        option: Some("OFF".into()),
        keys: vec![[0xEA5118B0, 55, 8]],
        files: [
            FileResult::Exception {
                code: 2,
                diagnostic: "Couldn't open file 'Data\\Effects\\config.fx'".into(),
            },
            FileResult::Parsed(3),
        ]
        .into(),
        ..Services::default()
    };
    EffectsBootstrap.initialize(&mut parser, 9, &mut collection, &mut host);
    let inputs = parsed(&host);
    assert_eq!(
        inputs.iter().map(|i| (i.path.as_str(), i.flags)).collect::<Vec<_>>(),
        [("Data\\Effects\\config.fx", 3), ("User\\main.fx", 2)]
    );
    assert!(!host.calls.iter().any(|c| matches!(c, Call::Exists(_))));
    assert_eq!(count(&host, &Call::Send(COMPLETE, 0)), 0);
    let progress = host
        .calls
        .iter()
        .position(|c| *c == Call::Progress(2, String::new()))
        .unwrap();
    assert_eq!(
        &host.calls[progress..progress + 3],
        &[Call::Progress(2, String::new()), Call::Detach, Call::EndCollection]
    );
    assert!(collection.get_description("resource8", None));
    assert!(collection.get_description("User\\main.fx", None));
}

#[test]
fn ef18_5_absent_resources_skips_packed_files_and_options_do_not_trim() {
    for option in [None, Some("off "), Some(" on"), Some("unknown"), Some("ON")] {
        let mut parser = EffectsParser::default();
        let mut collection = Collection::default();
        let mut host = Services {
            option: option.map(str::to_owned),
            resources: false,
            exists: true,
            ..Services::default()
        };
        EffectsBootstrap.initialize(&mut parser, 9, &mut collection, &mut host);
        assert!(!host.calls.iter().any(|c| matches!(
            c,
            Call::Parse(_) | Call::Exists(_) | Call::Enumerate(_) | Call::RenderPropertiesChanged
        )));
        assert_eq!(count(&host, &Call::BeginCollection), 1);
        assert_eq!(count(&host, &Call::EndCollection), 1);
    }
}

#[test]
fn ef18_12_optional_open_exception_and_other_optional_exceptions_succeed() {
    for code in [2, 7] {
        let mut parser = EffectsParser::default();
        let mut collection = Collection::default();
        let mut host = Services {
            option: Some("off".into()),
            resources: false,
            files: [
                FileResult::Parsed(17),
                FileResult::Exception {
                    code,
                    diagnostic: "optional".into(),
                },
            ]
            .into(),
            ..Services::default()
        };
        EffectsBootstrap.initialize(&mut parser, 9, &mut collection, &mut host);
        assert_eq!(parser.num_input_files(), 2);
        assert_eq!(parser.diagnostic, "Successfully parsed 17 lines");
        assert_eq!(count(&host, &Call::UiError(None)), 1);
        assert_eq!(count(&host, &Call::Send(COMPLETE, 0)), 1);
        assert_eq!(count(&host, &Call::Detach), 1);
    }
}

#[test]
fn packed_existing_main_retains_parser_and_resource_failures_do_not_abort() {
    let mut parser = EffectsParser::default();
    let mut collection = Collection::default();
    let mut host = Services {
        exists: true,
        missing: vec![1, 2],
        keys: vec![[0xEA5118B0, 0, 2], [0xEA5118B0, 0, 3]],
        ..Services::default()
    };
    EffectsBootstrap.initialize(&mut parser, 9, &mut collection, &mut host);
    assert_eq!(parser.inputs[0].flags, 3);
    assert_eq!(count(&host, &Call::Detach), 0);
    assert_eq!(count(&host, &Call::Read(3)), 1);
    assert_eq!(count(&host, &Call::Read(1)), 0);
    let parse = host.calls.iter().position(|c| matches!(c, Call::Parse(_))).unwrap();
    let enumerate = host.calls.iter().position(|c| matches!(c, Call::Enumerate(_))).unwrap();
    assert!(parse < enumerate);
}
