// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Result, Unsupported};
pub const IID: u32 = 0xE0203660;
pub const SERVICE_ID: u32 = 0x2073215D;
#[derive(Clone, Copy, Debug)]
pub enum Directory {
    Data,
    Plugin,
}
/// OS path syntax and INI location/key ambiguities are explicitly injected.
pub trait Host {
    fn application_directory(&mut self) -> String;
    fn directory_setting(&mut self, section: &str, directory: Directory, key: Option<&str>) -> Option<String>;
    fn resolve(&mut self, base: &str, path: &str) -> String;
    fn directory_exists(&mut self, path: &str) -> bool;
}
#[derive(Debug)]
pub struct FileSystem {
    plugin_name: String,
    data: String,
    plugin: String,
    data_valid: bool,
    plugin_valid: bool,
    pub use_cd_ini: bool,
    pub recursive_cd: bool,
    pub expected_cd_name: String,
    pub expected_cd_app_path: String,
}
impl Default for FileSystem {
    fn default() -> Self {
        Self {
            plugin_name: "Plugins".into(),
            data: String::new(),
            plugin: String::new(),
            data_valid: false,
            plugin_valid: false,
            use_cd_ini: false,
            recursive_cd: false,
            expected_cd_name: String::new(),
            expected_cd_app_path: String::new(),
        }
    }
}
impl FileSystem {
    pub fn query_interface(
        &self,
        host: &mut impl super::References,
        this: super::Token,
        base: super::Token,
        iid: u32,
    ) -> Option<super::Token> {
        if iid == IID {
            host.add_ref(this);
            Some(this)
        } else {
            host.query(base, iid)
        }
    }
    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
    /// Cache invalidation after this setter is not specified (GZF8).
    pub fn set_plugin_name(&mut self, name: &str) {
        self.plugin_name = name.into();
    }
    pub fn cached_data(&self) -> (&str, bool) {
        (&self.data, self.data_valid)
    }
    pub fn cached_plugin(&self) -> (&str, bool) {
        (&self.plugin, self.plugin_valid)
    }
    pub fn data_directory(&mut self, host: &mut impl Host) -> &str {
        if !self.data_valid {
            let base = host.application_directory();
            let setting = host
                .directory_setting("Directories", Directory::Data, None)
                .unwrap_or_default();
            self.data = if setting.is_empty() {
                base
            } else {
                host.resolve(&base, &setting)
            };
            self.data_valid = true;
        }
        &self.data
    }
    pub fn plugin_directory(&mut self, host: &mut impl Host) -> &str {
        if !self.plugin_valid {
            let base = host.application_directory();
            let setting = host
                .directory_setting("Directories", Directory::Plugin, Some("PlugIn"))
                .unwrap_or_default();
            self.plugin = host.resolve(
                &base,
                if setting.is_empty() {
                    &self.plugin_name
                } else {
                    &setting
                },
            );
            self.plugin_valid = true;
        }
        &self.plugin
    }
    pub fn set_data_directory(&mut self, host: &mut impl Host, path: &str) -> bool {
        self.data = path.into();
        let exists = host.directory_exists(path);
        if exists {
            self.data_valid = true;
        }
        exists
    }
    pub fn set_plugin_directory(&mut self, host: &mut impl Host, path: &str) -> bool {
        self.plugin = path.into();
        let exists = host.directory_exists(path);
        if exists {
            self.plugin_valid = true;
        }
        exists
    }
    pub fn cd_discovery(&self) -> Result<()> {
        Err(Unsupported("GZF8"))
    }
    pub fn application_components(&self) -> Result<()> {
        Err(Unsupported("GZF8"))
    }
}
