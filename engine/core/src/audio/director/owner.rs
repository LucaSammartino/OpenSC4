// SPDX-License-Identifier: GPL-3.0-or-later
//! Application lifetime, audio §11.3. Handles are owned references unless noted.

pub trait Host: super::AudioDirectorHost {
    fn register_hook(&mut self, argument: u64);
    fn start_child(&mut self, director: u64, argument: u64) -> bool;
    fn configuration(&mut self, group: u32, instance: u32) -> Option<u64>;
    fn construct_reader(&mut self, file: u64);
    fn enumerate_defaults(&mut self) -> bool;
    fn destroy_reader(&mut self);
    fn create(&mut self, class: u32, iid: u32) -> Option<u64>;
    fn call(&mut self, object: u64, operation: Operation) -> bool;
    fn register_radio(&mut self, radio: u64) -> bool;
    fn query(&mut self, object: u64, iid: u32) -> Option<u64>;
    fn release(&mut self, object: u64);
    fn destroy_base(&mut self);
    fn free_outer(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    InitRadio,
    InitHandler,
    InitAmbience,
    InitSound,
    InitListener,
    InitSem,
    InitHook,
    ShutdownHook,
    ShutdownSem,
    ShutdownListener,
    ShutdownSound,
    ShutdownAmbience,
    ShutdownScape,
}

/// Base offsets 4C,50,54,58,5C,60,64,68, in that order.
#[derive(Default, Debug)]
pub struct DirectorOwner {
    pub holders: [Option<u64>; 8],
}

impl DirectorOwner {
    pub fn on_start(&mut self, argument: u64, host: &mut impl Host) -> bool {
        host.register_hook(argument);
        for name in super::ON_START_ORDER {
            let director = host.get_director(name);
            host.add_director(director);
            host.start_child(director, argument);
        }
        true
    }

    pub fn post_app_init(&mut self, host: &mut impl Host) -> bool {
        let Some(file) = host.configuration(0x8A59_71C5, 0xEA8A_1115) else {
            return false;
        };
        host.construct_reader(file);
        if !host.enumerate_defaults() {
            host.destroy_reader();
            host.release(file);
            return false;
        }
        let radio = host.create(0x5CFB_DBBF, 0x287F_B697);
        let mut result = radio.is_some_and(|r| host.call(r, Operation::InitRadio) && host.register_radio(r));
        for (index, class, iid) in [
            (3, 0xAA35_5871, 0x287F_B697),
            (6, 0x29E2_797F, 0x29E3_9DB4),
            (0, 0xBDB6_171E, 0xBB52_3BB2),
            (1, 0x69E5_00BC, 0xBB52_3BB2),
            (2, 0x6A12_F51C, 0xBB52_3BB2),
            (4, 0x4A47_DC0D, 0xBB52_3BB2),
            (5, 0xCA4A_66B9, 0xBB52_3BB2),
        ] {
            if let Some(old) = self.holders[index].take() {
                host.release(old);
            }
            self.holders[index] = host.create(class, iid);
            result &= self.holders[index].is_some();
        }
        for (index, operation) in [
            (0, Operation::InitHandler),
            (2, Operation::InitAmbience),
            (4, Operation::InitSound),
            (5, Operation::InitListener),
            (3, Operation::InitSem),
        ] {
            // Literal required-pointer contract: no invented recoverable null path.
            let object = self.holders[index].expect("required director initializer output");
            result &= host.call(object, operation);
        }
        for index in [1, 2] {
            let object = self.holders[index].expect("required director hook output");
            if let Some(hook) = host.query(object, 0x89E5_1472) {
                host.call(hook, Operation::InitHook);
                host.release(hook);
            }
        }
        if let Some(radio) = radio {
            host.release(radio);
        }
        host.destroy_reader();
        host.release(file);
        result
    }

    pub fn pre_app_shutdown(&mut self, host: &mut impl Host) -> bool {
        for (index, operation) in [
            (1, Operation::ShutdownHook),
            (0, Operation::ShutdownHook),
            (3, Operation::ShutdownSem),
            (5, Operation::ShutdownListener),
            (4, Operation::ShutdownSound),
            (2, Operation::ShutdownAmbience),
            (6, Operation::ShutdownScape),
        ] {
            if let Some(object) = self.holders[index].take() {
                if operation == Operation::ShutdownHook {
                    if let Some(hook) = host.query(object, 0x89E5_1472) {
                        host.call(hook, operation);
                        host.release(hook);
                    }
                } else {
                    host.call(object, operation);
                }
                host.release(object);
            }
        }
        true
    }

    pub fn destroy(&mut self, flags: u32, host: &mut impl Host) {
        for holder in self.holders.iter_mut().rev() {
            if let Some(object) = holder.take() {
                host.release(object);
            }
        }
        host.destroy_base();
        if flags & 1 != 0 {
            host.free_outer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Trace {
        starts: Vec<(u64, u64)>,
        missing_file: bool,
        radio_ok: bool,
        fail: Option<Operation>,
        calls: Vec<Operation>,
        releases: Vec<u64>,
        next: u64,
    }
    impl super::super::AudioDirectorHost for Trace {
        fn get_director(&mut self, _: &'static str) -> u64 {
            1
        }
        fn add_director(&mut self, _: u64) {}
        fn start_director(&mut self, _: u64) {}
        fn add_class(&mut self, _: u32, _: u32) {}
    }
    impl Host for Trace {
        fn register_hook(&mut self, _: u64) {}
        fn start_child(&mut self, director: u64, argument: u64) -> bool {
            self.starts.push((director, argument));
            false
        }
        fn configuration(&mut self, g: u32, i: u32) -> Option<u64> {
            assert_eq!((g, i), (0x8A59_71C5, 0xEA8A_1115));
            (!self.missing_file).then_some(99)
        }
        fn construct_reader(&mut self, _: u64) {}
        fn enumerate_defaults(&mut self) -> bool {
            true
        }
        fn destroy_reader(&mut self) {}
        fn create(&mut self, _: u32, _: u32) -> Option<u64> {
            self.next += 1;
            Some(self.next)
        }
        fn call(&mut self, _: u64, op: Operation) -> bool {
            self.calls.push(op);
            self.fail != Some(op)
        }
        fn register_radio(&mut self, _: u64) -> bool {
            self.radio_ok
        }
        fn query(&mut self, _: u64, _: u32) -> Option<u64> {
            None
        }
        fn release(&mut self, object: u64) {
            self.releases.push(object);
        }
        fn destroy_base(&mut self) {}
        fn free_outer(&mut self) {}
    }
    #[test]
    fn au47_configuration_failure_and_nontransactional_initializers() {
        let mut owner = DirectorOwner::default();
        let mut host = Trace {
            missing_file: true,
            ..Trace::default()
        };
        assert!(owner.on_start(77, &mut host));
        assert_eq!(host.starts, vec![(1, 77); 5]);
        assert!(!owner.post_app_init(&mut host));
        assert_eq!(host.next, 0);
        host.missing_file = false;
        assert!(!owner.post_app_init(&mut host)); // failed radio registration stays failed
        assert_eq!(host.calls.last(), Some(&Operation::InitSem));
        host.radio_ok = true;
        host.fail = Some(Operation::InitSound);
        assert!(!owner.post_app_init(&mut host));
        assert_eq!(host.calls.last(), Some(&Operation::InitSem));
        host.calls.clear();
        assert!(owner.pre_app_shutdown(&mut host));
        assert_eq!(
            host.calls,
            [
                Operation::ShutdownSem,
                Operation::ShutdownListener,
                Operation::ShutdownSound,
                Operation::ShutdownAmbience,
                Operation::ShutdownScape
            ]
        );
        host.calls.clear();
        owner.pre_app_shutdown(&mut host);
        assert!(host.calls.is_empty());
        owner.holders = [Some(1), Some(2), Some(3), Some(4), Some(5), Some(6), Some(7), Some(8)];
        host.releases.clear();
        owner.destroy(1, &mut host);
        assert_eq!(host.releases, [8, 7, 6, 5, 4, 3, 2, 1]);
    }
}
