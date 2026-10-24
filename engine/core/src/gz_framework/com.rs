// SPDX-License-Identifier: GPL-3.0-or-later
use super::{References, Result, Token, Unsupported};
use std::collections::BTreeMap;

pub const IID: u32 = 0x4241330D;
pub trait Host: References {
    /// Both returned tokens are acquired references.
    fn acquire_framework(&mut self) -> Token;
    fn acquire_director(&mut self) -> Token;
    fn application_path(&mut self) -> String;
    fn init_director(&mut self, director: Token, com: Token, path: &str) -> bool;
    fn director_class(&mut self, director: Token, class: u32, iid: u32) -> Option<Token>;
    fn library_class(&mut self, library: Token, class: u32, iid: u32) -> Option<Token>;
    fn load_library(&mut self, name: &str) -> Option<Token>;
    fn library_director(&mut self, library: Token) -> Option<Token>;
    fn notify_com(&mut self, director: Token, com: Token);
    fn can_unload(&mut self, director: Token) -> bool;
    /// Inject the native, rate-limited dword generator, GZF2.
    fn ids(&mut self, output: &mut [u32]) -> bool;
}
#[derive(Default)]
pub struct Com {
    pub initialized: bool,
    running: u8,
    framework: Option<Token>,
    director: Option<Token>,
    libraries: BTreeMap<String, Token>,
    classes: BTreeMap<u32, Token>,
    id_lock: std::sync::Mutex<()>,
}
impl Com {
    pub fn set_service_running(&mut self, running: u8) {
        self.running = running;
    }
    pub fn service_running(&self) -> u8 {
        self.running
    }
    pub fn enumerate_libraries(&self, context: Token, callback: Option<&mut dyn FnMut(Token, Token) -> bool>) {
        if let Some(callback) = callback {
            for &library in self.libraries.values() {
                if !callback(library, context) {
                    break;
                }
            }
        }
    }
    pub fn library_array(&self, host: &mut impl References, output: Option<&mut [Token]>, count: &mut u32) -> u32 {
        let Some(output) = output else {
            *count = self.libraries.len() as u32;
            return *count;
        };
        *count = (*count).min(self.libraries.len() as u32);
        assert!(output.len() >= *count as usize, "writable output capacity required");
        for (slot, &library) in output.iter_mut().zip(self.libraries.values()).take(*count as usize) {
            host.add_ref(library);
            *slot = library;
        }
        *count
    }
    pub fn query_interface(&self, host: &mut impl References, this: Token, base: Token, iid: u32) -> Option<Token> {
        if iid == IID {
            host.add_ref(this);
            Some(this)
        } else {
            host.query(base, iid)
        }
    }
    pub fn framework(&self) -> Option<Token> {
        self.framework
    }
    pub fn real_init(&mut self, host: &mut impl Host, this: Token) -> bool {
        if self.initialized {
            return true;
        }
        // Retain acquired objects on retry; exact failure unwind is GZF2.
        self.framework.get_or_insert_with(|| host.acquire_framework());
        let d = *self.director.get_or_insert_with(|| host.acquire_director());
        let path = host.application_path();
        self.initialized = host.init_director(d, this, &path);
        self.initialized
    }
    pub fn get_class_object(&self, host: &mut impl Host, class: u32, iid: u32) -> Option<Token> {
        if let Some(&library) = self.classes.get(&class) {
            return host.library_class(library, class, iid);
        }
        self.director
            .and_then(|d| host.director_class(d, class, iid))
            .or_else(|| self.framework.and_then(|f| host.query(f, iid)))
    }
    /// Import a director's class index. Native registration callback remains GZF2.
    pub fn import_class_index(&mut self, class: u32, library: Token) {
        self.classes.insert(class, library);
    }
    pub fn add_library(&mut self, host: &mut impl Host, this: Token, name: &str) -> bool {
        if self.libraries.contains_key(name) {
            return true;
        }
        let Some(lib) = host.load_library(name) else {
            return false;
        };
        if let Some(d) = host.library_director(lib) {
            host.notify_com(d, this);
        }
        self.libraries.insert(name.to_owned(), lib);
        true
    }
    pub fn can_unload_library(&self, host: &mut impl Host, name: &str) -> Result<bool> {
        let Some(&lib) = self.libraries.get(name) else {
            return Ok(true);
        };
        let Some(d) = host.library_director(lib) else {
            return Err(Unsupported("GZF2"));
        };
        Ok(host.can_unload(d))
    }
    pub fn create_guids(&self, host: &mut impl Host, out: &mut [u32]) -> bool {
        if out.is_empty() {
            return true;
        }
        let _lock = self.id_lock.lock().unwrap();
        host.ids(out)
    }
    pub fn create_guid(&self, host: &mut impl Host) -> Option<u32> {
        let mut out = [0];
        self.create_guids(host, &mut out).then_some(out[0])
    }
    pub fn free_unused_libraries(&mut self) -> Result<()> {
        Err(Unsupported("GZF2"))
    }
    pub fn real_shutdown(&mut self) -> Result<()> {
        Err(Unsupported("GZF2"))
    }
}
