// SPDX-License-Identifier: GPL-3.0-or-later
//! Clean-room DX7 driver state machine from renderer-driver.md. The foreign
//! backend extends gdrivers_director::Host; graphics identities use its tokens.
pub mod backend;
pub mod context;
pub mod extended;
pub mod primitive;
pub mod stage;
pub mod texture;
pub mod vertex;
pub mod auxiliary;
use crate::gdrivers_director::FOREIGN_DX7_CLASS;
use backend::*;
use context::{Context, Request};
use primitive::PrimitiveManager;
use stage::Stage;
use texture::{TextureManager, Upload};
use std::sync::{Mutex, MutexGuard};

static DRIVER_INFO: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub words: [u32; 14],
}
impl Default for Mode {
    fn default() -> Self {
        let mut words = [0; 14];
        words[0] = u32::MAX;
        words[13] = u32::MAX;
        Self { words }
    }
}
impl Mode {
    pub fn byte(&self, offset: usize) -> u8 {
        (self.words[offset / 4] >> ((offset % 4) * 8)) as u8
    }
    pub fn set_byte(&mut self, offset: usize, value: u8) {
        let shift = (offset % 4) * 8;
        self.words[offset / 4] = (self.words[offset / 4] & !(255 << shift)) | ((value as u32) << shift);
    }
    pub fn public(&self) -> [u32; 13] {
        std::array::from_fn(|i| self.words[i])
    }
}
#[derive(Debug)]
pub struct Driver {
    pub identity: u32,
    pub primitive_identity: u32,
    pub texture_identity: u32,
    pub refs: u32,
    pub error: u32,
    pub initialized: bool,
    /// Section 4.1 specifies the Init write, but not this word's purpose (RD2)
    /// or its constructor value. None preserves that construction boundary.
    pub init_word: Option<u32>,
    pub lost: bool,
    pub scene: bool,
    pub ready: bool,
    pub window: u32,
    pub device: u32,
    pub dd: u32,
    pub d3d: u32,
    /// Exact owner +22C/+230/+234 slots. Their context aliases remain RD12.
    pub auxiliary_surfaces: [u32; 3],
    pub context: Context,
    pub modes: Vec<Mode>,
    pub current: Mode,
    pub stages: Vec<Stage>,
    pub current_stage: usize,
    pub stages_dirty: bool,
    pub follow_texture: bool,
    pub textures: TextureManager,
    pub primitives: PrimitiveManager,
    pub matrix_mode: u32,
    pub projection: [f32; 16],
    pub full_viewport: [u32; 6],
    pub clear_color: u32,
    pub clear_depth: f32,
    pub clear_stencil: u32,
    pub row_length: u32,
    pub fog_capable: bool,
    pub multiplier: bool,
    pub clear_on_begin: bool,
    pub vertex_colors: [bool; 2],
    pub material: [f32; 17],
    pub lighting_enabled: u8,
    pub lights: Vec<D3DLight>,
    pub regions: Vec<auxiliary::Region>,
    pub arrays: [u32; 3],
    pub client_screen: [u32; 4],
}
impl Driver {
    pub fn new(identity: u32, primitive_identity: u32) -> Self {
        let mut primitives = PrimitiveManager::default();
        primitives.refs = 1;
        Self {
            identity,
            primitive_identity,
            texture_identity: 0,
            refs: 0,
            error: 0,
            initialized: false,
            init_word: None,
            lost: false,
            scene: false,
            ready: false,
            window: 0,
            device: 0,
            dd: 0,
            d3d: 0,
            auxiliary_surfaces: [0; 3],
            context: Context::default(),
            modes: Vec::new(),
            current: Mode::default(),
            stages: Vec::new(),
            current_stage: 0,
            stages_dirty: false,
            follow_texture: false,
            textures: TextureManager { refs: 1, ..TextureManager::default() },
            primitives,
            matrix_mode: 0,
            projection: [0.; 16],
            full_viewport: [0; 6],
            clear_color: 0,
            clear_depth: 1.,
            clear_stencil: 0,
            row_length: 0,
            fog_capable: false,
            multiplier: false,
            clear_on_begin: false,
            vertex_colors: [true; 2],
            material: [0.; 17],
            lighting_enabled: 0,
            lights: Vec::new(),
            regions: Vec::new(),
            arrays: [0; 3],
            client_screen: [0; 4],
        }
    }
    pub fn class_id(&self) -> u32 {
        FOREIGN_DX7_CLASS
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut u32>) -> Result<bool> {
        let token = match iid {
            1 | 0xa4554849 => self.identity.wrapping_add(12),
            0x669565fe => self.identity,
            0xe69bfe2a => self.identity.wrapping_add(4),
            0x87e2b87d => self.identity.wrapping_add(8),
            0x09cd86f9 => self.primitive_identity,
            _ => return Ok(false),
        };
        *out.ok_or(Error::Foreign(crate::gdrivers_director::Fault::NullWrite))? = token;
        if iid == 0x09cd86f9 {
            self.primitives.refs = self.primitives.refs.wrapping_add(1);
        } else {
            self.add_ref();
        }
        Ok(true)
    }
    pub fn get_error(&mut self) -> u32 {
        std::mem::take(&mut self.error)
    }
    pub fn is_device_ready(&self) -> bool {
        !self.lost
    }
    pub fn get_boolean(&self, _pname: u32, _out: &mut bool) {}
    pub fn get_float(&self, _pname: u32, _out: &mut f32) {}
    pub fn tail(&self, _a: u32, _b: u32) -> u32 {
        0
    }
    pub fn get_integer(&self, pname: u32, out: &mut u32) {
        if matches!(pname, 0 | 7 | 8) {
            *out = self.row_length;
        }
    }
    pub fn pixel_store(&mut self, pname: u32, value: u32) {
        if pname == 0 {
            self.row_length = value;
        }
    }
    pub fn driver_info(&self) -> MutexGuard<'static, Vec<u8>> {
        // Shared borrowed storage; the guard prevents concurrent mutation while read.
        DRIVER_INFO.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn hr(&mut self, hr: u32) {
        if failed(hr) {
            self.error = u32::from(hr != LOST);
        }
    }
    fn submit(&mut self, host: &mut impl Device) {
        let hr = self.primitives.flush(host);
        self.hr(hr);
    }
    fn rs(&mut self, state: u32, value: u32, checked: bool, host: &mut impl Device) {
        let hr = dx(host, self.device, Call::RenderState, &[state, value]);
        if checked {
            self.hr(hr);
        }
    }
    pub fn init(&mut self, host: &mut impl Device) -> Result<()> {
        self.error = 0;
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        self.window = 0;
        self.init_word = Some(7);
        self.current.words[0] = u32::MAX;
        self.modes.clear();
        let hr = host.dx(0, Call::RegisterClass, &[3], b"GDriverClass (DirectX)", &mut []);
        if failed(hr) {
            self.error = 6;
            return Ok(());
        }
        let mut out = [0];
        let hr = host.dx(0, Call::CreateDirectDraw, &[], &[], &mut out);
        if failed(hr) {
            host.dx(
                0,
                Call::MessageBox,
                &[],
                b"Hardware Driver Error\0Could not initialize Direct Draw.",
                &mut [],
            );
            self.error = 6;
            return Ok(());
        }
        // Backend exposes the description (address,length), four version words,
        // GUID bytes address and PCI identifiers as immediate adapter properties.
        let mut adapter = [0; 11];
        let hr = host.dx(out[0], Call::AdapterInfo, &[], &[], &mut adapter);
        *self.driver_info() = b"Maxis 3D GDriver DirectX 2.0\n".to_vec();
        let result = if hr != 0 {
            Ok(())
        } else {
            self.build_info(adapter, host)
        };
        release(host, &mut out[0]);
        result
    }
    pub fn count_modes(&mut self, host: &mut impl Device) -> u32 {
        if !self.modes.is_empty() {
            return self.modes.len() as u32;
        }
        let mut lib = [0];
        if failed(host.dx(0, Call::LoadLibrary, &[], b"DDRAW.DLL", &mut lib)) || lib[0] == 0 {
            return 0;
        }
        let mut entry = [0];
        let mut dd = [0];
        if !failed(host.dx(lib[0], Call::ResolveCreate, &[], b"DirectDrawCreateEx", &mut entry))
            && entry[0] != 0
            && !failed(host.dx(entry[0], Call::CreateDirectDraw, &[], &[], &mut dd))
        {
            let mut count = [0];
            host.dx(dd[0], Call::EnumerateModes, &[], &[], &mut count);
            for i in 0..count[0] {
                let mut mode = [0; 3];
                host.dx(dd[0], Call::EnumerateModes, &[i], &[], &mut mode);
                for fullscreen in [1, 0] {
                    let mut r = Mode::default();
                    r.words[0] = self.modes.len() as u32;
                    r.words[1..4].copy_from_slice(&mode);
                    r.set_byte(0x20, fullscreen);
                    r.set_byte(0x22, 1);
                    self.modes.push(r);
                }
            }
            release(host, &mut dd[0]);
        }
        dx(host, lib[0], Call::UnloadLibrary, &[]);
        self.modes.len() as u32
    }
    pub fn mode_info(&mut self, index: u32, out: &mut [u32; 13]) {
        if let Some(m) = self.modes.get(index as usize) {
            *out = m.public();
        } else {
            self.error = 2;
        }
    }
    pub fn current_mode(&self) -> [u32; 13] {
        self.current.public()
    }
    pub fn end_scene(&mut self, host: &mut impl Device) {
        if self.scene && self.device != 0 {
            let hr = dx(host, self.device, Call::EndScene, &[]);
            self.hr(hr);
            self.scene = false;
        }
    }
    pub fn begin_scene(&mut self, host: &mut impl Device) -> Result<()> {
        if self.scene || self.device == 0 {
            return Ok(());
        }
        self.error = 0;
        let hr = dx(host, self.device, Call::BeginScene, &[]);
        self.scene = !failed(hr) || hr == 0x887602f8;
        self.hr(hr);
        for s in &mut self.stages {
            s.reset(host)?;
        }
        self.stages_dirty = true;
        if self.clear_on_begin {
            self.clear(0x4000, host);
        }
        Ok(())
    }
    pub fn destroy_viewport(&mut self, host: &mut impl Device) {
        self.end_scene(host);
        for i in 0..self.stages.len() {
            dx(host, self.device, Call::Texture, &[i as u32, 0]);
        }
        self.stages.clear();
        self.textures.shutdown(host);
        self.primitives.stop(host);
        for token in [&mut self.device, &mut self.dd, &mut self.d3d] { release(host, token); }
        for token in &mut self.auxiliary_surfaces { release(host, token); }
        self.context.destroy(host);
        self.device = 0;
        self.ready = false;
    }
    pub fn set_video_mode(
        &mut self,
        index: u32,
        procedure: u32,
        show: bool,
        flag: u32,
        host: &mut impl Device,
    ) -> Result<()> {
        self.end_scene(host);
        if index == u32::MAX {
            if self.current.words[0] != u32::MAX && self.window != 0 {
                self.destroy_viewport(host);
                dx(host, self.window, Call::DestroyWindow, &[]);
                self.window = 0;
                self.current.words[0] = u32::MAX;
            }
            return Ok(());
        }
        if index as usize >= self.modes.len() {
            self.error = 2;
            return Ok(());
        }
        if self.current.words[0] != u32::MAX {
            self.set_video_mode(u32::MAX, 0, false, 0, host)?;
        }
        self.current = self.modes[index as usize];
        let mut window = [0];
        let hr = host.dx(
            0,
            Call::CreateWindow,
            &[
                self.current.words[1],
                self.current.words[2],
                self.current.byte(0x20) as u32,
                procedure,
                self.identity,
            ],
            b"GDriverClass (DirectX)\0GDriverWindow (DirectX)",
            &mut window,
        );
        self.window = if failed(hr) { 0 } else { window[0] };
        if self.window != 0 {
            dx(
                host,
                self.window,
                Call::PositionWindow,
                &[0, 0, self.current.words[1], self.current.words[2]],
            );
        }
        let setup = if self.window != 0 {
            self.setup(flag as u8, host)
        } else {
            Ok(false)
        };
        let success = matches!(setup, Ok(true));
        if !success {
            self.destroy_viewport(host);
            if self.window != 0 {
                dx(host, self.window, Call::DestroyWindow, &[]);
                self.window = 0;
            }
            self.error = 6;
            if flag == 0 {
                return Err(Error::Foreign(crate::gdrivers_director::Fault::NullWrite));
            }
            host.write(flag, &u32::MAX.to_le_bytes())?;
            setup?;
        } else if show {
            dx(host, self.window, Call::ShowWindow, &[]);
            dx(host, self.window, Call::UpdateWindow, &[]);
        }
        Ok(())
    }
    fn setup(&mut self, flip: u8, host: &mut impl Device) -> Result<bool> {
        self.end_scene(host);
        let mut client = [0; 4];
        host.dx(self.window, Call::ClientRect, &[], &[], &mut client);
        let (context, hr) = Context::create(
            Request {
                selector: self.current.words[13],
                fullscreen: self.current.byte(0x20) != 0,
                flip,
                window: self.window,
                width: client[2].wrapping_sub(client[0]),
                height: client[3].wrapping_sub(client[1]),
                back_buffers: if self.current.byte(0x22) != 0 { 2 } else { 0 },
            },
            host,
        )?;
        self.context = context;
        self.device = self.context.device;
        self.dd = self.context.dd;
        self.d3d = self.context.d3d;
        for token in [self.device, self.dd, self.d3d] { acquire(host, token); }
        if failed(hr) {
            self.hr(hr);
            host.dx(
                0,
                Call::MessageBox,
                &[],
                b"Hardware Driver Error\0Could not initialize graphics card.",
                &mut [],
            );
            return Ok(false);
        }
        if self.device == 0 {
            host.dx(0, Call::MessageBox, &[], b"Internal Error.", &mut []);
            return Ok(false);
        }
        // First device word, stencil operations, maximum simultaneous textures,
        // words +0x44/+0x5c, and the fog capability word (bit 4).
        let mut caps = [0; 6];
        let hr = host.dx(self.device, Call::GetCaps, &[], &[], &mut caps);
        if failed(hr) {
            self.hr(hr);
            return Ok(false);
        }
        if caps[0] & 0x80000 == 0 {
            host.dx(0,Call::MessageBox,&[],b"Driver does not support DirectX. This may mean you have a Microsoft driver installed instead of the vendor driver...",&mut []);
            return Ok(false);
        }
        self.fog_capable = caps[5] & 16 != 0;
        host.dx(self.device, Call::GetViewport, &[], &[], &mut self.full_viewport);
        self.matrix_mode = 0;
        self.load_matrix(IDENTITY, host);
        let w = self.full_viewport[2] as f32;
        let h = self.full_viewport[3] as f32;
        let mut projection = IDENTITY;
        projection[0] = 2. / w;
        projection[5] = -2. / h;
        projection[10] = -1.;
        projection[12] = -1.;
        projection[13] = 1.;
        self.matrix_mode = 1;
        self.load_matrix(projection, host);
        self.matrix_mode = 0;
        self.clear(0x7000, host);
        let mut masks = [0; 4];
        let hr = host.dx(self.context.target, Call::PixelFormat, &[], &[], &mut masks);
        if failed(hr) {
            self.hr(hr);
            return Ok(false);
        }
        self.current.words[4..8].copy_from_slice(&masks);
        self.current.set_byte(0x21, u8::from(caps[1] != 0));
        self.current.words[9] = (self.current.words[9] & 0xffff0000) | (caps[2] & 65535);
        for (offset, value) in [
            (0x28, u8::from(caps[2] > 1)),
            (0x29, 1),
            (0x30, 1),
            (0x2a, 0),
            (0x2d, 0),
            (0x2e, 0),
            (0x2b, ((caps[3] >> 7) & 1) as u8),
            (0x2f, (caps[4] & 1) as u8),
        ] {
            self.current.set_byte(offset, value);
        }
        self.modes[self.current.words[0] as usize] = self.current;
        self.stages = vec![Stage::default(); (caps[2] & 65535) as usize];
        for s in &mut self.stages {
            s.reset(host)?;
        }
        self.textures.init(self.context.dd, self.device, host)?;
        self.primitives
            .init(self.context.dd, self.context.d3d, self.device, host);
        self.default_state(host);
        self.client_screen = client;
        for pair in self.client_screen.chunks_mut(2) {
            let args = [pair[0], pair[1]];
            host.dx(self.window, Call::ClientToScreen, &args, &[], pair);
        }
        self.ready = true;
        self.begin_scene(host)?;
        Ok(true)
    }
    pub fn shutdown(&mut self, host: &mut impl Device) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        self.initialized = false;
        self.set_video_mode(u32::MAX, 0, false, 0, host)?;
        self.modes.clear();
        dx(host, 0, Call::UnregisterClass, &[]);
        self.context.destroy(host);
        Ok(())
    }
    pub fn flush(&mut self, host: &mut impl Device) -> Result<()> {
        if self.context.dd == 0 && !self.lost {
            return Ok(());
        }
        let hr = dx(host, self.context.dd, Call::CooperativeLevel, &[]);
        if self.lost {
            if failed(hr) || failed(self.context.restore(host)?) {
                return Ok(());
            }
            let objects = [self.context.device, self.context.d3d, self.context.dd];
            for token in objects { acquire(host, token); }
            release(host, &mut self.device);
            release(host, &mut self.d3d);
            release(host, &mut self.dd);
            self.device = objects[0];
            self.d3d = objects[1];
            self.dd = objects[2];
            self.lost = false;
            self.default_state(host);
            self.primitives.init(objects[2], objects[1], objects[0], host);
            self.textures.init_device(objects[0], host)?;
            dx(host, 0, Call::PostMessage, &[0x8acaa95f]);
            dx(host, 0, Call::PostMessage, &[0x6acb92a9]);
            return Ok(());
        }
        if failed(hr) {
            self.lost = true;
            dx(host, 0, Call::PostMessage, &[0xaacb92a3]);
            self.textures.lose(host);
            self.primitives.stop(host);
            release(host, &mut self.device);
            release(host, &mut self.d3d);
            release(host, &mut self.dd);
            let mut placeholder = [0];
            host.dx(0, Call::PlaceholderDevice, &[], &[], &mut placeholder);
            self.device = placeholder[0];
            self.scene = false;
            return Ok(());
        }
        if self.device == 0 {
            return Ok(());
        }
        self.submit(host);
        self.end_scene(host);
        let hr = self.context.present(0, host);
        self.hr(hr);
        if failed(dx(host, self.primitives.buffer.token, Call::BufferLost, &[])) {
            let cap = self.primitives.buffer.capacity;
            self.primitives
                .buffer
                .recreate(cap, self.context.dd, self.context.d3d, host);
        }
        self.begin_scene(host)
    }
    pub fn default_state(&mut self, host: &mut impl Device) {
        for (state, value) in [
            (22, 1),
            (26, 1),
            (41, 0),
            (29, 0),
            (27, 0),
            (4, 1),
            (14, 1),
            (23, 4),
            (15, 0),
            (28, 0),
            (8, 3),
            (52, 0),
            (53, 1),
            (54, 6),
            (55, 1),
            (56, 8),
            (57, 1),
            (58, 1),
            (59, 1),
            (60, u32::MAX),
            (139, 0),
            (137, 0),
            (147, 1),
            (145, 1),
            (146, 0),
            (141, 1),
        ] {
            self.rs(state, value, false, host);
        }
        self.multiplier = false;
        self.clear_on_begin = false;
        self.vertex_colors = [true; 2];
        for (state, value) in [
            (1, 2),
            (12, 1),
            (11, 0),
            (16, 1),
            (17, 1),
            (18, 1),
            (2, 2),
            (3, 0),
            (4, 2),
            (5, 2),
            (6, 0),
        ] {
            dx(host, self.device, Call::StageState, &[0, state, value]);
        }
        dx(host, self.device, Call::StageState, &[1, 1, 1]);
        dx(host, self.device, Call::Texture, &[0, 0]);
        self.stages_dirty = true;
        for s in &mut self.stages {
            s.dirty = true;
        }
    }
    /// Immediate state slots 12–23,67. `args` hold integers or IEEE float bits;
    /// ClearDepth has a typed entry below to preserve double-to-single rounding.
    pub fn state(&mut self, slot: u8, args: &[u32], host: &mut impl Device) -> Result<()> {
        if matches!(slot, 18..=20) && self.current.byte(0x21) == 0 {
            self.error = 3;
            return Ok(());
        }
        self.submit(host);
        let a = |n: usize| args.get(n).copied().ok_or(Error::InvalidInput);
        let checked = matches!(slot, 18..=20 | 67);
        let values = match slot {
            12 => {
                self.clear_color = color([
                    f32::from_bits(a(0)?),
                    f32::from_bits(a(1)?),
                    f32::from_bits(a(2)?),
                    f32::from_bits(a(3)?),
                ]);
                Vec::new()
            }
            14 => {
                self.clear_stencil = a(0)?;
                Vec::new()
            }
            15 => Vec::new(),
            16 => vec![(23, compare(a(0)?)?)],
            17 => vec![(14, a(0)? & 255)],
            18 => vec![(58, a(2)?), (56, compare(a(0)?)?), (57, a(1)?)],
            19 => vec![(59, a(0)?)],
            20 => vec![(53, stencil(a(0)?)?), (54, stencil(a(1)?)?), (55, stencil(a(2)?)?)],
            21 => vec![(19, blend(a(0)?)?), (20, blend(a(1)?)?)],
            22 => vec![(25, compare(a(0)?)?), (24, quantize(f32::from_bits(a(1)?)))],
            23 => vec![(9, *[1, 2].get(a(0)? as usize).ok_or(Error::InvalidInput)?)],
            67 => vec![(47, a(0)?)],
            _ => return Err(Error::InvalidInput),
        };
        for (state, value) in values {
            self.rs(state, value, checked, host);
        }
        Ok(())
    }
    pub fn clear_depth(&mut self, value: f64, host: &mut impl Device) {
        self.submit(host);
        self.clear_depth = value as f32;
    }
    pub fn clear(&mut self, mask: u32, host: &mut impl Device) {
        self.submit(host);
        let flags = u32::from(mask & 0x4000 != 0)
            | (u32::from(mask & 0x1000 != 0) * 2)
            | (u32::from(mask & 0x2000 != 0 && self.current.byte(0x21) != 0) * 4);
        dx(
            host,
            self.device,
            Call::Clear,
            &[flags, self.clear_color, self.clear_depth.to_bits(), self.clear_stencil],
        );
    }
    pub fn capability(&mut self, cap: u32, enabled: bool, host: &mut impl Device) -> Result<()> {
        if cap > 7 {
            return Err(Error::InvalidInput);
        }
        if cap == 7 || cap == 6 && !self.fog_capable {
            return Ok(());
        }
        if cap == 5 {
            let s = self.stage_mut()?;
            s.enabled = enabled;
            s.dirty = true;
            self.stages_dirty = true;
            return Ok(());
        }
        // Section 5.4: Enable never touches the error, including queue failures.
        // Disable caps 0..4 replaces it using only the state-call result below.
        self.primitives.flush(host);
        let state = [15, 7, 52, 22, 27, 0, 28][cap as usize];
        let value = if cap == 3 {
            if enabled {
                2
            } else {
                1
            }
        } else {
            u32::from(enabled)
        };
        let hr = dx(host, self.device, Call::RenderState, &[state, value]);
        if !enabled && cap < 5 {
            self.error = u32::from(failed(hr) && hr != LOST);
        }
        Ok(())
    }
    pub fn is_enabled(&self, cap: u32, host: &mut impl Device) -> Result<bool> {
        if self.lost {
            return Ok(false);
        }
        if cap > 7 {
            return Err(Error::InvalidInput);
        }
        if cap == 7 {
            return Ok(true);
        }
        if cap == 5 {
            return Ok(self.stages.get(self.current_stage).ok_or(Error::InvalidInput)?.enabled);
        }
        let mut out = [0];
        host.dx(
            self.device,
            Call::GetRenderState,
            &[[15, 7, 52, 22, 27, 0, 0][cap as usize]],
            &[],
            &mut out,
        );
        Ok(if cap == 3 { out[0] != 1 } else { out[0] == 1 })
    }
    pub fn interleaved_arrays(&mut self, format: u32, stride: u32, pointer: u32) {
        self.arrays = [
            format,
            if stride == 0 { vertex::stride(format) } else { stride },
            pointer,
        ];
    }
    fn apply_stages(&mut self, host: &mut impl Device) -> Result<()> {
        if self.stages_dirty {
            self.submit(host);
            for (i, s) in self.stages.iter_mut().enumerate() {
                if s.dirty {
                    s.apply(i as u32, self.device, &self.textures, host)?;
                }
            }
            self.stages_dirty = false;
        }
        Ok(())
    }
    pub fn draw_arrays(&mut self, mode: u32, first: u32, count: i32, host: &mut impl Device) -> Result<()> {
        if !self.primitives.started {
            return Ok(());
        }
        self.apply_stages(host)?;
        if count < 1 {
            return Ok(());
        }
        let input: Vec<u16> = (0..count as u32).map(|i| i as u16).collect();
        let (primitive, indices, declared) = primitive::expanded_writes(mode, &input, count as usize, false)?;
        let indices = indices.map(|mut v| { v.truncate(declared); v });
        if let Some(v) = &indices {
            self.primitives.indices(v.len(), host);
        }
        let hr = self.primitives.queue(
            self.arrays[0],
            primitive,
            self.arrays[1],
            0,
            count as u32,
            self.arrays[2].wrapping_add(first.wrapping_mul(self.arrays[1])),
            indices,
            host,
        )?;
        self.hr(hr);
        Ok(())
    }
    pub fn draw_elements(
        &mut self,
        mode: u32,
        count: i32,
        kind: u32,
        address: u32,
        host: &mut impl Device,
    ) -> Result<()> {
        if !self.primitives.started {
            return Ok(());
        }
        self.apply_stages(host)?;
        if count < 1 {
            return Ok(());
        }
        let size = match kind {
            1 => 1,
            3 => 2,
            5 => 4,
            _ => return Err(Error::InvalidInput),
        };
        let logical = count as usize;
        let readable = if mode == 6 { ((3 * (logical / 2)) as u16 as usize + 5) / 6 * 4 }
            else if mode == 7 { ((logical.saturating_sub(2) + 1) / 2) * 2 + 2 }
            else { logical };
        let mut bytes = vec![0; readable.max(logical) * size];
        host.read(address, &mut bytes)?;
        let input: Vec<u16> = bytes
            .chunks(size)
            .map(|b| match size {
                1 => b[0] as u16,
                2 => u16::from_le_bytes([b[0], b[1]]),
                _ => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u16,
            })
            .collect();
        let minimum = *input[..logical].iter().min().unwrap() as u32;
        let maximum = *input[..logical].iter().max().unwrap() as u32;
        let (primitive, expanded, declared) = primitive::expanded_writes(mode, &input, logical, true)?;
        let mut indices = expanded.unwrap_or(input);
        indices.truncate(declared);
        self.primitives.indices(indices.len(), host);
        let hr = self.primitives.queue(
            self.arrays[0],
            primitive,
            self.arrays[1],
            minimum,
            maximum - minimum + 1,
            self.arrays[2],
            Some(indices),
            host,
        )?;
        self.hr(hr);
        Ok(())
    }
    pub fn load_matrix(&mut self, m: [f32; 16], host: &mut impl Device) {
        self.primitives.flush(host);
        if self.matrix_mode == 0 {
            host.dx(self.device, Call::Transform, &[1], &floats(&m), &mut []);
            let mut view = IDENTITY;
            view[10] = -1.;
            host.dx(self.device, Call::Transform, &[2], &floats(&view), &mut []);
        } else if self.matrix_mode == 1 {
            self.projection = m;
            let mut viewport = [0; 6];
            host.dx(self.device, Call::GetViewport, &[], &[], &mut viewport);
            let p = extended::projection(m, viewport[2], viewport[3]);
            host.dx(self.device, Call::Transform, &[3], &floats(&p), &mut []);
        }
    }
    pub fn load_identity(&mut self, host: &mut impl Device) {
        self.load_matrix(IDENTITY, host);
    }
    pub fn matrix_mode(&mut self, mode: u32) {
        self.matrix_mode = mode;
    }
    pub fn set_viewport(&mut self, rect: Option<[u32; 4]>, host: &mut impl Device) {
        self.submit(host);
        let mut v = self.full_viewport;
        if let Some([x, y, w, h]) = rect {
            let x = x.min(v[2]);
            let y = y.min(v[3]);
            let w = if x.wrapping_add(w) > v[2] { v[2] - x } else { w };
            let h = if y.wrapping_add(h) > v[3] { v[3] - y } else { h };
            v = [x, v[3].wrapping_sub(y).wrapping_sub(h), w, h, v[4], v[5]];
        }
        let hr = dx(host, self.device, Call::Viewport, &v);
        self.hr(hr);
        let mode = self.matrix_mode;
        self.matrix_mode = 1;
        self.load_matrix(self.projection, host);
        self.matrix_mode = mode;
    }
    pub fn get_viewport(&self, host: &mut impl Device) -> [u32; 4] {
        if self.lost {
            return [0; 4];
        }
        let mut v = [0; 6];
        host.dx(self.device, Call::GetViewport, &[], &[], &mut v);
        [
            v[0],
            self.full_viewport[3].wrapping_sub(v[1]).wrapping_sub(v[3]),
            v[0].wrapping_add(v[2]),
            self.full_viewport[3].wrapping_sub(v[1]),
        ]
    }
    pub fn blit_unsupported(&mut self) {
        self.error = 3;
    }
    fn stage_mut(&mut self) -> Result<&mut Stage> {
        self.stages.get_mut(self.current_stage).ok_or(Error::InvalidInput)
    }
    pub fn tex_stage(&mut self, i: u32) {
        if i as usize >= self.stages.len() {
            self.error = 2;
        } else {
            self.current_stage = i as usize;
        }
    }
    pub fn bind_texture(&mut self, target: u32, name: u32) -> Result<()> {
        if target != 0 {
            self.error = 1;
            return Ok(());
        }
        self.follow_texture = true;
        let parameters = self
            .textures
            .get(name)
            .or_else(|| self.textures.get(0))
            .map(|r| r.parameters)
            .ok_or(Error::InvalidInput)?;
        let alpha = self.textures.alpha(name);
        let s = self.stage_mut()?;
        s.name = name;
        s.parameters = parameters;
        s.alpha = alpha;
        s.invalidate();
        self.stages_dirty = true;
        Ok(())
    }
    pub fn set_texture(&mut self, name: u32, i: usize) -> Result<()> {
        let s = self.stages.get_mut(i).ok_or(Error::InvalidInput)?;
        if s.name != name {
            self.follow_texture = false;
            s.name = name;
            s.alpha = self.textures.alpha(name);
            s.invalidate();
            self.stages_dirty = true;
        }
        Ok(())
    }
    pub fn get_texture(&self, i: usize) -> Result<u32> {
        Ok(self.stages.get(i).ok_or(Error::InvalidInput)?.name)
    }
    pub fn tex_parameter(&mut self, pname: u32, value: u32) -> Result<()> {
        let name = self.stage_mut()?.name;
        if self.follow_texture {
            match self
                .textures
                .get_mut(name)
                .and_then(|r| r.parameters.get_mut(pname as usize))
            {
                Some(p) => *p = value,
                None => {
                    self.error = 1;
                    return Ok(());
                }
            }
        }
        let s = self.stage_mut()?;
        if let Some(p) = s.parameters.get_mut(pname as usize) {
            if *p == value {
                return Ok(());
            }
            *p = value;
        }
        s.dirty = true;
        self.stages_dirty = true;
        Ok(())
    }
    pub fn tex_env(&mut self, mode: u32) -> Result<()> {
        let s = self.stage_mut()?;
        s.environment = mode;
        s.dirty = true;
        self.stages_dirty = true;
        Ok(())
    }
    pub fn tex_env_color(&mut self, rgba: [f32; 4], host: &mut impl Device) {
        self.submit(host);
        self.rs(60, color(rgba), true, host);
    }
    pub fn tex_stage_coord(&mut self, v: u32, host: &mut impl Device) {
        let generation = match v & !7 {
            0 => 0,
            0x10 => 0x20000,
            0x20 => 0x10000,
            0x30 => 0x30000,
            _ => {
                self.error = 2;
                return;
            }
        };
        self.submit(host);
        let hr = dx(
            host,
            self.device,
            Call::StageState,
            &[self.current_stage as u32, 11, generation | (v & 7)],
        );
        self.hr(hr);
    }
    pub fn tex_stage_matrix(&mut self, m: Option<[f32; 16]>, count: u32, flags: u32, host: &mut impl Device) {
        self.submit(host);
        let mut m = match m {
            Some(m) if m != IDENTITY => m,
            _ => {
                dx(host, self.device, Call::StageState, &[self.current_stage as u32, 24, 0]);
                return;
            }
        };
        if count == 3 && flags & 1 == 0 || count == 4 && flags & 1 != 0 {
            for v in &mut m[8..12] {
                *v = -*v;
            }
        }
        dx(
            host,
            self.device,
            Call::StageState,
            &[
                self.current_stage as u32,
                24,
                count | if flags & 2 != 0 { 0x100 } else { 0 },
            ],
        );
        host.dx(
            self.device,
            Call::Transform,
            &[16 + self.current_stage as u32],
            &floats(&m),
            &mut [],
        );
    }
    pub fn combine_field(&mut self, slot: u8, which: usize, value: u32) -> Result<()> {
        if matches!(slot, 54 | 55 | 57) && self.current.byte(0x29) == 0 {
            self.error = 2;
            return Ok(());
        }
        let s = self.stage_mut()?;
        match slot {
            54 => {
                if which < 2 {
                    s.scale[which] = value;
                }
            }
            57 => {
                if which < 2 {
                    s.function[which] = value;
                }
            }
            55 | 56 => {
                let (c, i) = if which < 3 {
                    (0, which)
                } else if (4..7).contains(&which) {
                    (1, which - 4)
                } else {
                    s.invalidate();
                    self.stages_dirty = true;
                    return Ok(());
                };
                if slot == 55 {
                    s.operands[c][i] = value;
                } else {
                    s.sources[c][i] = value;
                }
            }
            _ => return Err(Error::InvalidInput),
        }
        s.invalidate();
        self.stages_dirty = true;
        Ok(())
    }
    pub fn set_combiner(&mut self, d: &[u8; 10], i: usize, host: &mut impl Device) -> Result<()> {
        self.submit(host);
        self.stages.get_mut(i).ok_or(Error::InvalidInput)?.packed(d)?;
        self.stages_dirty = true;
        Ok(())
    }
    pub fn gen_textures(&mut self, n: usize) -> Vec<u32> {
        (0..n).map(|_| self.textures.new_name()).collect()
    }
    pub fn delete_textures(&mut self, names: &[u32], host: &mut impl Device) {
        if self.ready {
            for &n in names {
                if n != 0 {
                    self.textures.free(n, host);
                }
            }
        }
    }
    pub fn is_texture(&self, name: u32) -> bool {
        self.textures.valid(name)
    }
    pub fn prioritize_textures(&mut self, names: &[u32], priorities: &[f32]) {
        for (&n, &p) in names.iter().zip(priorities) {
            self.textures.priority(n, p);
        }
    }
    pub fn resident(&self, names: &[u32], out: &mut [bool]) -> bool {
        for (&n, o) in names.iter().zip(out) {
            *o = self.textures.valid(n);
        }
        true
    }
    pub fn create_texture(
        &mut self,
        format: u32,
        width: u32,
        height: u32,
        levels: u32,
        flags: u32,
        host: &mut impl Device,
    ) -> Result<u32> {
        let name = self.textures.new_name();
        if !self.textures.create(name, flags, format, width, height, levels, host)? {
            self.error = 3;
        }
        Ok(name)
    }
    pub fn tex_image(&mut self, mut upload: Upload, format: u32, host: &mut impl Device) -> Result<()> {
        upload.name = self.stage_mut()?.name;
        upload.x = 0;
        upload.y = 0;
        upload.row_length = self.row_length;
        if upload.width < 1 || upload.height < 1 || upload.name == 0 {
            self.error = 2;
            return Ok(());
        }
        if self.textures.surface(upload.name) == 0 {
            let flags = self.textures.get(upload.name).map_or(0, |r| r.flags);
            if !self
                .textures
                .create(upload.name, flags, format, upload.width, upload.height, 1, host)?
            {
                self.error = 2;
                return Ok(());
            }
        }
        if !self.textures.load(upload, host)? {
            self.error = 3;
        }
        Ok(())
    }
    pub fn load_texture_level(&mut self, upload: Upload, host: &mut impl Device) -> Result<()> {
        self.submit(host);
        if !self.textures.load(upload, host)? {
            self.error = 3;
        }
        Ok(())
    }
    fn force_lighting(&mut self, host: &mut impl Device) {
        if !self.multiplier {
            self.rs(137, 1, true, host);
            self.multiplier = true;
        }
    }
    pub fn color_multiplier(&mut self, rgb: [f32; 3], host: &mut impl Device) {
        self.submit(host);
        self.force_lighting(host);
        self.material[4..7].copy_from_slice(&[1.; 3]);
        host.dx(self.device, Call::Material, &[], &floats(&self.material), &mut []);
        self.rs(139, color([rgb[0], rgb[1], rgb[2], 1.]), false, host);
    }
    pub fn alpha_multiplier(&mut self, a: f32, host: &mut impl Device) {
        self.submit(host);
        self.force_lighting(host);
        self.material[3] = a;
        host.dx(self.device, Call::Material, &[], &floats(&self.material), &mut []);
        if a < 1. {
            self.rs(145, 0, false, host);
        } else if self.vertex_colors[1] {
            self.rs(145, 1, false, host);
        }
    }
    pub fn enable_vertex_colors(&mut self, ambient: bool, diffuse: bool, host: &mut impl Device) {
        self.submit(host);
        self.vertex_colors = [ambient, diffuse];
        self.rs(141, 1, false, host);
        self.rs(147, u32::from(ambient), false, host);
        self.rs(145, u32::from(diffuse), false, host);
    }
    pub fn fog_float(&mut self, pname: u32, v: &[f32], host: &mut impl Device) -> Result<()> {
        self.submit(host);
        if pname == 0 {
            self.error = 1;
            return Ok(());
        }
        let (state, value) = match pname {
            1 => {
                if v.len() < 4 {
                    return Err(Error::InvalidInput);
                }
                (34, color([v[0], v[1], v[2], v[3]]))
            }
            2..=4 => (
                [0, 0, 38, 36, 37][pname as usize],
                v.first().ok_or(Error::InvalidInput)?.to_bits(),
            ),
            _ => return Ok(()),
        };
        self.rs(state, value, true, host);
        Ok(())
    }
    pub fn fog_int(&mut self, pname: u32, v: u32, host: &mut impl Device) -> Result<()> {
        self.submit(host);
        if !self.fog_capable {
            self.error = 3;
            return Ok(());
        }
        if pname == 0 {
            self.rs(
                140,
                *[1, 2, 3, 2].get(v as usize).ok_or(Error::InvalidInput)?,
                true,
                host,
            );
        } else if pname <= 4 {
            self.error = 1;
        }
        Ok(())
    }
    pub fn auxiliary_operation(&self) -> Result<()> {
        Err(Error::Unsupported("RD7"))
    }
    fn build_info(&mut self, mut adapter: [u32; 11], host: &mut impl Device) -> Result<()> {
        if adapter[2..6] == [0; 4] {
            self.file_version(&mut adapter[2..6], host)?;
        }
        let mut description = vec![0; adapter[1] as usize];
        host.read(adapter[0], &mut description)?;
        let mut guid = [0; 16];
        host.read(adapter[6], &mut guid)?;
        let a = u32::from_le_bytes(guid[0..4].try_into().unwrap());
        let b = u16::from_le_bytes(guid[4..6].try_into().unwrap());
        let c = u16::from_le_bytes(guid[6..8].try_into().unwrap());
        let mut info = b"Maxis 3D GDriver DirectX 2.0\n".to_vec();
        info.extend(description);
        info.extend(format!(
            "\n{}.{}.{}.{}, GUID: {a:08x}-{b:04x}-{c:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\nVendor: {:04x}, Device: {:04x}, Board: {:04x}, Chipset: {:04x}",
            adapter[2], adapter[3], adapter[4], adapter[5], guid[8], guid[9], guid[10], guid[11], guid[12], guid[13], guid[14], guid[15], adapter[7], adapter[8], adapter[9], adapter[10]
        ).bytes());
        *self.driver_info() = info;
        Ok(())
    }
    fn file_version(&self, version: &mut [u32], host: &mut impl Device) -> Result<()> {
        let mut directory = [0; 2];
        if failed(host.dx(0, Call::SystemDirectory, &[], &[], &mut directory)) { return Ok(()); }
        for fragment in ["\\", "\\Drivers\\", "32\\Drivers\\"] {
            for suffix in ["", ".dll", ".vxd", ".sys"] {
                // The backend uses the identifier's driver filename, not its description.
                let mut file = [0];
                let pattern = format!("{fragment}\0{suffix}");
                let hr = host.dx(0, Call::FindVersionFile, &directory, pattern.as_bytes(), &mut file);
                if failed(hr) || file[0] == 0 { continue; }
                let mut words = [0; 2];
                if !failed(host.dx(file[0], Call::FileVersion, &[], &[], &mut words)) {
                    version.copy_from_slice(&[words[0] >> 16, words[0] & 65535, words[1] >> 16, words[1] & 65535]);
                }
                return Ok(());
            }
        }
        Ok(())
    }
    /// Final-reference deletion is explicit because foreign COM references need a backend.
    pub fn release(&mut self, host: &mut impl Device) -> Result<u32> {
        self.refs = self.refs.saturating_sub(1);
        let refs = self.refs;
        if refs != 0 { return Ok(refs); }
        self.shutdown(host)?;
        for token in [&mut self.device, &mut self.dd, &mut self.d3d] { release(host, token); }
        for token in &mut self.auxiliary_surfaces { release(host, token); }
        self.context.destroy(host);
        for region in &mut self.regions { release(host, &mut region.surface); }
        self.regions.clear();
        self.stages.clear();
        self.lights.clear();
        self.modes.clear();
        self.primitives.refs = self.primitives.refs.saturating_sub(1);
        if self.primitive_identity != 0 { host.call(self.primitive_identity, crate::gdrivers_director::Op::Release, &[], &[], &mut []); }
        if self.primitives.refs == 0 { self.primitives.destroy(host); }
        // §4.1: the original AddRefs its texture manager instead of releasing it.
        self.textures.refs = self.textures.refs.wrapping_add(1);
        if self.texture_identity != 0 { acquire(host, self.texture_identity); }
        self.texture_identity = 0;
        self.primitive_identity = 0;
        self.device = 0;
        Ok(0)
    }
}

/// D3DLIGHT7 fields, with the range specified in §12.1.
#[derive(Clone, Debug, PartialEq)]
pub struct D3DLight {
    pub kind: u32,
    pub colors: [[f32; 4]; 3],
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub range: f32,
    pub falloff: f32,
    pub attenuation: [f32; 3],
    pub theta: f32,
    pub phi: f32,
}
impl Default for D3DLight {
    fn default() -> Self {
        Self {
            kind: 1,
            colors: [[0.; 4]; 3],
            position: [0.; 3],
            direction: [0., 0., 10.],
            range: f32::from_bits(0x5f7fffff),
            falloff: 1.,
            attenuation: [0.; 3],
            theta: f32::from_bits(0x3f47ae14),
            phi: f32::from_bits(0x3fc8f5c3),
        }
    }
}
fn compare(v: u32) -> Result<u32> {
    if v < 8 {
        Ok(v + 1)
    } else {
        Err(Error::InvalidInput)
    }
}
fn blend(v: u32) -> Result<u32> {
    if v < 11 {
        Ok(v + 1)
    } else {
        Err(Error::InvalidInput)
    }
}
fn stencil(v: u32) -> Result<u32> {
    [2, 1, 3, 4, 5, 6].get(v as usize).copied().ok_or(Error::InvalidInput)
}

#[cfg(test)]
mod tests;
