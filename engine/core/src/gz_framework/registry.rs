// SPDX-License-Identifier: GPL-3.0-or-later
//! Native command-server registry, independent of the host's COM helpers.
use crate::commands::{make_lower, parse_delimited_text, server::Info};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Registry(BTreeMap<u32, Info>);

impl Registry {
    pub fn register(&mut self, id: u32, name: Option<&[u8]>, parameters: Option<&[u8]>) -> bool {
        let Some(name) = name else { return false };
        if id == 0 || self.0.contains_key(&id) {
            return false;
        }
        self.0.insert(
            id,
            Info {
                name: name.to_vec(),
                parameters: parameters.unwrap_or_default().to_vec(),
            },
        );
        true
    }
    pub fn parameter_info(&mut self, id: u32, text: Option<&[u8]>) -> bool {
        let Some(info) = self.0.get_mut(&id) else { return false };
        if let Some(text) = text.filter(|v| !v.is_empty()) {
            info.parameters = text.to_vec();
        }
        true
    }
    pub fn info(&self, id: u32) -> Option<&Info> {
        self.0.get(&id)
    }
    pub fn lookup(&self, name: &[u8]) -> u32 {
        let name = make_lower(name);
        self.0
            .iter()
            .find(|(_, i)| make_lower(&i.name) == name)
            .map_or(0, |(&id, _)| id)
    }
    pub fn parse_id(&self, text: &[u8]) -> u32 {
        let (ok, words) = parse_delimited_text(text, b' ', 1);
        if ok {
            words.first().map_or(0, |w| self.lookup(w))
        } else {
            0
        }
    }
    pub fn unregister(&mut self, id: u32) -> bool {
        self.0.remove(&id).is_some()
    }
    pub fn ids(&self) -> Vec<u32> {
        self.0.keys().copied().collect()
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_identity_and_failure_outputs() {
        let mut r = Registry::default();
        assert!(!r.register(0, Some(b"x"), None));
        assert!(!r.register(1, None, None));
        assert!(r.register(9, Some(b"Name"), Some(b"original")));
        assert!(r.register(3, Some(b"NAME"), None));
        assert!(!r.register(9, Some(b"changed"), None));
        assert_eq!(r.lookup(b"name"), 3);
        assert_eq!(r.parse_id(b"\"nAmE\""), 3);
        assert_eq!(r.parse_id(b"name argument"), 0);
        assert!(!r.parameter_info(99, Some(b"x")));
        assert!(r.parameter_info(9, Some(b"")));
        assert_eq!(r.info(9).unwrap().parameters, b"original");
        assert_eq!(r.ids(), [3, 9]);
        assert!(r.unregister(3));
        assert_eq!(r.lookup(b"name"), 9);
        r.clear();
        assert!(r.info(9).is_none());
    }
}
