// SPDX-License-Identifier: GPL-3.0-or-later
//! Connection operations shared by replacement/native endpoint bridges (§4.6.2–4.6.3).

use super::{same_ref, AttachmentTables, PoleConnection, PoleUnsupported, PowerPole, References};
use crate::network_tools::power_line::{control_points, direction, lesser, sample_cubic, CurveParams};
use crate::stream::RefValue;
use crate::x87::{ftol32, r24};

/// Foreign primitives only; record ownership and geometry remain in the core.
pub trait PoleLinkHost: References {
    fn pole_cell(&mut self, pole: &RefValue) -> (i32, i32);
    fn attachment_points(&mut self, pole: &RefValue, orientation: u32) -> Result<[[f32; 3]; 4], PoleUnsupported>;
    fn cell_size(&mut self) -> [f32; 2];
    fn is_water(&mut self, cell: (i32, i32)) -> bool;
    fn post_message(&mut self, message: u32, data: u32);
}

fn curves(
    a: [[f32; 3]; 4],
    b: [[f32; 3]; 4],
    parameters: CurveParams,
    water: bool,
) -> Result<[Vec<Vec<[f32; 3]>>; 2], PoleUnsupported> {
    let mut result: [Vec<Vec<[f32; 3]>>; 2] = Default::default();
    for (a, b) in a.into_iter().zip(b) {
        let control = control_points(a, b, parameters);
        // SPEC-GAP: invalid resource counts have no owner recovery contract; report an unsupported invariant.
        let count = usize::try_from(control.count)
            .ok()
            .filter(|&n| n != 0)
            .ok_or(PoleUnsupported::CurveCountInvariant)?;
        let mut regular = Vec::with_capacity(count);
        regular.push(a);
        regular.extend(sample_cubic(a, control.first, control.second, b, count - 1));
        result[0].push(regular);
        if water {
            result[1].push(sample_cubic(a, control.first, control.second, b, 3));
        }
    }
    Ok(result)
}

impl PowerPole {
    fn connection_start(&self, size: [f32; 2]) -> (i32, i32) {
        let p = self.fields.position;
        (
            ftol32(r24(f64::from(p[0]) / f64::from(size[0]))),
            ftol32(r24(f64::from(p[2]) / f64::from(size[1]))),
        )
    }

    /// Direct enable always probes other; a matching enabled record probes it again.
    /// Ownership here uses the stored cell, unlike AddConnection's computed start.
    pub fn enable_connection(&mut self, other: &RefValue, enable: bool, host: &mut impl PoleLinkHost) {
        host.pole_cell(other);
        let Some(record) = self.fields.connections.iter_mut().find(|r| same_ref(&r.pole, other)) else {
            return;
        };
        record.flags[1] = if enable {
            u8::from(lesser(self.fields.cell, host.pole_cell(other)))
        } else {
            0
        };
    }

    /// §5.3 power-line-tool: duplicate cells, rather than duplicate pointers, suppress insertion.
    pub fn add_connection(
        &mut self,
        other: &RefValue,
        wires: &[RefValue],
        tables: &AttachmentTables,
        parameters: CurveParams,
        host: &mut impl PoleLinkHost,
    ) -> Result<(), PoleUnsupported> {
        if matches!(other, RefValue::Null) {
            return Ok(());
        }
        let destination = host.pole_cell(other);
        if self
            .fields
            .connections
            .iter()
            .any(|r| r.words == [destination.0 as u32, destination.1 as u32])
        {
            return Ok(());
        }
        let start = self.connection_start(host.cell_size());
        if start == destination {
            return Ok(());
        }
        let owning = lesser(start, destination);
        let mut record = PoleConnection {
            pole: other.clone(),
            flags: [0, u8::from(owning)],
            words: [destination.0 as u32, destination.1 as u32],
            ..Default::default()
        };
        host.add_ref(other);
        // Both owners probe terrain, and the second probe is short-circuited.
        let water = host.is_water(self.fields.cell) || host.is_water(destination);
        if owning {
            let build = (|| {
                let orientation = direction(start, destination);
                let a = self.attachment_points(orientation, tables)?;
                let b = host.attachment_points(other, orientation)?;
                curves(a, b, parameters, water)
            })();
            match build {
                Ok(vertices) => record.vertices = vertices,
                Err(error) => {
                    host.release(other);
                    return Err(error);
                }
            }
            for line in wires {
                host.add_ref(line);
                record.wire_points.push(line.clone());
            }
            host.post_message(0x99EF_1145, 0);
        }
        self.fields.connections.push(record);
        self.stale = true;
        Ok(())
    }

    /// Refresh regular wires and append coarse curves; a dry refresh preserves old coarse curves.
    pub fn rebuild_wires(
        &mut self,
        other: &RefValue,
        tables: &AttachmentTables,
        parameters: CurveParams,
        host: &mut impl PoleLinkHost,
    ) -> Result<(), PoleUnsupported> {
        if matches!(other, RefValue::Null) {
            return Ok(());
        }
        let Some(index) = self
            .fields
            .connections
            .iter()
            .position(|r| r.flags[1] != 0 && same_ref(&r.pole, other))
        else {
            return Ok(());
        };
        let destination = host.pole_cell(other);
        let start = self.connection_start(host.cell_size());
        if start == destination {
            return Ok(());
        }
        self.fields.connections[index].vertices[0].clear();
        let orientation = direction(start, destination);
        let a = self.attachment_points(orientation, tables)?;
        let b = host.attachment_points(other, orientation)?;
        let water = host.is_water(self.fields.cell) || host.is_water(destination);
        let [regular, coarse] = curves(a, b, parameters, water)?;
        let record = &mut self.fields.connections[index];
        record.vertices[0] = regular;
        record.vertices[1].extend(coarse);
        host.post_message(0x99EF_1145, 0);
        self.stale = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
