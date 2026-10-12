// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/budget.md`: a fake budget host (property
//! holders, lots, ordinance and neighbour stubs, history, LTEXT) and ops that
//! drive the budget simulator, its departments and line items. See README.md
//! ("Budget ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::{Rc, Weak};

use fc4_core::budget::{
    BudgetHost, BudgetMessage, BudgetSim, BudgetTuning, Department, LineItem, LotInfo, PropertySource,
    CLSID_BUDGET_SIMULATOR, CLSID_DEPARTMENT, CLSID_LINE_ITEM,
};
use fc4_core::store::{crc32_mpeg2, load_store, save_store, LoadRegistry, Package, CRC_LIMIT, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};
use fc4_core::{Message, Simulator};

use crate::foundations::hex_decode;
use crate::json::Json;
use crate::runner::{boolean, check_expect_bool, field, int, opt_int, rel_close, Event, HostRef, HostState};

type R<T> = Result<T, String>;

/// Test-only class ID of the fake property holders.
const HOLDER_CLASS: u32 = 0x0F4C_0001;

#[derive(Clone, Debug)]
enum PropVal {
    U32(Vec<u32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
}

impl PropVal {
    fn as_u32s(&self) -> Vec<u32> {
        match self {
            PropVal::U32(v) => v.clone(),
            PropVal::I64(v) => v.iter().map(|&x| x as u32).collect(),
            PropVal::F32(v) => v.iter().map(|&x| x as u32).collect(),
        }
    }
    fn as_i64s(&self) -> Vec<i64> {
        match self {
            PropVal::U32(v) => v.iter().map(|&x| i64::from(x)).collect(),
            PropVal::I64(v) => v.clone(),
            PropVal::F32(v) => v.iter().map(|&x| x as i64).collect(),
        }
    }
    fn as_f32s(&self) -> Vec<f32> {
        match self {
            PropVal::U32(v) => v.iter().map(|&x| x as f32).collect(),
            PropVal::I64(v) => v.iter().map(|&x| x as f32).collect(),
            PropVal::F32(v) => v.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct Env {
    difficulty: u32,
    year: i32,
    month: i32,
    established: bool,
    size: (f32, f32),
    ordinance_income: i64,
    ordinance_expense: i64,
    neighbor_income: i64,
    neighbor_expense: i64,
    rng_seed: u32,
}

impl Default for Env {
    fn default() -> Self {
        Env {
            difficulty: 2,
            year: 2000,
            month: 1,
            established: true,
            size: (1024.0, 1024.0),
            ordinance_income: 0,
            ordinance_expense: 0,
            neighbor_income: 0,
            neighbor_expense: 0,
            rng_seed: 1,
        }
    }
}

/// The fake world of the budget vectors.
pub(crate) struct World {
    hs: Weak<HostState>,
    me: Weak<World>,
    env: RefCell<Env>,
    labels: RefCell<HashMap<u64, String>>,
    tokens: RefCell<HashMap<String, u64>>,
    props: RefCell<HashMap<u64, BTreeMap<u32, PropVal>>>,
    occupants: RefCell<HashSet<u64>>,
    lots: RefCell<Vec<LotInfo>>,
    disasters: RefCell<HashSet<u32>>,
    ltexts: RefCell<HashMap<(u32, u32), String>>,
    next_token: Cell<u64>,
    counters: RefCell<HashMap<&'static str, u32>>,
    refs: RefCell<HashMap<u64, i64>>,
    budget: RefCell<Option<Rc<BudgetSim>>>,
    saves: RefCell<HashMap<String, SavedCity>>,
}

#[derive(Clone)]
struct SavedCity {
    pkg: Package,
    root: Vec<u8>,
    /// Object ID → host token of the holders referenced by the save.
    holders: HashMap<u32, u64>,
}

impl World {
    fn new(hs: &Rc<HostState>) -> Rc<World> {
        Rc::new_cyclic(|me| World {
            hs: Rc::downgrade(hs),
            me: me.clone(),
            env: RefCell::new(Env::default()),
            labels: RefCell::new(HashMap::new()),
            tokens: RefCell::new(HashMap::new()),
            props: RefCell::new(HashMap::new()),
            occupants: RefCell::new(HashSet::new()),
            lots: RefCell::new(Vec::new()),
            disasters: RefCell::new(HashSet::new()),
            ltexts: RefCell::new(HashMap::new()),
            next_token: Cell::new(0x1000),
            counters: RefCell::new(HashMap::new()),
            refs: RefCell::new(HashMap::new()),
            budget: RefCell::new(None),
            saves: RefCell::new(HashMap::new()),
        })
    }

    fn host(&self) -> Rc<dyn BudgetHost> {
        self.me.upgrade().expect("world alive") as Rc<dyn BudgetHost>
    }

    fn push(&self, e: Event) {
        if let Some(hs) = self.hs.upgrade() {
            hs.events.borrow_mut().push(e);
        }
    }

    fn label(&self, token: u64) -> String {
        if token == 0 {
            return String::new();
        }
        self.labels
            .borrow()
            .get(&token)
            .cloned()
            .unwrap_or_else(|| format!("#{token:x}"))
    }

    fn new_token(&self, label: String) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 0x10);
        self.labels.borrow_mut().insert(t, label.clone());
        self.tokens.borrow_mut().insert(label, t);
        self.refs.borrow_mut().insert(t, 1);
        t
    }

    fn next_label(&self, kind: &'static str) -> String {
        let mut c = self.counters.borrow_mut();
        let n = c.entry(kind).or_insert(0);
        *n += 1;
        format!("{kind}#{n}")
    }

    fn holder(&self, label: &str) -> u64 {
        if let Some(&t) = self.tokens.borrow().get(label) {
            return t;
        }
        self.new_token(label.to_owned())
    }

    fn budget(&self) -> R<Rc<BudgetSim>> {
        self.budget
            .borrow()
            .clone()
            .ok_or_else(|| "no budget: use budget_create first".to_owned())
    }
}

impl BudgetHost for World {
    fn post(&self, m: &BudgetMessage) {
        self.push(Event::BudgetPost {
            msg: Message::new(m.msg_type, m.data1, m.data2, m.data3),
            void1: self.label(m.void1),
            unknown: self.label(m.unknown),
        });
    }
    fn show_budget_window(&self, panel: u32) {
        self.push(Event::ShowBudgetWindow { panel });
    }
    fn difficulty(&self) -> u32 {
        self.env.borrow().difficulty
    }
    fn sim_date(&self) -> (i32, i32) {
        let e = self.env.borrow();
        (e.year, e.month)
    }
    fn city_established(&self) -> bool {
        self.env.borrow().established
    }
    fn city_size(&self) -> (f32, f32) {
        self.env.borrow().size
    }
    fn ordinance_monthly_income(&self) -> i64 {
        self.env.borrow().ordinance_income
    }
    fn ordinance_monthly_expense(&self) -> i64 {
        self.env.borrow().ordinance_expense
    }
    fn neighbor_deal_income(&self) -> i64 {
        self.env.borrow().neighbor_income
    }
    fn neighbor_deal_expense(&self) -> i64 {
        self.env.borrow().neighbor_expense
    }
    fn for_each_lot(&self, f: &mut dyn FnMut(&LotInfo)) {
        let lots = self.lots.borrow().clone();
        for l in &lots {
            f(l);
        }
    }
    fn record_history(&self, series: u32, value: i32) {
        self.push(Event::History { series, value });
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.push(Event::HistoryQueue { series, length });
    }
    fn try_start_disaster(&self, id: u32, x: f32, z: f32) -> bool {
        if !self.disasters.borrow().contains(&id) {
            return false;
        }
        self.push(Event::Disaster { id, x, z });
        true
    }
    fn rng_seed(&self) -> u32 {
        self.env.borrow().rng_seed
    }
    fn ltext(&self, group: u32, instance: u32) -> Vec<u8> {
        match self.ltexts.borrow().get(&(group, instance)) {
            Some(s) => s.clone().into_bytes(),
            None => format!("LT:{group:08X}:{instance:08X}").into_bytes(),
        }
    }
    fn building_type_name(&self, building_type: u32) -> Vec<u8> {
        format!("BT:{building_type:08X}").into_bytes()
    }
    fn get_u32s(&self, holder: u64, prop: u32) -> Option<Vec<u32>> {
        self.props.borrow().get(&holder)?.get(&prop).map(PropVal::as_u32s)
    }
    fn get_i64s(&self, holder: u64, prop: u32) -> Option<Vec<i64>> {
        self.props.borrow().get(&holder)?.get(&prop).map(PropVal::as_i64s)
    }
    fn get_f32s(&self, holder: u64, prop: u32) -> Option<Vec<f32>> {
        self.props.borrow().get(&holder)?.get(&prop).map(PropVal::as_f32s)
    }
    fn set_u32s(&self, holder: u64, prop: u32, values: &[u32]) -> bool {
        self.props
            .borrow_mut()
            .entry(holder)
            .or_default()
            .insert(prop, PropVal::U32(values.to_vec()));
        true
    }
    fn set_i64s(&self, holder: u64, prop: u32, values: &[i64]) -> bool {
        self.props
            .borrow_mut()
            .entry(holder)
            .or_default()
            .insert(prop, PropVal::I64(values.to_vec()));
        true
    }
    fn set_f32s(&self, holder: u64, prop: u32, values: &[f32]) -> bool {
        self.props
            .borrow_mut()
            .entry(holder)
            .or_default()
            .insert(prop, PropVal::F32(values.to_vec()));
        true
    }
    fn copy_property(&self, src: u64, dst: u64, prop: u32) -> bool {
        let v = self.props.borrow().get(&src).and_then(|m| m.get(&prop).cloned());
        match v {
            Some(v) => {
                self.props.borrow_mut().entry(dst).or_default().insert(prop, v);
                true
            }
            None => false,
        }
    }
    fn occupant_of(&self, holder: u64) -> Option<u64> {
        self.occupants.borrow().contains(&holder).then_some(holder)
    }
    fn class_of(&self, _token: u64) -> u32 {
        HOLDER_CLASS
    }
    fn add_ref(&self, token: u64) {
        if token == 0 {
            return;
        }
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
        self.push(Event::AddRef {
            agent: self.label(token),
        });
    }
    fn release(&self, token: u64) {
        if token == 0 {
            return;
        }
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        self.push(Event::Release {
            agent: self.label(token),
        });
    }
    fn create_object(&self, class_id: u32) -> Option<Rc<dyn SaveObject>> {
        let host = self.host();
        match class_id {
            CLSID_DEPARTMENT => {
                let t = self.new_token(self.next_label("department"));
                Some(Department::new(host, t) as Rc<dyn SaveObject>)
            }
            CLSID_LINE_ITEM => {
                let t = self.new_token(self.next_label("line"));
                Some(LineItem::new(host, t) as Rc<dyn SaveObject>)
            }
            _ => None,
        }
    }
}

// ---- tuning from JSON ---------------------------------------------------------------------------------

struct JsonProps(Vec<(u32, Vec<Json>)>);

impl JsonProps {
    fn parse(j: Option<&Json>) -> R<JsonProps> {
        let mut v = Vec::new();
        if let Some(j) = j {
            for (k, val) in j.as_object().ok_or("tuning must be an object of property IDs")? {
                let id = Json::Str(k.clone())
                    .as_i64()
                    .ok_or_else(|| format!("bad property ID '{k}'"))? as u32;
                let items = match val {
                    Json::Array(a) => a.clone(),
                    other => vec![other.clone()],
                };
                v.push((id, items));
            }
        }
        Ok(JsonProps(v))
    }
    fn get(&self, prop: u32) -> Option<&Vec<Json>> {
        self.0.iter().find(|(id, _)| *id == prop).map(|(_, v)| v)
    }
}

impl PropertySource for JsonProps {
    fn get_i64s(&self, prop: u32) -> Option<Vec<i64>> {
        self.get(prop).map(|v| {
            v.iter()
                .filter_map(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
                .collect()
        })
    }
    fn get_f32s(&self, prop: u32) -> Option<Vec<f32>> {
        self.get(prop)
            .map(|v| v.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
    }
}

// ---- values ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Val {
    Null,
    B(bool),
    I(i64),
    F(f64),
    S(String),
    List(Vec<Val>),
}

fn matches(got: &Val, want: &Json) -> bool {
    match (got, want) {
        (Val::Null, Json::Null) => true,
        (Val::B(b), w) => boolean(w) == Some(*b),
        (Val::I(i), w) => w.as_i64() == Some(*i),
        (Val::F(f), w) => w.as_f64().is_some_and(|x| rel_close(*f, x)),
        (Val::S(s), w) => w.as_str() == Some(s.as_str()),
        (Val::List(items), Json::Array(ws)) => {
            items.len() == ws.len() && items.iter().zip(ws).all(|(g, w)| matches(g, w))
        }
        _ => false,
    }
}

fn arg(args: &[Json], i: usize) -> R<&Json> {
    args.get(i).ok_or_else(|| format!("missing argument #{i}"))
}

fn arg_i(args: &[Json], i: usize) -> R<i64> {
    arg(args, i)?
        .as_i64()
        .ok_or_else(|| format!("argument #{i} must be an integer"))
}

fn arg_f(args: &[Json], i: usize) -> R<f32> {
    arg(args, i)?
        .as_f64()
        .map(|f| f as f32)
        .ok_or_else(|| format!("argument #{i} must be a number"))
}

fn arg_b(args: &[Json], i: usize) -> R<bool> {
    boolean(arg(args, i)?).ok_or_else(|| format!("argument #{i} must be a boolean"))
}

fn arg_s(args: &[Json], i: usize) -> R<String> {
    arg(args, i)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("argument #{i} must be a string"))
}

fn check(op: &Json, got: Val) -> R<()> {
    if let Some(w) = op.get("expect") {
        if !matches(&got, w) {
            return Err(format!("returned {got:?}, expected {w:?}"));
        }
    }
    // Range expectations for integer results (e.g. float tax totals).
    if let Val::I(v) = got {
        if let Some(lo) = opt_int(op, "expect_min")? {
            if v < lo {
                return Err(format!("returned {v}, expected at least {lo}"));
            }
        }
        if let Some(hi) = opt_int(op, "expect_max")? {
            if v > hi {
                return Err(format!("returned {v}, expected at most {hi}"));
            }
        }
    }
    Ok(())
}

fn string_of(b: Vec<u8>) -> String {
    String::from_utf8_lossy(&b).into_owned()
}

// ---- ops -------------------------------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    let hs = &host.0;
    if let Some(w) = hs.budget.borrow().as_ref() {
        return w.clone();
    }
    let w = World::new(hs);
    *hs.budget.borrow_mut() = Some(w.clone());
    w
}

/// Executes a budget op; `None` if `name` is not one.
pub(crate) fn exec(_sim: &Simulator, host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if !name.starts_with("budget_") {
        return None;
    }
    let w = world(host);
    let r = match name {
        "budget_env" => env(&w, op),
        "budget_create" => {
            let t = w.new_token("budget".to_owned());
            *w.budget.borrow_mut() = Some(BudgetSim::new(w.host(), t));
            Ok(())
        }
        "budget_init" => (|| {
            let props = JsonProps::parse(op.get("tuning"))?;
            let fix = op.get("fix_cs3").and_then(boolean).unwrap_or(false);
            let tuning = if op.get("tuning").is_some() {
                BudgetTuning::from_properties(&props, fix)
            } else {
                BudgetTuning::default()
            };
            check_expect_bool(op, w.budget()?.init(tuning))
        })(),
        "budget_shutdown" => w.budget().and_then(|b| check_expect_bool(op, b.shutdown())),
        "budget_message" => (|| {
            let b = w.budget()?;
            let t = crate::runner::parse_type(field(op, "type")?)?;
            let d1 = opt_int(op, "data1")?.unwrap_or(0);
            check_expect_bool(op, b.handle_message(t, d1))
        })(),
        "budget_cheat" => (|| {
            let b = w.budget()?;
            let id = match field(op, "id")?.as_str() {
                Some("RiskyMoney") => fc4_core::budget::cheat::RISKY_MONEY,
                Some("WeaknessPays") => fc4_core::budget::cheat::WEAKNESS_PAYS,
                Some("Moolah") => fc4_core::budget::cheat::MOOLAH,
                _ => int(op, "id")? as u32,
            };
            let text = op.get("text").and_then(Json::as_str).unwrap_or("");
            check_expect_bool(op, b.handle_cheat(id, text))
        })(),
        "budget_call" => call_budget(&w, op),
        "budget_dept" => call_dept(&w, op),
        "budget_line" => call_line(&w, op),
        "budget_holder" => set_holder(&w, op),
        "budget_expect_holder" => expect_holder(&w, op),
        "budget_lots" => set_lots(&w, op),
        "budget_expect" => expect_state(&w, op),
        "budget_expect_items" => expect_items(&w, op),
        "budget_expect_loans" => expect_loans(&w, op),
        "budget_expect_groups" => expect_groups(&w, op),
        "budget_expect_tuning" => expect_tuning(&w, op),
        "budget_expect_refs" => expect_refs(&w, op),
        "budget_save" => save(&w, op),
        "budget_load" => load(&w, op),
        "budget_expect_save" => expect_save(&w, op),
        "budget_patch" => patch(&w, op),
        other => Err(format!("unknown budget op '{other}'")),
    };
    Some(r)
}

fn env(w: &World, op: &Json) -> R<()> {
    let mut e = w.env.borrow_mut();
    if let Some(v) = opt_int(op, "difficulty")? {
        e.difficulty = v as u32;
    }
    if let Some(v) = opt_int(op, "year")? {
        e.year = v as i32;
    }
    if let Some(v) = opt_int(op, "month")? {
        e.month = v as i32;
    }
    if let Some(v) = op.get("established") {
        e.established = boolean(v).ok_or("established must be a boolean")?;
    }
    if let Some(v) = op.get("city_size").and_then(Json::as_array) {
        e.size = (v[0].as_f64().unwrap_or(0.0) as f32, v[1].as_f64().unwrap_or(0.0) as f32);
    }
    if let Some(v) = opt_int(op, "ordinance_income")? {
        e.ordinance_income = v;
    }
    if let Some(v) = opt_int(op, "ordinance_expense")? {
        e.ordinance_expense = v;
    }
    if let Some(v) = opt_int(op, "neighbor_income")? {
        e.neighbor_income = v;
    }
    if let Some(v) = opt_int(op, "neighbor_expense")? {
        e.neighbor_expense = v;
    }
    if let Some(v) = opt_int(op, "rng_seed")? {
        e.rng_seed = v as u32;
    }
    if op.get("advance_month").and_then(boolean) == Some(true) {
        e.month += 1;
        if e.month > 12 {
            e.month = 1;
            e.year += 1;
        }
    }
    drop(e);
    if let Some(list) = op.get("disasters").and_then(Json::as_array) {
        let mut d = w.disasters.borrow_mut();
        d.clear();
        for x in list {
            d.insert(x.as_i64().ok_or("disaster IDs must be integers")? as u32);
        }
    }
    if let Some(list) = op.get("ltext").and_then(Json::as_array) {
        for x in list {
            let g = int(x, "group")? as u32;
            let i = int(x, "instance")? as u32;
            let t = field(x, "text")?.as_str().ok_or("text must be a string")?.to_owned();
            w.ltexts.borrow_mut().insert((g, i), t);
        }
    }
    Ok(())
}

fn args_of(op: &Json) -> Vec<Json> {
    op.get("args")
        .and_then(Json::as_array)
        .map(<[Json]>::to_vec)
        .unwrap_or_default()
}

fn call_budget(w: &World, op: &Json) -> R<()> {
    let b = w.budget()?;
    let m = field(op, "method")?.as_str().ok_or("method must be a string")?;
    let a = args_of(op);
    let got = match m {
        "set_total_funds" => Val::B(b.set_total_funds(arg_i(&a, 0)?)),
        "deposit_funds" => Val::B(b.deposit_funds(arg_i(&a, 0)?)),
        "withdraw_funds" => Val::B(b.withdraw_funds(arg_i(&a, 0)?)),
        "tamper_funds" => {
            b.tamper_funds(arg_i(&a, 0)?);
            Val::Null
        }
        "funds" => Val::I(b.funds()),
        "shadow_funds" => Val::I(b.shadow_funds()),
        "min_allowable_funds" => Val::I(b.min_allowable_funds()),
        "ytd_income" => Val::I(b.ytd_income()),
        "ytd_expenses" => Val::I(b.ytd_expenses()),
        "est_income" => Val::I(b.est_income()),
        "est_expenses" => Val::I(b.est_expenses()),
        "total_monthly_expense" => Val::I(b.total_monthly_expense()),
        "total_yearly_expense" => Val::I(b.total_yearly_expense()),
        "total_monthly_income" => Val::I(b.total_monthly_income()),
        "total_yearly_income" => Val::I(b.total_yearly_income()),
        "tax_income" => Val::I(b.tax_income()),
        "tax_income_for" => Val::I(b.tax_income_for(arg_i(&a, 0)? as i32)),
        "set_total_monthly_expense" => Val::I(b.set_total_monthly_expense(arg_i(&a, 0)?)),
        "set_total_yearly_expense" => Val::I(b.set_total_yearly_expense(arg_i(&a, 0)?)),
        "set_total_monthly_income" => Val::I(b.set_total_monthly_income(arg_i(&a, 0)?)),
        "set_total_yearly_income" => Val::I(b.set_total_yearly_income(arg_i(&a, 0)?)),
        "show_budget_window" => Val::B(b.show_budget_window()),
        "funding" => Val::I(b.funding()),
        "tax_rate" => Val::F(f64::from(b.tax_rate(arg_i(&a, 0)? as u32))),
        "set_tax_rate" => Val::B(b.set_tax_rate(arg_i(&a, 0)? as u32, arg_f(&a, 1)?)),
        "weighted_assessed_tax_value" => Val::I(b.weighted_assessed_tax_value(arg_i(&a, 0)? as u32)),
        "bond_increment" => Val::I(b.bond_increment()),
        "issue_bond" => Val::B(b.issue_bond(arg_i(&a, 0)? as u32)),
        "total_borrowed" => Val::I(b.total_borrowed()),
        "current_borrowing_limit" => Val::I(b.current_borrowing_limit()),
        "current_bond_limit" => Val::I(b.current_bond_limit()),
        "current_max_outstanding_bonds_limit" => Val::I(b.current_max_outstanding_bonds_limit()),
        "total_monthly_bond_payments" => Val::I(b.total_monthly_bond_payments()),
        "loan_time_in_months" => Val::I(i64::from(b.loan_time_in_months())),
        "monthly_payment_for_loan" => Val::I(b.monthly_payment_for_loan(arg_i(&a, 0)?)),
        "full_cost_of_loan" => Val::I(b.full_cost_of_loan(arg_i(&a, 0)?)),
        "compute_tax" => Val::I(b.compute_tax()),
        "settle_month" => {
            b.settle_month();
            Val::Null
        }
        "simulate_day" => {
            b.simulate_day();
            Val::Null
        }
        "create_department_budget" => match b.create_department_budget(arg_i(&a, 0)? as u32, arg_i(&a, 1)? as u32) {
            Some(d) => Val::I(i64::from(d.id())),
            None => Val::Null,
        },
        "remove_department_budget" => Val::B(b.remove_department_budget(arg_i(&a, 0)? as u32)),
        "department_by_name" => match b.department_by_name(arg_s(&a, 0)?.as_bytes()) {
            Some(d) => Val::I(i64::from(d.id())),
            None => Val::Null,
        },
        "set_group_name" => Val::B(b.set_group_name(arg_i(&a, 0)? as u32, arg_s(&a, 1)?.as_bytes())),
        "departments_in_group" => Val::List(
            b.departments_in_group(arg_i(&a, 0)? as u32)
                .iter()
                .map(|d| Val::I(i64::from(d.id())))
                .collect(),
        ),
        "change_budget_item_line" => {
            Val::B(b.change_budget_item_line(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32, arg_i(&a, 2)? as u32))
        }
        "change_budget_item_cost" => {
            Val::B(b.change_budget_item_cost(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32, arg_i(&a, 2)?))
        }
        "change_budget_item_local_funding" => {
            Val::B(b.change_budget_item_local_funding(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32, arg_f(&a, 2)?))
        }
        "add_budget_item_to_budget" => {
            Val::B(b.add_budget_item_to_budget(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32))
        }
        "remove_budget_item_from_budget" => {
            Val::B(b.remove_budget_item_from_budget(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32))
        }
        "copy_budget_item_properties" => {
            Val::B(b.copy_budget_item_properties(w.holder(&arg_s(&a, 0)?), w.holder(&arg_s(&a, 1)?)))
        }
        other => return Err(format!("unknown budget method '{other}'")),
    };
    check(op, got)
}

fn dept_of(w: &World, op: &Json) -> R<Rc<Department>> {
    let id = int(op, "dept")? as u32;
    w.budget()?
        .department(id)
        .ok_or_else(|| format!("no department 0x{id:08X}"))
}

fn call_dept(w: &World, op: &Json) -> R<()> {
    let d = dept_of(w, op)?;
    let m = field(op, "method")?.as_str().ok_or("method must be a string")?;
    let a = args_of(op);
    let got = match m {
        "id" => Val::I(i64::from(d.id())),
        "name" => Val::S(string_of(d.name())),
        "set_department_name" => Val::B(d.set_department_name(arg_i(&a, 0)? as u32, arg_i(&a, 1)? as u32)),
        "budget_group" => Val::I(i64::from(d.budget_group())),
        "set_budget_group" => Val::B(d.set_budget_group(arg_i(&a, 0)? as u32)),
        "is_fixed_funding" => Val::B(d.is_fixed_funding()),
        "set_fixed_funding" => Val::B(d.set_fixed_funding(arg_b(&a, 0)?)),
        "funding_percentage" => Val::F(f64::from(d.funding_percentage())),
        "set_funding_percentage" => Val::B(d.set_funding_percentage(arg_f(&a, 0)?, arg_i(&a, 1).unwrap_or(0) as u32)),
        "max_funding_percentage" => Val::F(f64::from(d.max_funding_percentage())),
        "set_max_funding_percentage" => Val::B(d.set_max_funding_percentage(arg_f(&a, 0)?)),
        "ideal_monthly_funding" => Val::I(d.ideal_monthly_funding()),
        "total_expenses" => Val::I(d.total_expenses()),
        "total_income" => Val::I(d.total_income()),
        "create_line_item" => match d.create_line_item(arg_i(&a, 0)? as u32, arg_b(&a, 1)?) {
            Some(l) => Val::I(i64::from(l.id())),
            None => Val::Null,
        },
        "create_line_item_for_building_type" => {
            match d.create_line_item_for_building_type(arg_i(&a, 0)? as u32, arg_b(&a, 1)?) {
                Some(l) => Val::I(i64::from(l.id())),
                None => Val::Null,
            }
        }
        "remove_line_item" => Val::B(d.remove_line_item(arg_i(&a, 0)? as u32)),
        "all_line_items" => Val::List(d.all_line_items().iter().map(|l| Val::I(i64::from(l.id()))).collect()),
        "add_locally_funded_object" => {
            Val::B(d.add_locally_funded_object(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32))
        }
        "remove_locally_funded_object" => {
            Val::B(d.remove_locally_funded_object(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32))
        }
        "set_local_funding_percent" => {
            Val::B(d.set_local_funding_percent(w.holder(&arg_s(&a, 0)?), arg_f(&a, 1)?, arg_i(&a, 2)? as u32))
        }
        "set_local_full_funding" => {
            Val::B(d.set_local_full_funding(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)?, arg_i(&a, 2)? as u32))
        }
        "local_funding_percent" => Val::F(f64::from(
            d.local_funding_percent(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32),
        )),
        "local_full_funding" => Val::I(d.local_full_funding(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32)),
        "is_locally_funded_object" => {
            Val::B(d.is_locally_funded_object(w.holder(&arg_s(&a, 0)?), arg_i(&a, 1)? as u32))
        }
        "recalculate_all_line_item_current_expenses" => Val::B(d.recalculate_all_line_item_current_expenses()),
        "total_spending" => Val::I(d.total_spending()),
        "set_total_spending" => Val::B(d.set_total_spending(arg_i(&a, 0)?)),
        "locally_funded_items_by_purpose" => Val::List(
            d.locally_funded_items_by_purpose(arg_i(&a, 0)? as u32)
                .into_iter()
                .map(|h| Val::S(w.label(h)))
                .collect(),
        ),
        "has_locally_funded_items_by_purpose" => Val::B(d.has_locally_funded_items_by_purpose(arg_i(&a, 0)? as u32)),
        other => return Err(format!("unknown department method '{other}'")),
    };
    check(op, got)
}

fn call_line(w: &World, op: &Json) -> R<()> {
    let d = dept_of(w, op)?;
    let id = int(op, "line")? as u32;
    let l = d
        .line_item(id)
        .ok_or_else(|| format!("no line 0x{id:08X} in department 0x{:08X}", d.id()))?;
    let m = field(op, "method")?.as_str().ok_or("method must be a string")?;
    let a = args_of(op);
    let got = match m {
        "id" => Val::I(i64::from(l.id())),
        "item_type" => Val::I(i64::from(l.item_type())),
        "set_type" => Val::B(l.set_type(arg_i(&a, 0)? as u32)),
        "name" => Val::S(string_of(l.name())),
        "set_name" => Val::B(l.set_name(arg_i(&a, 0)? as u32, arg_i(&a, 1)? as u32)),
        "secondary_info" => Val::I(l.secondary_info()),
        "set_secondary_info" => Val::B(l.set_secondary_info(arg_i(&a, 0)?)),
        "full_expenses" => Val::I(l.full_expenses()),
        "current_expenses" => Val::I(l.current_expenses()),
        "set_full_expenses" => Val::B(l.set_full_expenses(arg_i(&a, 0)?)),
        "add_to_full_expenses" => Val::B(l.add_to_full_expenses(arg_i(&a, 0)?)),
        "income" => Val::I(l.income()),
        "set_income" => Val::B(l.set_income(arg_i(&a, 0)?)),
        "add_to_income" => Val::B(l.add_to_income(arg_i(&a, 0)?)),
        "display_flags" => Val::I(i64::from(l.display_flags())),
        "set_display_flag" => Val::B(l.set_display_flag(arg_i(&a, 0)? as u32, arg_b(&a, 1)?)),
        "is_local" => Val::B(l.is_local()),
        other => return Err(format!("unknown line item method '{other}'")),
    };
    check(op, got)
}

fn prop_values(v: &Json) -> R<PropVal> {
    let (k, arr) = v
        .as_object()
        .and_then(|o| o.first())
        .ok_or("property value must be {\"u32\"|\"i64\"|\"f32\": [...]}")?;
    let arr = arr.as_array().ok_or("property values must be an array")?;
    Ok(match k.as_str() {
        "u32" => PropVal::U32(
            arr.iter()
                .map(|x| x.as_i64().map(|v| v as u32).ok_or("u32 value"))
                .collect::<Result<_, _>>()?,
        ),
        "i64" => PropVal::I64(
            arr.iter()
                .map(|x| x.as_i64().ok_or("i64 value"))
                .collect::<Result<_, _>>()?,
        ),
        "f32" => PropVal::F32(
            arr.iter()
                .map(|x| x.as_f64().map(|v| v as f32).ok_or("f32 value"))
                .collect::<Result<_, _>>()?,
        ),
        other => return Err(format!("unknown property type '{other}'")),
    })
}

fn set_holder(w: &World, op: &Json) -> R<()> {
    let h = w.holder(field(op, "holder")?.as_str().ok_or("holder must be a string")?);
    if let Some(props) = op.get("props") {
        let mut map = BTreeMap::new();
        for (k, v) in props.as_object().ok_or("props must be an object")? {
            let id = Json::Str(k.clone())
                .as_i64()
                .ok_or_else(|| format!("bad property ID '{k}'"))? as u32;
            map.insert(id, prop_values(v)?);
        }
        w.props.borrow_mut().insert(h, map);
    }
    if let Some(o) = op.get("occupant") {
        if boolean(o).ok_or("occupant must be a boolean")? {
            w.occupants.borrow_mut().insert(h);
        } else {
            w.occupants.borrow_mut().remove(&h);
        }
    }
    Ok(())
}

fn expect_holder(w: &World, op: &Json) -> R<()> {
    let h = w.holder(field(op, "holder")?.as_str().ok_or("holder must be a string")?);
    let prop = int(op, "prop")? as u32;
    let got = w.props.borrow().get(&h).and_then(|m| m.get(&prop).cloned());
    if op.get("absent").and_then(boolean) == Some(true) {
        return match got {
            None => Ok(()),
            Some(v) => Err(format!("property present: {v:?}")),
        };
    }
    let got = got.ok_or_else(|| format!("property 0x{prop:08X} absent"))?;
    let want = prop_values(field(op, "values")?)?;
    let ok = match (&got, &want) {
        (PropVal::U32(a), PropVal::U32(b)) => a == b,
        (PropVal::I64(a), PropVal::I64(b)) => a == b,
        (PropVal::F32(a), PropVal::F32(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| rel_close(f64::from(*x), f64::from(*y)))
        }
        _ => false,
    };
    if !ok {
        return Err(format!("property 0x{prop:08X} is {got:?}, expected {want:?}"));
    }
    Ok(())
}

fn set_lots(w: &World, op: &Json) -> R<()> {
    let mut lots = Vec::new();
    for l in field(op, "lots")?.as_array().ok_or("lots must be an array")? {
        let get = |k: &str, d: i64| -> R<i64> { Ok(opt_int(l, k)?.unwrap_or(d)) };
        let repeat = get("repeat", 1)?;
        let info = LotInfo {
            has_building: l.get("has_building").and_then(boolean).unwrap_or(true),
            livable: l.get("livable").and_then(boolean).unwrap_or(true),
            purpose: get("purpose", 1)? as u32,
            building_wealth: get("building_wealth", 1)? as u32,
            lot_wealth: get("lot_wealth", 1)? as u32,
            building_value: get("value", 0)?,
            land_value: get("land_value", 0)? as u8,
            size_x: get("size_x", 1)? as u32,
            size_z: get("size_z", 1)? as u32,
        };
        for _ in 0..repeat {
            lots.push(info);
        }
    }
    *w.lots.borrow_mut() = lots;
    Ok(())
}

fn cmp_i(errors: &mut Vec<String>, name: &str, got: i64, v: &Json) {
    match v.as_i64().or_else(|| boolean(v).map(i64::from)) {
        Some(x) if x == got => {}
        _ => errors.push(format!("{name}: got {got}, expected {v:?}")),
    }
}

fn cmp_arr(errors: &mut Vec<String>, name: &str, got: &[i64], v: &Json) {
    match v {
        Json::Array(a) => {
            if a.len() != got.len() || a.iter().zip(got).any(|(w, g)| w.as_i64() != Some(*g)) {
                errors.push(format!("{name}: got {got:?}, expected {v:?}"));
            }
        }
        Json::Object(kv) => {
            for (k, w) in kv {
                let i: usize = k.parse().unwrap_or(usize::MAX);
                if got.get(i).copied() != w.as_i64() {
                    errors.push(format!("{name}[{k}]: got {:?}, expected {w:?}", got.get(i)));
                }
            }
        }
        _ => errors.push(format!("{name}: expected an array or object")),
    }
}

fn expect_state(w: &World, op: &Json) -> R<()> {
    let s = w.budget()?.snapshot();
    let mut errors = Vec::new();
    for (k, v) in field(op, "state")?.as_object().ok_or("state must be an object")? {
        match k.as_str() {
            "initialized" => cmp_i(&mut errors, k, s.initialized.into(), v),
            "loaded_from_save" => cmp_i(&mut errors, k, s.loaded_from_save.into(), v),
            "agriculture_rate_fix" => cmp_i(&mut errors, k, s.agriculture_rate_fix.into(), v),
            "panel_flag" => cmp_i(&mut errors, k, s.panel_flag.into(), v),
            "funds" => cmp_i(&mut errors, k, s.funds, v),
            "shadow_funds" => cmp_i(&mut errors, k, s.shadow_funds, v),
            "total_borrowed" => cmp_i(&mut errors, k, s.total_borrowed, v),
            "last_tax_income" => cmp_i(&mut errors, k, s.last_tax_income, v),
            "ytd_income" => cmp_i(&mut errors, k, s.ytd_income, v),
            "ytd_expense" => cmp_i(&mut errors, k, s.ytd_expense, v),
            "ytd_tax_income" => cmp_i(&mut errors, k, s.ytd_tax_income, v),
            "ytd_reserved" => cmp_arr(&mut errors, k, &s.ytd_reserved, v),
            "ytd_ordinance_income" => cmp_i(&mut errors, k, s.ytd_ordinance_income, v),
            "ytd_ordinance_expense" => cmp_i(&mut errors, k, s.ytd_ordinance_expense, v),
            "ordinance_income_by_month" => cmp_arr(&mut errors, k, &s.ordinance_income_by_month, v),
            "ordinance_expense_by_month" => cmp_arr(&mut errors, k, &s.ordinance_expense_by_month, v),
            "ytd_bond_payments" => cmp_i(&mut errors, k, s.ytd_bond_payments, v),
            "last_month_income" => cmp_i(&mut errors, k, s.last_month_income, v),
            "last_month_expense" => cmp_i(&mut errors, k, s.last_month_expense, v),
            "yearly_income" => cmp_i(&mut errors, k, s.yearly_income, v),
            "yearly_expense" => cmp_i(&mut errors, k, s.yearly_expense, v),
            "bond_count" => cmp_i(&mut errors, k, s.bond_count.into(), v),
            "risky_money_counter" => cmp_i(&mut errors, k, s.risky_money_counter.into(), v),
            "department_ids" => {
                let ids: Vec<i64> = s.department_ids.iter().map(|&x| i64::from(x)).collect();
                cmp_arr(&mut errors, k, &ids, v)
            }
            "tax" => {
                for (gk, gv) in v.as_object().ok_or("tax must be an object keyed by group")? {
                    let g: usize = gk.parse().map_err(|_| format!("bad tax group '{gk}'"))?;
                    let t = s.tax.get(g).ok_or_else(|| format!("tax group {g} out of range"))?;
                    for (fk, fv) in gv.as_object().ok_or("tax group must be an object")? {
                        let name = format!("tax[{g}].{fk}");
                        match fk.as_str() {
                            "rate" | "max_rate" => {
                                let got = f64::from(if fk == "rate" { t.rate } else { t.max_rate });
                                if !fv.as_f64().is_some_and(|x| rel_close(got, x)) {
                                    errors.push(format!("{name}: got {got}, expected {fv:?}"));
                                }
                            }
                            "weighted_assessed" => cmp_i(&mut errors, &name, t.weighted_assessed, fv),
                            "monthly_tax" => cmp_i(&mut errors, &name, t.monthly_tax, fv),
                            "reserved" => cmp_i(&mut errors, &name, t.reserved, fv),
                            "history" => cmp_arr(&mut errors, &name, &t.history, fv),
                            other => return Err(format!("unknown tax field '{other}'")),
                        }
                    }
                }
            }
            "bonds" => {
                let want = v.as_array().ok_or("bonds must be an array")?;
                if want.len() != s.bonds.len() {
                    errors.push(format!("bonds: got {} bond(s), expected {}", s.bonds.len(), want.len()));
                } else {
                    for (i, (b, wv)) in s.bonds.iter().zip(want).enumerate() {
                        for (fk, fv) in wv.as_object().ok_or("bond must be an object")? {
                            let got = match fk.as_str() {
                                "due_month" => i64::from(b.due_month),
                                "due_year" => i64::from(b.due_year),
                                "amount" => b.amount,
                                "full_cost" => b.full_cost,
                                "remaining" => b.remaining,
                                "paid" => b.paid,
                                "monthly_payment" => b.monthly_payment,
                                "principal_left" => b.principal_left,
                                other => return Err(format!("unknown bond field '{other}'")),
                            };
                            cmp_i(&mut errors, &format!("bonds[{i}].{fk}"), got, fv);
                        }
                    }
                }
            }
            other => return Err(format!("unknown budget state field '{other}'")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("budget state mismatch: {}", errors.join("; ")))
    }
}

fn expect_items(w: &World, op: &Json) -> R<()> {
    let b = w.budget()?;
    let h = w.holder(field(op, "holder")?.as_str().ok_or("holder must be a string")?);
    let got = if let Some(purpose) = opt_int(op, "purpose")? {
        b.budget_item_for_purpose(h, purpose as u32).map(|i| vec![i])
    } else {
        b.budget_item_info(h)
    };
    let want = field(op, "items")?;
    match (got, want) {
        (None, Json::Null) => Ok(()),
        (None, _) => Err("no budget items, expected some".into()),
        (Some(items), Json::Null) => Err(format!("got {items:?}, expected none")),
        (Some(items), Json::Array(ws)) => {
            if items.len() != ws.len() {
                return Err(format!("got {} item(s) {items:?}, expected {}", items.len(), ws.len()));
            }
            for (i, (it, wv)) in items.iter().zip(ws).enumerate() {
                for (k, v) in wv.as_object().ok_or("item must be an object")? {
                    let ok = match k.as_str() {
                        "department" => v.as_i64() == Some(i64::from(it.department_id)),
                        "line" => v.as_i64() == Some(i64::from(it.line_item)),
                        "purpose" => v.as_i64() == Some(i64::from(it.purpose)),
                        "cost" => v.as_i64() == Some(it.cost),
                        "index" => v.as_i64() == Some(i64::from(it.index)),
                        "local_funding" => v.as_f64().is_some_and(|x| rel_close(f64::from(it.local_funding), x)),
                        other => return Err(format!("unknown item field '{other}'")),
                    };
                    if !ok {
                        return Err(format!("item #{i}.{k}: got {it:?}, expected {v:?}"));
                    }
                }
            }
            Ok(())
        }
        _ => Err("'items' must be an array or null".into()),
    }
}

fn expect_loans(w: &World, op: &Json) -> R<()> {
    let loans = w.budget()?.all_loans();
    let want = field(op, "loans")?.as_array().ok_or("loans must be an array")?;
    if loans.len() != want.len() {
        return Err(format!(
            "got {} loan(s) {loans:?}, expected {}",
            loans.len(),
            want.len()
        ));
    }
    for (i, (l, wv)) in loans.iter().zip(want).enumerate() {
        for (k, v) in wv.as_object().ok_or("loan must be an object")? {
            let got = match k.as_str() {
                "months_remaining" => i64::from(l.months_remaining),
                "years_remaining" => i64::from(l.years_remaining),
                "total_loan_amount" => l.total_loan_amount,
                "remaining_payment" => l.remaining_payment,
                "monthly_payment" => l.monthly_payment,
                other => return Err(format!("unknown loan field '{other}'")),
            };
            if v.as_i64() != Some(got) {
                return Err(format!("loan #{i}.{k}: got {got}, expected {v:?}"));
            }
        }
    }
    Ok(())
}

fn expect_groups(w: &World, op: &Json) -> R<()> {
    let groups = w.budget()?.all_groups();
    let want = field(op, "groups")?.as_array().ok_or("groups must be an array")?;
    if groups.len() != want.len() {
        return Err(format!(
            "got {} group(s) {groups:?}, expected {}",
            groups.len(),
            want.len()
        ));
    }
    for (i, (g, wv)) in groups.iter().zip(want).enumerate() {
        for (k, v) in wv.as_object().ok_or("group must be an object")? {
            let ok = match k.as_str() {
                "id" => v.as_i64() == Some(i64::from(g.id)),
                "name" => v.as_str() == Some(String::from_utf8_lossy(&g.name).as_ref()),
                "total_expense" => v.as_i64() == Some(g.total_expense),
                "total_income" => v.as_i64() == Some(g.total_income),
                "departments" => {
                    let ids: Vec<i64> = g.departments.iter().map(|&x| i64::from(x)).collect();
                    v.as_array().is_some_and(|a| {
                        a.iter().map(Json::as_i64).collect::<Vec<_>>()
                            == ids.iter().map(|x| Some(*x)).collect::<Vec<_>>()
                    })
                }
                other => return Err(format!("unknown group field '{other}'")),
            };
            if !ok {
                return Err(format!("group #{i}.{k}: got {g:?}, expected {v:?}"));
            }
        }
    }
    Ok(())
}

fn expect_tuning(w: &World, op: &Json) -> R<()> {
    let t = w.budget()?.tuning();
    let mut errors = Vec::new();
    let f = |errors: &mut Vec<String>, name: &str, got: &[f32], v: &Json| {
        let want: Vec<f64> = match v {
            Json::Array(a) => a.iter().filter_map(Json::as_f64).collect(),
            other => other.as_f64().into_iter().collect(),
        };
        if want.len() != got.len() || want.iter().zip(got).any(|(w, g)| !rel_close(f64::from(*g), *w)) {
            errors.push(format!("{name}: got {got:?}, expected {v:?}"));
        }
    };
    for (k, v) in field(op, "tuning")?.as_object().ok_or("tuning must be an object")? {
        match k.as_str() {
            "initial_funds" => cmp_arr(&mut errors, k, &t.initial_funds, v),
            "max_tax_rate" => f(&mut errors, k, &[t.max_tax_rate], v),
            "min_allowable_funds" => cmp_i(&mut errors, k, t.min_allowable_funds, v),
            "bond_increment" => cmp_i(&mut errors, k, t.bond_increment, v),
            "max_bonds" => cmp_i(&mut errors, k, t.max_bonds.into(), v),
            "max_bond_amount" => cmp_i(&mut errors, k, t.max_bond_amount, v),
            "bond_interest_rate" => f(&mut errors, k, &[t.bond_interest_rate], v),
            "bond_payback_months" => cmp_i(&mut errors, k, t.bond_payback_months.into(), v),
            "initial_tax_rate" => f(&mut errors, k, &t.initial_tax_rate, v),
            "difficulty_tax_multiplier" => f(&mut errors, k, &t.difficulty_tax_multiplier, v),
            "waning_wealth" => f(&mut errors, k, &t.waning_wealth, v),
            "occ_weight" => f(&mut errors, k, &t.occ_weight, v),
            "land_value_curve" => {
                for pair in v.as_array().ok_or("land_value_curve must be [[x, y], ...]")? {
                    let p = pair.as_array().ok_or("each entry must be [x, y]")?;
                    let (x, y) = (p[0].as_f64().unwrap_or(0.0), p[1].as_f64().unwrap_or(0.0));
                    let got = f64::from(t.land_value_curve.eval(x as f32));
                    if !(rel_close(got, y) || (got - y).abs() < 1e-9) {
                        errors.push(format!("land_value_curve({x}) = {got}, expected {y}"));
                    }
                }
            }
            other => return Err(format!("unknown tuning field '{other}'")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("tuning mismatch: {}", errors.join("; ")))
    }
}

fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = *w
            .tokens
            .borrow()
            .get(k)
            .ok_or_else(|| format!("unknown object '{k}'"))?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if v.as_i64() != Some(got) {
            return Err(format!("references of {k}: {got}, expected {v:?}"));
        }
    }
    Ok(())
}

// ---- save / load ------------------------------------------------------------------------------------------

fn save_current(w: &World) -> R<SavedCity> {
    let b = w.budget()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&RefValue::Object(b as Rc<dyn SaveObject>));
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    let holders = reg
        .borrow()
        .entries
        .iter()
        .filter(|e| e.2.is_none())
        .map(|e| (e.1, e.3))
        .collect();
    Ok(SavedCity {
        pkg,
        root: root.into_bytes(),
        holders,
    })
}

fn save(w: &World, op: &Json) -> R<()> {
    let slot = field(op, "slot")?.as_str().ok_or("slot must be a string")?.to_owned();
    let s = save_current(w)?;
    w.saves.borrow_mut().insert(slot, s);
    Ok(())
}

fn class_arg(j: &Json) -> R<u32> {
    match j.as_str() {
        Some("budget") => Ok(CLSID_BUDGET_SIMULATOR),
        Some("department") => Ok(CLSID_DEPARTMENT),
        Some("line") => Ok(CLSID_LINE_ITEM),
        _ => j
            .as_i64()
            .map(|v| v as u32)
            .ok_or_else(|| "class must be budget/department/line or an ID".into()),
    }
}

fn load(w: &World, op: &Json) -> R<()> {
    let slot = field(op, "slot")?.as_str().ok_or("slot must be a string")?;
    let saved = w
        .saves
        .borrow()
        .get(slot)
        .cloned()
        .ok_or_else(|| format!("no saved slot '{slot}'"))?;
    let host = w.host();
    let me = w.me.upgrade().ok_or("world gone")?;
    let mut factory = |class: u32, object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_BUDGET_SIMULATOR => {
                let t = me.new_token("budget".to_owned());
                Some(RefValue::Object(BudgetSim::new(host.clone(), t) as Rc<dyn SaveObject>))
            }
            CLSID_DEPARTMENT => {
                let t = me.new_token(me.next_label("department"));
                Some(RefValue::Object(Department::new(host.clone(), t) as Rc<dyn SaveObject>))
            }
            CLSID_LINE_ITEM => {
                let t = me.new_token(me.next_label("line"));
                Some(RefValue::Object(LineItem::new(host.clone(), t) as Rc<dyn SaveObject>))
            }
            HOLDER_CLASS => {
                // The same game object comes back; the reader takes a reference.
                let token = *saved.holders.get(&object_id)?;
                me.add_ref(token);
                Some(RefValue::Host { token, class_id: class })
            }
            _ => None,
        }
    };
    let mut reg = LoadRegistry::new(BTreeMap::new(), Box::new(&mut factory));
    let report = load_store(&saved.pkg, &mut reg);
    // "expect": whether the store reports a clean load (index readable, every
    // class complete). Record Read results are ignored by the store
    // (foundations §3.5), so a rejected record shows up as an object left in
    // its initial state while the store itself still reports a clean load.
    let want_ok = op.get("expect").and_then(boolean).unwrap_or(true);
    let ok = report.index_ok && report.failed_classes.is_empty();
    let mut r = ByteReader::with_resolver(&saved.root, &mut reg);
    let root = r.get_ref();
    let budget = root.as_ref().and_then(|v| v.object::<BudgetSim>());
    if ok != want_ok {
        return Err(format!("store load clean = {ok}, expected {want_ok} ({report:?})"));
    }
    let b = budget.ok_or("the root reference did not resolve to a budget")?;
    *w.budget.borrow_mut() = Some(b);
    if let Some(rr) = op.get("read_ok") {
        // Whether the budget record itself was accepted (Read result).
        let want = boolean(rr).ok_or("read_ok must be a boolean")?;
        let got = w
            .budget
            .borrow()
            .as_ref()
            .is_some_and(|b| b.snapshot().loaded_from_save);
        if got != want {
            return Err(format!("budget record accepted = {got}, expected {want}"));
        }
    }
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let slot = field(op, "slot")?.as_str().ok_or("slot must be a string")?;
    let want = w
        .saves
        .borrow()
        .get(slot)
        .cloned()
        .ok_or_else(|| format!("no saved slot '{slot}'"))?;
    let got = save_current(w)?;
    if got.root != want.root {
        return Err("root records differ".into());
    }
    for (k, v) in &want.pkg {
        match got.pkg.get(k) {
            Some(g) if g == v => {}
            Some(g) => {
                return Err(format!(
                    "record {:08X}/{:08X}/{} differs:\n    got      {}\n    expected {}",
                    k.0,
                    k.1,
                    k.2,
                    crate::foundations::hex_encode(g),
                    crate::foundations::hex_encode(v)
                ))
            }
            None => return Err(format!("record {:08X} missing", k.0)),
        }
    }
    if got.pkg.len() != want.pkg.len() {
        return Err("record sets differ".into());
    }
    Ok(())
}

/// Patches bytes of an entry's payload in a saved slot and fixes its CRC.
fn patch(w: &World, op: &Json) -> R<()> {
    let slot = field(op, "slot")?.as_str().ok_or("slot must be a string")?;
    let class = class_arg(field(op, "class")?)?;
    let entry = opt_int(op, "entry")?.unwrap_or(0) as usize;
    let offset = int(op, "offset")? as usize;
    let bytes = hex_decode(field(op, "hex")?.as_str().ok_or("hex must be a string")?)?;
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(slot).ok_or_else(|| format!("no saved slot '{slot}'"))?;
    let rec = s
        .pkg
        .get_mut(&(class, STORE_GROUP, 0))
        .ok_or("no record for that class")?;
    let mut pos = 0usize;
    for _ in 0..entry {
        let size = u32::from_le_bytes(rec[pos..pos + 4].try_into().unwrap()) as usize;
        pos += size;
    }
    let size = u32::from_le_bytes(rec[pos..pos + 4].try_into().unwrap()) as usize;
    let start = pos + 12 + offset;
    if start + bytes.len() > pos + size {
        return Err("patch beyond the entry".into());
    }
    rec[start..start + bytes.len()].copy_from_slice(&bytes);
    let span = (size - 8).min(CRC_LIMIT);
    let crc = crc32_mpeg2(&rec[pos + 8..pos + 8 + span]);
    rec[pos + 4..pos + 8].copy_from_slice(&crc.to_le_bytes());
    Ok(())
}

// Keep the stream traits in scope for `put_ref` / `get_ref`.
#[allow(dead_code)]
fn _uses(_: &dyn OutStream, _: &dyn InStream) {}
