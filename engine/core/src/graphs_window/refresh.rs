// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisOutput {
    pub minimum: f32,
    pub maximum: f32,
    pub step: f32,
    pub fractional: bool,
}
pub fn axis(settings: Axis, automatic_maximum: f32, automatic_minimum: f32) -> Result<AxisOutput> {
    let set_max = settings.maximum != -f32::MAX;
    let mut maximum = if set_max { settings.maximum } else { automatic_maximum };
    let minimum = if settings.minimum != f32::MAX {
        settings.minimum
    } else {
        automatic_minimum
    };
    let range = (maximum - minimum).trunc();
    if !range.is_finite() || !(0.0..u32::MAX as f32).contains(&range) {
        return Err(Unsupported::Precondition("finite nonnegative unsigned axis range"));
    }
    let mut r = range as u32;
    let (mut step, fractional) = if r < 1 {
        maximum = 1.0;
        (0.2, true)
    } else if r < 3 {
        maximum = maximum.ceil();
        (0.5, true)
    } else {
        let mut u = 1u32;
        while r >= 100 {
            u *= 10;
            r = r.div_ceil(10);
        }
        let factor = if r > 50 {
            10
        } else if r > 20 {
            5
        } else if r > 10 {
            4
        } else if r > 5 {
            2
        } else {
            1
        };
        if !set_max {
            maximum *= 20.0 / 19.0;
        }
        ((factor * u) as f32, false)
    };
    if settings.step != 0.0 {
        step = settings.step;
    }
    Ok(AxisOutput {
        minimum,
        maximum,
        step,
        fractional,
    })
}

pub fn line_value(v: f32, v2: f32, operation: u32) -> f32 {
    match operation {
        1 => v + v2,
        2 => v - v2,
        3 => v * v2,
        4 if v2.abs() > 1e-7 => v / v2,
        4 => 0.0,
        _ => v,
    }
}
pub fn bar_value(v: f32, operation: u32, settings: Axis) -> f32 {
    if operation == 5 && settings.maximum != -f32::MAX && settings.minimum != f32::MAX {
        // SPEC-GAP: intermediate precision is unspecified; evaluate logarithms in f64 and store f32.
        let (v, bound) = (
            f64::from(v),
            f64::from(if v > 0.0 { settings.maximum } else { settings.minimum }),
        );
        if v != 0.0 {
            return (0.5 * bound * (v.abs().ln() / bound.abs().ln() + v / bound)) as f32;
        }
    }
    v
}

/// Returns the preceding period boundary `a` and exactly N+1 sample dates (§5.2).
pub fn sample_dates(today: i32, time_scale: usize) -> Result<(i32, Vec<i32>)> {
    let &(points, step, _) = TIMES
        .get(time_scale)
        .ok_or(Unsupported::Precondition("time scale 0..5"))?;
    let shifted = i64::from(today) - 4;
    let step = i64::from(step);
    let end = (shifted.div_euclid(step) + i64::from(shifted.rem_euclid(step) != 0)) * step;
    let a = end - (points as i64 + 1) * step;
    let convert = |v| i32::try_from(v).map_err(|_| Unsupported::Precondition("date arithmetic overflow"));
    Ok((
        convert(a)?,
        (1..=points + 1)
            .map(|i| convert(a + i as i64 * step))
            .collect::<Result<_>>()?,
    ))
}
fn date(value: Value) -> Result<i32> {
    match value {
        Value::Date(v) => Ok(v),
        _ => Err(Unsupported::Precondition("simulator date")),
    }
}
fn history(host: &mut impl Host, key: u32, date: i32) -> Result<f32> {
    match host.call(Call::Service(Service::History { key, date }))? {
        Value::Missing => Ok(0.0),
        value => float(value),
    }
}
impl Owner {
    pub fn refresh(&mut self, host: &mut impl Host) -> Result<()> {
        self.live()?;
        let Some(widget) = self.widget else {
            return Ok(());
        };
        match self.kind {
            1 => self.refresh_line(host, widget),
            2 => self.refresh_bar(host, widget),
            _ => Ok(()),
        }
    }
    fn apply_axis(&self, host: &mut impl Host, automatic_max: f32, automatic_min: f32) -> Result<()> {
        let output = axis(self.axis, automatic_max, automatic_min)?;
        self.g(host, 27, vec![Arg::U32(1), Arg::U32(u32::from(output.fractional))])?;
        for (slot, value) in [(15, output.minimum), (16, output.maximum), (24, output.step)] {
            self.g(host, slot, vec![Arg::U32(1), Arg::Float(value)])?;
        }
        Ok(())
    }
    fn refresh_bar(&mut self, host: &mut impl Host, widget: Handle) -> Result<()> {
        let bar = required(host.call(Call::Window(widget, WindowOp::Query(IID_BAR)))?)?;
        let today = date(host.call(Call::Service(Service::Today))?)?;
        host.call(Call::Service(Service::MakeDate(
            today
                .checked_sub(60)
                .ok_or(Unsupported::Precondition("date underflow"))?,
        )))?;
        if !(1..=16).contains(&self.groups) {
            return Err(Unsupported::Precondition("1..16 bar groups"));
        }
        let mut vectors = vec![vec![0.0; self.series.values.len()]; self.groups as usize];
        for (i, record) in self.series.values.iter().enumerate() {
            let v = history(host, record.key, today)?;
            let group = vectors
                .get_mut(record.group as usize)
                .ok_or(Unsupported::Precondition("series bar group outside allocated groups"))?;
            group[i] = bar_value(v, record.operation, self.axis) * record.scale;
        }
        for (j, values) in vectors.into_iter().enumerate() {
            host.graph(bar, Space::Bar, 21, vec![Arg::Values(values), Arg::U32(j as u32)])?;
        }
        let automatic = float(self.g(host, 34, vec![Arg::U32(1)])?)?;
        self.apply_axis(host, automatic, 0.0)?;
        host.call(Call::Release(bar))?;
        Ok(())
    }
    fn refresh_line(&mut self, host: &mut impl Host, widget: Handle) -> Result<()> {
        let line = required(host.call(Call::Window(widget, WindowOp::Query(IID_LINE)))?)?;
        let &(points, step, label) = TIMES
            .get(self.time_scale)
            .ok_or(Unsupported::Precondition("time scale 0..5"))?;
        host.graph(line, Space::Line, 8, vec![])?;
        host.graph(line, Space::Line, 5, vec![Arg::U32(self.series.values.len() as u32)])?;
        let mut visible_count = 0u32;
        for (i, record) in self.series.values.iter_mut().enumerate() {
            if let Some(checkbox) = Self::find(host, widget, 0x04000000 + i as u32, None)? {
                let state = word(host.call(Call::Window(checkbox, WindowOp::ButtonState(1)))?)? as u8;
                record.visible = state != 0;
                self.visibility.insert(record.key, state);
            } else {
                record.visible = true;
            }
            if record.visible {
                let color = word(host.call(Call::Service(Service::Rgb(record.color)))?)?;
                host.graph(
                    line,
                    Space::Line,
                    14,
                    vec![Arg::Style(LineStyle::new(color)), Arg::U32(visible_count)],
                )?;
                visible_count += 1;
            }
        }
        let start = date(host.call(Call::Service(Service::StartDate))?)?;
        let today = date(host.call(Call::Service(Service::Today))?)?;
        let (mut a, dates) = sample_dates(today, self.time_scale)?;
        for c in dates {
            for (k, record) in self.series.values.iter().filter(|r| r.visible).enumerate() {
                let (mut v, mut v2) = (0.0, 0.0);
                if start < a {
                    v = history(host, record.key, c)?;
                    if record.operation != 0 {
                        v2 = history(host, record.second_key, c)?;
                    }
                }
                host.graph(
                    line,
                    Space::Line,
                    20,
                    vec![
                        Arg::Float(line_value(v, v2, record.operation) * record.scale),
                        Arg::U32(k as u32),
                        Arg::U32(u32::MAX),
                    ],
                )?;
            }
            a = c;
        }
        debug_assert!(step > 0);
        host.graph(line, Space::Line, 19, vec![])?;
        for (id, index) in [(label + 1, 0), (0x8a5d420d, points as u32)] {
            let Value::Text(text) = host.call(Call::Service(Service::Text(id)))? else {
                return Err(Unsupported::Precondition("axis title text"));
            };
            host.axis_title(widget, text, index)?;
        }
        let automatic_max = float(self.g(host, 25, vec![Arg::U32(1)])?)?;
        let automatic_min = float(self.g(host, 26, vec![Arg::U32(1)])?)?.min(0.0);
        self.apply_axis(host, automatic_max, automatic_min)?;
        self.g(host, 15, vec![Arg::U32(0), Arg::Float(0.0)])?;
        // §5.2 does not explicitly list the final release; the QI reference is owned here.
        host.call(Call::Release(line))?;
        Ok(())
    }
}
