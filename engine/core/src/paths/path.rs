//! cRZPath's five owned strings and all published non-virtual methods (paths.md §2).

use super::helpers::{
    fix_directory_string, fix_drive_string_with_classifier, fix_extension_string, invariant_crt_is_alpha,
    resolve_path_with_classifier, splitpath, CrtIsAlpha, CurrentDirectory, PathParts, UnsupportedLocaleByte,
};

#[derive(Debug)]
pub struct RzPath {
    parts: PathParts,
    buffer: Vec<u8>,
    classifier: CrtIsAlpha,
}

impl Default for RzPath {
    fn default() -> Self {
        Self {
            parts: PathParts::default(),
            buffer: Vec::new(),
            classifier: invariant_crt_is_alpha,
        }
    }
}

impl RzPath {
    pub fn new(path: &[u8], provider: &mut impl CurrentDirectory) -> Result<Self, UnsupportedLocaleByte> {
        Self::with_classifier(path, provider, invariant_crt_is_alpha)
    }

    /// The callback is only the primitive thread CRT isalpha operation, never path resolution.
    pub fn with_classifier(
        path: &[u8],
        provider: &mut impl CurrentDirectory,
        classifier: CrtIsAlpha,
    ) -> Result<Self, UnsupportedLocaleByte> {
        let mut result = Self {
            classifier,
            ..Self::default()
        };
        result.set_path(path, provider)?;
        Ok(result)
    }

    pub fn set_path(
        &mut self,
        path: &[u8],
        provider: &mut impl CurrentDirectory,
    ) -> Result<bool, UnsupportedLocaleByte> {
        resolve_path_with_classifier(&mut self.buffer, path, None, provider, self.classifier)?;
        self.parts = splitpath(&self.buffer);
        Ok(true)
    }

    pub fn get_drive(&mut self) -> Result<&[u8], UnsupportedLocaleByte> {
        fix_drive_string_with_classifier(&mut self.parts.drive, self.classifier)?;
        Ok(&self.parts.drive)
    }

    pub fn get_directory(&mut self) -> &[u8] {
        fix_directory_string(&mut self.parts.directory);
        &self.parts.directory
    }

    pub fn get_file_name(&self) -> &[u8] {
        &self.parts.name
    }

    pub fn file_name(&mut self) -> &[u8] {
        if self.parts.name.is_empty() && !self.buffer.is_empty() {
            self.parts.name = splitpath(&self.buffer).name;
        }
        &self.parts.name
    }

    pub fn get_extension(&mut self) -> &[u8] {
        fix_extension_string(&mut self.parts.extension);
        &self.parts.extension
    }

    pub fn set_file_name(&mut self, name: &[u8], split: bool) -> bool {
        // PA4: literal stored-byte comparison, with no case or Unicode folding.
        if name != self.parts.name {
            self.parts.name = if split { splitpath(name).name } else { name.to_vec() };
        }
        true
    }

    pub fn set_extension(&mut self, extension: &[u8], split: bool) -> bool {
        if extension != self.get_extension() {
            if split {
                self.parts.extension = splitpath(extension).extension;
            } else {
                self.parts.extension = extension.to_vec();
                self.get_extension();
            }
        }
        true
    }

    pub fn set_buffer(&mut self, parts: u32) -> Result<(), UnsupportedLocaleByte> {
        // Portable unsupported-domain error leaves the previous join intact.
        if parts & 1 != 0 {
            self.get_drive()?;
        }
        self.buffer.clear();
        if parts & 1 != 0 {
            self.buffer.extend_from_slice(&self.parts.drive);
        }
        if parts & 2 != 0 {
            self.get_directory();
            self.buffer.extend_from_slice(&self.parts.directory);
        }
        if parts & 4 != 0 {
            self.buffer.extend_from_slice(&self.parts.name);
        }
        if parts & 8 != 0 {
            self.get_extension();
            self.buffer.extend_from_slice(&self.parts.extension);
        }
        Ok(())
    }

    pub fn get_path(&mut self, out: &mut Vec<u8>) -> Result<(), UnsupportedLocaleByte> {
        self.set_buffer(15)?;
        out.clone_from(&self.buffer);
        Ok(())
    }

    pub fn get_drive_and_directory(&mut self, out: &mut Vec<u8>) -> Result<(), UnsupportedLocaleByte> {
        self.set_buffer(3)?;
        out.clone_from(&self.buffer);
        Ok(())
    }

    pub fn get_file_name_and_extension(&mut self, out: &mut Vec<u8>) -> Result<(), UnsupportedLocaleByte> {
        self.set_buffer(12)?;
        out.clone_from(&self.buffer);
        Ok(())
    }
}
