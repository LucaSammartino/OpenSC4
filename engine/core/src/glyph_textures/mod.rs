// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable owners from `docs/specs/glyph-textures.md` §§1–3.
//!
//! Tokens replace native pointers; AddRef/Release remain explicit (copying a token
//! does not acquire a reference). Storage is exclusively mutated, with ordinary
//! shared getters and no RefCell. A host can put the owner behind a Mutex.
//! Providers implement only foreign string, font, renderer and framework services.
//! No provider implements a glyph map, cache decision, owner size or lifetime.
//!
//! Missing published contracts are tracked as GT3–GT6. In particular, this module
//! does not invent bodies for the inherited/shared service slots in the ABI table.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::fonts::FontRef;

#[cfg(test)]
mod tests;

pub const IID_GLYPH_TEXTURE_MANAGER: u32 = 0x0B54_C519;
pub const SERVICE_ID: u32 = 0xABEB_5E37;
pub const SERVICE_PRIORITY: i32 = -2_000_000;
pub const RESTORE_TEXTURES: u32 = 0x8ACA_A95F;
pub const DEFAULT_CACHE_BYTES: u32 = 1_048_576;

/// Owned foreign font reference. Dropping this value releases precisely the
/// reference transferred by lookup; the provider must not transfer a borrowed one.
pub trait TextFont {
    fn text_width(&self, text: &[u8]) -> u32;
    fn height(&self) -> u32;
}

/// Adapter to the existing portable font owner; no duplicate font measurement.
pub struct CoreFont(FontRef);

impl CoreFont {
    pub fn acquire(font: &FontRef) -> Self {
        font.add_ref();
        Self(font.clone())
    }
}

impl TextFont for CoreFont {
    fn text_width(&self, text: &[u8]) -> u32 {
        self.0.lock().get_string_width(text) as u32
    }

    fn height(&self) -> u32 {
        self.0.lock().char_height() as u32
    }
}

impl Drop for CoreFont {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// An owned foreign binding reference, released by Drop. Handle zero is allowed;
/// rendering success is distinct from the first handle's value.
pub trait TextureBinding {
    fn first_texture(&self) -> u32;
}

pub struct RenderedText<B> {
    pub binding: B,
    pub texture_size: [u32; 2],
    pub glyph_size: [u32; 2],
    pub extents: [f32; 4],
}

/// Only primitive foreign operations. `source_rect` is initially (0,0,1,1) for
/// glyphs; None denotes the string call, whose rectangle is not specified.
pub trait TextureServices {
    type Font: TextFont;
    type Binding: TextureBinding;

    fn lookup_font(&mut self, font_id: u32) -> Option<Self::Font>;
    fn render_text(
        &mut self,
        font: &Self::Font,
        text: &[u8],
        source_rect: Option<[f32; 4]>,
    ) -> Option<RenderedText<Self::Binding>>;
    fn compare_strings(&self, a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering;
    fn char_length(&self, remaining: &[u8]) -> usize;
    fn binding_factory_exists(&self) -> bool;
    fn register_framework_slot(&mut self, slot: u32);
    fn subscribe(&mut self, message: u32);
    fn unsubscribe(&mut self, message: u32);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphKey {
    pub character: Vec<u8>,
    pub font_id: u32,
}

/// Tokens are local to one owner, like native pointers. Stale-token operations
/// return None/false rather than dereferencing freed native storage [choice].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringId(u64);

pub struct GlyphTextureInfo<B> {
    key: GlyphKey,
    texture_size: [u16; 2],
    glyph_size: [u16; 2],
    uv: [f32; 4],
    binding: Option<B>,
    refs: u32,
    manager: bool,
}

impl<B> GlyphTextureInfo<B> {
    pub fn character(&self) -> &[u8] {
        &self.key.character
    }

    /// Slot 3 replaces the destination, rather than appending to it.
    pub fn copy_character(&self, out: &mut Vec<u8>) {
        out.clone_from(&self.key.character);
    }

    pub fn texture_size(&self) -> [u32; 2] {
        self.texture_size.map(u32::from)
    }

    pub fn glyph_size(&self) -> [u32; 2] {
        self.glyph_size.map(u32::from)
    }

    pub fn uv(&self) -> [f32; 4] {
        self.uv
    }

    /// Borrowed slot-7 observation; GT6 asks whether native slot 7 also AddRefs.
    pub fn binding(&self) -> Option<&B> {
        self.binding.as_ref()
    }

    pub fn reference_count(&self) -> u32 {
        self.refs
    }

    pub fn has_manager(&self) -> bool {
        self.manager
    }
}

pub struct StringTextureInfo<F> {
    size: [u16; 2],
    refs: u32,
    manager: bool,
    text: Vec<u8>,
    font: F,
}

impl<F> StringTextureInfo<F> {
    pub fn size(&self) -> [u32; 2] {
        self.size.map(u32::from)
    }

    pub fn reference_count(&self) -> u32 {
        self.refs
    }

    pub fn has_manager(&self) -> bool {
        self.manager
    }

    fn release(&mut self) -> u32 {
        self.refs = self.refs.wrapping_sub(1);
        // §3: the manager-less count-1 destruction quirk is intentional.
        if self.refs == 0 || (self.refs == 1 && !self.manager) {
            0
        } else {
            self.refs
        }
    }
}

struct CacheEntry<B> {
    binding: B,
    owner: StringId,
    bytes: u32,
    used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringError {
    MissingFont,
    /// Outside the published P(n) domain (GT3); no guessed overflow result.
    UnrepresentablePowerOfTwo,
    DeletedManager,
}

/// Literal §1.5 target, including truncation only after the subtraction (§7.4).
/// GT3 asks for native intermediate precision and overflow behavior. f64 is the
/// literal real-number reading and matches both published discriminating vectors.
pub fn sweep_target(bytes: u32, budget: u32) -> u32 {
    if f64::from(bytes) / f64::from(budget) > 1.1 {
        (f64::from(bytes) - 0.1 * f64::from(budget)) as u32
    } else {
        budget
    }
}

fn padded_dimension(value: u32) -> Result<u16, StringError> {
    // GT3: perform P(width+2), then the specified 16-bit field assignment.
    value
        .checked_add(2)
        .and_then(u32::checked_next_power_of_two)
        .map(|n| n as u16)
        .ok_or(StringError::UnrepresentablePowerOfTwo)
}

/// Owns the glyph map and string cache. Ordered IDs are safe pointer surrogates;
/// key comparison is the framework string primitive, then unsigned font ID.
pub struct GlyphTextureManager<P: TextureServices> {
    services: P,
    refs: u32,
    deleted: bool,
    initialized: bool,
    glyph_map: Vec<(GlyphKey, GlyphId)>,
    glyphs: BTreeMap<GlyphId, GlyphTextureInfo<P::Binding>>,
    strings: BTreeMap<StringId, StringTextureInfo<P::Font>>,
    cache: Vec<CacheEntry<P::Binding>>,
    budget: u32,
    bytes: u32,
    last_sweep: u32,
    misses: u32,
    next_id: u64,
}

impl<P: TextureServices> GlyphTextureManager<P> {
    /// GT4: constructor reference count is not published. The caller names its
    /// existing references explicitly; this is not a claimed native default.
    pub fn new(services: P, initial_references: u32) -> Self {
        Self {
            services,
            refs: initial_references,
            deleted: false,
            initialized: false,
            glyph_map: Vec::new(),
            glyphs: BTreeMap::new(),
            strings: BTreeMap::new(),
            cache: Vec::new(),
            budget: DEFAULT_CACHE_BYTES,
            bytes: 0,
            // GT4: literal zero initial time; constructor time is not published.
            last_sweep: 0,
            misses: 0,
            next_id: 0,
        }
    }

    pub fn services(&self) -> &P {
        &self.services
    }

    pub fn reference_count(&self) -> u32 {
        self.refs
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    pub fn deleted(&self) -> bool {
        self.deleted
    }

    pub fn budget(&self) -> u32 {
        self.budget
    }

    pub fn cached_bytes(&self) -> u32 {
        self.bytes
    }

    pub fn cached_entries(&self) -> usize {
        self.cache.len()
    }

    pub fn last_sweep(&self) -> u32 {
        self.last_sweep
    }

    /// The native counter is process-wide; an adapter must accumulate this delta
    /// across managers. This portable owner records only its own misses.
    pub fn cache_misses(&self) -> u32 {
        self.misses
    }

    pub fn glyph(&self, id: GlyphId) -> Option<&GlyphTextureInfo<P::Binding>> {
        self.glyphs.get(&id)
    }

    pub fn string(&self, id: StringId) -> Option<&StringTextureInfo<P::Font>> {
        self.strings.get(&id)
    }

    pub fn glyph_keys(&self) -> impl Iterator<Item = &GlyphKey> {
        self.glyph_map.iter().map(|(key, _)| key)
    }

    pub fn cache_order(&self) -> impl Iterator<Item = (StringId, bool)> + '_ {
        self.cache.iter().map(|entry| (entry.owner, entry.used))
    }

    pub fn add_ref(&mut self) -> Option<u32> {
        if self.deleted {
            return None;
        }
        self.refs = self.refs.wrapping_add(1);
        Some(self.refs)
    }

    pub fn query_interface(&mut self, iid: u32) -> bool {
        iid == IID_GLYPH_TEXTURE_MANAGER && self.add_ref().is_some()
    }

    pub fn release(&mut self) -> Option<u32> {
        if self.deleted {
            return None;
        }
        self.refs = self.refs.wrapping_sub(1);
        if self.refs == 0 {
            self.destroy();
        }
        Some(self.refs)
    }

    /// `cache_kb` is the render-properties value, or 1024 when absent. The
    /// provider performs the two published registration calls in their order.
    pub fn init(&mut self, cache_kb: u32) -> bool {
        if !self.initialized && !self.deleted {
            self.initialized = true;
            // GT3: literal 32-bit field assignment after multiplying by 1024.
            self.budget = cache_kb.wrapping_mul(1024);
            self.services.register_framework_slot(3);
            self.services.register_framework_slot(9);
            self.services.subscribe(RESTORE_TEXTURES);
        }
        true
    }

    pub fn shutdown(&mut self) -> bool {
        if self.initialized {
            let entries = std::mem::take(&mut self.glyph_map);
            for (_, id) in entries {
                if let Some(glyph) = self.glyphs.get_mut(&id) {
                    if glyph.manager {
                        glyph.manager = false;
                        self.refs = self.refs.wrapping_sub(1);
                    }
                }
                self.release_glyph(id);
            }
            self.services.unsubscribe(RESTORE_TEXTURES);
            self.initialized = false;
        }
        // §1.2 explicitly leaves every string cache entry alone.
        true
    }

    fn allocate_id(&mut self) -> u64 {
        self.next_id = self.next_id.checked_add(1).expect("portable token space exhausted");
        self.next_id
    }

    fn find_key(&self, key: &GlyphKey) -> Result<usize, usize> {
        self.glyph_map.binary_search_by(|(existing, _)| {
            self.services
                .compare_strings(&existing.character, &key.character, true)
                .then_with(|| existing.font_id.cmp(&key.font_id))
        })
    }

    pub fn get_glyph_texture(&mut self, key: GlyphKey) -> Option<GlyphId> {
        if self.deleted {
            return None;
        }
        let insertion = match self.find_key(&key) {
            Ok(index) => {
                let id = self.glyph_map[index].1;
                self.add_ref_glyph(id);
                return Some(id);
            }
            Err(index) => index,
        };
        let font = self.services.lookup_font(key.font_id)?;
        let rendered = self.services.render_text(&font, &key.character, Some([0.0, 0.0, 1.0, 1.0]))?;
        // GT3: extents divided by the renderer's full dimensions before the
        // independent u16 field assignments; zero follows IEEE division.
        let [w, h] = rendered.texture_size;
        let [l, t, r, b] = rendered.extents;
        let info = GlyphTextureInfo {
            key: key.clone(),
            texture_size: rendered.texture_size.map(|n| n as u16),
            glyph_size: rendered.glyph_size.map(|n| n as u16),
            uv: [l / w as f32, t / h as f32, r / w as f32, b / h as f32],
            binding: Some(rendered.binding),
            refs: 2, // Map and caller; the manager link itself is counted below.
            manager: true,
        };
        let id = GlyphId(self.allocate_id());
        self.glyphs.insert(id, info);
        self.glyph_map.insert(insertion, (key, id));
        self.refs = self.refs.wrapping_add(1);
        Some(id)
    }

    pub fn add_ref_glyph(&mut self, id: GlyphId) -> Option<u32> {
        let info = self.glyphs.get_mut(&id)?;
        info.refs = info.refs.wrapping_add(1);
        Some(info.refs)
    }

    pub fn release_glyph(&mut self, id: GlyphId) -> Option<u32> {
        let info = self.glyphs.get_mut(&id)?;
        info.refs = info.refs.wrapping_sub(1);
        if info.refs == 1 && info.manager {
            // §2: remove the map's reference when it is the only one left.
            self.glyph_map.retain(|(_, existing)| *existing != id);
            info.refs = 0;
        }
        let count = info.refs;
        if count == 0 {
            let mut info = self.glyphs.remove(&id)?;
            if info.manager {
                info.manager = false;
                self.release(); // Destruction releases manager before binding (§2).
            }
            drop(info.binding.take());
        }
        Some(count)
    }

    /// Appends to the existing vector, retaining successful characters even if
    /// others fail. Character boundaries come from the framework code page.
    pub fn string_to_glyph_texture_array(&mut self, font_id: u32, text: &[u8], out: &mut Vec<GlyphId>) {
        let mut offset = 0;
        while offset < text.len() {
            let length = self.services.char_length(&text[offset..]);
            // Invalid provider boundaries have no published native contract.
            // GT6: stop safely rather than loop forever or split arbitrary bytes.
            if length == 0 || length > text.len() - offset {
                break;
            }
            let key = GlyphKey { character: text[offset..offset + length].to_vec(), font_id };
            if let Some(id) = self.get_glyph_texture(key) {
                out.push(id);
            }
            offset += length;
        }
    }

    pub fn get_cached_string_texture(&mut self, font_id: u32, text: &[u8]) -> Result<StringId, StringError> {
        if self.deleted {
            return Err(StringError::DeletedManager);
        }
        // GT6: null font behavior is not published. Return a typed error without
        // rendering or modifying owner state; successful calls still always true.
        let font = self.services.lookup_font(font_id).ok_or(StringError::MissingFont)?;
        let size = [padded_dimension(font.text_width(text))?, padded_dimension(font.height())?];
        let id = StringId(self.allocate_id());
        self.strings.insert(id, StringTextureInfo { size, refs: 1, manager: true, text: text.to_vec(), font });
        self.refs = self.refs.wrapping_add(1);
        Ok(id)
    }

    pub fn add_ref_string(&mut self, id: StringId) -> Option<u32> {
        let info = self.strings.get_mut(&id)?;
        info.refs = info.refs.wrapping_add(1);
        Some(info.refs)
    }

    pub fn release_string(&mut self, id: StringId) -> Option<u32> {
        let count = self.strings.get_mut(&id)?.release();
        if count == 0 {
            let info = self.strings.remove(&id)?;
            let manager = info.manager;
            // §3 specifies font, text, manager release ordering.
            drop(info.font);
            drop(info.text);
            if manager {
                self.release();
            }
        }
        Some(count)
    }

    /// §3 slot 4 / §1.4: a token identifies a string object, not its text content.
    pub fn string_texture(&mut self, id: StringId) -> u32 {
        if !self.strings.contains_key(&id) || self.deleted {
            return 0;
        }
        if let Some(index) = self.cache.iter().position(|entry| entry.owner == id) {
            let mut entry = self.cache.remove(index);
            entry.used = true;
            let handle = entry.binding.first_texture();
            self.cache.push(entry);
            return handle;
        }
        self.misses = self.misses.wrapping_add(1);
        let info = &self.strings[&id];
        let Some(rendered) = self.services.render_text(&info.font, &info.text, None) else {
            return 0;
        };
        // GT3: literal w*h*4 of the string owner's stored 16-bit dimensions;
        // whether native accounting instead uses render outputs is unpublished.
        let [w, h] = info.size();
        let bytes = w.wrapping_mul(h).wrapping_mul(4);
        let handle = rendered.binding.first_texture();
        self.add_ref_string(id);
        self.cache.push(CacheEntry { binding: rendered.binding, owner: id, bytes, used: true });
        self.bytes = self.bytes.wrapping_add(bytes);
        handle
    }

    fn drop_cache_entry(&mut self, index: usize) {
        let entry = self.cache.remove(index);
        self.bytes = self.bytes.wrapping_sub(entry.bytes);
        drop(entry.binding);
        self.release_string(entry.owner);
    }

    pub fn on_tick(&mut self, time: u32) {
        if self.deleted {
            return;
        }
        if time < self.last_sweep {
            self.last_sweep = time;
        }
        if time - self.last_sweep > 50 {
            self.last_sweep = time;
            if self.bytes > self.budget {
                let target = sweep_target(self.bytes, self.budget);
                let mut index = 0;
                // One oldest-to-newest pass; do not revisit just-cleared entries.
                while index < self.cache.len() && self.bytes > target {
                    if self.cache[index].used {
                        self.cache[index].used = false;
                        index += 1;
                    } else {
                        self.drop_cache_entry(index);
                    }
                }
            }
        }
    }

    pub fn do_message(&mut self, message: u32) -> bool {
        if message == RESTORE_TEXTURES && !self.deleted {
            self.clear_cache();
            if self.services.binding_factory_exists() {
                for (_, id) in &self.glyph_map {
                    let info = self.glyphs.get_mut(id).expect("live glyph map reference");
                    drop(info.binding.take()); // Must precede lookup and re-render.
                    if let Some(font) = self.services.lookup_font(info.key.font_id) {
                        if let Some(rendered) = self.services.render_text(
                            &font,
                            &info.key.character,
                            Some([0.0, 0.0, 1.0, 1.0]),
                        ) {
                            // §1.6 only replaces the binding. GT5 asks if restore
                            // also writes dimensions/UVs, or retains on failure.
                            info.binding = Some(rendered.binding);
                        }
                    }
                }
            }
        }
        true
    }

    fn clear_cache(&mut self) {
        while !self.cache.is_empty() {
            self.drop_cache_entry(0);
        }
        self.bytes = 0;
    }

    fn destroy(&mut self) {
        if self.deleted {
            return;
        }
        self.deleted = true;
        // GT4: destructor body is not published. Release owned resources in the
        // order shutdown then cache cleanup; do not claim full manager coverage.
        self.shutdown();
        self.clear_cache();
    }
}

impl<P: TextureServices> Drop for GlyphTextureManager<P> {
    fn drop(&mut self) {
        // Dropping the entire portable context is explicit host teardown, even
        // if the host abandoned tokens without issuing their native Releases.
        self.destroy();
    }
}
