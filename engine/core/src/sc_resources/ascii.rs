// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Error, Result};
use crate::{persistence_runtime::Key, rz_streams::GzOStream};
use std::sync::Arc;

pub fn hex(value: u32) -> String {
    format!("0x{value:08x}")
}
pub fn key(value: Key) -> String {
    format!("G:{} I:{} T:{}", hex(value.group), hex(value.instance), hex(value.ty))
}
#[derive(Clone, Copy, Debug)]
pub enum Scalar {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
}
impl Scalar {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::I8(_) => "Sint8",
            Self::U8(_) => "Uint8",
            Self::I16(_) => "Sint16",
            Self::U16(_) => "Uint16",
            Self::I32(_) => "Sint32",
            Self::U32(_) => "Uint32",
            Self::F32(_) => "Float32",
            Self::F64(_) => "Float64",
        }
    }
    fn output(self, stream: &mut dyn GzOStream) {
        match self {
            Self::Bool(v) => {
                stream.set_rz_char_str(if v { b"true" } else { b"false" });
            }
            Self::I8(v) => {
                stream.set_sint8(v);
            }
            Self::U8(v) => {
                stream.set_uint8(v);
            }
            Self::I16(v) => {
                stream.set_sint16(v);
            }
            Self::U16(v) => {
                stream.set_uint16(v);
            }
            Self::I32(v) => {
                stream.set_sint32(v);
            }
            Self::U32(v) => {
                stream.set_uint32(v);
            }
            Self::F32(v) => {
                stream.set_float32(v);
            }
            Self::F64(v) => {
                stream.set_float64(v);
            }
        }
    }
}
pub trait Serializable: Send + Sync {
    fn class(&self) -> u32;
    fn write(&self, writer: &mut AsciiWriter) -> Result<()>;
}
pub trait Dependency: Send + Sync {
    fn complete(&self);
}
#[derive(Clone)]
pub struct ResourceInfo {
    pub identity: u64,
    pub name: String,
    pub key: Key,
}
#[derive(Clone)]
pub struct DependencyInfo {
    pub index: u32,
    pub resource: ResourceInfo,
    pub completion: Arc<dyn Dependency>,
}
#[derive(Clone)]
pub struct ObjectInfo {
    pub identity: u64,
    pub index: u32,
    pub type_name: String,
    pub object: Arc<dyn Serializable>,
}
#[derive(Clone)]
pub struct Graph {
    pub root: ResourceInfo,
    pub dependencies: Vec<DependencyInfo>,
    pub objects: Vec<ObjectInfo>,
}
/// Surveying and stream provision are foreign. All formatting/order stays here.
pub trait Surveyor {
    fn graph(&self) -> Graph;
    fn stream(&mut self) -> &mut dyn GzOStream;
    fn release(&mut self);
}
pub trait SurveyorFactory {
    fn create(&mut self, class: u32, iid: u32, root: u64) -> Option<Box<dyn Surveyor>>;
}
pub struct Referent {
    pub resource_identity: u64,
    pub object_identity: u64,
    pub id: u32,
    pub type_name: String,
    pub class: u32,
}
pub trait UnknownReferent {
    /// Successful query owns one reference, balanced even if formatting fails.
    fn query(&mut self, iid: u32) -> Option<Referent>;
    fn release(&mut self);
}
#[derive(Default)]
pub struct AsciiWriter {
    references: u32,
    stack: Vec<Box<dyn Surveyor>>,
}
impl AsciiWriter {
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut bool>) -> bool {
        let Some(out) = out else { return false };
        if !matches!(iid, 1 | 0x1b0a_ab0e) {
            return false;
        }
        self.add_ref();
        *out = true;
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self) -> u32 {
        self.references = self.references.wrapping_sub(1);
        if self.references == 0 {
            self.clear();
        }
        self.references
    }
    fn clear(&mut self) {
        for mut s in self.stack.drain(..).rev() {
            s.release();
        }
    }
    pub fn output_stream(&mut self) -> Result<&mut (dyn GzOStream + '_)> {
        match self.stack.last_mut() {
            Some(s) => Ok(s.stream()),
            None => Err(Error::InvalidContext),
        }
    }
    fn text(&mut self, text: &str) -> Result<()> {
        self.output_stream()?.set_rz_char_str(text.as_bytes());
        Ok(())
    }
    pub fn scalar(&mut self, name: &str, value: Scalar) -> Result<()> {
        self.text(&format!("      {} {name} = ", value.name()))?;
        value.output(self.output_stream()?);
        self.text("\n")
    }
    /// `kind` supplies the type of an empty array, otherwise must match all values.
    pub fn array(&mut self, name: &str, kind: Scalar, values: &[Scalar]) -> Result<()> {
        if values.iter().any(|v| v.name() != kind.name()) {
            return Err(Error::InvalidContext);
        }
        self.text(&format!("      {} {name}[{}] = ", kind.name(), values.len()))?;
        for v in values {
            v.output(self.output_stream()?);
            self.text(", ")?;
        }
        self.text("\n")
    }
    pub fn string(&mut self, name: &str, bytes: &[u8], gz_string: bool) -> Result<()> {
        self.text(&format!(
            "      {} {name} = ",
            if gz_string { "RZString" } else { "RZChar*" }
        ))?;
        if gz_string {
            self.output_stream()?.set_gz_str(bytes);
        } else {
            self.output_stream()?.set_rz_char_str(bytes);
        }
        self.text("\n")
    }
    pub fn object(&mut self, _name: &str, object: Option<&dyn Serializable>) -> Result<()> {
        self.text(&format!(
            "      Object GZCLSID: {}",
            hex(object.map_or(0, Serializable::class))
        ))?;
        if let Some(object) = object {
            self.text("\n      {\n")?;
            object.write(self)?;
            self.text("      }\n\n")
        } else {
            self.text("\n")
        }
    }
    pub fn type_info(&mut self, name: &str, class: u32, version: u32) -> Result<()> {
        self.text(&format!("      TypeName: {name} GZCLSID: {} Version: ", hex(class)))?;
        self.output_stream()?.set_uint32(version);
        self.text("\n")
    }
    pub fn unknown_reference(&mut self, name: &str, object: Option<&mut dyn UnknownReferent>) -> Result<()> {
        if let Some(object) = object {
            let referent = object.query(0xbb11_a53e);
            let result = self.reference(name, referent.as_ref());
            if referent.is_some() {
                object.release();
            }
            result
        } else {
            self.reference(name, None)
        }
    }
    pub fn reference(&mut self, name: &str, referent: Option<&Referent>) -> Result<()> {
        let graph = self.stack.last().ok_or(Error::InvalidContext)?.graph();
        let prefix = format!("      cIGZPersistSerializableReferent* {name} = ");
        let Some(r) = referent else {
            return self.text(&(prefix + "0\n"));
        };
        if r.resource_identity == graph.root.identity {
            let index = graph
                .objects
                .iter()
                .find(|o| o.identity == r.object_identity)
                .ok_or(Error::InvalidContext)?
                .index;
            self.text(&prefix)?;
            self.text("Object #")?;
            self.output_stream()?.set_uint32(index);
            self.text(&format!(" {} GZCLSID: {}\n", r.type_name, hex(r.class)))
        } else {
            let dep = graph
                .dependencies
                .iter()
                .find(|d| d.resource.identity == r.resource_identity)
                .ok_or(Error::InvalidContext)?;
            self.text(&prefix)?;
            self.text("Object ID ")?;
            self.output_stream()?.set_uint32(r.id);
            self.text(&format!(" {} in dep. resource #", r.type_name))?;
            self.output_stream()?.set_uint32(dep.index);
            self.text(&format!(" {}\n", key(dep.resource.key)))
        }
    }
    fn directory(&mut self, object: &ObjectInfo) -> Result<()> {
        self.text("   Object #")?;
        self.output_stream()?.set_uint32(object.index);
        self.text(&format!(" {}\n", object.type_name))
    }
    pub fn write_root(&mut self, root: u64, factory: &mut impl SurveyorFactory) -> Result<()> {
        let surveyor = factory
            .create(0xbb19_4c78, 0x9b19_4c66, root)
            .ok_or(Error::InvalidContext)?;
        let graph = surveyor.graph();
        self.stack.push(surveyor);
        let result = self.write_graph(&graph);
        // No transactional output: errors preserve the written prefix.
        self.output_stream()?.flush();
        for dep in &graph.dependencies {
            dep.completion.complete();
        }
        self.stack.pop().expect("active surveyor").release();
        result
    }
    fn write_graph(&mut self, graph: &Graph) -> Result<()> {
        self.text(&format!("Resource {} {}\n", graph.root.name, key(graph.root.key)))?;
        for dep in &graph.dependencies {
            self.text("   Dep. Resource #")?;
            self.output_stream()?.set_uint32(dep.index);
            self.text(&format!(" {} {}\n", dep.resource.name, key(dep.resource.key)))?;
        }
        for object in &graph.objects {
            self.directory(object)?;
        }
        self.text("\n")?;
        for object in &graph.objects {
            self.directory(object)?;
            self.text("   {\n")?;
            object.object.write(self)?;
            self.text("   }\n\n")?;
        }
        self.text("\n")
    }
}
impl Drop for AsciiWriter {
    fn drop(&mut self) {
        self.clear();
    }
}
