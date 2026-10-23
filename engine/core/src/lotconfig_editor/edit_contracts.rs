// SPDX-License-Identifier: GPL-3.0-or-later
//! Published edit kernels with explicit inputs from unresolved numeric helpers.
use super::*;

impl Editor {
    /// §9.3/9.4 insertion after coordinate conversion. `family_bounds` is the
    /// raw-offset rectangle produced by the manager-bound helper for kinds
    /// 0/1/4, not an assumed width/height algorithm. That helper remains LE3.
    /// Clock creation and converted coordinates are primitive service facts.
    pub fn add_object_at(
        &mut self,
        point: [f32; 2],
        creation: u32,
        family_bounds: Option<[f32; 4]>,
        h: &mut impl Host,
    ) -> Result<Option<RecordId>> {
        let kind = match self.mode {
            1 => 0,
            2 => 4,
            3 => 1,
            4 => return Err(Unsupported::Dependency("LE3 physical type-7 allocation/fourth store")),
            5 => 2,
            6 => 5,
            7 => 6,
            _ => return Ok(None),
        };
        if matches!(self.mode, 1..=3 | 5) && self.pending_ids.is_empty() {
            return Ok(None);
        }
        let rectangle = if matches!(kind, 0 | 1 | 4) {
            family_bounds.ok_or(Unsupported::Dependency("LE3 family-based bounds helper"))?
        } else {
            // Literal native stores, reversed relative to §9's min/max labels.
            [point[0] + 8.0, point[1] + 8.0, point[0] - 8.0, point[1] - 8.0]
        };
        let mut data = LotConfigObject {
            kind,
            offset: [point[0], 0.0, point[1]],
            tag: creation,
            max_x: rectangle[0],
            max_z: rectangle[1],
            min_x: rectangle[2],
            min_z: rectangle[3],
            ids: self.pending_ids.clone(),
            ..Default::default()
        };
        move_within(&mut data, self.config()?.size, kind != 4);
        let id = self.insert(data, kind != 2);
        let first = self.config()?.objects.first().map(|r| r.id);
        self.set_selection(first, h);
        self.edited(h);
        Ok(Some(id))
    }
    /// §3 accepted-selector postlude. Bounds are explicit manager/helper output;
    /// obtaining them is still LE3/LE4, not a foreign editor-policy callback.
    /// None represents cancellation and must leave every field unchanged.
    pub fn selector_commit(
        &mut self,
        id: RecordId,
        accepted: Option<(Vec<u32>, [f32; 4])>,
        h: &mut impl Host,
    ) -> Result<()> {
        if let Some((ids, b)) = accepted {
            let record = self.record_mut(id).ok_or(Unsupported::MissingRecord)?;
            record.ids = ids;
            [record.max_x, record.max_z, record.min_x, record.min_z] = b;
            self.set_dirty(1, h);
            h.effect(Effect::RedrawRequired);
        }
        Ok(())
    }
    /// §9.3 confirmation kernel given the open LE5 name helper's actual result.
    pub fn delete_with_name(&mut self, id: RecordId, name: &str, h: &mut impl Host) -> Result<bool> {
        if h.prompt(&format!("Are you sure you want to delete this {name}?"), 3) == 0x53018149 {
            return Ok(false);
        }
        self.delete(id, false, h)
    }
    /// §11 dispatch after a separately established hit result. Remaining drag
    /// setup, foundation modal and hit-code production retain their open status.
    pub fn edit_mouse_down_hit(
        &mut self,
        code: usize,
        object: Option<RecordId>,
        fence_point: [i32; 2],
        h: &mut impl Host,
    ) -> Result<bool> {
        match code {
            0 | 1 if code == 0 || self.mode == 0 => {
                if self.selected.is_some() {
                    self.set_selection(None, h);
                    self.draw(h);
                }
            }
            2 if self.mode == 0 => return Err(Unsupported::Dependency("LE4 foundation dialog before selection clear")),
            3 if self.mode == 8 => {
                self.toggle_fence_pixels(fence_point[0], fence_point[1], h)?;
            }
            3 if self.mode == 0 => {
                self.set_selection(None, h);
                h.effect(Effect::RedrawRequired);
            }
            100.. if self.mode == 0 => {
                self.set_selection(object, h);
                return Err(Unsupported::Dependency("LE5 object drag setup"));
            }
            _ => (),
        }
        Ok(true)
    }
}
