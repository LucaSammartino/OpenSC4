// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn text(host: &mut impl Host, id: u32) -> Result<String> {
    match host.call(Call::Service(Service::Text(id)))? {
        Value::Text(s) => Ok(s),
        _ => Err(Unsupported::Precondition("localized text")),
    }
}
fn rgb(host: &mut impl Host, color: [u8; 3]) -> Result<u32> {
    word(host.call(Call::Service(Service::Rgb(color)))?)
}
fn font(host: &mut impl Host, id: u32) -> Result<Handle> {
    required(host.call(Call::Service(Service::Font(id)))?)
}

impl Owner {
    pub(super) fn g(&self, host: &mut impl Host, slot: u32, args: Vec<Arg>) -> Result<Value> {
        host.graph(
            self.widget.ok_or(Unsupported::Precondition("graph widget"))?,
            Space::Graph,
            slot,
            args,
        )
    }
    pub fn set_expanded(&mut self, host: &mut impl Host, on: bool) -> Result<()> {
        self.live()?;
        if on {
            Self::show(host, self.collapsed, false)?;
            Self::show(host, self.expanded_panel, true)?;
        } else {
            Self::show(host, self.expanded_panel, false)?;
            Self::show(host, self.collapsed, true)?;
        }
        self.expanded = on;
        Ok(())
    }
    pub fn select_option(&mut self, host: &mut impl Host, graph: u32) -> Result<()> {
        self.live()?;
        let main = self.main.ok_or(Unsupported::Precondition("main window"))?;
        for (g, on) in [(self.graph_id, 0), (graph, 1)] {
            for base in [0x4000u32, 0x4100] {
                if let Some(button) = Self::find(host, main, base.wrapping_add(g), None)? {
                    host.select_button(button, [1, on])?;
                }
            }
        }
        Ok(())
    }
    pub fn show_window(&mut self, host: &mut impl Host) -> Result<()> {
        self.live()?;
        self.show_graph(host, self.graph_id)?;
        self.select_option(host, self.graph_id)?;
        self.shown = true;
        Self::show(host, self.main, true)?;
        Self::show(
            host,
            if self.expanded {
                self.expanded_panel
            } else {
                self.collapsed
            },
            true,
        )
    }
    pub fn hide_window(&mut self, host: &mut impl Host) -> Result<()> {
        self.live()?;
        if self.main.is_some() {
            self.show_graph(host, 0)?;
            Self::show(host, self.main, false)?;
            self.shown = false;
        }
        Self::show(host, self.collapsed, false)?;
        Self::show(host, self.expanded_panel, false)
    }
    pub fn show_graph_type(&mut self, host: &mut impl Host, graph: u32) -> Result<()> {
        self.live()?;
        if graph != self.graph_id {
            self.select_option(host, graph)?;
            self.show_graph(host, graph)?;
            self.graph_id = graph;
        }
        self.show_window(host)?;
        self.set_expanded(host, true)
    }
    pub fn set_time_scale(&mut self, host: &mut impl Host, index: usize) -> Result<()> {
        self.live()?;
        self.time_scale = index;
        let &(_, _, label) = TIMES.get(index).ok_or(Unsupported::Precondition("time scale 0..5"))?;
        if let Some(panel) = self.expanded_panel {
            if let Some(control) = Self::find(host, panel, 0x8a2871d9, Some(IID_TEXT))? {
                let label = text(host, label)?;
                host.call(Call::Window(control, WindowOp::Text(label)))?;
                host.call(Call::Release(control))?;
            }
        }
        if let Some(widget) = self.widget {
            self.refresh(host)?;
            host.call(Call::Window(widget, WindowOp::Repaint))?;
        }
        Ok(())
    }
    pub fn message(&mut self, host: &mut impl Host, id: u32) -> Result<bool> {
        self.live()?;
        if id != UPDATE {
            return Ok(false);
        }
        if let Some(widget) = self.widget {
            self.refresh(host)?;
            host.call(Call::Window(widget, WindowOp::Repaint))?;
        }
        Ok(true)
    }
    pub fn window_message(&mut self, host: &mut impl Host, _source: Handle, message: [u32; 3]) -> Result<bool> {
        self.live()?;
        let main = self.main.ok_or(Unsupported::Precondition("main window"))?;
        if !boolean(host.call(Call::Window(main, WindowOp::Visible))?)? || message[0] != 3 {
            return Ok(false);
        }
        if !boolean(host.call(Call::Service(Service::Filter(message)))?)? {
            return Ok(true);
        }
        let [_, subtype, id] = message;
        if subtype == 0x887113a3 && id == 0xea4a7cf2 {
            let panel = self.expanded_panel.ok_or(Unsupported::Precondition("expanded panel"))?;
            let slider = Self::find(host, panel, id, None)?.ok_or(Unsupported::Precondition("time slider"))?;
            let value = word(host.call(Call::Window(slider, WindowOp::SliderValue))?)? as usize;
            if value != self.time_scale {
                self.set_time_scale(host, value)?;
            }
        } else if subtype == 0x287259f6 {
            match id {
                0x0a8b7e00 => self.set_expanded(host, false)?,
                0x0a8b7e01 => self.set_expanded(host, true)?,
                0xca8b82f9 => self.set_expanded(host, !self.expanded)?,
                0x4000..=0x4fff => {
                    let graph = id & 0xff;
                    self.set_expanded(host, true)?;
                    self.select_option(host, graph)?;
                    if graph != self.graph_id {
                        self.show_graph(host, graph)?;
                        self.graph_id = graph;
                    }
                }
                0x04000000..=0x0400001f => self.refresh(host)?,
                _ => {}
            }
        }
        Ok(false)
    }

    pub fn show_graph(&mut self, host: &mut impl Host, graph: u32) -> Result<()> {
        self.live()?;
        let Some(panel) = self.expanded_panel else {
            return Ok(());
        };
        self.drop_widget(host)?;
        let Some(definition) = host.definition(graph)? else {
            return Ok(());
        };
        if definition.kind != self.kind {
            self.series.clear();
            self.drop_widget(host)?;
            self.kind = definition.kind;
        }
        let bytes = match self.kind {
            1 => 0x260,
            2 => 0x268,
            3 => 0x28c,
            _ => {
                // SPEC-GAP: resource cleanup on an unknown kind is unspecified; release the acquired exemplar.
                host.call(Call::Release(definition.resource))?;
                return Ok(());
            }
        };
        self.widget =
            Some(required(host.call(Call::Service(Service::CreateWidget {
                kind: self.kind,
                bytes,
            }))?)?);
        for id in [0xea4a7cf2, 0x8a2871d9, 0x8a2871d8] {
            if let Some(control) = Self::find(host, panel, id, None)? {
                if id == 0x8a2871d8 {
                    host.line_control(control, self.kind == 1)?;
                } else {
                    Self::show(host, Some(control), self.kind == 1)?;
                }
                host.call(Call::Window(control, WindowOp::Repaint))?;
            }
        }
        let area_control =
            Self::find(host, panel, 0x8a4b0aa0, None)?.ok_or(Unsupported::Precondition("graph area control"))?;
        let Value::Rect(area) = host.call(Call::Window(area_control, WindowOp::Area))? else {
            return Err(Unsupported::Precondition("graph area"));
        };
        let widget = self.widget.ok_or(Unsupported::Precondition("created widget"))?;
        host.call(Call::Window(widget, WindowOp::SetArea(area)))?;
        host.call(Call::Window(panel, WindowOp::AddChild(widget)))?;
        let c0 = rgb(host, [0x3f, 0x49, 0x67])?;
        let c1 = rgb(host, [0xaa, 0xbb, 0xc3])?;
        let f1 = font(host, 0xe9c86b5f)?;
        let f2 = font(host, 0xe9c86b6e)?;
        self.g(host, 39, vec![Arg::Handle(f1), Arg::U32(c0)])?;
        for slot in [21, 29] {
            for index in [0, 1] {
                self.g(host, slot, vec![Arg::U32(index), Arg::Handle(f2), Arg::U32(c0)])?;
            }
        }
        self.g(host, 4, vec![Arg::U32(0)])?;
        let color = rgb(host, [0xda, 0xe0, 0xe5])?;
        self.g(host, 5, vec![Arg::U32(color)])?;
        self.g(host, 8, vec![Arg::U32(1), Arg::U32(1)])?;
        for (slot, color) in [(9, [0xae, 0xbf, 0xc0]), (10, [0xef, 0xf3, 0xf7])] {
            let color = rgb(host, color)?;
            self.g(host, slot, vec![Arg::U32(color)])?;
        }
        self.g(host, 18, vec![Arg::U32(c1)])?;
        self.g(host, 31, vec![Arg::U32(c1)])?;
        self.g(host, 30, vec![Arg::U32(0), Arg::U32(1)])?;
        self.g(
            host,
            37,
            vec![
                Arg::Text(definition.title.clone().unwrap_or_else(|| definition.name.clone())),
                Arg::U32(0),
            ],
        )?;
        let size = [area[2] - area[0], area[3] - area[1]];
        match self.kind {
            1 => self.setup_line(host, &definition, size, [c0, c1])?,
            2 => self.setup_bar(host, &definition, size)?,
            _ => {} // Pie intentionally has no data population.
        }
        Self::show(host, self.widget, true)?;
        host.call(Call::Release(definition.resource))?;
        Ok(())
    }

    fn read_axis(&mut self, definition: &Definition) {
        self.axis = Axis::default();
        if let Some(v) = definition.minimum {
            self.axis.minimum = v;
        }
        if let Some(v) = definition.maximum {
            self.axis.maximum = v;
        }
        if let Some(v) = definition.step {
            self.axis.step = v;
        }
    }
    fn read_series(&mut self, host: &mut impl Host, definition: &Definition, line: bool) -> Result<()> {
        self.series.resize(definition.keys.len());
        self.groups = 1;
        for (index, record) in self.series.values.iter_mut().enumerate() {
            let key = definition.keys[index];
            *record = Series {
                key,
                second_key: definition.second_keys.get(index).copied().unwrap_or(0),
                group: definition.groups.get(index).copied().unwrap_or(0),
                operation: definition.operations.get(index).copied().unwrap_or(0),
                color: definition.colors.get(index).copied().unwrap_or([0; 3]),
                scale: definition.scales.get(index).copied().unwrap_or(1.0),
                label: if let Some(&id) = definition.labels.get(index) {
                    text(host, id)?
                } else {
                    String::new()
                },
                visible: line && self.visibility.get(&key).copied().unwrap_or(1) != 0,
            };
            self.groups = self.groups.max(record.group.saturating_add(1));
        }
        self.groups = self.groups.min(16);
        Ok(())
    }
    fn setup_bar(&mut self, host: &mut impl Host, definition: &Definition, [w, h]: [i32; 2]) -> Result<()> {
        self.g(host, 12, vec![Arg::Rect([45, 20, w - 2, h - 20])])?;
        let widget = self.widget.ok_or(Unsupported::Precondition("bar widget"))?;
        let bar = required(host.call(Call::Window(widget, WindowOp::Query(IID_BAR)))?)?;
        host.graph(bar, Space::Bar, 5, vec![Arg::U32(1)])?;
        self.read_series(host, definition, false)?;
        for (i, record) in self.series.values.iter().enumerate() {
            host.graph(
                bar,
                Space::Bar,
                12,
                vec![Arg::Text(record.label.clone()), Arg::U32(i as u32)],
            )?;
        }
        host.graph(bar, Space::Bar, 5, vec![Arg::U32(self.groups)])?;
        for j in 0..self.groups as usize {
            let fill = rgb(host, definition.fills.get(j).copied().unwrap_or([0xaa, 0xbb, 0xc3]))?;
            host.graph(bar, Space::Bar, 15, vec![Arg::U32(fill), Arg::U32(j as u32)])?;
            let edge = rgb(host, definition.edges.get(j).copied().unwrap_or([0x67, 0x71, 0x76]))?;
            host.graph(bar, Space::Bar, 16, vec![Arg::U32(edge), Arg::U32(j as u32)])?;
        }
        self.read_axis(definition);
        self.refresh(host)?;
        host.call(Call::Release(bar))?;
        Ok(())
    }
    fn setup_line(
        &mut self,
        host: &mut impl Host,
        definition: &Definition,
        [w, h]: [i32; 2],
        [c0, c1]: [u32; 2],
    ) -> Result<()> {
        self.g(host, 12, vec![Arg::Rect([45, 20, w - 110, h - 20])])?;
        let f3 = font(host, 0xe9c86b5e)?;
        let widget = self.widget.ok_or(Unsupported::Precondition("line widget"))?;
        let line = required(host.call(Call::Window(widget, WindowOp::Query(IID_LINE)))?)?;
        self.g(host, 23, vec![Arg::U32(0), Arg::U32(1)])?;
        self.g(host, 32, vec![Arg::U32(0), Arg::U32(0)])?;
        self.read_series(host, definition, true)?;
        let mut y = 20;
        for (i, record) in self.series.values.iter().enumerate() {
            let mut x = w - 106;
            if self.series.values.len() > 2 {
                let checkbox = required(host.call(Call::Service(Service::CreateCheckbox {
                    look: [0x856ddbac, 0x46a006b0, 0x14416245],
                    id: 0x04000000 + i as u32,
                    area: [w - 108, y, w - 92, y + 16],
                    target: self.main.ok_or(Unsupported::Precondition("notification target"))?,
                    visible: record.visible,
                    sound: 0x4a5c31d7,
                }))?)?;
                host.call(Call::Window(widget, WindowOp::AddChild(checkbox)))?;
                // SPEC-GAP: factory returns a created reference; the parent retains the checkbox.
                host.call(Call::Release(checkbox))?;
                x = w - 90;
            }
            let color = rgb(host, record.color)?;
            self.g(
                host,
                51,
                vec![Arg::Swatch {
                    rect: [x, y + 3, x + 10, y + 9],
                    flags: [1, 1, 1],
                    edge: c1,
                    fill: color,
                }],
            )?;
            let label = required(self.g(
                host,
                49,
                vec![Arg::Label {
                    rect: [x + 14, y, w - 4, y + 10],
                    font: f3,
                    color: c0,
                    text: record.label.clone(),
                }],
            )?)?;
            y += word(host.call(Call::Window(label, WindowOp::Height))?)? as i32 + 4;
        }
        self.read_axis(definition);
        self.refresh(host)?;
        host.call(Call::Release(line))?;
        Ok(())
    }
}
