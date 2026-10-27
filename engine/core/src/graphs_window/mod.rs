// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4GraphsViewWinProc, from `docs/specs/graphs-window.md`.
//! Window/control implementations, graph drawing and city services are foreign.
pub use crate::widget_controls::{Handle, Rect, Result, Unsupported};
use std::collections::BTreeMap;

mod display;
mod refresh;
#[cfg(test)]
mod tests;
pub use refresh::{axis, bar_value, line_value, sample_dates, AxisOutput};

pub const UPDATE: u32 = 0x6695_6816;
pub const IID_LINE: u32 = 0x12cf_2351;
pub const IID_BAR: u32 = 0x22cf_2351;
pub const IID_WIN_GEN: u32 = 0x5386_d516;
pub const IID_TEXT: u32 = 0x212c_dc1f;
pub const TIMES: [(usize, i32, u32); 6] = [
    (12, 30, 0x2a5d4165),
    (60, 30, 0xca5d417f),
    (120, 30, 0xca5d4184),
    (100, 182, 0x8a5d4189),
    (100, 365, 0x8a5d418e),
    (100, 1825, 0x2a5d4194),
];

/// Raw exemplar properties (§2.1), not a host-computed panel or series result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Definition {
    /// Counted resource reference, released by ShowGraph.
    pub resource: Handle,
    pub kind: u32,
    pub title: Option<String>,
    pub name: String,
    pub keys: Vec<u32>,
    pub second_keys: Vec<u32>,
    pub operations: Vec<u32>,
    pub labels: Vec<u32>,
    pub colors: Vec<[u8; 3]>,
    pub scales: Vec<f32>,
    pub groups: Vec<u32>,
    pub fills: Vec<[u8; 3]>,
    pub edges: Vec<[u8; 3]>,
    pub minimum: Option<f32>,
    pub maximum: Option<f32>,
    pub step: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series {
    pub key: u32,
    pub second_key: u32,
    pub group: u32,
    pub operation: u32,
    pub color: [u8; 3],
    pub scale: f32,
    pub label: String,
    pub visible: bool,
}

/// Owned equivalents of §7's string-bearing vector helpers. The logical capacity
/// preserves the specified growth rule independently of the Rust allocator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeriesRecords {
    values: Vec<Series>,
    capacity: usize,
}
impl SeriesRecords {
    pub fn values(&self) -> &[Series] {
        &self.values
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn resize(&mut self, size: usize) {
        if size > self.capacity {
            self.capacity = self.values.len() + self.values.len().max(size - self.values.len());
            self.values.reserve_exact(self.capacity - self.values.len());
        }
        self.values.resize(size, Series::default());
    }
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis {
    pub maximum: f32,
    pub minimum: f32,
    pub step: f32,
}
impl Default for Axis {
    fn default() -> Self {
        Self {
            maximum: -f32::MAX,
            minimum: f32::MAX,
            step: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineStyle {
    pub bytes: [u8; 2],
    pub width: u32,
    pub byte8: u8,
    pub color: u32,
    pub word10: u32,
    pub text: String,
}
impl LineStyle {
    pub fn new(color: u32) -> Self {
        Self {
            bytes: [0, 0x41],
            width: 12,
            byte8: 1,
            color,
            word10: 1,
            text: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Graph,
    Line,
    Bar,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    U32(u32),
    Float(f32),
    Handle(Handle),
    Text(String),
    Rect(Rect),
    Values(Vec<f32>),
    Style(LineStyle),
    Swatch {
        rect: Rect,
        flags: [u32; 3],
        edge: u32,
        fill: u32,
    },
    Label {
        rect: Rect,
        font: Handle,
        color: u32,
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum WindowOp {
    Parent,
    Find { id: u32, iid: Option<u32> },
    Query(u32),
    Procedure(Option<(Handle, u32)>),
    Notification(Handle),
    Show(bool),
    Visible,
    Flag(u32, bool),
    Repaint,
    Area,
    Height,
    SetArea(Rect),
    AddChild(Handle),
    DeleteChild(Handle),
    Children,
    Text(String),
    SliderValue,
    ButtonState(u32),
    Button { slot: u32, args: Vec<u32> },
}
#[derive(Clone, Debug, PartialEq)]
pub enum Service {
    Subscribe(Handle, u32),
    Unsubscribe(Handle, u32),
    Rgb([u8; 3]),
    Font(u32),
    Text(u32),
    Today,
    StartDate,
    History {
        key: u32,
        date: i32,
    },
    MakeDate(i32),
    /// Returns a created counted widget; the owner consumes that reference.
    CreateWidget {
        kind: u32,
        bytes: u32,
    },
    CreateCheckbox {
        look: [u32; 3],
        id: u32,
        area: Rect,
        target: Handle,
        visible: bool,
        sound: u32,
    },
    Filter([u32; 3]),
    AntialiasedLine([u32; 8]),
    LineQuery {
        widget: Handle,
        iid: u32,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    AddRef(Handle),
    Release(Handle),
    Window(Handle, WindowOp),
    Service(Service),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Unit,
    Handle(Option<Handle>),
    Handles(Vec<Handle>),
    Bool(bool),
    Word(u32),
    Date(i32),
    Float(f32),
    Text(String),
    Rect(Rect),
    Missing,
}

/// Each method is a single foreign primitive. The four open questions retain
/// explicit Unsupported defaults; adapters may supply raw slot contracts without
/// guessing widget semantics in this owner.
pub trait Host {
    fn call(&mut self, call: Call) -> Result<Value>;
    fn definition(&mut self, graph: u32) -> Result<Option<Definition>>;
    fn graph(&mut self, _widget: Handle, _space: Space, _slot: u32, _args: Vec<Arg>) -> Result<Value> {
        Err(Unsupported::Open("GW1: graph widget slots"))
    }
    fn axis_title(&mut self, _widget: Handle, _text: String, _index: u32) -> Result<()> {
        Err(Unsupported::Open("GW2: G17 second argument"))
    }
    fn select_button(&mut self, _button: Handle, _args: [u32; 2]) -> Result<()> {
        Err(Unsupported::Open("GW3: SelectOption button slot 5"))
    }
    fn line_control(&mut self, _window: Handle, _show: bool) -> Result<()> {
        Err(Unsupported::Open("GW4: line-only control 8A2871D8"))
    }
}

#[derive(Debug)]
pub struct Owner {
    pub identity: Handle,
    main: Option<Handle>,
    collapsed: Option<Handle>,
    expanded_panel: Option<Handle>,
    widget: Option<Handle>,
    initialized: bool,
    destroyed: bool,
    pub kind: u32,
    pub graph_id: u32,
    pub shown: bool,
    pub expanded: bool,
    pub series: SeriesRecords,
    pub visibility: BTreeMap<u32, u8>,
    pub groups: u32,
    pub time_scale: usize,
    pub axis: Axis,
}

fn handle(value: Value) -> Result<Option<Handle>> {
    match value {
        Value::Handle(h) => Ok(h),
        _ => Err(Unsupported::Precondition("pointer output")),
    }
}
fn required(value: Value) -> Result<Handle> {
    handle(value)?.ok_or(Unsupported::Precondition("required foreign object"))
}
fn word(value: Value) -> Result<u32> {
    match value {
        Value::Word(v) => Ok(v),
        _ => Err(Unsupported::Precondition("word output")),
    }
}
fn boolean(value: Value) -> Result<bool> {
    match value {
        Value::Bool(v) => Ok(v),
        _ => Err(Unsupported::Precondition("boolean output")),
    }
}
fn float(value: Value) -> Result<f32> {
    match value {
        Value::Float(v) => Ok(v),
        _ => Err(Unsupported::Precondition("float output")),
    }
}
impl Owner {
    pub fn new(host: &mut impl Host, identity: Handle, main: Option<Handle>) -> Result<Self> {
        if let Some(main) = main {
            host.call(Call::AddRef(main))?;
        }
        Ok(Self {
            identity,
            main,
            collapsed: None,
            expanded_panel: None,
            widget: None,
            initialized: false,
            destroyed: false,
            kind: 0,
            graph_id: 1,
            shown: false,
            expanded: true,
            series: SeriesRecords::default(),
            visibility: BTreeMap::new(),
            groups: 0,
            time_scale: 0,
            axis: Axis {
                maximum: 0.0,
                minimum: 0.0,
                step: 0.0,
            },
        })
    }
    pub fn widget(&self) -> Option<Handle> {
        self.widget
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    fn live(&self) -> Result<()> {
        if self.destroyed {
            Err(Unsupported::Precondition("destroyed graphs owner"))
        } else {
            Ok(())
        }
    }
    fn find(host: &mut impl Host, window: Handle, id: u32, iid: Option<u32>) -> Result<Option<Handle>> {
        handle(host.call(Call::Window(window, WindowOp::Find { id, iid }))?)
    }
    fn show(host: &mut impl Host, window: Option<Handle>, on: bool) -> Result<()> {
        if let Some(w) = window {
            host.call(Call::Window(w, WindowOp::Show(on)))?;
        }
        Ok(())
    }
    fn procedure(&self, host: &mut impl Host, window: Handle, install: bool) -> Result<()> {
        let generator = required(host.call(Call::Window(window, WindowOp::Query(IID_WIN_GEN)))?)?;
        host.call(Call::Window(
            generator,
            WindowOp::Procedure(install.then_some((self.identity, 4))),
        ))?;
        host.call(Call::Release(generator))?;
        Ok(())
    }
    fn drop_widget(&mut self, host: &mut impl Host) -> Result<()> {
        if let Some(widget) = self.widget.take() {
            let panel = self.expanded_panel.ok_or(Unsupported::Precondition("graph parent"))?;
            host.call(Call::Window(panel, WindowOp::DeleteChild(widget)))?;
            host.call(Call::Release(widget))?;
        }
        Ok(())
    }
    pub fn init(&mut self, host: &mut impl Host) -> Result<bool> {
        self.live()?;
        let main = self.main.ok_or(Unsupported::Precondition("main window"))?;
        if !self.initialized {
            let parent = required(host.call(Call::Window(main, WindowOp::Parent))?)?;
            self.collapsed = Self::find(host, parent, 0x8a8b5b72, None)?;
            if let Some(w) = self.collapsed {
                host.call(Call::AddRef(w))?;
            }
            self.expanded_panel = Self::find(host, parent, 0x8a8b5b71, None)?;
            if let Some(w) = self.expanded_panel {
                host.call(Call::AddRef(w))?;
            }
            let collapsed = self.collapsed.ok_or(Unsupported::Precondition("collapsed panel"))?;
            let expanded = self.expanded_panel.ok_or(Unsupported::Precondition("expanded panel"))?;
            self.set_time_scale(host, 0)?;
            Self::show(host, self.collapsed, false)?;
            Self::show(host, self.expanded_panel, false)?;
            for window in [main, collapsed, expanded] {
                self.procedure(host, window, true)?;
                host.call(Call::Window(window, WindowOp::Flag(0x800, false)))?;
            }
            host.call(Call::Service(Service::Subscribe(self.identity, UPDATE)))?;
            self.initialized = true;
        }
        for (panel, id) in [
            (self.collapsed, 0x0a8b7e01),
            (self.expanded_panel, 0x0a8b7e00),
            (self.collapsed, 0xca8b82f9),
            (self.expanded_panel, 0xca8b82f9),
        ] {
            if let Some(panel) = panel {
                if let Some(button) = Self::find(host, panel, id, None)? {
                    host.call(Call::Window(button, WindowOp::Notification(main)))?;
                }
            }
        }
        if let Some(container) = Self::find(host, main, 0xaa9244b6, None)? {
            let Value::Handles(children) = host.call(Call::Window(container, WindowOp::Children))? else {
                return Err(Unsupported::Precondition("child enumeration"));
            };
            for child in children {
                host.call(Call::Window(child, WindowOp::Notification(main)))?;
            }
        }
        Ok(true)
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> Result<bool> {
        self.live()?;
        if self.initialized {
            self.initialized = false;
            host.call(Call::Service(Service::Unsubscribe(self.identity, UPDATE)))?;
            self.drop_widget(host)?;
            for window in [self.main, self.collapsed, self.expanded_panel].into_iter().flatten() {
                self.procedure(host, window, false)?;
            }
            for window in [self.collapsed.take(), self.expanded_panel.take()]
                .into_iter()
                .flatten()
            {
                host.call(Call::Release(window))?;
            }
        }
        Ok(true)
    }
    pub fn destroy(&mut self, host: &mut impl Host) -> Result<()> {
        self.shutdown(host)?;
        if let Some(main) = self.main.take() {
            host.call(Call::Release(main))?;
        }
        self.visibility.clear();
        self.series = SeriesRecords::default();
        for object in [
            self.widget.take(),
            self.expanded_panel.take(),
            self.collapsed.take(),
            self.main.take(),
        ]
        .into_iter()
        .flatten()
        {
            host.call(Call::Release(object))?;
        }
        self.destroyed = true;
        Ok(())
    }
}

/// §7 overrides forward to existing drawing/query services, without a graph renderer.
pub fn antialiased_line(host: &mut impl Host, args: [u32; 7]) -> Result<()> {
    let mut forwarded = [0; 8];
    forwarded[..7].copy_from_slice(&args);
    host.call(Call::Service(Service::AntialiasedLine(forwarded)))?;
    Ok(())
}
pub fn line_query(host: &mut impl Host, widget: Handle, iid: u32) -> Result<Option<Handle>> {
    handle(host.call(Call::Service(Service::LineQuery { widget, iid }))?)
}
