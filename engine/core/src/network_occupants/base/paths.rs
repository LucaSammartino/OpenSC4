//! Network-rules §11.1: the base owner's path-resource pipeline.
use super::*;

/// Path-info methods are foreign primitives owned by the traffic/path subsystem.
/// Returned references each carry one reference; cached GetPathInfo is borrowed.
pub trait NetworkPathHost: References {
    fn map_path(&mut self, id: u32) -> u32;
    fn load_path(&mut self, key: ResKey, iid: u32) -> RefValue;
    fn uses_elevation(&mut self, path: &RefValue) -> bool;
    fn clone_path(&mut self, path: &RefValue) -> RefValue;
    fn set_elevation(&mut self, path: &RefValue);
    fn transformed_copy(&mut self, path: &RefValue, occupant: &NetworkOccupant) -> RefValue;
    fn offset_and_scissor(&mut self, path: &RefValue, offset: [f32; 3], cell_bounds: [f32; 4]) -> RefValue;
    fn remove_wrong_way(&mut self, path: &RefValue, directions: u8);
    fn verify_edges(&mut self, path: &RefValue, occupant: &NetworkOccupant);
    fn path_connections(&mut self, path: &RefValue) -> Vec<(u8, u8, u8)>;
    fn persist_key(&mut self, path: &RefValue) -> ResKey;
}

impl NetworkOccupant {
    pub fn get_path_info(&mut self, host: &mut (impl NetworkPathHost + PieceHost)) -> &RefValue {
        if !matches!(self.path_info, RefValue::Null) {
            return &self.path_info;
        }
        let overridden = self.piece.path_id_override != 0;
        let (id, group_id) = if overridden {
            (self.piece.path_id_override, 0xA966_883F)
        } else {
            (self.piece_id, 0x6966_8828)
        };
        let key = ResKey {
            type_id: 0x2966_78F7,
            group: group_id,
            instance: host.map_path(id),
        };
        let mut source = host.load_path(key, 0xA966_7793);
        if matches!(source, RefValue::Null) {
            return &self.path_info;
        }
        if overridden && !host.uses_elevation(&source) {
            let clone = host.clone_path(&source);
            clear_ref(&mut source, host);
            source = clone;
            if matches!(source, RefValue::Null) {
                return &self.path_info;
            }
            host.set_elevation(&source);
        }
        self.path_info = host.transformed_copy(&source, self);
        clear_ref(&mut source, host);
        if overridden && !matches!(self.path_info, RefValue::Null) {
            let [x, z] = self.path_origin;
            let cell = [
                self.quad[0].pos[0],
                self.quad[0].pos[2],
                self.quad[2].pos[0],
                self.quad[2].pos[2],
            ];
            let clipped = host.offset_and_scissor(&self.path_info, [x, 0.0, z], cell);
            self.release_path_info(host);
            self.path_info = clipped;
        }
        if !matches!(self.path_info, RefValue::Null) {
            host.remove_wrong_way(&self.path_info, self.one_way_directions());
            host.verify_edges(&self.path_info, self);
            self.piece.connections = [[0; 4]; 8];
            for (kind, entry, exit) in host.path_connections(&self.path_info) {
                self.piece.add_path_connection(kind, entry, exit, host);
            }
            self.path_info_key = host.persist_key(&self.path_info);
        }
        &self.path_info
    }

    pub fn path_connection(
        &mut self,
        kind: u8,
        entry: u8,
        exit: u8,
        host: &mut (impl NetworkPathHost + PieceHost),
    ) -> u8 {
        if self.piece.network_flags & FLAG_PATHS_BUILT == 0 {
            self.get_path_info(host);
        }
        // The native query still tests the existing bytes if resource loading fails.
        // Suppress the helper's build callback: the owner just ran GetPathInfo.
        let flags = self.piece.network_flags;
        self.piece.network_flags |= FLAG_PATHS_BUILT;
        let result = self.piece.path_connection(kind, entry, exit, host);
        self.piece.network_flags = flags;
        result
    }

    pub fn restore_path_info(&mut self, host: &mut impl NetworkPathHost) {
        let key = self.path_info_key;
        self.release_path_info(host);
        if key != ResKey::default() {
            self.path_info = host.load_path(key, 0xA966_7793);
            if !matches!(self.path_info, RefValue::Null) {
                self.path_info_key = key;
            }
        }
    }
}
