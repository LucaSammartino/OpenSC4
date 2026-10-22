// SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    BufferHandle, BufferResource, Format, ImageCodec, Key, RawStream, SerialRecord,
    BUFFER_FACTORY_IID, BUFFER_TYPE, FACTORY_IID, RESOURCE_IID, SERIAL_IID, UNKNOWN_IID,
};
use std::sync::{Arc, Mutex};

pub type ResourceHandle = Arc<Mutex<BufferResource>>;
#[derive(Clone)]
pub enum FactoryOutput {
    Resource(ResourceHandle),
    Buffer(BufferHandle),
}
impl FactoryOutput {
    fn query(&self, iid: u32) -> Option<Self> {
        match self {
            Self::Resource(resource) => {
                let view = resource
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .query_interface(iid)?;
                match view.1 {
                    Some(buffer) => Some(Self::Buffer(buffer)),
                    None => Some(Self::Resource(resource.clone())),
                }
            }
            Self::Buffer(buffer) => {
                if buffer
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .query_interface(iid)
                {
                    Some(self.clone())
                } else {
                    None
                }
            }
        }
    }
    pub fn release(self) {
        match self {
            Self::Resource(resource) => {
                resource.lock().unwrap_or_else(|e| e.into_inner()).release();
            }
            Self::Buffer(buffer) => {
                buffer.lock().unwrap_or_else(|e| e.into_inner()).release();
            }
        }
    }
}

pub(super) mod sealed {
    pub trait RuntimeHooks {}
}
/// Derived runtime bodies, not foreign host callbacks. Sealed to the
/// implementations here (and test failure fixtures); raw hosts can allocate
/// buffers and perform pixel conversion, never implement SerialRead/Load.
pub trait FactoryHooks: sealed::RuntimeHooks {
    fn allocate(&mut self, ty: u32, iid: u32) -> Option<FactoryOutput>;
    fn create_type(&mut self, ty: u32, iid: u32, _a: u32, _b: u32) -> Option<FactoryOutput> {
        self.allocate(ty, iid)
    }
    fn serial_read<S: RawStream>(
        &mut self,
        resource: &ResourceHandle,
        record: &mut SerialRecord<S>,
    ) -> bool;
    fn serial_write<S: RawStream>(
        &mut self,
        resource: &ResourceHandle,
        record: &mut SerialRecord<S>,
    ) -> bool;
}
/// A foreign record may decline serial QI. On success the concrete serial
/// view is ours and owns all typed field algorithms; QI/release are primitives.
pub trait FactoryRecord {
    type Stream: RawStream;
    fn key(&self) -> Key;
    fn query_serial(&mut self) -> bool;
    fn serial(&mut self) -> &mut SerialRecord<Self::Stream>;
    fn release_serial(&mut self);
}
impl<S: RawStream> FactoryRecord for SerialRecord<S> {
    type Stream = S;
    fn key(&self) -> Key {
        self.record.key()
    }
    fn query_serial(&mut self) -> bool {
        self.query_interface(SERIAL_IID).is_some()
    }
    fn serial(&mut self) -> &mut Self {
        self
    }
    fn release_serial(&mut self) {
        self.record.release();
    }
}

pub struct ResourceFactory {
    references: Mutex<u32>,
}
impl ResourceFactory {
    /// Initial count is explicit (PR9). Mutex poisoning is recovered, not
    /// propagated through the interface. Reentrant foreign calls unsupported.
    pub fn new(references: u32) -> Self {
        Self {
            references: Mutex::new(references),
        }
    }
    pub fn add_ref(&self) -> u32 {
        let mut count = self.references.lock().unwrap_or_else(|e| e.into_inner());
        *count = count.wrapping_add(1);
        *count
    }
    pub fn release(&self) -> u32 {
        let mut count = self.references.lock().unwrap_or_else(|e| e.into_inner());
        if *count < 2 {
            0
        } else {
            *count -= 1;
            *count
        }
    }
    pub fn query_interface(&self, iid: u32) -> bool {
        if iid == FACTORY_IID || iid == UNKNOWN_IID {
            self.add_ref();
            true
        } else {
            false
        }
    }
    pub fn create_type(
        &self,
        hooks: &mut impl FactoryHooks,
        ty: u32,
        iid: u32,
        out: &mut Option<FactoryOutput>,
        a: u32,
        b: u32,
    ) -> bool {
        // §4.2 slot 4: base forwarding ignores a,b; derived overrides may
        // observe them. They are forwarded in this exact order.
        if let Some(value) = hooks.create_type(ty, iid, a, b) {
            *out = Some(value);
            true
        } else {
            false
        }
    }
    pub fn create_record(
        &self,
        hooks: &mut impl FactoryHooks,
        record: &mut impl FactoryRecord,
        iid: u32,
        out: &mut Option<FactoryOutput>,
        a: u32,
        b: u32,
    ) -> bool {
        let _guard = self.references.lock().unwrap_or_else(|e| e.into_inner());
        let key = record.key();
        let temporary = if a == 0 && b == 0 {
            hooks.allocate(key.ty, RESOURCE_IID)
        } else {
            hooks.create_type(key.ty, RESOURCE_IID, a, b)
        };
        let Some(temporary) = temporary else {
            return false;
        };
        let result = if let FactoryOutput::Resource(resource) = &temporary {
            resource
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .base
                .set_key(key);
            if read_write(hooks, resource, record, false) {
                if let Some(view) = temporary.query(iid) {
                    *out = Some(view);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        temporary.release();
        result
    }
    pub fn create(
        &self,
        hooks: &mut impl FactoryHooks,
        record: &mut impl FactoryRecord,
        iid: u32,
        out: &mut Option<FactoryOutput>,
    ) -> bool {
        self.create_record(hooks, record, iid, out, 0, 0)
    }
    pub fn read(
        &self,
        hooks: &mut impl FactoryHooks,
        resource: &ResourceHandle,
        record: &mut impl FactoryRecord,
    ) -> bool {
        let _guard = self.references.lock().unwrap_or_else(|e| e.into_inner());
        read_write(hooks, resource, record, false)
    }
    pub fn write(
        &self,
        hooks: &mut impl FactoryHooks,
        resource: &ResourceHandle,
        record: &mut impl FactoryRecord,
    ) -> bool {
        let _guard = self.references.lock().unwrap_or_else(|e| e.into_inner());
        read_write(hooks, resource, record, true)
    }
}
fn read_write(
    hooks: &mut impl FactoryHooks,
    resource: &ResourceHandle,
    record: &mut impl FactoryRecord,
    write: bool,
) -> bool {
    if !record.query_serial() {
        return false;
    }
    let result = if write {
        hooks.serial_write(resource, record.serial())
    } else {
        hooks.serial_read(resource, record.serial())
    };
    record.release_serial();
    result
}

/// File/module opening is an OS primitive only. Access bits and close ordering
/// are runtime-owned. Module None denotes the caller's own-module primitive.
pub trait RecordOpener {
    type Stream: RawStream;
    fn open_file(&mut self, path: &str, access: u16) -> Option<SerialRecord<Self::Stream>>;
    fn open_bitmap(&mut self, module: Option<usize>, id: u32)
        -> Option<SerialRecord<Self::Stream>>;
}

pub struct BufferResourceFactory<C, A> {
    pub base: Arc<ResourceFactory>,
    target: Format,
    versions: Vec<u32>,
    codec: C,
    allocate_buffer: A,
}
impl<C: ImageCodec, A: FnMut() -> Option<BufferHandle>> BufferResourceFactory<C, A> {
    pub fn new(codec: C, allocate_buffer: A, initial_references: u32) -> Self {
        Self {
            base: Arc::new(ResourceFactory::new(initial_references)),
            target: Format::new(3, 16),
            versions: vec![2],
            codec,
            allocate_buffer,
        }
    }
    pub fn target(&self) -> Format {
        self.target
    }
    pub fn set_colour_type(&mut self, target: Format) {
        self.target = target;
    }
    pub fn write_version(&self) -> u32 {
        self.versions.last().copied().unwrap_or(0)
    }
    pub fn push_write_version(&mut self, version: u32) {
        self.versions.push(version);
    }
    pub fn pop_write_version(&mut self) -> bool {
        if self.versions.len() > 1 {
            self.versions.pop();
            true
        } else {
            false
        }
    }
    pub fn query_interface(&self, iid: u32) -> bool {
        if iid == BUFFER_FACTORY_IID {
            self.base.add_ref();
            true
        } else {
            self.base.query_interface(iid)
        }
    }
    pub fn load<S: RawStream>(&mut self, record: &mut SerialRecord<S>) -> Option<BufferHandle> {
        let mut resource = BufferResource::new((self.allocate_buffer)());
        if resource.load(record, self.target, &self.codec) {
            resource.get_buffer()
        } else {
            None
        }
    }
    pub fn save<S: RawStream>(
        &mut self,
        buffer: Option<BufferHandle>,
        record: &mut SerialRecord<S>,
    ) -> bool {
        let Some(buffer) = buffer else {
            return false;
        };
        let mut resource = BufferResource::new(None);
        resource.set_buffer(Some(buffer));
        resource.save(record, self.write_version(), &self.codec)
    }
    pub fn load_file(
        &mut self,
        opener: &mut impl RecordOpener,
        path: &str,
        target: Format,
    ) -> Option<BufferHandle> {
        let mut record = opener.open_file(path, 1)?;
        let mut resource = BufferResource::new((self.allocate_buffer)());
        let result = if resource.load(&mut record, target, &self.codec) {
            resource.get_buffer()
        } else {
            None
        };
        record.raw_mut().close();
        result
    }
    pub fn save_file(
        &mut self,
        opener: &mut impl RecordOpener,
        path: &str,
        buffer: Option<BufferHandle>,
    ) -> bool {
        let Some(buffer) = buffer else {
            return false;
        };
        let Some(mut record) = opener.open_file(path, 3) else {
            return false;
        };
        let result = self.save(Some(buffer), &mut record);
        record.raw_mut().close();
        result
    }
    pub fn load_resource_id(
        &mut self,
        opener: &mut impl RecordOpener,
        module: Option<usize>,
        id: u32,
        own_module_target: Format,
    ) -> Option<BufferHandle> {
        let mut record = opener.open_bitmap(module, id)?;
        let target = if module.is_none() {
            own_module_target
        } else {
            self.target
        };
        let mut resource = BufferResource::new((self.allocate_buffer)());
        if resource.load(&mut record, target, &self.codec) {
            resource.get_buffer()
        } else {
            None
        }
    }
}
impl<C, A> sealed::RuntimeHooks for BufferResourceFactory<C, A> {}
impl<C: ImageCodec, A: FnMut() -> Option<BufferHandle>> FactoryHooks
    for BufferResourceFactory<C, A>
{
    fn allocate(&mut self, ty: u32, iid: u32) -> Option<FactoryOutput> {
        if ty != BUFFER_TYPE {
            return None;
        }
        let resource = Arc::new(Mutex::new(BufferResource::new((self.allocate_buffer)())));
        resource
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .base
            .add_ref();
        let temporary = FactoryOutput::Resource(resource);
        let result = temporary.query(iid);
        temporary.release();
        result
    }
    fn serial_read<S: RawStream>(
        &mut self,
        resource: &ResourceHandle,
        record: &mut SerialRecord<S>,
    ) -> bool {
        let mut resource = resource.lock().unwrap_or_else(|e| e.into_inner());
        if resource.base.resource_type() != BUFFER_TYPE {
            return false;
        }
        let Some((_, buffer)) = resource.query_interface(super::BUFFER_RESOURCE_IID) else {
            return false;
        };
        if let Some(buffer) = buffer {
            // A buffer supplying the resource IID is an unsupported foreign
            // derived-interface domain, not a reference on this resource.
            buffer.lock().unwrap_or_else(|e| e.into_inner()).release();
            return false;
        }
        let result = resource.load(record, self.target, &self.codec);
        resource.release();
        result
    }
    fn serial_write<S: RawStream>(
        &mut self,
        resource: &ResourceHandle,
        record: &mut SerialRecord<S>,
    ) -> bool {
        let mut resource = resource.lock().unwrap_or_else(|e| e.into_inner());
        if resource.base.resource_type() != BUFFER_TYPE {
            return false;
        }
        let Some((_, buffer)) = resource.query_interface(super::BUFFER_RESOURCE_IID) else {
            return false;
        };
        if let Some(buffer) = buffer {
            buffer.lock().unwrap_or_else(|e| e.into_inner()).release();
            return false;
        }
        let result = resource.save(record, self.write_version(), &self.codec);
        resource.release();
        result
    }
}
