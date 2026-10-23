// SPDX-License-Identifier: GPL-3.0-or-later
//! Primitive foreign operations. Identity tokens, allocation and COM ownership use
//! the director's Host contract. No Windows APIs or platform pointers live here.
use crate::gdrivers_director::{Fault, Host, Op};

pub const LOST: u32 = 0x8876_01c2;
pub const FAIL: u32 = 0x8000_4005;
pub const OUT_OF_MEMORY: u32 = 0x8007_000e;
pub fn failed(hr: u32) -> bool {
    (hr as i32) < 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Foreign(Fault),
    /// An explicitly unproved contract, never an invented native error code.
    Unsupported(&'static str),
    /// Safe representation of a caller input which is unchecked/undefined natively.
    InvalidInput,
}
impl From<Fault> for Error {
    fn from(value: Fault) -> Self {
        Self::Foreign(value)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    RenderState,
    GetRenderState,
    StageState,
    Texture,
    Transform,
    Material,
    Clear,
    BeginScene,
    EndScene,
    Viewport,
    GetViewport,
    GetCaps,
    PixelFormat,
    RestoreAll,
    CooperativeLevel,
    CreateVertexBuffer,
    LockVertexBuffer,
    UnlockVertexBuffer,
    BufferLost,
    Draw,
    DrawIndexed,
    DrawBuffer,
    DrawIndexedBuffer,
    CreateSurface,
    AttachedSurface,
    LockSurface,
    UnlockSurface,
    RestoreSurface,
    LoadLibrary,
    UnloadLibrary,
    ResolveCreate,
    CreateDirectDraw,
    QueryDirect3D,
    EnumerateModes,
    RegisterClass,
    UnregisterClass,
    CreateWindow,
    DestroyWindow,
    PositionWindow,
    ShowWindow,
    UpdateWindow,
    ClientRect,
    ClientToScreen,
    AdapterInfo,
    MessageBox,
    CooperativeFlags,
    DisplayMode,
    CreateClipper,
    ClipperWindow,
    AttachClipper,
    Flip,
    Blt,
    PostMessage,
    PlaceholderDevice,
    TextureFormats,
}

pub trait Device: Host {
    /// HRESULT result. Inputs and outputs are immediate little-endian words;
    /// byte data is copied during the call, never retained as a Rust pointer.
    fn dx(&mut self, target: u32, call: Call, args: &[u32], data: &[u8], out: &mut [u32]) -> u32;
    /// Only unsettled spec contracts may cross this boundary. Default refuses them.
    /// RD6: descriptor/converter selection and pixel arithmetic; RD8: device/depth
    /// selection; RD9: stage constructor defaults and unnamed lifecycle helper.
    fn unresolved(&mut self, question: &'static str, _args: &[u32], _out: &mut [u32]) -> Result<()> {
        Err(Error::Unsupported(question))
    }
}
pub fn dx(host: &mut impl Device, target: u32, op: Call, args: &[u32]) -> u32 {
    host.dx(target, op, args, &[], &mut [])
}
pub fn acquire(host: &mut impl Device, token: u32) {
    if token != 0 {
        host.call(token, Op::AddRef, &[], &[], &mut []);
    }
}
pub fn release(host: &mut impl Device, token: &mut u32) {
    if *token != 0 {
        host.call(*token, Op::Release, &[], &[], &mut []);
        *token = 0;
    }
}
pub fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub const IDENTITY: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
pub fn quantize(value: f32) -> u32 {
    let v = (value + 49152.0).to_bits().wrapping_sub(0x4740_0000);
    v.wrapping_sub(v >> 8) & 255
}
pub fn color(v: [f32; 4]) -> u32 {
    quantize(v[3]) << 24 | quantize(v[0]) << 16 | quantize(v[1]) << 8 | quantize(v[2])
}
