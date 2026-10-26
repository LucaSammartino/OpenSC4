// SPDX-License-Identifier: GPL-3.0-or-later
//! Stable, reference-owning view-object lists (render3d §11.1).
use super::{Handle, References, Render3d};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewObject {
    pub object: Handle,
    pub key: u32,
}

#[derive(Debug, Default)]
pub(super) struct ViewObjects {
    lists: [Vec<ViewObject>; 4],
    pub(super) terrain_view: Option<Handle>,
    pub(super) terrain: Option<Handle>,
}

fn index(kind: i32) -> Option<usize> {
    match kind {
        3 => Some(0),
        5 => Some(1),
        0 => Some(2),
        2 => Some(3),
        _ => None,
    }
}

impl Render3d {
    pub fn add_view_object(&mut self, host: &mut impl References, object: Handle, kind: i32, key: u32) -> bool {
        let Some(index) = index(kind) else {
            return false;
        };
        host.add_ref(object);
        let list = &mut self.objects.lists[index];
        let position = list.iter().position(|node| node.key > key).unwrap_or(list.len());
        list.insert(position, ViewObject { object, key });
        if index >= 2 {
            self.state.backing_saved = false;
        }
        true
    }

    pub fn view_objects(&self, kind: i32) -> Option<&[ViewObject]> {
        index(kind).map(|i| self.objects.lists[i].as_slice())
    }

    pub fn find_view_object(&self, object: Option<Handle>) -> bool {
        self.objects.terrain_view == object
            || [1, 0, 3, 2]
                .into_iter()
                .any(|index| self.objects.lists[index].iter().any(|node| Some(node.object) == object))
    }

    pub fn remove_view_object(&mut self, host: &mut impl References, object: Option<Handle>) -> bool {
        let mut result = false;
        if self.objects.terrain_view == object {
            if let Some(old) = self.objects.terrain_view.take() {
                host.release(old);
            }
            result = true;
        }
        for index in [1, 0, 3, 2] {
            let list = &mut self.objects.lists[index];
            if let Some(position) = list.iter().position(|node| Some(node.object) == object) {
                host.release(list.remove(position).object);
                result = true;
                if index >= 2 {
                    self.state.backing_saved = false;
                }
            }
        }
        result
    }

    /// PostCityInit only acquires this interface while its slot is empty.
    pub fn post_city_init_terrain_view(&mut self, host: &mut impl References, view: Option<Handle>) {
        if self.objects.terrain_view.is_none() {
            self.objects.terrain_view = view;
            if let Some(view) = view {
                host.add_ref(view);
            }
        }
    }

    pub(super) fn clear_view_object_lists(&mut self, host: &mut impl References) {
        for list in &mut self.objects.lists {
            for node in list.drain(..) {
                host.release(node.object);
            }
        }
    }
}
