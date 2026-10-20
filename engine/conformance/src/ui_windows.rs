// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/ui-windows.md` §11: the window object,
//! `cSC4View3DWin`, its catalog menus and the dialog close path. Every op name
//! starts with `uiw_`. See README.md ("UI window ops").
//!
//! One vector per conformance test: `ui11_NN_*.json` where `NN` is the test
//! number of §11. Tests 20, 23 and 24 are marked **[game]** in the spec — they
//! need the original; their vectors pin the replacement-side behaviour that
//! makes the observation true and say so in their `description`.

use std::cell::RefCell;
use std::rc::Rc;

use fc4_core::ui_windows::catalog::{
    close_secondary_menu, close_tertiary_menu, invoke_secondary_menu, invoke_tertiary_menu, CatalogHost, InvokeResult,
    SecondaryInvoke, TertiaryInvoke,
};
use fc4_core::ui_windows::dialog::{advisor_popup_instance, popup_win_proc, AutoGamePause, HiddenPause};
use fc4_core::ui_windows::message::{deliver, GzMessage, Handled, MsgKind};
use fc4_core::ui_windows::slots::{
    as_igzwin, RetChannel, VIEW3D_OFF_CATALOG_STATE, VIEW3D_OFF_WIN, VIEW3D_SLOTS, VIEW3D_SLOT_COUNT, VIEW3D_VTABLE,
    WIN_GEN_VTABLE,
};
use fc4_core::ui_windows::view3d::{CatalogState, View3DSave, View3DWin};
use fc4_core::ui_windows::win::{WinOp, WinStore, WinToken};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, check_expect, check_state, i64_of, ji, opt, u32_of, EventLog, R};

/// The object address the view is built at in every vector.
const THIS_ADDR: u32 = 0x2000;
/// A pre-seeded secondary catalog menu's token.
const TOKEN_SECONDARY: WinToken = 0x0020;
/// A pre-seeded tertiary catalog menu's token.
const TOKEN_TERTIARY: WinToken = 0x0030;

#[derive(Default)]
struct World {
    events: EventLog,
    /// Whether the script factory answers (a false models a missing
    /// `cGZUIScriptService`, §5.1 / quirk 8).
    create_ok: RefCell<bool>,
    /// Whether `QueryInterface(0x8A2C_AD8B)` answers (§4.1).
    catalog_iface_ok: RefCell<bool>,
    slot17: RefCell<u32>,
    slot9: RefCell<u32>,
    /// Tokens the factory hands out.
    minted: RefCell<WinToken>,
    /// The last `AlignWindowToWindow` call (conformance test 19). On the
    /// world, not the scene: the host holds `&World` while `with_scene` has
    /// the scene mutably borrowed.
    last_align: RefCell<Option<(u32, u32, u32, u32)>>,
    view: RefCell<Option<Rc<RefCell<Scene>>>>,
}

/// The whole scene: the view, the window store and the pause counter.
struct Scene {
    view: View3DWin,
    store: WinStore,
    pause: HiddenPause,
    /// Whether the view has been painted (§7, conformance test 24).
    painted: RefCell<bool>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World::default());
        *w.create_ok.borrow_mut() = true;
        *w.catalog_iface_ok.borrow_mut() = true;
        *w.slot17.borrow_mut() = 17;
        *w.slot9.borrow_mut() = 9;
        *w.minted.borrow_mut() = 0x9000;
        w
    }

    fn scene(&self) -> R<Rc<RefCell<Scene>>> {
        self.view.borrow().clone().ok_or_else(|| "no view (uiw_create)".into())
    }
}

/// The host the catalog path calls back into.
struct Host<'a> {
    w: &'a World,
}

impl CatalogHost for Host<'_> {
    fn create_window_from_script(&mut self, group: u32, parent: WinToken, instance: u32) -> Option<WinToken> {
        self.w.events.push(
            "create_window_from_script",
            vec![
                ("group", ji(group)),
                ("parent", ji(parent as i64)),
                ("instance", ji(instance)),
            ],
        );
        if !*self.w.create_ok.borrow() {
            return None;
        }
        let mut m = self.w.minted.borrow_mut();
        *m += 1;
        Some(*m)
    }

    fn query_catalog_iface(&mut self, win: WinToken, iid: u32) -> Option<u64> {
        self.w
            .events
            .push("query_catalog_iface", vec![("win", ji(win as i64)), ("iid", ji(iid))]);
        if *self.w.catalog_iface_ok.borrow() {
            Some(0x00C0)
        } else {
            None
        }
    }

    fn catalog_iface_slot17(&mut self, _iface: u64) -> u32 {
        self.w.events.push("catalog_slot17", vec![]);
        *self.w.slot17.borrow()
    }

    fn catalog_iface_slot9(&mut self, _iface: u64) -> u32 {
        self.w.events.push("catalog_slot9", vec![]);
        *self.w.slot9.borrow()
    }

    fn catalog_iface_release(&mut self, iface: u64) {
        self.w
            .events
            .push("catalog_iface_release", vec![("iface", ji(iface as i64))]);
    }

    fn set_win_gen_flag(&mut self, win: WinToken, flag: u32, value: bool) {
        self.w.events.push(
            "set_win_gen_flag",
            vec![
                ("win", ji(win as i64)),
                ("flag", ji(flag)),
                ("value", Json::Bool(value)),
            ],
        );
    }

    fn align_window_to_window(&mut self, win_id: u32, anchor: u32, target: u32, flags: u32) {
        self.w.events.push(
            "align_window_to_window",
            vec![
                ("win", ji(win_id)),
                ("anchor", ji(anchor)),
                ("target", ji(target)),
                ("flags", ji(flags)),
            ],
        );
        *self.w.last_align.borrow_mut() = Some((win_id, anchor, target, flags));
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "ui_windows", World::new)
}

/// One [`WinOp`] as an event, so a vector can pin teardown **order**.
fn op_event(op: &WinOp) -> (&'static str, Vec<(&'static str, Json)>) {
    let t = |x: WinToken| ji(x as i64);
    match op {
        WinOp::ChildAdd { parent, child, ok } => (
            "child_add",
            vec![("parent", t(*parent)), ("child", t(*child)), ("ok", Json::Bool(*ok))],
        ),
        WinOp::ChildRemove { parent, child, ok } => (
            "child_remove",
            vec![("parent", t(*parent)), ("child", t(*child)), ("ok", Json::Bool(*ok))],
        ),
        WinOp::ChildDelete { parent, child } => ("child_delete", vec![("parent", t(*parent)), ("child", t(*child))]),
        WinOp::DestroyWindow(w) => ("destroy_window", vec![("win", t(*w))]),
        WinOp::HideWindow(w) => ("hide_window", vec![("win", t(*w))]),
        WinOp::ShowWindow(w) => ("show_window", vec![("win", t(*w))]),
        WinOp::PullToFront(w) => ("pull_to_front", vec![("win", t(*w))]),
        WinOp::AddMessageFilter { win, offset } => {
            ("add_message_filter", vec![("win", t(*win)), ("offset", ji(*offset))])
        }
        WinOp::RemoveMessageFilter { win, offset } => {
            ("remove_message_filter", vec![("win", t(*win)), ("offset", ji(*offset))])
        }
        WinOp::RemoveAllViewInputControls(f) => ("remove_all_view_input_controls", vec![("force", ji(*f))]),
        WinOp::CreateWindow {
            parent,
            group,
            instance,
        } => (
            "create_window",
            vec![
                ("parent", t(*parent)),
                ("group", ji(*group)),
                ("instance", ji(*instance)),
            ],
        ),
        WinOp::WindowCreated { token, ok } => ("window_created", vec![("win", t(*token)), ("ok", Json::Bool(*ok))]),
        WinOp::AlignWindowToWindow {
            win,
            anchor,
            target,
            flags,
        } => (
            "align_window_to_window",
            vec![
                ("win", ji(*win)),
                ("anchor", ji(*anchor)),
                ("target", ji(*target)),
                ("flags", ji(*flags)),
            ],
        ),
        WinOp::SetWinGenFlag { flag, value } => (
            "set_win_gen_flag",
            vec![("flag", ji(*flag)), ("value", Json::Bool(*value))],
        ),
        WinOp::QueryInterface { win, iid, ok } => (
            "query_interface",
            vec![("win", t(*win)), ("iid", ji(*iid)), ("ok", Json::Bool(*ok))],
        ),
        WinOp::Release(w) => ("release", vec![("win", t(*w))]),
        WinOp::DoMessage { win, msg } => ("do_message", vec![("win", t(*win)), ("msg", ji(*msg))]),
        WinOp::DoWinProcMessage { win, msg } => ("do_win_proc_message", vec![("win", t(*win)), ("msg", ji(*msg))]),
        WinOp::GZOnCommand { win, command, data } => (
            "gz_on_command",
            vec![("win", t(*win)), ("command", ji(*command)), ("data", ji(*data))],
        ),
        WinOp::IsWindowValid(w) => ("is_window_valid", vec![("win", t(*w))]),
        WinOp::CatalogStateWrite { offset, value } => (
            "catalog_state_write",
            vec![("offset", ji(*offset)), ("value", ji(*value))],
        ),
        WinOp::GetID(w) => ("get_id", vec![("win", t(*w))]),
        WinOp::QueryWinGen(w) => ("query_win_gen", vec![("win", t(*w))]),
    }
}

/// The `expect_ops` comparison after an optional `only` kind filter.
fn ops_of(s: &Scene, op: &Json) -> R<Vec<Json>> {
    let only: Vec<String> = match op.get("only") {
        Some(Json::Array(a)) => a
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "'only' must be strings".to_string())
            })
            .collect::<R<Vec<_>>>()?,
        Some(_) => return Err("'only' must be an array".into()),
        None => Vec::new(),
    };
    Ok(s.store
        .ops()
        .iter()
        .filter_map(|o| {
            let (kind, fields) = op_event(o);
            if !only.is_empty() && !only.iter().any(|k| k == kind) {
                return None;
            }
            let mut kv = vec![("event".to_owned(), Json::Str(kind.to_owned()))];
            kv.extend(fields.into_iter().map(|(k, v)| (k.to_owned(), v)));
            Some(Json::Object(kv))
        })
        .collect())
}

fn ret_name(r: RetChannel) -> &'static str {
    match r {
        RetChannel::Al => "al",
        RetChannel::Eax => "eax",
        RetChannel::EaxOrVoid => "eax-or-void",
        RetChannel::None => "none",
        RetChannel::X87 => "x87",
    }
}

/// The state a vector can assert (`uiw_expect`).
fn state_of(w: &World) -> R<Vec<(String, Json)>> {
    let scene = w.scene()?;
    let s = scene.borrow();
    let v = &s.view;
    let cs = v.catalog_state();
    // Read through the RefCell into a local first: the temporary `Ref` must
    // not outlive the statement that builds the result.
    let painted = *s.painted.borrow();
    Ok(vec![
        ("this_addr".into(), ji(v.this_addr())),
        ("win".into(), ji(v.as_igzwin() as i64)),
        ("as_igzwin".into(), ji(u64::from(as_igzwin(v.this_addr())) as i64)),
        ("win_offset".into(), ji(VIEW3D_OFF_WIN)),
        ("secondary_win".into(), ji(v.secondary_win().unwrap_or(0) as i64)),
        ("tertiary_win".into(), ji(v.tertiary_win().unwrap_or(0) as i64)),
        ("secondary_kind".into(), ji(v.secondary_kind())),
        ("tertiary_kind".into(), ji(v.tertiary_kind())),
        ("catalog_state_ptr".into(), ji(v.catalog_state_ptr())),
        ("catalog_field0".into(), ji(cs.map(|c| c.field0).unwrap_or(0))),
        ("catalog_field1".into(), ji(cs.map(|c| c.field1).unwrap_or(0))),
        ("view_manager".into(), ji(v.view_manager().unwrap_or(0) as i64)),
        ("input_control".into(), ji(v.input_control().unwrap_or(0))),
        ("refs".into(), ji(v.refs())),
        ("hidden_pause_count".into(), ji(s.pause.count())),
        ("hidden_paused".into(), Json::Bool(s.pause.is_hidden_paused())),
        ("paused".into(), Json::Bool(s.pause.is_paused())),
        ("painted".into(), Json::Bool(painted)),
    ])
}

fn with_scene<T>(w: &World, f: impl FnOnce(&World, &mut Scene, &mut Host) -> R<T>) -> R<T> {
    let scene = w.scene()?;
    let mut s = scene.borrow_mut();
    let mut host = Host { w };
    f(w, &mut s, &mut host)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("uiw_")?;
    let w = world(host);
    Some(match rest {
        // ---- world ------------------------------------------------------
        "env" => (|| {
            if let Some(v) = opt(op, "create_ok") {
                *w.create_ok.borrow_mut() = scalar::bool_of(v, "create_ok")?;
            }
            if let Some(v) = opt(op, "catalog_iface_ok") {
                *w.catalog_iface_ok.borrow_mut() = scalar::bool_of(v, "catalog_iface_ok")?;
            }
            if let Some(v) = opt(op, "slot17") {
                *w.slot17.borrow_mut() = u32_of(v, "slot17")?;
            }
            if let Some(v) = opt(op, "slot9") {
                *w.slot9.borrow_mut() = u32_of(v, "slot9")?;
            }
            Ok(())
        })(),

        // ---- lifecycle --------------------------------------------------
        "create" => {
            let mut store = WinStore::new();
            let win = u64::from(as_igzwin(THIS_ADDR));
            store.adopt(win, 0, 0);
            let scene = Scene {
                view: View3DWin::new(THIS_ADDR, win),
                store,
                pause: HiddenPause::new(),
                painted: RefCell::new(false),
            };
            *w.view.borrow_mut() = Some(Rc::new(RefCell::new(scene)));
            w.events.push("create", vec![("this_addr", ji(THIS_ADDR))]);
            Ok(())
        }
        "expect" => (|| {
            let got = state_of(&w)?;
            check_state(&got, field(op, "state")?)
        })(),

        // ---- layout and slots (conformance tests 1, 2, 3) ---------------
        "layout" => (|| {
            let want = field(op, "state")?.as_object().ok_or("'state' must be an object")?;
            let known = [
                ("as_igzwin_null", Json::Int(0)),
                ("as_igzwin", ji(u64::from(as_igzwin(THIS_ADDR)) as i64)),
                ("win_offset", ji(VIEW3D_OFF_WIN)),
                ("vtable_primary", ji(VIEW3D_VTABLE)),
                ("vtable_window", ji(WIN_GEN_VTABLE)),
                ("slot_count", ji(VIEW3D_SLOT_COUNT)),
                ("catalog_state_offset", ji(VIEW3D_OFF_CATALOG_STATE)),
            ];
            let mut got: Vec<(String, Json)> = known.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect();
            for n in 0..VIEW3D_SLOT_COUNT {
                let s = &VIEW3D_SLOTS[n as usize];
                got.push((format!("slot{n}_ret"), Json::Str(ret_name(s.ret).to_string())));
                got.push((format!("slot{n}_produces_value"), Json::Bool(s.ret.produces_value())));
            }
            let mut errors = Vec::new();
            for (k, v) in want {
                match got.iter().find(|(gk, _)| gk == k) {
                    None => errors.push(format!("unknown layout field '{k}'")),
                    // `matches` coerces a hex string to the integer, as every
                    // other expectation does.
                    Some((_, g)) if !scalar::matches(g, v, false) => {
                        errors.push(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(v)))
                    }
                    _ => {}
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        })(),

        // ---- the catalog menus (tests 4-19) -----------------------------
        "seed_secondary" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let kind = u32_of(field(op, "kind")?, "kind")?;
            let parent = s.view.as_igzwin();
            let tok = TOKEN_SECONDARY;
            s.store.adopt(tok, 0x0020, parent);
            s.view.set_secondary(Some(tok), kind);
            w.events
                .push("seed_secondary", vec![("win", ji(tok as i64)), ("kind", ji(kind))]);
            Ok(())
        })(),
        "seed_tertiary" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let kind = u32_of(field(op, "kind")?, "kind")?;
            let parent = s.view.as_igzwin();
            let tok = TOKEN_TERTIARY;
            s.store.adopt(tok, 0x0030, parent);
            s.view.set_tertiary(Some(tok), kind);
            w.events
                .push("seed_tertiary", vec![("win", ji(tok as i64)), ("kind", ji(kind))]);
            Ok(())
        })(),
        "set_catalog_state" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let present = opt(op, "present")
                .map(|v| scalar::bool_of(v, "present"))
                .transpose()?
                .unwrap_or(true);
            let f0 = opt(op, "field0").map(|v| u32_of(v, "field0")).transpose()?.unwrap_or(0);
            let f1 = opt(op, "field1").map(|v| u32_of(v, "field1")).transpose()?.unwrap_or(0);
            s.view
                .set_catalog_state(present.then_some(CatalogState { field0: f0, field1: f1 }));
            Ok(())
        })(),
        "set_tool" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let tool = u32_of(field(op, "tool")?, "tool")?;
            s.view.set_input_control(tool);
            w.events.push("set_tool", vec![("tool", ji(tool))]);
            Ok(())
        })(),
        "clear_ops" => (|| {
            let scene = w.scene()?;
            scene.borrow_mut().store.clear_ops();
            w.events.clear();
            Ok(())
        })(),
        "close_secondary" => with_scene(&w, |_, s, h| {
            close_secondary_menu(h, &mut s.view, &mut s.store);
            Ok(())
        }),
        "close_tertiary" => with_scene(&w, |_, s, h| {
            close_tertiary_menu(h, &mut s.view, &mut s.store);
            Ok(())
        }),
        "invoke_secondary" => with_scene(&w, |_, s, h| {
            let args = SecondaryInvoke {
                instance: u32_of(field(op, "instance")?, "instance")?,
                kind: u32_of(field(op, "kind")?, "kind")?,
                caller_data: opt(op, "caller_data")
                    .map(|v| u32_of(v, "caller_data"))
                    .transpose()?
                    .unwrap_or(0),
                target_win_id: opt(op, "target_win_id")
                    .map(|v| u32_of(v, "target_win_id"))
                    .transpose()?
                    .unwrap_or(1),
            };
            let r = invoke_secondary_menu(h, &mut s.view, &mut s.store, args);
            check_expect(op, Json::Str(invoke_name(r).into()))
        }),
        "invoke_tertiary" => with_scene(&w, |_, s, h| {
            let args = TertiaryInvoke {
                instance: u32_of(field(op, "instance")?, "instance")?,
                kind: u32_of(field(op, "kind")?, "kind")?,
                caller_data: opt(op, "caller_data")
                    .map(|v| u32_of(v, "caller_data"))
                    .transpose()?
                    .unwrap_or(0),
                target_win_id: opt(op, "target_win_id")
                    .map(|v| u32_of(v, "target_win_id"))
                    .transpose()?
                    .unwrap_or(1),
                // UIW-2: arg5 is a small negative integer the secondary form
                // lacks. Recorded, not interpreted.
                margin: opt(op, "margin").map(|v| i64_of(v, "margin")).transpose()?.unwrap_or(0) as i32,
            };
            let r = invoke_tertiary_menu(h, &mut s.view, &mut s.store, args);
            check_expect(op, Json::Str(invoke_name(r).into()))
        }),

        // ---- observability (tests 6, 7, 13, 14, 18, 19) -----------------
        "expect_ops" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            let got = ops_of(&s, op)?;
            let want = scalar::arr(field(op, "ops")?, "ops")?;
            if got.len() != want.len() || !got.iter().zip(want).all(|(g, x)| scalar::matches(g, x, true)) {
                return Err(format!(
                    "ops differ:\n  got      {}\n  expected {}",
                    got.iter().map(scalar::show).collect::<Vec<_>>().join("\n           "),
                    want.iter().map(scalar::show).collect::<Vec<_>>().join("\n           ")
                ));
            }
            Ok(())
        })(),
        "expect_ops_contain" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            let want = scalar::arr(field(op, "ops")?, "ops")?;
            let got = ops_of(&s, op)?;
            for x in want {
                if !got.iter().any(|g| scalar::matches(g, x, true)) {
                    return Err(format!(
                        "missing op {}\n  got {}",
                        scalar::show(x),
                        got.iter().map(scalar::show).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            Ok(())
        })(),
        "expect_ops_absent" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            let want = scalar::arr(field(op, "ops")?, "ops")?;
            let got = ops_of(&s, op)?;
            for x in want {
                if let Some(g) = got.iter().find(|g| scalar::matches(g, x, true)) {
                    return Err(format!("unexpected op {}", scalar::show(g)));
                }
            }
            Ok(())
        })(),
        "expect_last_align" => (|| {
            let (win, anchor, target, flags) = w.last_align.borrow().ok_or("no AlignWindowToWindow call")?;
            check_state(
                &[
                    ("win".into(), ji(win)),
                    ("anchor".into(), ji(anchor)),
                    ("target".into(), ji(target)),
                    ("flags".into(), ji(flags)),
                ],
                field(op, "state")?,
            )
        })(),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }

        // ---- messages (tests 21, 22) -----------------------------------
        "deliver" => (|| {
            let scene = w.scene()?;
            let (kind, msg) = message_of(op)?;
            let mut s = scene.borrow_mut();
            let parent = s.view.as_igzwin();
            s.store.clear_ops();
            let stages = deliver(
                &mut s.store,
                parent,
                &msg,
                |_, _| Handled::No,
                |_, _| Handled::No,
                |_, _| Handled::No,
            );
            let _ = kind;
            check_expect(
                op,
                Json::Array(stages.iter().map(|x| Json::Str((*x).to_string())).collect()),
            )
        })(),

        // ---- dialogs (test 20) -----------------------------------------
        "popup" => (|| {
            let flags = opt(op, "advice_flags")
                .map(|v| u32_of(v, "advice_flags"))
                .transpose()?
                .unwrap_or(2);
            let (_kind, msg) = message_of(op)?;
            let action = match popup_win_proc(flags, &msg) {
                fc4_core::ui_windows::dialog::PopupAction::KeepOpen => "keep_open",
                fc4_core::ui_windows::dialog::PopupAction::End => "end",
                fc4_core::ui_windows::dialog::PopupAction::FollowLinkThenEnd => "follow_link_then_end",
                fc4_core::ui_windows::dialog::PopupAction::Swallow => "swallow",
                fc4_core::ui_windows::dialog::PopupAction::Ignored => "ignored",
            };
            check_expect(op, Json::Str(action.into()))
        })(),
        "popup_instance" => (|| {
            let mood = opt(op, "mood").map(|v| i64_of(v, "mood")).transpose()?.unwrap_or(0) as i32;
            check_expect(op, ji(advisor_popup_instance(mood)))
        })(),

        // ---- the hidden pause (test 23) --------------------------------
        "pause_enter" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let is_dialog = opt(op, "is_dialog")
                .map(|v| scalar::bool_of(v, "is_dialog"))
                .transpose()?
                .unwrap_or(true);
            s.pause.enter(AutoGamePause::advisor_popup(), is_dialog);
            Ok(())
        })(),
        "pause_leave" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let is_dialog = opt(op, "is_dialog")
                .map(|v| scalar::bool_of(v, "is_dialog"))
                .transpose()?
                .unwrap_or(true);
            s.pause.leave(AutoGamePause::advisor_popup(), is_dialog);
            Ok(())
        })(),

        // ---- the draw pass (test 24) -----------------------------------
        "paint" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            *s.painted.borrow_mut() = true;
            // The draw pass runs because the city view paints (§7). A view
            // that is never painted never reaches the effects draw, so no
            // effect end message is posted.
            w.events.push("effects_update", vec![]);
            Ok(())
        })(),
        "sweep_effects" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            if *s.painted.borrow() {
                w.events.push("effect_end_message", vec![]);
            }
            Ok(())
        })(),

        // ---- save (test 25) --------------------------------------------
        "save" => (|| {
            let scene = w.scene()?;
            let s = scene.borrow();
            let rec: View3DSave = s.view.do_save();
            let (group, type_id, instance, version) = View3DSave::header();
            check_state(
                &[
                    ("group".into(), ji(group)),
                    ("type_id".into(), ji(type_id)),
                    ("instance".into(), ji(instance)),
                    ("version".into(), ji(version)),
                    ("ref_f0_class".into(), ji(rec.ref_f0.class_id)),
                    ("ref_f0_present".into(), Json::Bool(rec.ref_f0.present)),
                    ("ref_f4_class".into(), ji(rec.ref_f4.class_id)),
                    ("ref_f4_present".into(), Json::Bool(rec.ref_f4.present)),
                    ("carries_ui_state".into(), Json::Bool(rec.carries_ui_state())),
                ],
                field(op, "state")?,
            )
        })(),
        "load" => (|| {
            let scene = w.scene()?;
            let mut s = scene.borrow_mut();
            let (group, type_id, instance, version) = View3DSave::header();
            let rec = View3DSave {
                group: opt(op, "group")
                    .map(|v| u32_of(v, "group"))
                    .transpose()?
                    .unwrap_or(group),
                type_id: opt(op, "type_id")
                    .map(|v| u32_of(v, "type_id"))
                    .transpose()?
                    .unwrap_or(type_id),
                instance: opt(op, "instance")
                    .map(|v| u32_of(v, "instance"))
                    .transpose()?
                    .unwrap_or(instance),
                version: opt(op, "version")
                    .map(|v| u32_of(v, "version"))
                    .transpose()?
                    .unwrap_or(version),
                ref_f0: fc4_core::ui_windows::view3d::SaveRef::null(),
                ref_f4: fc4_core::ui_windows::view3d::SaveRef::null(),
            };
            let ok = s.view.do_load(&rec).is_ok();
            check_expect(op, Json::Bool(ok))
        })(),

        _ => return None,
    })
}

fn invoke_name(r: InvokeResult) -> &'static str {
    match r {
        InvokeResult::Toggled => "toggled",
        InvokeResult::ReplacedThenFailed => "replaced_then_failed",
        InvokeResult::Opened(_) => "opened",
    }
}

/// The `uiw_deliver` / `uiw_popup` message arguments.
fn message_of(op: &Json) -> R<(MsgKind, GzMessage)> {
    let kind = match field(op, "msg")?.as_str().ok_or("'msg' must be a string")? {
        "command" => MsgKind::Command,
        "link" => MsgKind::Link,
        "attach" => MsgKind::AttachNotice,
        "detach" => MsgKind::DetachNotice,
        "modal_begin" => MsgKind::ModalBegin,
        "modal_end" => MsgKind::ModalEnd,
        "modal_end_posted" => MsgKind::ModalEndPosted,
        "key_down" => MsgKind::KeyDown,
        "key_up" => MsgKind::KeyUp,
        other => return Err(format!("unknown message '{other}'")),
    };
    let data1 = opt(op, "data1").map(|v| u32_of(v, "data1")).transpose()?.unwrap_or(0);
    let data2 = opt(op, "data2").map(|v| u32_of(v, "data2")).transpose()?.unwrap_or(0);
    let data3 = opt(op, "data3").map(|v| u32_of(v, "data3")).transpose()?.unwrap_or(0);
    let msg = GzMessage {
        kind,
        msg_type: if kind == MsgKind::Command {
            fc4_core::ui_windows::message::MSG_COMMAND
        } else {
            data1
        },
        data1: if kind == MsgKind::Command {
            fc4_core::ui_windows::message::MSG_COMMAND
        } else {
            data1
        },
        data2,
        data3,
    };
    Ok((kind, msg))
}
