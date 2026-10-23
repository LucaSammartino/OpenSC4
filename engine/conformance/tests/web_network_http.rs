//! web-network sections 5.1–5.4: independent runtime/notification vectors.
use fc4_core::www::{http::Client, notification::*};

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
