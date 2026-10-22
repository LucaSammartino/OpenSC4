// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[derive(Default)]
struct Host {
    calls: Vec<String>,
    properties: bool,
    file_available: bool,
    exemplar_available: bool,
    zone: i32,
}

impl Resources for Host {
    fn register_reload(&mut self, arg: u32) {
        assert_eq!(arg, 0);
        self.calls.push("register".into());
    }
    fn reload_rules(&mut self) {
        self.calls.push("rules".into());
    }
    fn base_init(&mut self) -> bool {
        self.calls.push("init".into());
        false
    }
    fn base_shutdown(&mut self) -> bool {
        self.calls.push("shutdown".into());
        false
    }
    fn exemplar(&mut self, tgi: (u32, u32, u32), iid: u32) -> Option<Reference> {
        assert_eq!((tgi, iid), (PARAM_EXEMPLAR, 0xA521_60F5));
        self.calls.push("exemplar".into());
        self.exemplar_available.then_some(Reference(1))
    }
    fn uint_property(&mut self, _: Reference, id: u32) -> Option<u32> {
        self.calls.push(format!("{id:08X}"));
        self.properties.then_some(if id == PROP_3A4 { 10 } else { 15 })
    }
    fn float_property(&mut self, _: Reference, id: u32) -> Option<f32> {
        self.calls.push(format!("{id:08X}"));
        self.properties.then_some(if id == PROP_3A8 { 8.5 } else { 2.0 })
    }
    fn release_resource(&mut self, r: Reference) {
        self.calls.push(format!("release{}", r.0));
    }
    fn file(&mut self, tgi: (u32, u32, u32), arg: u32) -> Option<Reference> {
        assert_eq!((tgi, arg), ((0, 0x8A59_71C5, 0x8A59_93B9), 0));
        self.calls.push("file".into());
        self.file_available.then_some(Reference(2))
    }
    fn open_ini(&mut self, _: Reference) -> Option<Reference> {
        self.calls.push("open".into());
        Some(Reference(3))
    }
    fn enumerate_ini(&mut self, _: Reference, section: &str) -> Vec<(u32, u32)> {
        assert_eq!(section, "PowerPoles");
        self.calls.push("enumerate".into());
        vec![(7, 90), (7, 91)]
    }
    fn destroy_ini(&mut self, _: Reference) {
        self.calls.push("destroy".into());
    }
}
impl FailureText for Host {
    fn base_reason(&mut self, reason: u32, output: &mut String) {
        self.calls.push(format!("reason{reason}"));
        *output = "base".into();
    }
    fn zone_at(&mut self, cell: (i32, i32)) -> i32 {
        self.calls.push(format!("zone{},{}", cell.0, cell.1));
        self.zone
    }
    fn load_string(&mut self, g: u32, i: u32, output: &mut String, a: u32) {
        assert_eq!((g, i, a), (0x2A59_2FD1, 0x8AAD_D2B5, 0));
        self.calls.push("string".into());
        *output = "special".into();
    }
}

#[test]
fn pl_l1_l2_l3_defaults_reinit_and_once_only_resource_failure() {
    let mut h = Host::default();
    let mut globals = Globals::default();
    let mut tool = PowerLineTool::default();
    assert!(init(&mut tool, &mut globals, &mut h));
    let first = h.calls.clone();
    assert!(init(&mut tool, &mut globals, &mut h));
    assert_eq!(h.calls, first);
    assert_eq!(first, ["register", "rules", "file", "init", "exemplar"]);
    assert_eq!(tool.params, PowerLineParams::default());
    tool.params.cost_per_pole = 88;
    tool.params.cost_per_wire = 3.25;
    tool.params.v_3a4 = 4;
    tool.params.v_3a8 = 2.0;
    assert!(!shutdown(&mut tool, &mut h));
    assert!(!tool.initialized);
    h.exemplar_available = true;
    h.calls.clear();
    assert!(init(&mut tool, &mut globals, &mut h));
    assert_eq!(
        h.calls,
        ["init", "exemplar", "098B25C8", "098B27D0", "4A5F5953", "0A5F596B", "release1"]
    );
    assert_eq!(
        (
            tool.params.v_3a4,
            tool.params.v_3a8,
            tool.params.cost_per_pole,
            tool.params.cost_per_wire
        ),
        (20, 7.0, 88, 3.25)
    );
    h.calls.clear();
    h.file_available = true;
    let mut second = PowerLineTool::default();
    init(&mut second, &mut globals, &mut h);
    assert!(!h.calls.contains(&"file".to_string()));
}

#[test]
fn resource_enumeration_destroys_ini_before_releasing_file_and_last_key_wins() {
    let mut h = Host {
        file_available: true,
        ..Host::default()
    };
    let mut globals = Globals::default();
    read_power_pole_infos(&mut globals, &mut h);
    assert_eq!(h.calls, ["file", "open", "enumerate", "destroy", "release2"]);
    assert_eq!(globals.pole_exemplars[&7], 91);
}

#[test]
fn pl_f1_f2_f3_only_last_anchor_and_no_zero_reason_fallback() {
    let mut h = Host::default();
    let mut tool = PowerLineTool::default();
    let mut output = "prefilled".to_string();
    failure_reason(&tool, &mut h, 0, &mut output);
    assert_eq!(output, "prefilled");
    assert!(h.calls.is_empty());
    tool.anchors = vec![(1, 2), (3, 4)];
    failure_reason(&tool, &mut h, 0, &mut output);
    assert_eq!(output, "prefilled");
    assert_eq!(h.calls, ["zone3,4"]);
    h.calls.clear();
    h.zone = 1;
    failure_reason(&tool, &mut h, 0, &mut output);
    assert_eq!(h.calls, ["zone3,4", "string"]);
    assert_eq!(output, "special");
    h.calls.clear();
    failure_reason(&tool, &mut h, 7, &mut output);
    assert_eq!(h.calls, ["reason7"]);
    assert_eq!(output, "base");
}
