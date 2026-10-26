// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

/// Buffer identities are kept separate from the byte offsets within them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewVertexBuffer {
    Land,
    Water,
}

/// A draw-context call, not a driver call. Count arguments are retained even
/// where the current context ignores them (terrain.md §15.5.15.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewRowCall {
    SetVertexBuffer {
        buffer: ViewVertexBuffer,
        format: u32,
        count: u32,
        byte_offset: u32,
    },
    SetIndexBuffer {
        count: u32,
        byte_offset: u32,
    },
    DrawPrimsIndexed {
        primitive: u32,
        first: u32,
        count: u32,
    },
}

/// A row from an already computed frustum cache. right is the exclusive
/// vertex-column bound, not an inclusive cell bound. The caller visits rows
/// in ascending order and supplies the OR of the bounding vertex-row flags.
/// This type neither computes frustum boundaries nor produces vertex bytes.
#[derive(Clone, Copy, Debug)]
pub struct PlainViewRow {
    pub row: u32,
    pub left: u32,
    pub right: u32,
    pub bounding_rows_visible: [bool; 2],
}

impl PlainViewRow {
    /// The floor subsequence, §15.5.15.2. There is no positive-count guard.
    /// RGB copies and surrounding render-state calls belong to the caller.
    pub fn floor_calls(self, verts_x: u32, index_stride: u32) -> Option<[ViewRowCall; 3]> {
        if !self.bounding_rows_visible.into_iter().any(|v| v) {
            return None;
        }
        Some([
            self.vertices(ViewVertexBuffer::Land, verts_x, 2 * verts_x),
            self.indices(index_stride),
            self.draw(),
        ])
    }

    /// Water binds indices first and supplies W rather than 2W as its vertex
    /// count (§15.5.15.3), although the indices still reference two rows.
    pub fn water_calls(self, verts_x: u32, index_stride: u32) -> Option<[ViewRowCall; 3]> {
        if !self.bounding_rows_visible.into_iter().any(|v| v) || self.right.saturating_sub(self.left) <= 1 {
            return None;
        }
        Some([
            self.indices(index_stride),
            self.vertices(ViewVertexBuffer::Water, verts_x, verts_x),
            self.draw(),
        ])
    }

    fn vertices(self, buffer: ViewVertexBuffer, verts_x: u32, count: u32) -> ViewRowCall {
        ViewRowCall::SetVertexBuffer {
            buffer,
            format: 0x0B,
            count,
            byte_offset: 32 * verts_x * self.row,
        }
    }

    fn indices(self, stride: u32) -> ViewRowCall {
        ViewRowCall::SetIndexBuffer {
            count: stride,
            byte_offset: 2 * stride * self.row,
        }
    }

    fn draw(self) -> ViewRowCall {
        ViewRowCall::DrawPrimsIndexed {
            primitive: 0,
            first: 6 * self.left,
            count: 6_u32.wrapping_mul(self.right.wrapping_sub(self.left).wrapping_sub(1)),
        }
    }
}
