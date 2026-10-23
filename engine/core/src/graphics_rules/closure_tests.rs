// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn section11_first_match_partial_any_and_skipped_children() {
    let (mut m, _, _, _) = fixture("rule a\natLeast memory 512\nend\nrule b\nend");
    m.environment.numbers[1] = 1024.0;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["a"]);
    m.environment.numbers[1] = 256.0;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["b"]);
    let (mut m, _, _, _) = fixture("partialRule a\nend\npartialRule b\nend");
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["a", "b"]);
    let (mut m, _, _, _) = fixture("rule -any\natLeast memory 4096\nhas MMX\nend");
    m.environment.numbers[1] = 512.0;
    m.environment.flags = 1 << 3;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["unknown"]);
    let (mut m, p, _, _) = fixture("rule fail\nhas FPU\nrule child\nstringMatch CPU é\nproperty TestInt 3\nend\nend\nrule -any\nproperty TestInt 4\nend");
    // Testing the child would fail the ASCII-only lowercase boundary.
    assert!(m.apply_rules(&AsciiLowercase).unwrap().is_empty());
    assert!(trace(&p).is_empty());
}

#[test]
fn v11_arithmetic_precedence_conversions_and_dispatch() {
    for (input, expected) in [
        ("2^3^2", 512.0),
        ("-2^2", 4.0),
        ("-5%3", 1.0),
        ("(16777216+1)-16777216", 1.0),
        ("8/4/2", 1.0),
        ("8-4-2", 2.0),
        ("0x10+2", 18.0),
        ("2^-2", 0.25),
    ] {
        assert_eq!(real(input).unwrap(), expected, "{input}");
    }
    let result = parse("set command partialRule\n$command arithmetic\nproperty TestInt 3.5\nproperty TestInt 2.5\nproperty TestInt 8388608\nproperty TestFloat \"2^3^2\"\noption 2.9 3.9\nend\neval \"partialRule evaluated\"\nend").unwrap();
    assert_eq!(result.rules[0].properties.ints, [(0, 4), (0, 2), (0, 6291456)]);
    assert_eq!(result.rules[0].properties.floats, [(0, 512.0)]);
    assert_eq!(result.rules[0].recommendations, [(2, 3)]);
    assert_eq!(result.rules[1].name, "evaluated");
    let names = parse("set n Chosen\npartialRule \"before-${n}-after\"\nend").unwrap();
    assert_eq!(names.rules[0].name, "before-Chosen-after");
    assert_eq!(
        parse("partialRule ${missing}\nend").unwrap_err().message,
        "Unknown variable"
    );
    assert_eq!(
        parse("eval \"partialRule one\" extra").unwrap_err().message,
        "Bad argument count"
    );
}

#[test]
fn v13_lf_exhaustion_retains_cr_and_identity_and_permissive_ids() {
    let mut env = Environment::default();
    env.strings[12] = "retained".into();
    let mut provider = Provider::default();
    provider.data.driver_description = "header\nsgl\n\ndrv\r\nver".into();
    env.gather(&mut provider, false);
    assert_eq!(&env.strings[6..12], ["sgl", "", "drv\r", "ver", "", ""]);
    assert_eq!(env.strings[12], "retained");
    for (vendor, device, expected) in [
        ("12G4", "GGGG", (18, 0)),
        (" 0xF", "0Xab", (15, 171)),
        ("\t123", "a\0ff", (291, 10)),
    ] {
        let text = format!("xxxxxxxx{vendor}xxxxxxxxxx{device}x");
        assert_eq!(text.len(), 27);
        assert_eq!(card_ids(&text).unwrap(), expected);
        assert_eq!(card_ids(&text[..26]).unwrap(), (0, 0));
    }
}

#[test]
fn v14_optional_error_keeps_callbacks_and_required_error_visits_later_roots() {
    let (mut m, _, _, mut files) = fixture("partialRule primary\nend");
    files.text.insert(
        "user/Graphics Options.sgr".into(),
        "partialRule retained\nend\nbad".into(),
    );
    let context = InitContext::from_directories("data", "user", false);
    assert!(m.parse_config_file(&mut files, &context));
    assert_eq!(
        m.rules().iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["primary", "retained"]
    );
    assert!(m.diagnostics.last().unwrap().contains("Unknown command"));
    let published = m.rules().to_vec();
    files.text.insert("data/Graphics Rules.sgr".into(), "bad".into());
    files
        .text
        .insert("user/Graphics Options.sgr".into(), "sinclude visited".into());
    files
        .text
        .insert("user/visited".into(), "partialRule later\nend".into());
    files.reads.clear();
    assert!(!m.parse_config_file(&mut files, &context));
    assert_eq!(
        files.reads,
        ["data/Graphics Rules.sgr", "user/Graphics Options.sgr", "user/visited"]
    );
    assert_eq!(m.rules(), published);
}

#[derive(Default)]
struct ByteLog {
    open: bool,
    calls: Vec<Vec<u8>>,
    failures: BTreeSet<usize>,
    close_ok: bool,
    closes: usize,
    shut_after: Option<usize>,
}
impl LogFile for ByteLog {
    fn open(&mut self, _: &str) -> bool {
        self.open = true;
        true
    }
    fn is_open(&self) -> bool {
        self.open
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        let index = self.calls.len();
        self.calls.push(bytes.to_vec());
        if self.shut_after == Some(index) {
            self.open = false;
        }
        !self.failures.contains(&index)
    }
    fn close(&mut self) -> bool {
        self.closes += 1;
        self.open = false;
        self.close_ok
    }
}

#[derive(Default)]
struct ByteProperties {
    entries: Vec<(Vec<u8>, PropertyValue)>,
    writes: Vec<(u32, bool)>,
}
impl Properties for ByteProperties {
    fn find_bool(&self, _: &str) -> Option<u32> {
        None
    }
    fn find_int(&self, _: &str) -> Option<u32> {
        None
    }
    fn find_float(&self, _: &str) -> Option<u32> {
        None
    }
    fn get_bool(&self, _: u32) -> Option<bool> {
        Some(false)
    }
    fn set_bool(&mut self, i: u32, v: bool) -> bool {
        self.writes.push((i, v));
        false
    }
    fn set_int(&mut self, _: u32, _: i32) -> bool {
        false
    }
    fn set_float(&mut self, _: u32, _: f32) -> bool {
        false
    }
    fn dump(&self) -> Vec<(Vec<u8>, PropertyValue)> {
        self.entries.clone()
    }
}

#[test]
fn v15_empty_options_raw_names_nuls_two_writes_and_padding() {
    let mut log = ByteLog {
        open: true,
        ..ByteLog::default()
    };
    logging::options(&mut log, &ByteProperties::default(), None, |_| 0).unwrap();
    assert_eq!(
        log.calls.concat(),
        b"Options\r\n-------\r\n\r\nProperties\r\n-------\r\n\r\n\r\n\r\n"
    );
    assert_eq!(log.calls.len(), 16);
    assert!(log.calls.chunks_exact(2).all(|pair| pair[1] == b"\r\n"));
    let properties = ByteProperties {
        entries: vec![
            (vec![255, 37, 88], PropertyValue::Bool(true)),
            (b"A\0B".to_vec(), PropertyValue::Bool(true)),
        ],
        ..ByteProperties::default()
    };
    log.calls.clear();
    logging::options(&mut log, &properties, Some(&[255; 12]), |_| u32::MAX).unwrap();
    assert!(log.calls.contains(&b"\xff%X = true".to_vec()));
    assert!(log.calls.contains(&b"A = true".to_vec()));
    for call in log.calls.iter().filter(|c| c.windows(8).any(|w| w == b"current:")) {
        assert_eq!(&call[21..], b"current: 255 recommended:  -1");
    }
}

#[test]
fn v15_write_and_close_failures_are_ignored_and_open_is_checked_per_line() {
    let mut log = ByteLog {
        open: true,
        failures: [0].into(),
        ..ByteLog::default()
    };
    assert!(!logging::line(&mut log, b"payload"));
    assert_eq!(log.calls, [b"payload".to_vec()]);
    log.calls.clear();
    log.failures = [1].into();
    assert!(!logging::line(&mut log, b"payload"));
    assert_eq!(log.calls, [b"payload".to_vec(), b"\r\n".to_vec()]);
    let mut manager = GraphicsRuleManager::<ByteProperties, _>::new(log);
    let properties = Arc::new(Mutex::new(ByteProperties::default()));
    manager.init(
        &properties,
        &mut Provider::default(),
        &mut Files::default(),
        &InitContext::from_directories("data", "user", false),
    );
    assert!(manager.last_init_error.is_none());
    assert_eq!(manager.rules()[0].properties.bools, [(u32::MAX, false); 4]);
    manager.apply_rules(&AsciiLowercase).unwrap();
    assert_eq!(properties.lock().unwrap().writes, [(u32::MAX, false); 4]);
    assert!(manager
        .log_configuration_to_file("log", &LogStamp::default(), &mut Provider::default())
        .unwrap());
    assert_eq!(manager.logger().closes, 1);
    let mut log = ByteLog {
        open: true,
        shut_after: Some(0),
        ..ByteLog::default()
    };
    let mut env = Environment::default();
    env.numbers[0] = f32::NAN;
    logging::configuration(&mut log, &env, &LogStamp::default()).unwrap();
    // The terminator is attempted after successful payload; later lines are skipped.
    assert_eq!(log.calls.len(), 2);
}

#[test]
fn nonfinite_log_profile_is_explicitly_unsupported_but_rules_still_apply() {
    let (mut m, p, _, _) = fixture("partialRule active\nproperty TestInt 9\nend");
    m.environment.numbers[9] = f32::INFINITY;
    assert_eq!(
        m.set_options_from_configuration(&LogContext::default(), &AsciiLowercase),
        Err(ManagerError::Unsupported("GR6 nonfinite %g spelling"))
    );
    assert_eq!(trace(&p), [Write::Int(0, 9)]);
    assert_eq!(m.logger().closes, 1);
    assert!(!m.logger().is_open());
}
