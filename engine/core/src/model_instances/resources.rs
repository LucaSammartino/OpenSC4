// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const SPRITE_ANIMATION_TYPE: u32 = 0x29A5D1EC;
pub const SPRITE_TABLE_TYPE: u32 = 0x09ADCD75;
pub const IID_SPRITE_TABLE: u32 = 0x69B6F01C;
const ZOOM_SIZES: [f32; 5] = [8., 16., 32., 73., 146.];

#[derive(Clone, Debug, PartialEq)]
pub struct SpriteAnimation {
    pub instance_id: u32,
    pub header: [u32; 5],
    pub trailing_value: u32,
    pub keys: [u32; 5],
    pub source_zooms: [usize; 5],
    pub scales: [f32; 5],
}
impl SpriteAnimation {
    pub fn read(instance_id: u32, bytes: &[u8]) -> Option<Self> {
        if instance_id == 0 {
            return None;
        }
        let mut reader = Words { bytes, offset: 0 };
        if reader.next()? != SPRITE_ANIMATION_TYPE {
            return None;
        }
        let mut header = [0; 5];
        for value in &mut header {
            *value = reader.next()?;
        }
        let mut keys = [0; 5];
        for value in &mut keys {
            *value = reader.next()?;
        }
        keys[0] = 0;
        keys[1] = 0;
        for key in &mut keys {
            if *key < instance_id {
                *key = key.wrapping_add(instance_id);
            }
        }
        // MI13 [choice]: a wrapped zero cannot supply a downward source.
        // Until the interior-zero path is specified, fail rather than invent
        // a sheet/scale for it. The published I=0 failure is handled above.
        if keys.contains(&0) {
            return None;
        }
        let trailing_value = reader.next()?;
        let mut source_zooms = [0; 5];
        let mut scales = [1.; 5];
        let mut source = 4;
        source_zooms[4] = 4;
        for z in (0..4).rev() {
            if keys[z] == keys[source] {
                scales[z] = ZOOM_SIZES[z] / ZOOM_SIZES[source];
            } else {
                source = z;
            }
            source_zooms[z] = source;
        }
        if keys[4] == instance_id {
            // MI7: before a non-I source exists, leave the downward result
            // unchanged. This also handles an all-I resource without invention.
            let mut source = None;
            for z in 0..5 {
                if keys[z] != instance_id {
                    source = Some(z);
                } else if let Some(s) = source {
                    keys[z] = keys[s];
                    source_zooms[z] = s;
                    scales[z] = ZOOM_SIZES[z] / ZOOM_SIZES[s];
                }
            }
        }
        Some(Self {
            instance_id,
            header,
            trailing_value,
            keys,
            source_zooms,
            scales,
        })
    }
}
impl Interfaces for SpriteAnimation {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        match iid {
            IID_UNKNOWN | SPRITE_ANIMATION_TYPE => Some(InterfacePart::Resource),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteAnimationTable {
    block: Vec<u8>,
}
impl SpriteAnimationTable {
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let mut reader = Words { bytes, offset: 0 };
        if reader.next()? != SPRITE_TABLE_TYPE {
            return None;
        }
        let block = bytes.get(4..)?;
        let count = u32::from_le_bytes(block.get(28..32)?.try_into().ok()?);
        let expected = 32_usize.checked_add(usize::try_from(count).ok()?.checked_mul(8)?)?;
        if block.len() != expected {
            return None;
        }
        Some(Self { block: block.to_vec() })
    }
    pub fn header(&self) -> &[u8] {
        &self.block[..32]
    }
    pub fn entry_count(&self) -> u32 {
        u32::from_le_bytes(self.block[28..32].try_into().expect("validated header"))
    }
    pub fn entry(&self, index: usize) -> Option<&[u8]> {
        let offset = 32_usize.checked_add(index.checked_mul(8)?)?;
        self.block.get(offset..offset.checked_add(8)?)
    }
    pub fn block(&self) -> &[u8] {
        &self.block
    }
}
impl Interfaces for SpriteAnimationTable {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        match iid {
            IID_UNKNOWN | IID_SPRITE_TABLE => Some(InterfacePart::Resource),
            _ => None,
        }
    }
}

/// Unread resources have no invented header/entry defaults. A successful read
/// installs a complete value. [choice] Failed reads leave the old value intact;
/// this is not a generic native persistence implementation or rollback guarantee.
pub enum SpriteResource {
    Animation(Option<SpriteAnimation>),
    Table(Option<SpriteAnimationTable>),
}
impl SpriteResource {
    pub fn resource_type(&self) -> u32 {
        match self {
            Self::Animation(_) => SPRITE_ANIMATION_TYPE,
            Self::Table(_) => SPRITE_TABLE_TYPE,
        }
    }
    pub fn animation(&self) -> Option<&SpriteAnimation> {
        match self {
            Self::Animation(value) => value.as_ref(),
            _ => None,
        }
    }
    pub fn table(&self) -> Option<&SpriteAnimationTable> {
        match self {
            Self::Table(value) => value.as_ref(),
            _ => None,
        }
    }
    fn read(&mut self, instance_id: u32, bytes: &[u8]) -> bool {
        match self {
            Self::Animation(value) => {
                let Some(parsed) = SpriteAnimation::read(instance_id, bytes) else {
                    return false;
                };
                *value = Some(parsed);
                true
            }
            Self::Table(value) => {
                let Some(parsed) = SpriteAnimationTable::read(bytes) else {
                    return false;
                };
                *value = Some(parsed);
                true
            }
        }
    }
}
impl Interfaces for SpriteResource {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        let own_iid = match self {
            Self::Animation(_) => SPRITE_ANIMATION_TYPE,
            Self::Table(_) => IID_SPRITE_TABLE,
        };
        if iid == IID_UNKNOWN || iid == own_iid {
            Some(InterfacePart::Resource)
        } else {
            None
        }
    }
}

pub struct SpriteAnimationFactory;
impl SpriteAnimationFactory {
    pub fn create_instance_into(
        &self,
        resource_type: u32,
        iid: u32,
        output: &mut Option<Reference<SpriteResource>>,
    ) -> bool {
        *output = None; // unlike failed QueryInterface, creation clears output
        let Some(resource) = self.create_instance(resource_type) else {
            return false;
        };
        if resource.read().interface(iid).is_none() {
            return false;
        }
        *output = Some(resource);
        true
    }
    pub fn create_instance(&self, resource_type: u32) -> Option<Reference<SpriteResource>> {
        match resource_type {
            SPRITE_ANIMATION_TYPE => Some(Reference::new(SpriteResource::Animation(None))),
            SPRITE_TABLE_TYPE => Some(Reference::new(SpriteResource::Table(None))),
            _ => None,
        }
    }
    pub fn serial_read_into(&self, resource: &Reference<SpriteResource>, instance_id: u32, bytes: &[u8]) -> bool {
        resource.write().read(instance_id, bytes)
    }
    /// Portable create-and-read convenience, distinct from native SerialRead.
    pub fn serial_read(&self, resource_type: u32, instance_id: u32, bytes: &[u8]) -> Option<Reference<SpriteResource>> {
        let resource = self.create_instance(resource_type)?;
        if self.serial_read_into(&resource, instance_id, bytes) {
            Some(resource)
        } else {
            None
        }
    }
}
struct Words<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Words<'_> {
    fn next(&mut self) -> Option<u32> {
        let end = self.offset.checked_add(4)?;
        let value = u32::from_le_bytes(self.bytes.get(self.offset..end)?.try_into().ok()?);
        self.offset = end;
        Some(value)
    }
}
