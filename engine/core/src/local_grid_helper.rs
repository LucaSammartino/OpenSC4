// SPDX-License-Identifier: GPL-3.0-or-later
//! LocalGridHelper, from docs/specs/local-grid-helper.md §§2–6.
//!
//! This is a portable primitive boundary, not a native FP-environment guarantee.
//! LGH1 remains open. See `NumericMode` and the domain/proof ledger alongside the
//! independent integration tests. No Drop implementation performs foreign calls:
//! callers explicitly consume the helper with `destroy` to end its lifetime.

/// Raw ST(0) replies are exact dyadics supplied as binary64, not presumed f32
/// service returns. Only this explicit qualified mode implements arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericMode {
    QualifiedDyadicNearest,
    /// Ambient x87 precision/rounding/exception state has no portable contract.
    Unqualified,
}

/// Individual position loads remain behind the null-H guard.
pub trait Position {
    fn read_x(&mut self) -> u32;
    fn read_y(&mut self) -> u32;
    fn read_z(&mut self) -> u32;
}

impl Position for [u32; 3] {
    fn read_x(&mut self) -> u32 {
        self[0]
    }
    fn read_y(&mut self) -> u32 {
        self[1]
    }
    fn read_z(&mut self) -> u32 {
        self[2]
    }
}

/// Completed, synchronous, nonreentrant primitives with stable interfaces.
/// The host owns neither snapping nor flags/descriptor construction. Identities
/// are opaque tokens; no helper AddRef, refcount, or renderer behavior is implied.
pub trait Host {
    type Effect: Copy;
    type Service: Copy;

    /// The actual helper field is the writable output channel. Status is ignored.
    fn raw_factory(&mut self, name: &[u8], output: &mut Option<Self::Effect>) -> u32;
    fn active_query(&mut self, effect: Self::Effect) -> u32;
    fn start(&mut self, effect: Self::Effect, argument: u32);
    fn stop(&mut self, effect: Self::Effect, argument: u32);
    /// `helper` is a read-only portable observation of H at primitive entry,
    /// not an additional native argument. It cannot mutate live helper storage.
    fn release(&mut self, effect: Self::Effect, helper: &LocalGridHelper<Self::Effect>);
    fn capture_service(&mut self) -> Self::Service;
    fn raw_scalar(&mut self, service: Self::Service) -> f64;
    fn raw_floor(&mut self, binary64_input: f64) -> f64;
    /// Temporary synchronous view only; no retained pointer lifetime is promised.
    fn set_transform(&mut self, effect: Self::Effect, descriptor: &Descriptor);
}

/// Safe semantic descriptor. No Rust layout or native padding is exposed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    flags: u8,
    translation: [u32; 3],
    scale: u32,
}

impl Descriptor {
    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn translation_words(&self) -> [u32; 3] {
        self.translation
    }

    pub fn scale_word(&self) -> u32 {
        self.scale
    }

    /// Exactly 54 bytes are defined; native bytes +2/+3 are unspecified.
    pub const DEFINED_BYTES: [bool; 56] = {
        let mut mask = [true; 56];
        mask[2] = false;
        mask[3] = false;
        mask
    };

    /// Portable backing choice: preserve caller bytes +2/+3, write every defined
    /// byte explicitly in little-endian order. No claim that native padding is 0.
    pub fn project_into(&self, backing: &mut [u8; 56]) {
        backing[0] = 2;
        backing[1] = self.flags;
        let identity = [0x3f80_0000_u32, 0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000];
        for (i, word) in identity.into_iter().enumerate() {
            backing[4 + i * 4..8 + i * 4].copy_from_slice(&word.to_le_bytes());
        }
        for (i, word) in self.translation.into_iter().enumerate() {
            backing[40 + i * 4..44 + i * 4].copy_from_slice(&word.to_le_bytes());
        }
        backing[52..56].copy_from_slice(&self.scale.to_le_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    CaptureService,
    ReadX,
    ReadY,
    ReadZ,
    Scalar(u8),
    Floor(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    NumericMode,
    Position,
    Scalar(u8),
    FloorReply(u8),
}

/// Portable qualification failure, not a native false/error return. `reached`
/// records the completed prefix; no final transform is sent on this path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    pub reason: UnsupportedReason,
    pub reached: Vec<Step>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionOutcome {
    NullEffect,
    Dispatched,
}

/// H is the only persistent helper state. Destruction consumes this value;
/// Rust Drop deliberately has no implicit Shutdown or Release behavior.
pub struct LocalGridHelper<E: Copy> {
    h: Option<E>,
}

impl<E: Copy> Default for LocalGridHelper<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Copy> LocalGridHelper<E> {
    pub fn new() -> Self {
        Self { h: None }
    }

    pub fn retained_effect(&self) -> Option<E> {
        self.h
    }

    pub fn init(&mut self, host: &mut impl Host<Effect = E>) {
        if self.h.is_none() {
            let _ = host.raw_factory(b"local_grid\0", &mut self.h);
        }
    }

    pub fn show_effect(&self, host: &mut impl Host<Effect = E>, argument: u32) {
        if let Some(effect) = self.h {
            let active = host.active_query(effect) as u8 != 0;
            let requested = argument as u8 != 0;
            if requested && !active {
                if let Some(effect) = self.h {
                    host.start(effect, 0);
                }
            } else if !requested && active {
                if let Some(effect) = self.h {
                    host.stop(effect, 0);
                }
            }
        }
    }

    pub fn shutdown(&mut self, host: &mut impl Host<Effect = E>) {
        if let Some(effect) = self.h {
            if host.active_query(effect) as u8 != 0 {
                if let Some(effect) = self.h {
                    host.stop(effect, 0);
                }
            }
            if let Some(effect) = self.h {
                self.h = None;
                host.release(effect, self);
            }
        }
    }

    pub fn destroy(self, host: &mut impl Host<Effect = E>) {
        if let Some(effect) = self.h {
            host.release(effect, &self);
        }
    }

    pub fn set_position(
        &self,
        host: &mut impl Host<Effect = E>,
        position: &mut impl Position,
        mode: NumericMode,
    ) -> Result<PositionOutcome, Unsupported> {
        if self.h.is_none() {
            return Ok(PositionOutcome::NullEffect);
        }
        let mut reached = Vec::new();
        if mode != NumericMode::QualifiedDyadicNearest {
            return Err(unsupported(UnsupportedReason::NumericMode, reached));
        }
        let service = host.capture_service();
        reached.push(Step::CaptureService);
        let x = f64::from(f32::from_bits(position.read_x()));
        reached.push(Step::ReadX);
        if !bounded_dyadic(x) {
            return Err(unsupported(UnsupportedReason::Position, reached));
        }
        let s1 = scalar(host, service, 1, &mut reached)?;
        let px = floor_product(host, x, s1, 1, &mut reached)?;
        let s2 = scalar(host, service, 2, &mut reached)?;
        let y = position.read_y();
        reached.push(Step::ReadY);
        let z = f64::from(f32::from_bits(position.read_z()));
        reached.push(Step::ReadZ);
        if !f32::from_bits(y).is_finite() || !bounded_dyadic(z) {
            return Err(unsupported(UnsupportedReason::Position, reached));
        }
        // Separate multiply and add, then the R32 store; see bounded proof.
        let ox = s2 * 0.0625;
        let tx = nearest32(exact_add_nearest(px, ox));
        let s3 = scalar(host, service, 3, &mut reached)?;
        let pz = floor_product(host, z, s3, 2, &mut reached)?;
        let s4 = scalar(host, service, 4, &mut reached)?;
        let oz = s4 * 0.0625;
        let tz = nearest32(exact_add_nearest(pz, oz));
        let s5 = scalar(host, service, 5, &mut reached)?;
        let held_scale = s5 * 16.0;
        let scale = nearest32(held_scale);
        let descriptor = Descriptor {
            flags: if held_scale == 1.0 { 1 } else { 3 },
            translation: [tx, y, tz],
            scale,
        };
        if let Some(effect) = self.h {
            host.set_transform(effect, &descriptor);
        }
        Ok(PositionOutcome::Dispatched)
    }
}

fn unsupported(reason: UnsupportedReason, reached: Vec<Step>) -> Unsupported {
    Unsupported { reason, reached }
}

// Finite multiples of 2^-30, magnitude <= 2^16. Multiplication by 2^30 is
// an exact exponent shift; accepted integers have at most 47 significant bits.
fn bounded_dyadic(value: f64) -> bool {
    value.is_finite() && value.abs() <= 65536.0 && (value * 1073741824.0).fract() == 0.0
}

fn divisor(value: f64) -> bool {
    let bits = value.to_bits();
    value > 0.0 && (1.0 / 65536.0..=65536.0).contains(&value) && bits & ((1_u64 << 52) - 1) == 0
}

fn scalar<H: Host>(host: &mut H, service: H::Service, index: u8, reached: &mut Vec<Step>) -> Result<f64, Unsupported> {
    let value = host.raw_scalar(service);
    reached.push(Step::Scalar(index));
    if !bounded_dyadic(value) || ((index == 1 || index == 3) && !divisor(value)) {
        return Err(unsupported(UnsupportedReason::Scalar(index), reached.clone()));
    }
    // s1/s3 are explicitly materialized as binary64 here, also reused below.
    Ok(value)
}

fn floor_product<H: Host>(
    host: &mut H,
    coordinate: f64,
    stored_divisor: f64,
    index: u8,
    reached: &mut Vec<Step>,
) -> Result<f64, Unsupported> {
    // Power-of-two division is exact; this f64 is the actual R64 floor input.
    let quotient = coordinate / stored_divisor;
    let reply = host.raw_floor(quotient);
    reached.push(Step::Floor(index));
    // Validate the declared finite ordinary-floor domain, never replace the
    // primitive reply. Signed zero is preserved by that primitive contract.
    if !reply.is_finite() || reply.to_bits() != quotient.floor().to_bits() {
        return Err(unsupported(UnsupportedReason::FloorReply(index), reached.clone()));
    }
    // Product is materialized binary64 before the fresh scalar request.
    Ok(reply * stored_divisor)
}

fn exact_add_nearest(a: f64, b: f64) -> f64 {
    let sum = a + b;
    // All magnitudes are exact here. Explicitly choose nearest-mode zero sign
    // as well: cancellation is +0, and only (-0)+(-0) produces -0.
    if sum == 0.0 {
        f64::from_bits(a.to_bits() & b.to_bits() & (1_u64 << 63))
    } else {
        sum
    }
}

/// Deterministic nearest-even binary32 store for the proved bounded domain.
/// Every nonzero result is normal binary32 (minimum magnitude 2^-34).
/// Extract/round the significand with integers; no ambient float-to-f32 cast.
fn nearest32(value: f64) -> u32 {
    let bits = value.to_bits();
    let sign = ((bits >> 32) as u32) & 0x8000_0000;
    if value == 0.0 {
        return sign;
    }
    let mut exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    let significand = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let mut rounded = significand >> 29;
    let remainder = significand & ((1_u64 << 29) - 1);
    let half = 1_u64 << 28;
    if remainder > half || (remainder == half && rounded & 1 != 0) {
        rounded += 1;
    }
    if rounded == 1_u64 << 24 {
        rounded >>= 1;
        exponent += 1;
    }
    sign | (((exponent + 127) as u32) << 23) | ((rounded as u32) & 0x007f_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DirectOutput {
        expected: usize,
    }

    impl Host for DirectOutput {
        type Effect = u32;
        type Service = u32;

        fn raw_factory(&mut self, name: &[u8], output: &mut Option<u32>) -> u32 {
            assert_eq!(name, b"local_grid\0");
            assert_eq!(output as *mut Option<u32> as usize, self.expected);
            *output = Some(91);
            0 // Failure status must not suppress this direct output write.
        }
        fn active_query(&mut self, _: u32) -> u32 {
            panic!("unexpected query")
        }
        fn start(&mut self, _: u32, _: u32) {
            panic!("unexpected start")
        }
        fn stop(&mut self, _: u32, _: u32) {
            panic!("unexpected stop")
        }
        fn release(&mut self, _: u32, _: &LocalGridHelper<u32>) {
            panic!("unexpected release")
        }
        fn capture_service(&mut self) -> u32 {
            panic!("unexpected capture")
        }
        fn raw_scalar(&mut self, _: u32) -> f64 {
            panic!("unexpected scalar")
        }
        fn raw_floor(&mut self, _: f64) -> f64 {
            panic!("unexpected floor")
        }
        fn set_transform(&mut self, _: u32, _: &Descriptor) {
            panic!("unexpected transform")
        }
    }

    #[test]
    fn factory_receives_the_actual_h_field_not_a_temporary() {
        let mut helper = LocalGridHelper::<u32>::new();
        let mut host = DirectOutput {
            expected: &mut helper.h as *mut Option<u32> as usize,
        };
        helper.init(&mut host);
        assert_eq!(helper.h, Some(91));
        helper.init(&mut host);
    }

    // Tests the integer store conversion against independent halfway literals,
    // including ties to odd/even, carry into the exponent, and negative values.
    #[test]
    fn nearest_even_store_boundaries() {
        for (input, word) in [
            (1.0 + 1.0 / 16777216.0, 0x3f80_0000),
            (1.0 + 3.0 / 16777216.0, 0x3f80_0002),
            (2.0 - 1.0 / 16777216.0, 0x4000_0000),
            (-1.0 - 1.0 / 16777216.0, 0xbf80_0000),
            (-1.0 - 3.0 / 16777216.0, 0xbf80_0002),
            (1.0 / 17179869184.0, 0x2e80_0000),
            (-0.0, 0x8000_0000),
        ] {
            assert_eq!(nearest32(input), word);
        }
        assert_eq!(exact_add_nearest(-1.0, 1.0).to_bits(), 0);
        assert_eq!(exact_add_nearest(-0.0, -0.0).to_bits(), 1_u64 << 63);
    }
}
