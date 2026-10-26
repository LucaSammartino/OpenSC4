// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent ordinary-prop effects for the developer observer (PV17/PV18,
//! prop-developer §§3.3.1–3.3.3). No native object or generator is mutated.

use crate::dates::{jday, mdy};
use crate::prop_occupant::DateRange;
use crate::rng::RzRandomSuper;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub range: Option<DateRange>,
    pub timer: u8,
    pub chance: u8,
    pub state: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum Call {
    State(u32),
    Chance(u8),
    Timer(u8, bool),
    Range {
        interval: u32,
        duration: u32,
        month: u8,
        day: u8,
        date: u32,
    },
    Tick(u32),
}

/// State calls and changes are distinct: requesting the current state still
/// calls the setter, but creates no notification or model work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub timer_call: Option<(u8, bool)>,
    pub state_call: Option<u32>,
    pub state_changed: bool,
    pub random_draw: Option<u32>,
}

fn set_state(s: &mut State, value: u32, e: &mut Effects) {
    e.state_call = Some(value);
    e.state_changed = u32::from(s.state) != value;
    if e.state_changed {
        s.state = value as u8;
    }
}

fn timer(s: &mut State, mask: u8, on: bool, rng: &mut RzRandomSuper, e: &mut Effects) {
    e.timer_call = Some((mask, on));
    let before = s.timer & 15 == 15;
    s.timer = if on { s.timer | mask } else { s.timer & !mask };
    let after = s.timer & 15 == 15;
    if !before && after {
        let draw = rng.uniform_u32(100);
        e.random_draw = Some(draw);
        if draw < u32::from(s.chance) {
            set_state(s, 0, e);
        }
    } else if before && !after {
        set_state(s, 1, e);
    }
}

pub fn apply(s: &mut State, call: Call, rng: &mut RzRandomSuper) -> Effects {
    let mut e = Effects::default();
    match call {
        Call::State(value) => set_state(s, value, &mut e),
        Call::Chance(value) => s.chance = value,
        Call::Timer(mask, on) => timer(s, mask, on, rng, &mut e),
        Call::Range { interval: 0, .. } => s.range = None,
        Call::Range {
            interval,
            duration,
            month,
            day,
            date,
        } => {
            let duration = if duration == 0 { 1 } else { duration };
            let (_, _, year) = mdy(date);
            let mut start = jday(u32::from(month), u32::from(day), year);
            if start < date {
                start = jday(u32::from(month), u32::from(day), year.wrapping_add(1));
            }
            let end = start.wrapping_add(duration);
            s.range = Some(DateRange {
                interval,
                duration,
                start_day: start,
                end_day: end,
            });
            timer(s, 2, start <= date && date < end, rng, &mut e);
        }
        Call::Tick(date) => {
            if let Some(mut r) = s.range.filter(|r| r.interval != 0) {
                if s.timer & 2 == 0 && date >= r.start_day {
                    timer(s, 2, true, rng, &mut e);
                    r.start_day = if r.interval == 365 {
                        let (m, d, y) = mdy(r.start_day);
                        jday(m, d, y.wrapping_add(1))
                    } else {
                        r.start_day.wrapping_add(r.interval)
                    };
                } else if s.timer & 2 != 0 && date >= r.end_day {
                    timer(s, 2, false, rng, &mut e);
                    r.end_day = r.start_day.wrapping_add(r.duration);
                }
                s.range = Some(r);
            }
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::FixedSeeds;

    fn random() -> RzRandomSuper {
        RzRandomSuper::new(
            1,
            &FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            },
        )
    }
    fn ranged(timer: u8, start: u32, end: u32) -> State {
        State {
            range: Some(DateRange {
                interval: 10,
                duration: 2,
                start_day: start,
                end_day: end,
            }),
            timer,
            chance: 100,
            state: 1,
        }
    }

    #[test]
    fn pv18_01_tick_activates_once_then_deactivates_without_deletion() {
        let mut rng = random();
        let mut s = ranged(13, 100, 102);
        let e = apply(&mut s, Call::Tick(200), &mut rng);
        assert_eq!(s, ranged(15, 110, 102).with_state(0));
        assert_eq!(e.timer_call, Some((2, true)));
        assert_eq!(e.state_call, Some(0));
        assert!(e.state_changed && e.random_draw.is_some());
        let e = apply(&mut s, Call::Tick(200), &mut rng);
        assert_eq!(s, ranged(13, 110, 112));
        assert_eq!(e.timer_call, Some((2, false)));
        assert_eq!(e.state_call, Some(1));
        assert_eq!(e.random_draw, None);
    }

    impl State {
        fn with_state(mut self, state: u8) -> Self {
            self.state = state;
            self
        }
    }

    #[test]
    fn pv18_02_dates_compare_unsigned_and_wrap() {
        let mut rng = random();
        let mut s = ranged(9, 0xffff_fffc, 0xffff_fffe);
        let before = rng.raw_state();
        assert_eq!(apply(&mut s, Call::Tick(0x8000_0000), &mut rng), Effects::default());
        let e = apply(&mut s, Call::Tick(0xffff_fffc), &mut rng);
        assert_eq!(s, ranged(11, 6, 0xffff_fffe));
        assert_eq!(e.random_draw, None);
        apply(&mut s, Call::Tick(0xffff_fffe), &mut rng);
        assert_eq!(s, ranged(9, 6, 8));
        assert_eq!(rng.raw_state(), before);
    }

    #[test]
    fn pv18_03_zero_and_hundred_chance_each_consume_a_word() {
        for chance in [0, 100] {
            let mut rng = random();
            let mut expected = rng.clone();
            let draw = expected.uniform_u32(100);
            let mut s = ranged(13, 100, 102);
            s.chance = chance;
            let e = apply(&mut s, Call::Tick(100), &mut rng);
            assert_eq!(e.random_draw, Some(draw));
            assert_eq!(e.state_call, if chance == 0 { None } else { Some(0) });
            assert_eq!(rng.raw_state(), expected.raw_state());
        }
    }

    #[test]
    fn absent_or_zero_interval_tick_and_zero_interval_setter_do_not_touch_timer() {
        let mut rng = random();
        for range in [
            None,
            Some(DateRange {
                interval: 0,
                ..DateRange::default()
            }),
        ] {
            let mut s = State {
                range,
                timer: 15,
                chance: 100,
                state: 0,
            };
            let initial = s;
            let before = rng.raw_state();
            assert_eq!(apply(&mut s, Call::Tick(u32::MAX), &mut rng), Effects::default());
            assert_eq!(s, initial);
            assert_eq!(
                apply(
                    &mut s,
                    Call::Range {
                        interval: 0,
                        duration: 0,
                        month: 0,
                        day: 0,
                        date: 100
                    },
                    &mut rng
                ),
                Effects::default()
            );
            assert_eq!(s.timer, 15);
            assert_eq!(s.range, None);
            assert_eq!(rng.raw_state(), before);
        }
    }

    #[test]
    fn pv18_07_calendar_pair_and_default_duration() {
        let mut rng = random();
        let mut s = State::default();
        apply(
            &mut s,
            Call::Range {
                interval: 10,
                duration: 0,
                month: 3,
                day: 15,
                date: jday(3, 1, 2026),
            },
            &mut rng,
        );
        assert_eq!(
            s.range,
            Some(DateRange {
                interval: 10,
                duration: 1,
                start_day: jday(3, 15, 2026),
                end_day: jday(3, 16, 2026)
            })
        );
    }

    #[test]
    fn annual_recurrence_uses_calendar_year_including_invalid_leap_day() {
        let mut rng = random();
        for (m, d, y) in [(3, 1, 2027), (2, 29, 2028)] {
            let mut s = ranged(0, jday(m, d, y), jday(m, d, y) + 2);
            s.range.as_mut().unwrap().interval = 365;
            apply(&mut s, Call::Tick(jday(m, d, y)), &mut rng);
            assert_eq!(s.range.unwrap().start_day, jday(m, d, y + 1));
        }
    }

    #[test]
    fn full_state_argument_is_compared_before_byte_storage() {
        let mut s = State::default();
        let mut rng = random();
        let e = apply(&mut s, Call::State(256), &mut rng);
        assert_eq!(s.state, 0);
        assert_eq!(e.state_call, Some(256));
        assert!(e.state_changed);
        assert!(!apply(&mut s, Call::State(0), &mut rng).state_changed);
    }
}
