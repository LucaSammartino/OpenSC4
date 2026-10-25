// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

// Primary-slot audit (widget-controls 6.1/6.6): 0 query_interface, 1/2 host
// reference ownership, 3..18 properties in settled.rs (stored getters use
// fields), 19..22 images, 23/24 wrappers below, 25 insert, 26 move_node,
// 27 remove_node, 28 clear_root, 29 descriptor_into, 30 apply_descriptor,
// 31..36 text/data/images, 37 default font, 38/39 node font, 40..43 colors,
// 44/45 expanded, 46/47 visibility, 48 child count, 49 children, 50 selected,
// 51 parent, 52 insertion_reference, 53 count, 54 first_selected,
// 55 copy_selection, 56..58 selection, 59 window. The borrowed node getters
// below also implement the missing-handle results; O21 uses image_into.
// W's owner overrides map to query_interface, do_message, init/shutdown,
// paint, key_down/up, mouse_button/mouse_move, wheel and host destruction.
impl TreeView {
    /// O21 preserves both output objects when the unsigned index is absent.
    pub fn image_into(&self, index: u32, buffer: &mut Option<Handle>, rect: &mut Rect) -> bool {
        let Some((image, bounds)) = self.image(index) else {
            return false;
        };
        *buffer = image;
        *rect = bounds;
        true
    }

    /// O38 returns a borrowed font, including null for an absent handle.
    pub fn node_font(&self, handle: NodeHandle) -> Option<Handle> {
        self.nodes.get(&handle).and_then(|node| node.font)
    }

    /// O44, O48 and O50 do not refresh layout or acquire references.
    pub fn expanded(&self, handle: NodeHandle) -> bool {
        self.nodes.get(&handle).is_some_and(|node| node.expanded)
    }
    pub fn child_count(&self, handle: NodeHandle) -> u32 {
        self.nodes.get(&handle).map_or(0, |node| node.children.len() as u32)
    }
    pub fn is_selected(&self, handle: NodeHandle) -> bool {
        self.nodes.get(&handle).is_some_and(|node| node.selected)
    }

    /// O23. The four image arguments retain their native closed/selected-closed/
    /// open/selected-open order. Initialize before copying current defaults.
    pub fn insert_images(
        &mut self,
        parent: NodeHandle,
        reference: i32,
        text: &[u8],
        images: [i32; 4],
        f: &mut impl TreeForeign,
    ) -> Result<NodeHandle> {
        self.internal_init(f);
        self.insert(
            Descriptor {
                parent,
                reference,
                text: text.to_vec(),
                data: 0,
                images,
                font: self.default_font,
                colors: self.colors,
                expanded: false,
            },
            f,
        )
    }

    /// O24. This wrapper supplies the default image indices, not O23's data=0.
    pub fn insert_text(
        &mut self,
        parent: NodeHandle,
        reference: i32,
        text: &[u8],
        data: u32,
        f: &mut impl TreeForeign,
    ) -> Result<NodeHandle> {
        self.internal_init(f);
        self.insert(
            Descriptor {
                parent,
                reference,
                text: text.to_vec(),
                data,
                images: [0, 1, 2, 3],
                font: self.default_font,
                colors: self.colors,
                expanded: false,
            },
            f,
        )
    }

    /// O28 removes descendants of handle zero, preserving the mapped root.
    pub fn clear_root(&mut self, f: &mut impl Foreign) -> bool {
        self.remove_node(0, f)
    }

    /// O29 leaves the entire caller descriptor unchanged for an absent node.
    pub fn descriptor_into(&mut self, handle: NodeHandle, output: &mut NodeDescriptor) -> bool {
        let Some(value) = self.descriptor(handle) else {
            return false;
        };
        *output = value;
        true
    }

    /// O55's caller owns sufficient writable storage. Null is safe only for
    /// an empty selection; model invalid memory as a precondition error.
    pub fn copy_selection(&self, output: Option<&mut [NodeHandle]>) -> Result<bool> {
        if self.selection.is_empty() {
            return Ok(true);
        }
        let output = output
            .filter(|out| out.len() >= self.selection.len())
            .ok_or(Unsupported::Precondition("TreeView selection output storage"))?;
        output[..self.selection.len()].copy_from_slice(&self.selection);
        Ok(true)
    }
}
