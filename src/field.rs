use core::marker::PhantomData;

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
        self.put(target, value.as_u64())
    }
    #[inline(always)]
    pub const fn put(&self, target: u64, value: u64) -> u64 {
        (target & !(self.mask << self.position)) | ((value & self.mask) << self.position)
    }
}

#[derive(Clone, Copy)]
pub enum Spec {
    Unsigned(u32),
    Signed(u32),
}

impl Spec {
    pub const fn of<T: Integer>() -> Self {
        if T::IS_SIGNED { Spec::Signed(T::BITS as u32) } else { Spec::Unsigned(T::BITS as u32) }
    }

    const fn width(self) -> u32 {
        match self {
            Spec::Unsigned(width) | Spec::Signed(width) => width,
        }
    }
}

pub trait Fields: Sized {
    const SPECS: &'static [Spec];
    fn unpack(layout: &Layout<Self>, target: u64) -> Self;
    fn pack(self, layout: &Layout<Self>) -> u64;
}

macro_rules! fields_for_tuples {
    ($(($($t:ident $i:tt),+))+) => {$(
        impl<$($t: Integer),+> Fields for ($($t,)+) {
            const SPECS: &'static [Spec] = &[$(Spec::of::<$t>()),+];

            fn unpack(layout: &Layout<Self>, target: u64) -> Self {
                ($(layout.field($i).get(target),)+)
            }

            fn pack(self, layout: &Layout<Self>) -> u64 {
                let mut target = 0;
                $(target = layout.field($i).set(target, self.$i);)+
                target
            }
        }
    )+};
}

fields_for_tuples! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
    (A 0, B 1, C 2, D 3, E 4)
    (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11)
}

impl Fields for () {
    const SPECS: &'static [Spec] = &[];

    fn unpack(_: &Layout<Self>, _: u64) -> Self {}

    fn pack(self, _: &Layout<Self>) -> u64 {
        0
    }
}

pub struct Layout<T: Fields> {
    specs:  &'static [Spec],
    bits:   u32,
    marker: PhantomData<T>,
}

impl<T: Fields> Default for Layout<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Fields> Layout<T> {
    pub const fn new() -> Self {
        Self::from_specs(T::SPECS)
    }

    const fn width_of(specs: &[Spec]) -> u32 {
        let mut bits = 0;
        let mut i = 0;
        while i < specs.len() {
            bits += specs[i].width();
            i += 1;
        }
        bits
    }

    const fn from_specs(specs: &'static [Spec]) -> Self {
        let bits = Self::width_of(specs);
        assert!(bits <= 64);
        Self { specs, bits, marker: PhantomData }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }

    pub const fn bytes(&self) -> usize {
        self.bits.div_ceil(8) as usize
    }

    pub const fn count(&self) -> usize {
        self.specs.len()
    }

    pub const fn field(&self, index: usize) -> Field {
        let mut position = 0;
        let mut i = 0;
        while i < index {
            position += self.specs[i].width();
            i += 1;
        }
        match self.specs[index] {
            Spec::Unsigned(width) => Field::new(position, width),
            Spec::Signed(width) => Field::new_signed(position, width),
        }
    }

    #[inline(always)]
    pub fn get<I: Integer>(&self, target: u64, index: usize) -> I {
        self.field(index).get(target)
    }

    #[inline(always)]
    pub fn set<I: Integer>(&self, target: u64, index: usize, value: I) -> u64 {
        self.field(index).set(target, value)
    }

    pub const fn pack_raw(&self, values: &[u64]) -> u64 {
        assert!(values.len() == self.specs.len());
        let mut target = 0u64;
        let mut i = 0;
        while i < values.len() {
            target = self.field(i).put(target, values[i]);
            i += 1;
        }
        target
    }

    pub fn pack(&self, values: T) -> u64 {
        values.pack(self)
    }

    pub fn unpack(&self, target: u64) -> T {
        T::unpack(self, target)
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
    use alloc::{boxed::Box, vec::Vec};
    use core::array::from_fn;

    use arbitrary_int::{i3, i10, u1, u4, u7, u9, u13};

    use super::{
        Spec::{Signed, Unsigned},
        *,
    };
    use crate::Rng;

    #[test]
    fn widest_and_topmost_fields() {
        let f = Field::new(63, 1);
        assert_eq!(f.set(0u64, 1u64), 1 << 63);
        let g = Field::new(0, 63);
        assert_eq!(g.get::<u64>(!0u64), (1 << 63) - 1);
    }

    #[test]
    fn layout_full_width_has_no_spare_bits() {
        let l = Layout::<(u32, u32)>::new();
        assert_eq!(l.pack((u32::MAX, u32::MAX)), !0u64);
    }

    #[test]
    fn specs_follow_the_tuple_types() {
        const L: Layout<(u9, i10, u1, u7, i3, u13)> = Layout::new();
        let widths: Vec<(u32, bool)> = (0..L.count())
            .map(|i| {
                let f = L.field(i);
                (f.mask.count_ones(), f.signed)
            })
            .collect();
        assert_eq!(
            widths,
            [(9, false), (10, true), (1, false), (7, false), (3, true), (13, false)]
        );
        assert_eq!(L.bits(), 43);
    }

    #[test]
    fn fields_follow_the_declaration_order_from_the_lowest_bit() {
        const L: Layout<(u4, u4, u8)> = Layout::new();
        assert_eq!((L.field(0).position, L.field(1).position, L.field(2).position), (0, 4, 8));
        let raw = L.pack((u4::new(1), u4::new(2), 0x34u8));
        assert_eq!(raw, 0x3421);
        let (word, length) = L.encode(raw);
        assert_eq!(&word[..length], &[0x21, 0x34]);
        assert_eq!(L.decode(&[0x21, 0x34]), Some(raw));
    }

    fn typed_round_trip<T: Fields>(seed: u64) {
        let layout = Layout::<T>::new();
        let mut rng = Rng::new(seed);
        let raw = rng.next_u64() >> (64 - layout.bits());
        let repacked = layout.pack(layout.unpack(raw));
        assert_eq!(repacked, raw, "seed {seed}");
        let mut by_index = 0u64;
        for i in 0..layout.count() {
            by_index = layout.set(by_index, i, layout.get::<i64>(raw, i));
        }
        assert_eq!(by_index, raw, "seed {seed}");
    }

    #[test]
    fn typed_tuples_round_trip_through_pack_and_unpack() {
        for seed in 0..1000 {
            typed_round_trip::<(u9, u9, i10, i10)>(seed);
            typed_round_trip::<(u9, u9, i10, i10, u7)>(seed);
            typed_round_trip::<(u13, u1, i3, u8, i16)>(seed);
            typed_round_trip::<(i32, u32)>(seed);
        }
    }

    #[test]
    fn random_layouts_round_trip_and_fields_never_leak_into_each_other() {
        const FIELDS: usize = 5;
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let widths: [u32; FIELDS] = from_fn(|_| 1 + rng.below(12) as u32);
            let signed: [bool; FIELDS] = from_fn(|_| rng.chance(50));
            let specs: [Spec; FIELDS] =
                from_fn(|i| if signed[i] { Signed(widths[i]) } else { Unsigned(widths[i]) });
            let layout = Layout::<()>::from_specs(Box::leak(Box::new(specs)));

            let bits = widths.iter().sum::<u32>();
            assert_eq!((layout.bits(), layout.bytes()), (bits, bits.div_ceil(8) as usize));
            let mut position = 0;
            for (index, width) in widths.iter().enumerate() {
                assert_eq!(layout.field(index).position, position, "seed {seed} field {index}");
                position += width;
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
            let values: [u64; FIELDS] = from_fn(|_| rng.next_u64());
            let raw = layout.pack_raw(&values);
            assert_eq!(raw >> bits, 0, "seed {seed}");
            let unpacked: [i64; FIELDS] = from_fn(|i| layout.get(raw, i));
            for index in 0..FIELDS {
                assert_eq!(
                    unpacked[index],
                    expect(index, values[index]),
                    "seed {seed} field {index}"
                );
            }

            let index = rng.below(FIELDS);
            let replacement = rng.next_u64();
            let changed: [i64; FIELDS] =
                from_fn(|i| layout.get(layout.set(raw, index, replacement), i));
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
