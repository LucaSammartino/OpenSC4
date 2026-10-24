// SPDX-License-Identifier: GPL-3.0-or-later
//! Raw AEMS bank IO from audio §10.7; the library registrar is foreign IO.
//! Owner execution and backend lifetime: audio §10.7.1.

pub mod owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BankTgi {
    pub kind: u32,
    pub group: u32,
    pub instance: u32,
}

pub trait BankFile {
    fn size(&mut self) -> usize;
    fn read(&mut self, destination: &mut [u8]) -> bool;
    fn close(&mut self) -> bool;
    /// Reports the original unsafe read request without corrupting Rust storage.
    fn oversized_read(&mut self, allocated: &mut [u8], requested: usize) -> bool;
}

pub trait BankResources {
    type File: BankFile;
    fn resource_as_file(&mut self, tgi: BankTgi, flags: u32) -> Option<Self::File>;
}

#[derive(Debug, Default)]
pub struct AemsSoundBank {
    pub image: Vec<u8>,
    /// None is the all-zero constructor TGI; only a successful Read changes it.
    pub cached_tgi: Option<BankTgi>,
}

impl AemsSoundBank {
    /// Raw slurp: do not impose a magic/version/header validator here (§10.7).
    /// The returned boolean is IO status, not a claimed native return contract.
    pub fn read_file(&mut self, file: &mut (impl BankFile + ?Sized)) -> bool {
        let allocated = file.size();
        let read_size = file.size();
        self.image.resize(allocated, 0);
        if read_size > allocated {
            return file.oversized_read(&mut self.image, read_size);
        }
        file.read(&mut self.image[..read_size])
    }

    pub fn read_resource(&mut self, tgi: BankTgi, resources: &mut impl BankResources) -> bool {
        let Some(mut file) = resources.resource_as_file(tgi, 0) else {
            return false;
        };
        let read = self.read_file(&mut file);
        if read {
            self.cached_tgi = Some(tgi);
        }
        let closed = file.close();
        read && closed
    }
}

/// Registrar invokes the EA library boundary; it does not choose core linkage.
pub trait BankRegistrar {
    fn add_module_bank(&mut self, bank: &mut AemsSoundBank, name: &str, userdata: u64) -> i32;
}

#[derive(Debug)]
pub struct AemsEventSystem {
    pub service_id: u32,
    pub service_priority: u32,
    pub banks: Vec<AemsSoundBank>,
    pub scale_110: f32,
    pub scale_114: f32,
    pub value_118: u32,
    pub curve: Vec<(f32, f32)>,
    pub zoom: [f32; 5],
}

impl Default for AemsEventSystem {
    fn default() -> Self {
        Self {
            service_id: 0xAC1F_AE31,
            service_priority: 0x8000_0000,
            banks: Vec::new(),
            scale_110: 1.0,
            scale_114: 1.0,
            value_118: 37,
            curve: vec![(1.0, 1.0)],
            zoom: [0.0, 0.0, 0.0, 0.5, 1.0],
        }
    }
}

impl AemsEventSystem {
    pub const fn bank_tgi(instance: u32) -> BankTgi {
        BankTgi {
            kind: 0,
            group: 0x8A59_71C5,
            instance,
        }
    }

    /// Compatibility continuation. The owner maps registrar result zero to true.
    pub fn link_registered_bank(&mut self, bank: AemsSoundBank, succeeded: bool) -> Option<AemsSoundBank> {
        if succeeded {
            self.banks.push(bank);
            None
        } else {
            Some(bank)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct ScriptedFile {
        sizes: [usize; 2],
        size_calls: usize,
        read: bool,
        closed: bool,
        requested: Rc<Cell<usize>>,
        drops: Rc<Cell<u32>>,
    }
    impl Drop for ScriptedFile {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    impl BankFile for ScriptedFile {
        fn size(&mut self) -> usize {
            let size = self.sizes[self.size_calls];
            self.size_calls += 1;
            size
        }
        fn read(&mut self, destination: &mut [u8]) -> bool {
            self.requested.set(destination.len());
            self.read
        }
        fn close(&mut self) -> bool {
            self.closed
        }
        fn oversized_read(&mut self, destination: &mut [u8], requested: usize) -> bool {
            assert_eq!(destination.len(), 4);
            self.requested.set(requested);
            self.read
        }
    }
    struct ScriptedResources(Option<ScriptedFile>);
    impl BankResources for ScriptedResources {
        type File = ScriptedFile;
        fn resource_as_file(&mut self, _: BankTgi, _: u32) -> Option<ScriptedFile> {
            self.0.take()
        }
    }
    #[test]
    fn au48_two_sizes_read_close_cache_and_owned_file_destruction() {
        let requested = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let tgi = AemsEventSystem::bank_tgi(7);
        for (sizes, read, closed, cache) in [
            ([8, 4], false, true, false),
            ([8, 4], true, false, true),
            ([4, 8], true, true, true),
        ] {
            let mut bank = AemsSoundBank::default();
            let mut resources = ScriptedResources(Some(ScriptedFile {
                sizes,
                size_calls: 0,
                read,
                closed,
                requested: requested.clone(),
                drops: drops.clone(),
            }));
            assert_eq!(bank.read_resource(tgi, &mut resources), read && closed);
            assert_eq!(bank.image.len(), sizes[0]);
            assert_eq!(requested.get(), sizes[1]);
            assert_eq!(bank.cached_tgi, cache.then_some(tgi));
        }
        assert_eq!(drops.get(), 3);
    }
    struct File {
        sizes: usize,
        image: Vec<u8>,
    }
    impl BankFile for File {
        fn close(&mut self) -> bool {
            true
        }
        fn oversized_read(&mut self, _: &mut [u8], _: usize) -> bool {
            panic!("unexpected oversized read")
        }
        fn size(&mut self) -> usize {
            self.sizes += 1;
            self.image.len()
        }
        fn read(&mut self, destination: &mut [u8]) -> bool {
            destination.copy_from_slice(&self.image);
            true
        }
    }
    struct Resources {
        available: bool,
        tgi: Option<BankTgi>,
    }
    impl BankResources for Resources {
        type File = File;
        fn resource_as_file(&mut self, tgi: BankTgi, flags: u32) -> Option<File> {
            assert_eq!(flags, 0);
            self.tgi = Some(tgi);
            self.available.then(|| File {
                sizes: 0,
                image: vec![1, 2, 3],
            })
        }
    }
    #[test]
    fn bank_raw_slurp_calls_size_twice_and_never_checks_magic() {
        let mut file = File {
            sizes: 0,
            image: vec![0xFF, 0xEE, 99],
        };
        let mut bank = AemsSoundBank::default();
        assert!(bank.read_file(&mut file));
        assert_eq!(file.sizes, 2);
        assert_eq!(bank.image, [0xFF, 0xEE, 99]);
    }
    #[test]
    fn resource_cache_and_success_only_bank_linkage() {
        let mut resources = Resources {
            available: false,
            tgi: None,
        };
        let tgi = AemsEventSystem::bank_tgi(7);
        let mut bank = AemsSoundBank::default();
        assert!(!bank.read_resource(tgi, &mut resources));
        assert_eq!(bank.cached_tgi, None);
        resources.available = true;
        assert!(bank.read_resource(tgi, &mut resources));
        assert_eq!(bank.cached_tgi, Some(tgi));
        let mut system = AemsEventSystem::default();
        let bank = system.link_registered_bank(bank, false).unwrap();
        assert!(system.banks.is_empty());
        assert!(system.link_registered_bank(bank, true).is_none());
        assert_eq!(system.banks.len(), 1);
        assert_eq!((system.scale_110, system.scale_114, system.value_118), (1.0, 1.0, 37));
    }
}
