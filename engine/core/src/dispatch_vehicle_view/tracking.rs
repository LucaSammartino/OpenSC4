// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_HANDLE: AtomicU32 = AtomicU32::new(0);

fn kind_index(kind: u32) -> Result<usize, Unsupported> {
    if kind < 7 {
        Ok(kind as usize)
    } else {
        Err(Unsupported::Kind(kind))
    }
}

impl DispatchVehicleView {
    pub fn init(&mut self, host: &mut impl Services) -> Result<bool, Unsupported> {
        self.states = Some(host.render_states().ok_or(Unsupported::OpenRenderStates)?);
        for (icon, instance) in self.icons.iter_mut().zip(ICON_INSTANCES) {
            *icon = host.icon_handle(instance);
        }
        self.limits = [u32::MAX; 7];
        host.subscribe(VIEW_CHANGED, true);
        self.initialized = true;
        Ok(true)
    }

    pub fn track(
        &mut self,
        occupant: Token,
        kind: u32,
        number: u32,
        label: Option<&str>,
        host: &mut impl Services,
    ) -> Result<u32, Unsupported> {
        if occupant == 0 {
            return Ok(0);
        }
        let index = kind_index(kind)?;
        if self.limits[index] != 0 && self.counts[index] >= self.limits[index] {
            return Ok(0);
        }
        let icon_index = [0, 1, 6, 2, 4, 5, 3][index];
        let icon = if kind == 6 { None } else { self.icons[icon_index] };
        let mut info = TrackInfo::for_tracking(occupant, kind, number, label, icon);
        if kind == 6 {
            if let Some(model) = host.load_model(MODEL_KEY) {
                host.model_op(model, ModelOp::BindOccupant(occupant));
                host.model_op(model, ModelOp::Mode(1));
                let camera = host.camera();
                host.model_op(model, ModelOp::ZoomRotation(camera.zoom, camera.rotation));
                // Safe boundary for the specified native null-this call, not a
                // claim that the native program recovers allocation failure.
                let Some(processor) = host.allocate_processor() else {
                    host.resource_op(ResourceOp::Release(Reference::Model, model));
                    return Err(Unsupported::NativeProcessorAllocation);
                };
                let base = host.model_base(model);
                host.model_op(model, ModelOp::ProcessorModel { processor, base });
                info.processor = Some(processor);
                info.flags |= flags::PROGRESS;
                let mut material = host.model_material(model);
                material.flags |= 0x10;
                host.model_op(model, ModelOp::Material(material));
                let bounds = host.model_bounds(model);
                info.display_size[1] = (f64::from(bounds[1][1]) - f64::from(bounds[0][1])).abs() as f32;
                host.model_op(model, ModelOp::Rate(f32::from_bits(0x3c44_9ba6)));
                // The inserted copy holds one model reference; loader releases
                // its own. Move the processor instead of cloning its ownership.
                host.resource_op(ResourceOp::Acquire(Reference::Model, model));
                info.model = Some(model);
                host.resource_op(ResourceOp::Release(Reference::Model, model));
            }
        }
        let occupant_type = host.occupant_type(occupant);
        info.set(flags::BUILDING, occupant_type == 0x2781_28a0);
        if [0x496e_5783, 0xc7cc_75f1].contains(&occupant_type) {
            if let Some(automaton) = host.automaton(occupant) {
                host.set_automaton_flag(automaton, 0x200000, true);
                host.resource_op(ResourceOp::Release(Reference::Automaton, automaton));
            }
        }
        host.resource_op(ResourceOp::Acquire(Reference::Occupant, occupant));
        if let Err(error) = build_display(&mut info, host) {
            self.cleanup_resources(&mut info, host);
            return Err(error);
        }
        self.counts[index] = self.counts[index].wrapping_add(1);
        info.handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        let handle = info.handle;
        self.insert(info);
        self.sort_priority(Some(occupant));
        Ok(handle)
    }

    pub fn find(&self, handle: u32) -> Option<&TrackInfo> {
        self.tracks.iter().find(|t| t.handle == handle)
    }
    pub(crate) fn find_mut(&mut self, handle: u32) -> Option<&mut TrackInfo> {
        self.tracks.iter_mut().find(|t| t.handle == handle)
    }
    pub fn set_indicator_max(&mut self, kind: u32, limit: u32) -> Result<(), Unsupported> {
        self.limits[kind_index(kind)?] = limit;
        Ok(())
    }
    pub fn lock(&mut self, handle: u32, on: bool) {
        if let Some(info) = self.find_mut(handle) {
            info.set(flags::LOCKED, on);
        }
    }
    pub fn flash(&mut self, handle: u32, on: bool, period: f32) {
        if let Some(info) = self.find_mut(handle) {
            info.set(flags::FLASHING, on);
            if on {
                info.flash_period = period;
                if info.flash_accumulator > period {
                    info.flash_accumulator = period;
                }
            }
        }
    }
    pub fn active_occupant(&self) -> Option<Token> {
        self.find(self.active).map(|t| t.occupant)
    }
    pub fn use_alternate_model(&mut self, kind: u8) -> bool {
        if let Some(info) = self.tracks.iter_mut().find(|t| t.key() == kind) {
            info.flags |= flags::ALTERNATE;
            true
        } else {
            false
        }
    }
    pub fn is_selectable(&self, info: &TrackInfo) -> bool {
        info.has(flags::SELECTABLE)
            && info.has(flags::VISIBLE)
            && !info.has(flags::REMOVING)
            && info.fade >= f32::from_bits(0x3f66_6666)
            && !self.hidden.contains(&info.kind)
    }
    pub fn find_indicator_handle(&self, x: i32, y: i32) -> u32 {
        self.tracks
            .iter()
            .rev()
            .find(|t| self.is_selectable(t) && t.rectangle.contains(x, y))
            .map_or(0, |t| t.handle)
    }
    pub fn set_active(&mut self, handle: u32, frame: u32, host: &mut impl DisplayServices) -> Result<(), Unsupported> {
        if handle == self.active {
            return Ok(());
        }
        if let Some(old) = self.find_mut(self.active) {
            old.frame = 1;
            build_display(old, host)?;
        }
        self.active = 0;
        if handle != 0 && self.find(handle).is_some_and(|t| self.is_selectable(t)) {
            self.active = handle;
            let info = self.find_mut(handle).expect("handle found above");
            info.frame = frame;
            build_display(info, host)?;
        }
        Ok(())
    }
    pub fn show_all(&mut self, kind: u32, show: bool) {
        if show {
            self.hidden.remove(&kind);
        } else {
            self.hidden.insert(kind);
        }
        self.sort_priority(None);
    }
    pub fn sort_priority(&mut self, occupant: Option<Token>) {
        let Some(occupant) = occupant else {
            let occupants: BTreeSet<_> = self.tracks.iter().map(|t| t.occupant).filter(|&t| t != 0).collect();
            for occupant in occupants {
                self.sort_priority(Some(occupant));
            }
            return;
        };
        let top = self
            .tracks
            .iter()
            .filter(|t| t.occupant == occupant && !t.has(flags::REMOVING) && !self.hidden.contains(&t.kind))
            .map(|t| t.kind)
            .max()
            .unwrap_or(0);
        for info in &mut self.tracks {
            if info.occupant == occupant {
                info.set(flags::TOP, info.kind == top);
            }
        }
    }
    pub fn last_screen_icon_rect(&self, handle: u32) -> Option<&Rect> {
        self.find(handle)
            .filter(|t| t.rectangle.has_extents())
            .map(|t| &t.rectangle)
    }
    pub fn handle_view_changed(&mut self, host: &mut impl DisplayServices) -> Result<(), Unsupported> {
        for info in &mut self.tracks {
            build_display(info, host)?;
        }
        Ok(())
    }
    pub fn do_message(&mut self, id: u32, host: &mut impl DisplayServices) -> Result<bool, Unsupported> {
        if id == VIEW_CHANGED {
            self.handle_view_changed(host)?;
        }
        Ok(true)
    }

    fn cleanup_resources(&self, info: &mut TrackInfo, host: &mut impl Services) {
        if let Some(texture) = info.texture.take() {
            host.resource_op(ResourceOp::ReleaseTexture(texture));
        }
        if let Some(model) = info.model.take() {
            host.resource_op(ResourceOp::Release(Reference::Model, model));
        }
        if let Some(processor) = info.processor.take() {
            host.model_op(0, ModelOp::DestroyProcessor(processor));
        }
        host.resource_op(ResourceOp::Release(Reference::Occupant, info.occupant));
    }
    pub(crate) fn remove_at(&mut self, index: usize, host: &mut impl Services) {
        let info = &mut self.tracks[index];
        let kind = info.kind as usize;
        if self.counts[kind] != 0 {
            self.counts[kind] -= 1;
        }
        if let Some(texture) = info.texture.take() {
            host.resource_op(ResourceOp::ReleaseTexture(texture));
        }
        let automaton = host.automaton(info.occupant);
        if let Some(automaton) = automaton {
            host.set_automaton_flag(automaton, 0x200000, false);
        }
        if let Some(model) = info.model.take() {
            host.resource_op(ResourceOp::Release(Reference::Model, model));
        }
        if let Some(processor) = info.processor.take() {
            host.model_op(0, ModelOp::DestroyProcessor(processor));
        }
        host.resource_op(ResourceOp::Release(Reference::Occupant, info.occupant));
        self.tracks.remove(index);
        if let Some(automaton) = automaton {
            host.resource_op(ResourceOp::Release(Reference::Automaton, automaton));
        }
    }
    pub fn untrack(&mut self, handle: u32, immediate: bool, host: &mut impl Services) -> bool {
        if handle == 0 {
            return false;
        }
        let Some(index) = self.tracks.iter().position(|t| t.handle == handle) else {
            return false;
        };
        if !immediate && self.tracks[index].has(flags::FADES) {
            self.tracks[index].flags |= flags::REMOVING;
            return true;
        }
        self.complete_removal(index, host);
        true
    }
    pub(crate) fn complete_removal(&mut self, index: usize, host: &mut impl Services) {
        let occupant = self.tracks[index].occupant;
        host.resource_op(ResourceOp::Acquire(Reference::Occupant, occupant));
        self.remove_at(index, host);
        self.sort_priority(Some(occupant));
        host.resource_op(ResourceOp::Release(Reference::Occupant, occupant));
    }
    pub fn shutdown(&mut self, host: &mut impl Services) -> bool {
        // SPEC-GAP: per-entry Shutdown cleanup detail is absent; use the
        // specified RemoveTrackInfo ownership cleanup before freeing the map.
        while !self.tracks.is_empty() {
            self.remove_at(0, host);
        }
        host.subscribe(VIEW_CHANGED, false);
        self.initialized = false;
        true
    }
}
