//! HTTP §5.2–5.3 synchronous borrowed notifications and the §5.1 runtime boundary.
use super::{http::Client, Contract, Unsupported};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payload<'a> {
    Word(u32),
    Progress(&'a [u32; 3]),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message<'a> {
    pub kind: u32,
    pub object: Option<u32>,
    pub second: Payload<'a>,
    pub third: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    AddObject(u32),
    CreateMessage,
    AddMessage,
    ReleaseMessage,
    ReleaseObject(u32),
}
pub trait Host {
    fn effect(&mut self, effect: Effect);
    /// Delivery is synchronous. Neither target nor payload words are retained.
    fn deliver(&mut self, target: u32, message: Message<'_>);
}
impl Client {
    pub fn send_notification(&self, host: &mut impl Host, message: Message<'_>) {
        let Some(target) = self.target else {
            return;
        };
        if let Some(object) = message.object {
            host.effect(Effect::AddObject(object));
        }
        host.effect(Effect::CreateMessage);
        host.effect(Effect::AddMessage);
        host.deliver(target, message);
        host.effect(Effect::ReleaseMessage);
        if let Some(object) = message.object {
            host.effect(Effect::ReleaseObject(object));
        }
    }
    pub fn update(
        &self,
        host: &mut impl Host,
        client_identity: u32,
        job: u32,
        received: u32,
        expected: u32,
        previous: &mut u32,
    ) -> bool {
        if self.target.is_none() || expected as i32 <= 2048 {
            return false;
        }
        let (normalized, percent) = if expected == i32::MAX as u32 {
            (0, 50)
        } else {
            (expected, (received.wrapping_mul(100) / expected).min(100))
        };
        if percent.wrapping_sub(*previous) < 10 {
            return false;
        }
        self.send_notification(
            host,
            Message {
                kind: 0x789af323,
                object: Some(client_identity),
                second: Payload::Progress(&[received, normalized, percent]),
                third: job,
            },
        );
        *previous = percent;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub weekday: i32,
    pub day: i32,
    pub month: i32,
    pub year: i32,
    pub yearday: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeCalendar {
    pub calendar: Calendar,
    pub is_dst: i32,
}
pub trait Runtime {
    fn mktime(&mut self, calendar: RuntimeCalendar) -> u32;
    fn adjustment(&mut self) -> u32;
}
pub fn time_to_seconds(runtime: &mut impl Runtime, mut calendar: Calendar) -> u32 {
    calendar.month = calendar.month.wrapping_sub(1);
    calendar.year = calendar.year.wrapping_sub(1900);
    calendar.weekday = calendar.weekday.wrapping_sub(1);
    calendar.yearday = calendar.yearday.wrapping_sub(1);
    runtime
        .mktime(RuntimeCalendar { calendar, is_dst: 1 })
        .wrapping_sub(runtime.adjustment())
}
pub fn time_string_to_seconds(_text: &[u8]) -> Contract<u32> {
    Err(Unsupported::OwnedBody("Q13 HTTP date parser acceptance"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Recorder {
        events: Vec<Effect>,
        calls: Vec<(u32, [u32; 3], u32)>,
    }
    impl Host for Recorder {
        fn effect(&mut self, e: Effect) {
            self.events.push(e);
        }
        fn deliver(&mut self, target: u32, message: Message<'_>) {
            assert_eq!(self.events.last(), Some(&Effect::AddMessage));
            assert_eq!(message.kind, 0x789af323);
            if let Payload::Progress(words) = message.second {
                self.calls.push((target, *words, message.third));
            }
        }
    }
    #[test]
    fn http01_through_http09() {
        let mut client = Client::new(None);
        let mut host = Recorder::default();
        assert!(!client.update(&mut host, 9, 8, 1000, 10000, &mut 0));
        assert!(host.events.is_empty()); // HTTP08
        client.target = Some(7);
        for (received, expected, before, after, delivered) in [
            (1000, 2048, 0, 0, false),
            (1000, 10000, 0, 10, true),
            (1900, 10000, 10, 10, false),
            (2000, 10000, 10, 20, true),
            (123, 2147483647, 0, 50, true),
            (1000, 10000, 90, 10, true),
            (42949673, 10000, 0, 0, false),
            (100, 0x80000000, 0, 0, false),
        ] {
            let mut previous = before;
            host.calls.clear();
            host.events.clear();
            assert_eq!(
                client.update(&mut host, 9, 8, received, expected, &mut previous),
                delivered
            );
            assert_eq!(previous, after);
            if delivered {
                assert_eq!(
                    host.calls,
                    [(
                        7,
                        [received, if expected == 2147483647 { 0 } else { expected }, after],
                        8
                    )]
                );
                assert_eq!(
                    host.events,
                    [
                        Effect::AddObject(9),
                        Effect::CreateMessage,
                        Effect::AddMessage,
                        Effect::ReleaseMessage,
                        Effect::ReleaseObject(9)
                    ]
                ); // HTTP09
            } else {
                assert!(host.calls.is_empty());
                assert!(host.events.is_empty());
            }
        }
    }
    #[test]
    fn calendar_offsets_and_runtime_adjustment() {
        struct Clock;
        impl Runtime for Clock {
            fn mktime(&mut self, c: RuntimeCalendar) -> u32 {
                assert_eq!(c.is_dst, 1);
                assert_eq!(c.calendar.month, 0);
                assert_eq!(c.calendar.year, 126);
                assert_eq!(c.calendar.weekday, 0);
                assert_eq!(c.calendar.yearday, 0);
                5
            }
            fn adjustment(&mut self) -> u32 {
                10
            }
        }
        assert_eq!(
            time_to_seconds(
                &mut Clock,
                Calendar {
                    weekday: 1,
                    day: 2,
                    month: 1,
                    year: 2026,
                    yearday: 1,
                    hour: 3,
                    minute: 4,
                    second: 5
                }
            ),
            u32::MAX - 4
        );
        assert!(time_string_to_seconds(b"unqualified").is_err());
    }
}
