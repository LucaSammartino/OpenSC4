// SPDX-License-Identifier: GPL-3.0-or-later
//! SimGL image binding ownership from gz-graphics §4.4.
use std::sync::Arc;

pub trait ImageTexture: Send + Sync {}
pub trait ImageCache: Send + Sync {
    fn query_texture(&self, iid: u32) -> Option<Arc<dyn ImageTexture>>;
}
pub trait ImageSource {
    fn cache(&self) -> Option<&Arc<dyn ImageCache>>;
    fn allocate_cache(&mut self, bytes: usize) -> Option<Arc<dyn ImageCache>>;
    fn attach_cache(&mut self, cache: &Arc<dyn ImageCache>) -> bool;
}
#[derive(Default)]
pub struct ImageBinding {
    pub cache: Option<Arc<dyn ImageCache>>,
    pub texture: Option<Arc<dyn ImageTexture>>,
}
impl ImageBinding {
    pub fn set_image(&mut self, source: Option<&mut dyn ImageSource>) -> u8 {
        self.cache = None;
        let Some(source) = source else { return 0 };
        self.cache = if let Some(cache) = source.cache() {
            Some(Arc::clone(cache))
        } else if let Some(temporary) = source.allocate_cache(0x68) {
            if source.attach_cache(&temporary) {
                Some(Arc::clone(&temporary))
            } else {
                None
            }
        } else {
            None
        };
        self.texture = None;
        self.texture = self.cache.as_ref().and_then(|cache| cache.query_texture(0xeb4d101f));
        if self.texture.is_none() {
            self.cache = None;
            return 0;
        }
        1
    }
}
