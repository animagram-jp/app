use arbitrary_int::traits::Integer;

#[derive(Clone, Copy)]
pub struct Field {
    pub position: u32,
    mask:         u64,
    signed:       bool,
}

impl Field {
    pub const fn new(position: u32, width: u32) -> Self {
        Self::build(position, width, false)
    }
    pub const fn new_signed(position: u32, width: u32) -> Self {
        Self::build(position, width, true)
    }
    const fn build(position: u32, width: u32, signed: bool) -> Self {
        assert!(width >= 1 && width < 64 && position + width <= 64);
        Self { position, mask: (1u64 << width) - 1, signed }
    }
    #[inline(always)]
    pub fn get<T: Integer>(&self, target: u64) -> T {
        let value = (target >> self.position) & self.mask;
        if self.signed {
            let shift = 64 - self.mask.count_ones();
            ((value << shift) as i64 >> shift).as_::<T>()
        } else {
            value.as_::<T>()
        }
    }
    #[inline(always)]
    pub fn set<T: Integer>(&self, target: u64, value: T) -> u64 {
        (target & !(self.mask << self.position)) | ((value.as_u64() & self.mask) << self.position)
    }
}

#[derive(Clone, Copy)]
pub enum Spec {
    Unsigned(u32),
    Signed(u32),
}

impl Spec {
    const fn width(self) -> u32 {
        match self {
            Spec::Unsigned(width) | Spec::Signed(width) => width,
        }
    }
}

pub struct Layout<const N: usize> {
    fields: [Field; N],
    bits:   u32,
}

impl<const N: usize> Layout<N> {
    pub const fn new(specs: [Spec; N]) -> Self {
        Self::with_padding(specs, 0)
    }

    pub const fn with_padding(specs: [Spec; N], padding: u32) -> Self {
        let mut bits = padding;
        let mut i = 0;
        while i < N {
            bits += specs[i].width();
            i += 1;
        }
        assert!(bits <= 64);
        let mut fields = [Field::new(0, 1); N];
        let mut position = bits;
        let mut i = 0;
        while i < N {
            position -= specs[i].width();
            fields[i] = match specs[i] {
                Spec::Unsigned(width) => Field::new(position, width),
                Spec::Signed(width) => Field::new_signed(position, width),
            };
            i += 1;
        }
        Self { fields, bits }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }

    pub const fn bytes(&self) -> usize {
        self.bits.div_ceil(8) as usize
    }

    pub const fn field(&self, index: usize) -> &Field {
        &self.fields[index]
    }

    #[inline(always)]
    pub fn get<T: Integer>(&self, target: u64, index: usize) -> T {
        self.fields[index].get(target)
    }

    #[inline(always)]
    pub fn set<T: Integer>(&self, target: u64, index: usize, value: T) -> u64 {
        self.fields[index].set(target, value)
    }

    pub fn pack(&self, values: [u64; N]) -> u64 {
        let mut target = 0u64;
        for (field, value) in self.fields.iter().zip(values) {
            target = field.set(target, value);
        }
        target
    }

    pub fn unpack<T: Integer>(&self, target: u64) -> [T; N] {
        core::array::from_fn(|i| self.fields[i].get(target))
    }

    pub fn decode(&self, bytes: &[u8]) -> Option<u64> {
        let bytes = bytes.get(..self.bytes())?;
        let mut word = [0u8; 8];
        word[..bytes.len()].copy_from_slice(bytes);
        Some(u64::from_le_bytes(word))
    }

    pub fn encode(&self, target: u64) -> ([u8; 8], usize) {
        (target.to_le_bytes(), self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;

    #[test]
    fn widest_and_topmost_fields() {
        let f = Field::new(63, 1);
        assert_eq!(f.set(0u64, 1u64), 1 << 63);
        let g = Field::new(0, 63);
        assert_eq!(g.get::<u64>(!0u64), (1 << 63) - 1);
    }

    use super::Spec::{Signed, Unsigned};

    #[test]
    fn layout_full_width_has_no_spare_bits() {
        let l = Layout::new([Unsigned(32), Unsigned(32)]);
        assert_eq!(l.pack([u32::MAX as u64, u32::MAX as u64]), !0u64);
    }

    #[test]
    fn random_layouts_round_trip_and_fields_never_leak_into_each_other() {
        const FIELDS: usize = 5;
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let widths: [u32; FIELDS] = core::array::from_fn(|_| 1 + rng.below(12) as u32);
            let signed: [bool; FIELDS] = core::array::from_fn(|_| rng.chance(50));
            let specs = core::array::from_fn(|i| {
                if signed[i] { Signed(widths[i]) } else { Unsigned(widths[i]) }
            });
            let padding = rng.below(5) as u32;
            let layout = Layout::with_padding(specs, padding);

            let bits = widths.iter().sum::<u32>() + padding;
            assert_eq!((layout.bits(), layout.bytes()), (bits, bits.div_ceil(8) as usize));
            let mut position = bits;
            for (index, width) in widths.iter().enumerate() {
                position -= width;
                assert_eq!(layout.field(index).position, position, "seed {seed} field {index}");
            }

            let expect = |index: usize, value: u64| -> i64 {
                let width = widths[index];
                let low = value & ((1u64 << width) - 1);
                if signed[index] && low >> (width - 1) == 1 {
                    low as i64 - (1i64 << width)
                } else {
                    low as i64
                }
            };
            let values: [u64; FIELDS] = core::array::from_fn(|_| rng.next_u64());
            let raw = layout.pack(values);
            assert_eq!(raw >> bits, 0, "seed {seed}");
            assert_eq!(raw & ((1u64 << padding) - 1), 0, "seed {seed}");
            let unpacked = layout.unpack::<i64>(raw);
            for index in 0..FIELDS {
                assert_eq!(
                    unpacked[index],
                    expect(index, values[index]),
                    "seed {seed} field {index}"
                );
            }

            let index = rng.below(FIELDS);
            let replacement = rng.next_u64();
            let changed = layout.unpack::<i64>(layout.set(raw, index, replacement));
            for other in 0..FIELDS {
                let want =
                    if other == index { expect(index, replacement) } else { unpacked[other] };
                assert_eq!(changed[other], want, "seed {seed} set {index} field {other}");
            }

            let (word, length) = layout.encode(raw);
            assert_eq!(layout.decode(&word[..length]), Some(raw), "seed {seed}");
            assert_eq!(layout.decode(&word[..length - 1]), None, "seed {seed}");
        }
    }
}
