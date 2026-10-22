// SPDX-License-Identifier: GPL-3.0-or-later
//! cS3DTextureBinding, texture-bindings.md §2.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Live(u32),
    FreeToPool,
    Destroyed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_word_wraps_without_freeing_on_add() {
        let mut b = Binding {
            textures: vec![5],
            references: u32::MAX,
            has_factory: true,
        };
        assert_eq!(b.add_ref(), Some(0));
        assert_eq!(b.textures(), &[5]);
        assert_eq!(b.release(), Some(Release::Live(u32::MAX)));
        assert_eq!(b.textures(), &[5]);
    }
}

impl Release {
    pub fn count(self) -> u32 {
        match self {
            Self::Live(n) => n,
            Self::FreeToPool => 1,
            Self::Destroyed => 0,
        }
    }
}

#[derive(Debug)]
pub struct Binding {
    textures: Vec<u32>,
    references: u32,
    has_factory: bool,
}

impl Binding {
    /// One owned initial reference; factories retain it when returning a caller reference.
    pub fn new(textures: Vec<u32>, has_factory: bool) -> Self {
        Self {
            textures,
            references: 1,
            has_factory,
        }
    }

    pub fn textures(&self) -> &[u32] {
        &self.textures
    }

    pub fn frame_count(&self) -> usize {
        self.textures.len()
    }

    pub fn reference_count(&self) -> u32 {
        self.references
    }

    pub fn add_ref(&mut self) -> Option<u32> {
        self.references = self.references.wrapping_add(1);
        Some(self.references)
    }

    pub fn release(&mut self) -> Option<Release> {
        self.references = self.references.wrapping_sub(1);
        Some(if self.has_factory && self.references == 1 {
            Release::FreeToPool
        } else if self.references == 0 {
            self.textures.clear();
            Release::Destroyed
        } else {
            Release::Live(self.references)
        })
    }
}
