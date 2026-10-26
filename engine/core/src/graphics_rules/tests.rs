// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent expected traces from graphics-rules.md §§1–7/9/11 and V1–V10.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
enum Write {
    Bool(u32, bool),
    Int(u32, i32),
    Float(u32, f32),
}

#[derive(Default)]
struct Registry {
    bools: [bool; 6],
    integer: i32,
    float: f32,
    trace: Vec<Write>,
    fail_setters: bool,
}
impl Properties for Registry {
    fn find_bool(&self, name: &str) -> Option<u32> {
        [
            "forceSoftwareDriver",
            "TestBool",
            "RenderCloudsAndFog",
            "RenderShadows",
            "RenderDecals",
            "UseSecondStage",
        ]
        .iter()
        .position(|&n| n == name)
        .map(|i| i as u32)
        .or_else(|| (name == "Collision").then_some(1))
    }
    fn find_int(&self, name: &str) -> Option<u32> {
        matches!(name, "TestInt" | "Collision").then_some(0)
    }
    fn find_float(&self, name: &str) -> Option<u32> {
        matches!(name, "TestFloat" | "Collision").then_some(0)
    }
    fn get_bool(&self, index: u32) -> Option<bool> {
        self.bools.get(index as usize).copied()
    }
    fn set_bool(&mut self, index: u32, value: bool) -> bool {
        self.trace.push(Write::Bool(index, value));
        if self.fail_setters {
            return false;
        }
        self.bools[index as usize] = value;
        true
    }
    fn set_int(&mut self, index: u32, value: i32) -> bool {
        self.trace.push(Write::Int(index, value));
        if self.fail_setters {
            return false;
        }
        self.integer = value;
        true
    }
    fn set_float(&mut self, index: u32, value: f32) -> bool {
        self.trace.push(Write::Float(index, value));
        if self.fail_setters {
            return false;
        }
        self.float = value;
        true
    }
    fn dump(&self) -> Vec<(Vec<u8>, PropertyValue)> {
        vec![
            (b"forceSoftwareDriver".to_vec(), PropertyValue::Bool(self.bools[0])),
            (b"TestInt".to_vec(), PropertyValue::Int(self.integer)),
            (b"TestFloat".to_vec(), PropertyValue::Float(self.float)),
        ]
    }
}

#[derive(Default)]
struct Files {
    text: BTreeMap<String, String>,
    fail: BTreeSet<String>,
    reads: Vec<String>,
}
impl ScriptFiles for Files {
    fn read(&mut self, path: &str) -> Result<Option<String>, String> {
        self.reads.push(path.into());
        if self.fail.contains(path) {
            Err("Read failed".into())
        } else {
            Ok(self.text.get(path).cloned())
        }
    }
}

struct Provider {
    data: Measurements,
    measures: usize,
    probes: usize,
    fsaa: bool,
}
impl Default for Provider {
    fn default() -> Self {
        Self {
            data: Measurements {
                cpu_features: 0,
                cpu_mhz: 0,
                physical_bytes: 0,
                available_bytes: 0,
                screen: [0; 3],
                video_bytes: 0,
                texture_bytes: 0,
                os_strings: ["user", "app", "version", "os", "ALPHA", "CPU"].map(str::to_owned),
                driver_record: [0; 0x34],
                driver_description: "header\nSGL\n1\nDriver\n2\nCard\n3".into(),
                sound_names: None,
                backing_store: false,
            },
            measures: 0,
            probes: 0,
            fsaa: false,
        }
    }
}
impl InformationProvider for Provider {
    fn measure(&mut self) -> Measurements {
        self.measures += 1;
        self.data.clone()
    }
    fn probe_forced_fsaa(&mut self, _: &[u8; 0x34]) -> bool {
        self.probes += 1;
        self.fsaa
    }
}

struct Log {
    allow: bool,
    open: bool,
    paths: Vec<String>,
    text: String,
    closes: usize,
}
impl Default for Log {
    fn default() -> Self {
        Self {
            allow: true,
            open: false,
            paths: Vec::new(),
            text: String::new(),
            closes: 0,
        }
    }
}
impl LogFile for Log {
    fn open(&mut self, path: &str) -> bool {
        self.paths.push(path.into());
        self.open = self.allow;
        self.allow
    }
    fn is_open(&self) -> bool {
        self.open
    }
    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        self.text.push_str(std::str::from_utf8(bytes).unwrap());
        true
    }
    fn close(&mut self) -> bool {
        self.closes += 1;
        self.open = false;
        true
    }
}

type Manager = GraphicsRuleManager<Registry, Log>;

#[path = "closure_tests.rs"]
mod closure_tests;
fn fixture(script: &str) -> (Manager, Arc<Mutex<Registry>>, Provider, Files) {
    let properties = Arc::new(Mutex::new(Registry::default()));
    let mut files = Files::default();
    files.text.insert("data/Graphics Rules.sgr".into(), script.into());
    let mut provider = Provider::default();
    let mut manager = Manager::new(Log::default());
    manager.init(
        &properties,
        &mut provider,
        &mut files,
        &InitContext::from_directories("data/", "user/", false),
    );
    (manager, properties, provider, files)
}
fn parse(script: &str) -> Result<ParsedRules, ParseError> {
    RuleParser::new(&Registry::default(), 0, 0, false).parse_text(&mut Files::default(), "fixture.sgr", script)
}
fn trace(properties: &Arc<Mutex<Registry>>) -> Vec<Write> {
    properties.lock().unwrap().trace.clone()
}

#[test]
fn v1_thresholds_and_typed_write_order() {
    let (mut m, p, _, _) = fixture("partialRule lower\n atLeast memory 2.9\n property TestFloat 1.25\n property TestInt 7\n property TestBool true\n property TestInt 9\nend\npartialRule upper\n atMost memory 2.9\n property TestBool false\nend");
    assert_eq!(m.rules()[0].lower, [(1, 2)]);
    assert_eq!(m.rules()[1].upper, [(1, 2)]);
    m.environment.numbers[1] = 2.5;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["lower"]);
    assert_eq!(
        trace(&p),
        [
            Write::Bool(1, true),
            Write::Int(0, 7),
            Write::Int(0, 9),
            Write::Float(0, 1.25)
        ]
    );
    p.lock().unwrap().trace.clear();
    m.environment.numbers[1] = 2.0;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["lower", "upper"]);
    assert_eq!(trace(&p).last(), Some(&Write::Bool(1, false)));
}

#[test]
fn threshold_conversion_is_signed64_low32_without_binary32_rounding() {
    for (input, expected) in [
        ("-2.9", -2),
        ("2147483648", i32::MIN),
        ("4294967295", -1),
        ("4294967296", 0),
        ("16777217", 16777217),
        ("-9223372036854775808", 0),
        ("9223372036854775808", 0),
        ("1e999", 0),
    ] {
        assert_eq!(threshold_from_real(real(input).unwrap()), expected, "{input}");
    }
    assert_eq!(threshold_from_real(f64::NAN), 0);
    let mut env = Environment::default();
    env.numbers[1] = -2.5;
    let rule = &parse("rule\natLeast memory -2.9\nend").unwrap().rules[0];
    assert!(!rule.matches(&env, &AsciiLowercase).unwrap());
    let rule = &parse("rule\natLeast memory 16777217\nend").unwrap().rules[0];
    env.numbers[1] = 16777216.0;
    assert!(!rule.matches(&env, &AsciiLowercase).unwrap());
    let rule = &parse("rule\natMost memory 2147483647\nend").unwrap().rules[0];
    env.numbers[1] = 2147483648.0;
    assert!(!rule.matches(&env, &AsciiLowercase).unwrap());
}

#[test]
fn gr5_nan_follows_main_spec_all_pass_any_no_match() {
    let mut env = Environment::default();
    env.numbers[1] = f32::NAN;
    for condition in [
        "atLeast memory 2",
        "atMost memory 2",
        "atLeast memory 2\natMost memory 2",
    ] {
        let all = parse(&format!("rule\n{condition}\nend")).unwrap();
        let any = parse(&format!("rule -any\n{condition}\nend")).unwrap();
        assert!(all.rules[0].matches(&env, &AsciiLowercase).unwrap());
        assert!(!any.rules[0].matches(&env, &AsciiLowercase).unwrap());
        let failing = parse(&format!("rule\nhas MMX\n{condition}\nend")).unwrap();
        assert!(!failing.rules[0].matches(&env, &AsciiLowercase).unwrap());
    }
}

#[test]
fn v2_flags_aggregate_empty_modes() {
    let (mut m, p, _, _) = fixture("partialRule \"last flag wins\" -any\nhas MMX\nhasNo MMX\nproperty TestInt 1\nend\npartialRule \"empty any\" -any\nproperty TestInt 2\nend\nrule \"empty all\"\nproperty TestInt 3\nend");
    assert_eq!((m.rules()[0].flag_mask, m.rules()[0].flag_expected), (8, 0));
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["last flag wins", "empty all"]);
    assert_eq!(trace(&p), [Write::Int(0, 1), Write::Int(0, 3)]);
    m.environment.flags = 8;
    p.lock().unwrap().trace.clear();
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["empty all"]);
    assert_eq!(trace(&p), [Write::Int(0, 3)]);
    let any = &parse("rule -any\nhas MMX\nhasNo FPU\nend").unwrap().rules[0];
    for (flags, expected) in [(0, true), (8, true), (16, false), (24, true)] {
        m.environment.flags = flags;
        assert_eq!(any.matches(&m.environment, &AsciiLowercase).unwrap(), expected);
    }
}

#[test]
fn v3_wildcards_and_negative_polarity() {
    for (value, pattern, expected) in [
        ("GeForce FX", "gEfOrCe*", true),
        ("GeForce FX", "force", false),
        ("GeForce FX", "*force*", true),
        ("AB", "a?", true),
        ("A", "a?", false),
        ("", "*", true),
        ("", "?", false),
        ("a/b", "a*b", true),
        ("a", "[a]", false),
        ("abbbbbc", "a*b*c", true),
        ("ab", "**?*?**", true),
        ("a\\b", "a*b", true),
        ("*ba", "*a", true),
        ("**", "*", true),
    ] {
        for negative in [false, true] {
            let command = if negative { "stringNotMatch" } else { "stringMatch" };
            let parsed = parse(&format!("rule\n{command} CPU \"{pattern}\"\nend")).unwrap();
            let mut env = Environment::default();
            env.strings[5] = value.into();
            assert_eq!(
                parsed.rules[0].matches(&env, &AsciiLowercase).unwrap(),
                expected != negative,
                "{value}/{pattern}"
            );
            assert_eq!(env.strings[5], value);
            assert_eq!(
                if negative {
                    &parsed.rules[0].negative[0].1
                } else {
                    &parsed.rules[0].positive[0].1
                },
                pattern
            );
        }
    }
}

#[test]
fn lowercase_is_a_service_but_byte_wildcard_decisions_stay_in_core() {
    struct Identity;
    impl Lowercase for Identity {
        fn lowercase(&self, b: &[u8]) -> Result<Vec<u8>, MatchError> {
            Ok(b.to_vec())
        }
    }
    assert_eq!(wildcard_match("é", "?", &Identity), Ok(false));
    assert_eq!(wildcard_match("é", "??", &Identity), Ok(true));
    assert_eq!(wildcard_match("é", "*", &AsciiLowercase), Err(MatchError));
    assert_eq!(wildcard_match("*", "\\*", &AsciiLowercase), Ok(false));
}

#[test]
fn condition_storage_and_short_circuit_order() {
    struct Spy(Mutex<Vec<Vec<u8>>>);
    impl Lowercase for Spy {
        fn lowercase(&self, b: &[u8]) -> Result<Vec<u8>, MatchError> {
            self.0.lock().unwrap().push(b.to_vec());
            Ok(b.to_ascii_lowercase())
        }
    }
    let parsed = parse("rule\nstringNotMatch CPU never\nstringMatch CPU C*\natMost memory 9\natLeast memory 1\nhas MMX\natLeast memory 2\nstringMatch CPU *U\nend").unwrap();
    let r = &parsed.rules[0];
    assert_eq!(r.lower, [(1, 1), (1, 2)]);
    let spy = Spy(Mutex::new(Vec::new()));
    let mut e = Environment::default();
    e.numbers[1] = 3.0;
    e.strings[5] = "CPU".into();
    assert!(!r.matches(&e, &spy).unwrap());
    assert!(spy.0.lock().unwrap().is_empty());
    e.flags = 8;
    e.numbers[1] = 10.0;
    assert!(!r.matches(&e, &spy).unwrap());
    assert!(spy.0.lock().unwrap().is_empty());
    e.numbers[1] = 3.0;
    assert!(r.matches(&e, &spy).unwrap());
    assert_eq!(
        *spy.0.lock().unwrap(),
        [
            b"CPU".to_vec(),
            b"C*".to_vec(),
            b"CPU".to_vec(),
            b"*U".to_vec(),
            b"CPU".to_vec(),
            b"never".to_vec()
        ]
    );
}

const NESTED: &str = "rule parent\n property TestInt 1\n rule child\n has MMX\n property TestInt 8\n end\n rule \"child alternative\"\n property TestInt 9\n end\n property TestInt 4\nend\nrule \"parent alternative\"\n property TestInt 99\nend";
#[test]
fn v4_nested_current_rule_is_restored_after_child_end() {
    for (flags, names, writes) in [
        (8, vec!["parent", "child"], vec![1, 4, 8]),
        (0, vec!["parent", "child alternative"], vec![1, 4, 9]),
    ] {
        let (mut m, p, _, _) = fixture(NESTED);
        m.environment.flags = flags;
        assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), names);
        assert_eq!(
            trace(&p),
            writes.into_iter().map(|v| Write::Int(0, v)).collect::<Vec<_>>()
        );
        assert_eq!(m.rules().iter().map(|r| r.depth).collect::<Vec<_>>(), [0, 1, 1, 0]);
    }
    let script = NESTED.replacen("rule parent", "rule parent\nhas FPU", 1);
    let (mut m, p, _, _) = fixture(&script);
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["parent alternative"]);
    assert_eq!(trace(&p), [Write::Int(0, 99)]);
}

#[test]
fn first_match_partial_and_multilevel_depth_stack() {
    let (mut m, _, _, _) = fixture("rule a\natLeast memory 512\nend\nrule b\nend");
    for (memory, expected) in [(1024.0, "a"), (256.0, "b")] {
        m.environment.numbers[1] = memory;
        assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), [expected]);
    }
    let (mut m, _, _, _) = fixture("partialRule p\nrule c\nrule grandchild\nend\nrule skippedGrandchild\nend\nend\nrule skippedChild\nend\nend\npartialRule q\nend\nrule z\nend\nrule skippedRoot\nend");
    assert_eq!(
        m.apply_rules(&AsciiLowercase).unwrap(),
        ["p", "c", "grandchild", "q", "z"]
    );
    let (mut m, _, _, _) = fixture("rule -any\nrule untested\nend\nend\npartialRule x\nend\npartialRule y\nend");
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["x", "y"]);
}

#[test]
fn unnamed_rules_use_last_emitted_name_and_switch_alias() {
    let parsed = parse("rule\nrule named\nend\nrule -partial -any\nend\nend\npartialRule\nend").unwrap();
    assert_eq!(
        parsed.rules.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["unknown", "named", "named>", "named>>"]
    );
    assert!(parsed.rules[2].partial && parsed.rules[2].any && parsed.rules[3].partial);
}

#[test]
fn v5_dense_defaults_reopen_and_recommendations_are_not_application() {
    let script =
        "optionGroup 2\noption 3\nproperty TestInt 7\nend\npartialRule recommend\noption 2 3\noption 5 99\nend";
    let (mut m, p, _, _) = fixture(script);
    assert_eq!(m.groups().len(), 3);
    assert_eq!(m.groups()[2].options.len(), 4);
    assert!(m.groups().iter().all(|g| g.recommendation == 0));
    assert!(m.groups()[0].options.is_empty());
    assert_eq!(m.groups()[2].options[0], PropertySet::default());
    assert!(trace(&p).is_empty());
    m.apply_rules(&AsciiLowercase).unwrap();
    assert_eq!(m.groups().len(), 6);
    assert_eq!(
        (
            m.get_option_group_auto_setting(2),
            m.get_option_group_auto_setting(5),
            m.get_option_group_auto_setting(99)
        ),
        (3, 99, 0)
    );
    assert!(m.groups()[5].options.is_empty());
    assert!(trace(&p).is_empty());
    for (g, o) in [(2, 0), (5, 99), (99, 0), (u32::MAX, 0), (2, u32::MAX)] {
        m.apply_option_group_setting(g, o).unwrap();
    }
    assert!(trace(&p).is_empty());
    m.apply_option_group_setting(2, 3).unwrap();
    assert_eq!(trace(&p), [Write::Int(0, 7)]);
    let (mut m, p, _, _) = fixture(&format!("{script}\noptionGroup 2\noption 3\nproperty TestInt 8\nend"));
    m.apply_option_group_setting(2, 3).unwrap();
    assert_eq!(trace(&p), [Write::Int(0, 7), Write::Int(0, 8)]);
    assert_eq!(m.get_option_group_auto_setting(2), 0);
}

#[test]
fn recommendation_duplicates_last_win_and_option_apply_does_not_clear_flags() {
    let (mut m, p, _, _) = fixture(
        "optionGroup 0\noption 0\nproperty forceSoftwareDriver true\nend\npartialRule\noption 0 7\noption 0 9\nend",
    );
    m.environment.flags = 1;
    m.apply_option_group_setting(0, 0).unwrap();
    assert_eq!(m.environment.flags, 1);
    assert_eq!(m.get_option_group_auto_setting(0), 0);
    assert_eq!(trace(&p), [Write::Bool(0, true)]);
    m.apply_rules(&AsciiLowercase).unwrap();
    assert_eq!(m.get_option_group_auto_setting(0), 9);
    assert_eq!(m.environment.flags, 0);
}

#[test]
fn v6_force_software_clear_is_persistent_until_gather() {
    let (mut m, p, mut provider, _) = fixture("partialRule force\nhas hardwareDriver\nproperty forceSoftwareDriver true\nend\npartialRule undo\nproperty forceSoftwareDriver false\nend\npartialRule hw\nhas hardwareDriver\nproperty TestInt 8\nend\npartialRule sw\nhasNo hardwareDriver\nproperty TestInt 9\nend");
    m.environment.flags = 1;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["force", "undo", "sw"]);
    assert_eq!(
        trace(&p),
        [Write::Bool(0, true), Write::Bool(0, false), Write::Int(0, 9)]
    );
    p.lock().unwrap().trace.clear();
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["undo", "sw"]);
    assert_eq!(trace(&p), [Write::Bool(0, false), Write::Int(0, 9)]);
    provider.data.driver_record[0x30] = 1;
    m.gather_information(&mut provider).unwrap();
    assert_ne!(m.environment.flags & 1, 0);
}

#[test]
fn v7_record_flags_units_unsigned_conversions_and_bogo_defaults() {
    let mut p = Provider::default();
    p.data.driver_record[0x20] = 2;
    p.data.driver_record[0x24] = 3;
    p.data.driver_record[0x2c] = 7;
    p.data.driver_record[0x30] = 1;
    p.data.cpu_features = (1 << 23) | 1;
    p.fsaa = true;
    p.data.physical_bytes = 257 * (1 << 20) + 123;
    p.data.available_bytes = 129 * (1 << 20) - 1;
    p.data.video_bytes = 3 * (1 << 20) + 999;
    p.data.texture_bytes = (1 << 20) - 1;
    let mut e = Environment::default();
    assert!(e.numbers.iter().all(|x| x.to_bits() == 0));
    e.gather(&mut p, false);
    assert_eq!(e.flags, 0x5f);
    assert_eq!(e.numbers[8], 3.0);
    assert_eq!(&e.numbers[1..3], &[260.0, 128.0]);
    assert_eq!(&e.numbers[6..8], &[3.0, 0.0]);
    assert_eq!(&e.numbers[9..], &[0.0, 0.0]);
    assert_eq!(&e.strings[6..12], &["SGL", "1", "Driver", "2", "Card", "3"]);
    assert_eq!(&e.strings[13..], &["Unknown", "Unknown", "Release"]);
    e.gather(&mut p, true);
    assert_eq!(e.flags, 0x5e);
    assert_eq!(p.probes, 2);
    p.data.backing_store = true;
    p.data.cpu_mhz = u32::MAX;
    p.data.screen = [u32::MAX, 2, 32];
    p.data.driver_record[0x24..0x28].copy_from_slice(&u32::MAX.to_le_bytes());
    p.data.physical_bytes = 0;
    p.data.available_bytes = u32::MAX;
    p.data.sound_names = Some(["SoundDrv".into(), "SoundCard".into()]);
    e.numbers[9] = 123.0;
    e.numbers[10] = -0.0;
    e.strings[12] = "identity".into();
    e.gather(&mut p, true);
    assert_eq!(e.flags, 0x7e);
    assert_eq!(e.numbers[0], 4294967296.0);
    assert_eq!((e.numbers[1], e.numbers[2]), (0.0, 4096.0));
    assert_eq!(e.numbers[8], 4294967296.0);
    assert_eq!(e.numbers[9], 123.0);
    assert_eq!(e.numbers[10].to_bits(), (-0.0f32).to_bits());
    assert_eq!(e.strings[12], "identity");
    assert_eq!(&e.strings[13..15], &["SoundDrv", "SoundCard"]);
}

#[test]
fn v7_readback_provider_helper_ignores_alpha_and_needs_two_samples() {
    let mut pixels = [[0, 0, 0, 255]; 64];
    assert!(!forced_fsaa_from_samples(false, Some(&pixels)));
    assert!(!forced_fsaa_from_samples(true, None));
    assert!(!forced_fsaa_from_samples(true, Some(&pixels)));
    pixels[0][0] = 1;
    assert!(!forced_fsaa_from_samples(true, Some(&pixels)));
    pixels[63][2] = 1;
    assert!(forced_fsaa_from_samples(true, Some(&pixels)));
    assert!(!forced_fsaa_from_samples(false, Some(&pixels)));
}

#[test]
fn v8_exact_fallback_rules_and_boundary_stage_two() {
    for (hardware, stages, names, values) in [
        (false, 0.0, vec!["standard software"], [false; 4]),
        (false, 8.0, vec!["standard software"], [false; 4]),
        (true, 1.0, vec!["standard hardware"], [true, true, true, false]),
        (true, 2.0, vec!["standard hardware", "two stages"], [true; 4]),
    ] {
        let (mut m, p, _, _) = fixture("invalid command");
        assert!(m.last_parse_error.is_some());
        assert!(m.last_init_error.is_none());
        assert!(m.groups().is_empty());
        assert!(trace(&p).is_empty());
        assert_eq!(m.rules().iter().map(|r| r.depth).collect::<Vec<_>>(), [0, 0, 1]);
        assert_eq!(m.rules()[1].flag_mask, 0);
        assert!(m.rules().iter().all(|r| !r.any && !r.partial));
        m.environment.flags = u32::from(hardware);
        m.environment.numbers[8] = stages;
        assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), names);
        assert_eq!(&p.lock().unwrap().bools[2..], &values);
        if hardware && stages == 2.0 {
            assert_eq!(&trace(&p)[3..], &[Write::Bool(5, false), Write::Bool(5, true)]);
        }
        assert_eq!(m.diagnostics, ["Using fallback rules."]);
    }
}

#[test]
fn v9_vendor_latch_fixed_offsets_and_unmatched_vendor_reset() {
    let script = "vendor First 0x1234\ncard 0x5678 Chosen\ncard 0x5678 \"Too late\"\nend\nvendor Second 0x1234\ncard 0x5678 \"Also too late\"\nend";
    let result = RuleParser::new(&Registry::default(), 0x1234, 0x5678, false)
        .parse_text(&mut Files::default(), "cards", script)
        .unwrap();
    assert_eq!(result.card_identity.as_deref(), Some("First, Chosen"));
    let changed = script.replacen("card 0x5678", "card 0x5679", 2);
    let result = RuleParser::new(&Registry::default(), 0x1234, 0x5678, false)
        .parse_text(&mut Files::default(), "cards", &changed)
        .unwrap();
    assert_eq!(result.card_identity.as_deref(), Some("Second, Also too late"));
    let unmatched = "vendor First 0x1234\ncard 0x9999 Nope\nend\nvendor Other 0x9999\ncard 0x5678 WrongVendor\nend";
    assert_eq!(
        RuleParser::new(&Registry::default(), 0x1234, 0x5678, false)
            .parse_text(&mut Files::default(), "cards", unmatched)
            .unwrap()
            .card_identity,
        None
    );
    let mut v = vec![b'x'; 27];
    v[8..12].copy_from_slice(b"1234");
    v[22..26].copy_from_slice(b"5678");
    let v = String::from_utf8(v).unwrap();
    assert_eq!(card_ids(&v).unwrap(), (0x1234, 0x5678));
    assert_eq!(card_ids(&v[..26]).unwrap(), (0, 0));
    let (mut m, p, mut provider, mut files) = fixture("");
    provider.data.driver_description = format!("header\nSGL\n1\nDriver\n2\nUnlisted\n{v}");
    files.text.insert("data/Graphics Rules.sgr".into(), script.into());
    m.init(
        &p,
        &mut provider,
        &mut files,
        &InitContext::from_directories("data/", "user/", false),
    );
    assert_eq!(m.environment.strings[12], "First, Chosen");
    files.text.insert("data/Graphics Rules.sgr".into(), "bad".into());
    assert!(!m.parse_config_file(&mut files, &InitContext::from_directories("data/", "user/", false)));
    assert_eq!(m.environment.strings[12], "Unlisted");
}

#[test]
fn v10_explicit_logging_regathers_without_evaluation_and_failed_open_still_gathers() {
    let (mut m, p, mut provider, _) = fixture("rule\nproperty TestInt 7\nend");
    provider.data.cpu_mhz = 999;
    let stamp = LogStamp {
        date: "2026-10-22".into(),
        hour: 9,
        minute: 7,
    };
    assert_eq!(
        m.log_configuration_to_file("explicit.txt", &stamp, &mut provider),
        Ok(true)
    );
    assert_eq!(provider.measures, 2);
    assert_eq!(m.environment.numbers[0], 999.0);
    assert!(trace(&p).is_empty());
    assert_eq!(m.logger().closes, 1);
    assert!(m.logger().text.starts_with("Log generated on 2026-10-22, 09:07\r\n"));
    assert!(!m.logger().text.contains("Rules\r\n"));
    assert!(!m.logger().text.contains("recommended:"));
    m.log.allow = false;
    provider.data.cpu_mhz = 1001;
    assert_eq!(
        m.log_configuration_to_file("failed.txt", &stamp, &mut provider),
        Ok(false)
    );
    assert_eq!(provider.measures, 3);
    assert_eq!(m.environment.numbers[0], 1001.0);
    assert_eq!(m.logger().closes, 1);
    let context = LogContext {
        write_log: Some("oFf".into()),
        ..LogContext::default()
    };
    m.set_options_from_configuration(&context, &AsciiLowercase).unwrap();
    assert_eq!(trace(&p), [Write::Int(0, 7)]);
    assert_eq!(provider.measures, 3);
    assert_eq!(m.logger().paths, ["explicit.txt", "failed.txt"]);
}

#[test]
fn v10_errors_never_publish_partial_lists_and_init_installs_fallback() {
    for (script, expected) in [
        ("rule\nproperty noSuchThing 1\nend", "Unknown render property"),
        ("rule\nhas mmx\nend", "Unknown enum: mmx"),
        ("rule\natLeast memory \"2x\"\nend", "Bad real number"),
    ] {
        let (mut m, p, mut provider, mut files) =
            fixture("optionGroup 2\noption 0\nproperty TestInt 3\nend\nrule keep\nend");
        let rules = m.rules.clone();
        let groups = m.groups.clone();
        files.text.insert(
            "data/Graphics Rules.sgr".into(),
            format!("partialRule partly\nend\n{script}"),
        );
        assert!(!m.parse_config_file(&mut files, &InitContext::from_directories("data/", "user/", false)));
        assert_eq!(m.rules(), rules);
        assert_eq!(m.groups(), groups);
        assert_eq!(m.last_parse_error.as_ref().unwrap().message, expected);
        m.init(
            &p,
            &mut provider,
            &mut files,
            &InitContext::from_directories("data/", "user/", false),
        );
        assert_eq!(m.rules()[0].name, "standard software");
        assert_eq!(m.groups(), groups);
        assert!(trace(&p).is_empty());
    }
}

#[test]
fn log_filename_preferences_property_dump_and_failed_open_evaluation() {
    let (mut m, p, provider, _) = fixture("partialRule r\noption 3 77\nproperty TestInt -7\nend");
    let context = LogContext {
        app_directory: "app/".into(),
        preferences: Some([255; 12]),
        ..LogContext::default()
    };
    m.set_options_from_configuration(&context, &AsciiLowercase).unwrap();
    assert_eq!(m.logger().paths, ["app/ALPHA-config-log.txt"]);
    assert_eq!(provider.measures, 1);
    assert!(m
        .logger()
        .text
        .contains("Rules\r\n-----\r\napplying r\r\n  option 3 = 77\r\n"));
    assert!(m.logger().text.contains("TestInt = -7\r\n"));
    let rows: Vec<_> = m.logger().text.lines().filter(|s| s.contains("recommended:")).collect();
    assert_eq!(rows.len(), 12);
    assert!(rows[0].starts_with("Visual Effects"));
    assert!(rows[11].starts_with("Texture Quality"));
    assert!(rows[11].contains("current: 255 recommended:  77"));
    m.log.allow = false;
    m.log.text.clear();
    m.set_options_from_configuration(&context, &AsciiLowercase).unwrap();
    assert!(m.logger().text.is_empty());
    assert_eq!(trace(&p), [Write::Int(0, -7), Write::Int(0, -7)]);
    m.log.allow = true;
    m.log.text.clear();
    m.set_options_from_configuration(&LogContext::default(), &AsciiLowercase)
        .unwrap();
    assert!(!m.logger().text.contains("recommended:"));
    assert!(m.logger().text.contains("TestInt = -7"));
}

#[test]
fn ownership_defaults_reinit_same_and_replacement_are_counted() {
    let mut m = Manager::new(Log::default());
    assert!(m.rules().is_empty() && m.groups().is_empty());
    assert!(m.environment.strings.iter().all(String::is_empty));
    assert!(m.environment.numbers.iter().all(|v| v.to_bits() == 0));
    assert_eq!((CLASS_ID, INTERFACE_ID), (0x0a1d8285, 0x0a1d8284));
    let old = Arc::new(Mutex::new(Registry::default()));
    let new = Arc::new(Mutex::new(Registry::default()));
    let mut files = Files::default();
    files.text.insert("r".into(), "rule\nproperty TestInt 9\nend".into());
    let c = InitContext {
        rules_path: "r".into(),
        options_path: "o".into(),
        prefs_first_time: false,
    };
    let mut provider = Provider::default();
    m.init(&old, &mut provider, &mut files, &c);
    assert_eq!(Arc::strong_count(&old), 2);
    m.init(&old, &mut provider, &mut files, &c);
    assert_eq!(Arc::strong_count(&old), 2);
    assert!(trace(&old).is_empty());
    m.init(&new, &mut provider, &mut files, &c);
    assert_eq!(Arc::strong_count(&old), 1);
    assert_eq!(Arc::strong_count(&new), 2);
    drop(m);
    assert_eq!(Arc::strong_count(&new), 1);
}

#[test]
fn typed_lookup_precedence_duplicates_and_setter_failures() {
    let parsed = parse("rule\nproperty TestFloat 2.5\nproperty TestInt -3\nproperty Collision true\nproperty TestBool false\nproperty TestInt 4\nend").unwrap();
    let mut p = Registry {
        fail_setters: true,
        ..Registry::default()
    };
    parsed.rules[0].properties.apply(&mut p);
    assert_eq!(
        p.trace,
        [
            Write::Bool(1, true),
            Write::Bool(1, false),
            Write::Int(0, -3),
            Write::Int(0, 4),
            Write::Float(0, 2.5)
        ]
    );
    assert_eq!(p.integer, 0);
}

#[test]
fn lexical_literal_subset_comments_symbols_includes_and_optional_user_order() {
    let mut files = Files::default();
    files.text.insert("data/Graphics Rules.sgr".into(), "# heading\r\nset n 7\r\npartialRule \"Name # one\" # comment\r\nproperty TestInt $n\r\nproperty TestBool $prefsFirstTime\r\nend\r\nsinclude \"Video Cards.sgr\"".into());
    files.text.insert(
        "data/Video Cards.sgr".into(),
        "partialRule included\nproperty TestInt 8\nend".into(),
    );
    files.text.insert(
        "user/Graphics Options.sgr".into(),
        "partialRule user\nproperty TestInt 9\nend".into(),
    );
    let parsed = RuleParser::new(&Registry::default(), 0, 0, true)
        .parse_files(&mut files, "data/Graphics Rules.sgr", "user/Graphics Options.sgr")
        .unwrap();
    assert_eq!(
        files.reads,
        [
            "data/Graphics Rules.sgr",
            "data/Video Cards.sgr",
            "user/Graphics Options.sgr"
        ]
    );
    assert_eq!(
        parsed.rules.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["Name # one", "included", "user"]
    );
    assert_eq!(parsed.rules[0].properties.bools, [(1, true)]);
    assert_eq!(parsed.rules[0].properties.ints, [(0, 7)]);
    let false_prefs = RuleParser::new(&Registry::default(), 0, 0, false)
        .parse_text(
            &mut Files::default(),
            "r",
            "rule\nproperty TestBool $prefsFirstTime\nend",
        )
        .unwrap();
    assert_eq!(false_prefs.rules[0].properties.bools, [(1, false)]);
}

#[test]
fn unsupported_or_malformed_grammar_fails_instead_of_inventing_native_semantics() {
    // GR4/GR6: recoverable rejection is an embedding policy, not native parity.
    for (script, expected) in [
        ("rule \"unclosed", "Unclosed quote"),
        ("end", "Unexpected end"),
        ("rule", "Unclosed block"),
        ("rule -mystery\nend", "Bad rule arguments"),
        ("Rule\nend", "Unknown command"),
        (
            "rule\natLeast memory \"sqrt(4)\"\nend",
            "Unsupported GR4 numeric primary/function",
        ),
        ("rule\nhas MMX extra\nend", "Bad argument count"),
        ("rule\nproperty TestBool True\nend", "Bad boolean"),
        ("rule\nproperty TestInt $missing\nend", "Unknown variable"),
        (
            "optionGroup 1\nproperty TestInt 9\nend",
            "Property requires a rule or selected option",
        ),
        (
            "optionGroup 1\noption 0\nend\nproperty TestInt 9",
            "Property requires a rule or selected option",
        ),
        ("optionGroup 4294967295\nend", "Index overflow"),
        ("sinclude missing", "Missing included file"),
    ] {
        assert_eq!(parse(script).unwrap_err().message, expected, "{script}");
    }
    let mut files = Files::default();
    files.text.insert("cycle".into(), "sinclude cycle".into());
    assert_eq!(
        RuleParser::new(&Registry::default(), 0, 0, false)
            .parse_text(&mut files, "cycle", "sinclude cycle")
            .unwrap_err()
            .message,
        "Include cycle"
    );
    for bad in ["", ".", "+", "1e", "1e+", "1 2", "2x"] {
        assert_eq!(real(bad).unwrap_err(), "Bad real number");
    }
    for (good, expected) in [("+.5", 0.5), ("2.", 2.0), ("-1.25e+2", -125.0)] {
        assert_eq!(real(good).unwrap(), expected);
    }
}

#[test]
fn general_six_significant_digits_and_exponent_boundaries() {
    for (value, expected) in [
        (0.0, "0"),
        (-0.0, "-0"),
        (1.25, "1.25"),
        (1234567.0, "1.23457e+06"),
        (0.00001, "1e-05"),
        (0.0001, "0.0001"),
        (999999.0, "999999"),
        (1000000.0, "1e+06"),
        (f32::MIN_POSITIVE, "1.17549e-38"),
    ] {
        assert_eq!(general(value).unwrap(), expected, "{value}");
    }
}

#[test]
fn production_render_properties_adapter_fallback_without_foreign_rule_results() {
    use crate::render_properties::RenderProperties;
    let p = Arc::new(Mutex::new(RenderProperties::default()));
    let mut m = GraphicsRuleManager::<RenderProperties>::default();
    m.init(
        &p,
        &mut Provider::default(),
        &mut Files::default(),
        &InitContext::from_directories("data/", "user/", false),
    );
    assert!(m.last_init_error.is_none());
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["standard software"]);
    let p = p.lock().unwrap();
    assert_eq!(p.get_bool(p.find_bool("RenderCloudsAndFog")), Some(false));
}

#[test]
fn wildcard_matches_independent_exhaustive_short_byte_language() {
    // Independent table recurrence, not the matcher's greedy star traversal.
    fn reference(v: &[u8], p: &[u8]) -> bool {
        let mut d = vec![vec![false; p.len() + 1]; v.len() + 1];
        d[0][0] = true;
        for j in 1..=p.len() {
            d[0][j] = d[0][j - 1] && p[j - 1] == b'*';
        }
        for i in 1..=v.len() {
            for j in 1..=p.len() {
                d[i][j] = if p[j - 1] == b'*' {
                    d[i][j - 1] || d[i - 1][j]
                } else {
                    d[i - 1][j - 1] && (p[j - 1] == b'?' || p[j - 1] == v[i - 1])
                };
            }
        }
        d[v.len()][p.len()]
    }
    fn words(alphabet: &[u8], depth: usize) -> Vec<String> {
        let mut result = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..depth {
            level = level
                .iter()
                .flat_map(|p| alphabet.iter().map(move |&b| format!("{p}{}", char::from(b))))
                .collect();
            result.extend(level.iter().cloned());
        }
        result
    }
    let values = words(b"ab*?", 3);
    let patterns = words(b"ab*?", 4);
    for v in &values {
        for p in &patterns {
            assert_eq!(
                wildcard_match(v, p, &AsciiLowercase).unwrap(),
                reference(v.as_bytes(), p.as_bytes()),
                "{v:?} / {p:?}"
            );
        }
    }
}

#[test]
fn acquired_file_failures_respect_registration_bits() {
    let registry = Registry::default();
    let mut files = Files::default();
    files.text.insert("r".into(), "rule primary\nend".into());
    let result = RuleParser::new(&registry, 0, 0, false)
        .parse_files(&mut files, "r", "o")
        .unwrap();
    assert_eq!(result.rules[0].name, "primary");
    files.fail.insert("o".into());
    let result = RuleParser::new(&registry, 0, 0, false)
        .parse_files(&mut files, "r", "o")
        .unwrap();
    let e = &result.diagnostics[0];
    assert_eq!((e.path.as_str(), e.message.as_str()), ("o", "Read failed"));
    files.fail.clear();
    files.text.remove("r");
    assert_eq!(
        RuleParser::new(&registry, 0, 0, false)
            .parse_files(&mut files, "r", "o")
            .unwrap_err()
            .message,
        "Missing rules file"
    );
}

#[test]
fn conditions_after_child_target_parent_and_recommendations_persist_when_nothing_matches() {
    let (mut m, p, _, _) =
        fixture("partialRule parent\npartialRule child\nproperty TestInt 8\nend\nhas FPU\noption 2 3\nend");
    assert_eq!(m.rules()[0].flag_mask, 16);
    assert_eq!(m.rules()[1].flag_mask, 0);
    assert!(m.apply_rules(&AsciiLowercase).unwrap().is_empty());
    assert!(trace(&p).is_empty());
    m.environment.flags = 16;
    assert_eq!(m.apply_rules(&AsciiLowercase).unwrap(), ["parent", "child"]);
    assert_eq!(m.get_option_group_auto_setting(2), 3);
    m.environment.flags = 0;
    assert!(m.apply_rules(&AsciiLowercase).unwrap().is_empty());
    assert_eq!(m.get_option_group_auto_setting(2), 3);
    assert_eq!(trace(&p), [Write::Int(0, 8)]);
}

#[test]
fn parse_success_replaces_both_lists_and_uint32_recommendations_keep_bits() {
    let (mut m, _, _, mut files) = fixture("optionGroup 7\noption 3\nend\nrule old\nend");
    files.text.insert(
        "data/Graphics Rules.sgr".into(),
        "partialRule new\noption 3 4294967295\nend".into(),
    );
    assert!(m.parse_config_file(&mut files, &InitContext::from_directories("data/", "user/", false)));
    assert_eq!(m.rules()[0].name, "new");
    assert!(m.groups().is_empty());
    m.set_options_from_configuration(
        &LogContext {
            preferences: Some([0; 12]),
            ..LogContext::default()
        },
        &AsciiLowercase,
    )
    .unwrap();
    assert_eq!(m.get_option_group_auto_setting(3), u32::MAX);
    assert!(m
        .logger()
        .text
        .lines()
        .any(|s| s.starts_with("Texture Quality") && s.ends_with("recommended:  -1")));
}

#[test]
fn permissive_ids_and_explicit_unsafe_index_embedding_boundary() {
    assert_eq!(card_ids("xxxxxxxxGHIJxxxxxxxxxx0000x").unwrap(), (0, 0));
    assert_eq!(card_ids("xxxxxxxx+234xxxxxxxxxx0000x").unwrap(), (0, 0));
    let (mut m, _, _, _) = fixture("partialRule\noption 4294967295 0\nend");
    assert_eq!(
        m.apply_rules(&AsciiLowercase),
        Err(ManagerError::Allocation("Index overflow".into()))
    );
}
