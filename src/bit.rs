use core::{
    assert,
    clone::Clone,
    marker::Copy,
    ops::Range,
    primitive::{i8, i16, i32, i64, u8, u16, u32, u64, usize},
};

use arbitrary_int::traits::Integer;

#[derive(Clone, Copy)]
pub enum Unit {
    Bit,
    Byte,
}

impl Unit {
    pub const fn bits(self) -> u32 {
        match self {
            Self::Bit => 1,
            Self::Byte => 8,
        }
    }

    pub const fn valid(self, bits: u32) -> bool {
        match self {
            Self::Bit => bits <= 57,
            Self::Byte => bits % 8 == 0,
        }
    }
}

pub trait Field: Copy {
    const BITS: u32;
    const UNIT: Unit;
    const MASK: &'static [u8];
    fn read(bytes: &[u8], position: u32) -> Self;
    fn write(self, bytes: &mut [u8], position: u32);
}

pub trait Bits: Copy + From<Self::Value> + Into<Self::Value> {
    type Value: Integer;
}

impl<T: Bits> Field for T {
    const BITS: u32 = bits::<T::Value>();
    const UNIT: Unit = Unit::Bit;
    const MASK: &'static [u8] = &[];

    fn read(bytes: &[u8], position: u32) -> Self {
        Self::from(read(bytes, position))
    }

    fn write(self, bytes: &mut [u8], position: u32) {
        write(bytes, position, self.into())
    }
}

pub trait Composite<F: Field> {
    const POSITION: u32;
    const RANGE: Range<u32>;
    fn get(&self) -> F;
    fn set(&mut self, field: F);
}

pub trait Zero {
    const ZERO: Self;
}

macro_rules! primitive {
    (@zero $($t:ty),+) => {$(
        impl Zero for $t {
            const ZERO: Self = 0;
        }
    )+};
    (@field $($t:ty),+) => {$(
        impl Bits for $t {
            type Value = $t;
        }
    )+};
}
primitive!(@zero u8, u16, u32, i8, i16, i32);
primitive!(@field u8, u16, u32, i8, i16, i32);

pub const fn bits<V: Integer>() -> u32 {
    let b = V::BITS as u32;
    assert!(Unit::Bit.valid(b));
    b
}

pub const fn bytes(bits: u32) -> usize {
    bits.div_ceil(8) as usize
}

pub const fn align(end: u32, unit: Unit) -> u32 {
    end.next_multiple_of(unit.bits())
}

pub const fn fill<const N: usize>(
    mut mask: [u8; N],
    unit: Unit,
    range: Range<u32>,
    field: &[u8],
) -> [u8; N] {
    match unit {
        Unit::Bit => {
            let mut i = range.start;
            while i < range.end {
                mask[(i / 8) as usize] |= 1 << (i % 8);
                i += 1;
            }
        }
        Unit::Byte => {
            let start = (range.start / 8) as usize;
            let mut j = 0;
            while j < field.len() {
                mask[start + j] |= field[j];
                j += 1;
            }
        }
    }
    mask
}

const fn mask(bits: u32) -> u64 {
    (1 << bits) - 1
}

const fn range(position: u32, bits: u32) -> Range<usize> {
    (position / 8) as usize..(position + bits).div_ceil(8) as usize
}

pub fn read<V: Integer>(bytes: &[u8], position: u32) -> V {
    let b = const { bits::<V>() };
    let r = range(position, b);
    let mut word = [0u8; 8];
    word[..r.len()].copy_from_slice(&bytes[r]);
    let value = (u64::from_le_bytes(word) >> (position % 8)) & mask(b);
    if V::IS_SIGNED {
        let shift = 64 - b;
        ((value << shift) as i64 >> shift).as_::<V>()
    } else {
        value.as_::<V>()
    }
}

pub fn write<V: Integer>(bytes: &mut [u8], position: u32, value: V) {
    let b = const { bits::<V>() };
    let r = range(position, b);
    let offset = position % 8;
    let mut word = [0u8; 8];
    word[..r.len()].copy_from_slice(&bytes[r.clone()]);
    let cleared = u64::from_le_bytes(word) & !(mask(b) << offset);
    let value = (value.as_u64() & mask(b)) << offset;
    bytes[r.clone()].copy_from_slice(&(cleared | value).to_le_bytes()[..r.len()]);
}

pub fn read_bytes<const N: usize>(bytes: &[u8], position: u32) -> [u8; N] {
    let start = (position / 8) as usize;
    let mut value = [0u8; N];
    value.copy_from_slice(&bytes[start..start + N]);
    value
}

pub fn write_bytes<const N: usize>(bytes: &mut [u8], position: u32, value: [u8; N]) {
    let start = (position / 8) as usize;
    bytes[start..start + N].copy_from_slice(&value);
}

macro_rules! field {
    ($($(#[$meta:meta])* $vis:vis $name:ident($($value:tt)+)),+ $(,)?) => {$(
        $crate::bit::field!(@one $(#[$meta])* $vis $name ($($value)+));
    )+};
    (@one $(#[$meta:meta])* $vis:vis $name:ident ([u8; $n:expr])) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis struct $name(pub [u8; $n]);

        impl $crate::bit::Field for $name {
            const BITS: u32 = $n as u32 * 8;
            const UNIT: $crate::bit::Unit = $crate::bit::Unit::Byte;
            const MASK: &'static [u8] = &[0xFF; $n];

            fn read(bytes: &[u8], position: u32) -> Self {
                Self($crate::bit::read_bytes(bytes, position))
            }

            fn write(self, bytes: &mut [u8], position: u32) {
                $crate::bit::write_bytes(bytes, position, self.0)
            }
        }

        impl $crate::bit::Zero for $name {
            const ZERO: Self = Self([0; $n]);
        }

        impl From<[u8; $n]> for $name {
            fn from(value: [u8; $n]) -> Self {
                Self(value)
            }
        }

        impl From<$name> for [u8; $n] {
            fn from(field: $name) -> Self {
                field.0
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.0.iter().rev().cmp(other.0.iter().rev())
            }
        }
    };
    (@one $(#[$meta:meta])* $vis:vis $name:ident ($value:ty)) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        $vis struct $name(pub $value);

        impl $crate::bit::Bits for $name {
            type Value = $value;
        }

        impl $crate::bit::Zero for $name {
            const ZERO: Self = Self(<$value as ::arbitrary_int::traits::Integer>::ZERO);
        }

        impl From<$value> for $name {
            fn from(value: $value) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $value {
            fn from(field: $name) -> Self {
                field.0
            }
        }
    };
}
pub(crate) use field;

macro_rules! composite {
    (@end $acc:expr;) => { $acc };
    (@end $acc:expr; $first:ty $(, $rest:ty)*) => {
        $crate::bit::composite!(@end
            $crate::bit::align($acc, <$first as $crate::bit::Field>::UNIT)
                + <$first as $crate::bit::Field>::BITS;
            $($rest),*)
    };
    (@position $name:ident [$($previous:ty),*]) => {};
    (@position $name:ident [$($previous:ty),*] $field:ty $(, $rest:ty)*) => {
        impl $crate::bit::Composite<$field> for $name {
            const POSITION: u32 = $crate::bit::align(
                $crate::bit::composite!(@end 0; $($previous),*),
                <$field as $crate::bit::Field>::UNIT,
            );
            const RANGE: ::core::ops::Range<u32> =
                <$name as $crate::bit::Composite<$field>>::POSITION
                    ..<$name as $crate::bit::Composite<$field>>::POSITION
                        + <$field as $crate::bit::Field>::BITS;

            fn get(&self) -> $field {
                <$field as $crate::bit::Field>::read(
                    &self.0,
                    <Self as $crate::bit::Composite<$field>>::POSITION,
                )
            }

            fn set(&mut self, field: $field) {
                <$field as $crate::bit::Field>::write(
                    field,
                    &mut self.0,
                    <Self as $crate::bit::Composite<$field>>::POSITION,
                )
            }
        }

        $crate::bit::composite!(@position $name [$($previous,)* $field] $($rest),*);
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident { $($field:ty),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis struct $name([u8; $crate::bit::bytes($crate::bit::composite!(@end 0; $($field),+))]);

        impl $name {
            pub const BITS: u32 = $crate::bit::composite!(@end 0; $($field),+);
            pub const BYTES: usize = $crate::bit::bytes(Self::BITS);
            pub const MASK: [u8; Self::BYTES] = {
                let mut mask = [0u8; Self::BYTES];
                $(mask = $crate::bit::fill(
                    mask,
                    <$field as $crate::bit::Field>::UNIT,
                    <Self as $crate::bit::Composite<$field>>::RANGE,
                    <$field as $crate::bit::Field>::MASK,
                );)+
                mask
            };

            pub const fn from_le_bytes(mut bytes: [u8; Self::BYTES]) -> Self {
                let mut i = 0;
                while i < Self::BYTES {
                    bytes[i] &= Self::MASK[i];
                    i += 1;
                }
                Self(bytes)
            }

            pub const fn to_le_bytes(self) -> [u8; Self::BYTES] {
                self.0
            }

            pub fn get<F: $crate::bit::Field>(&self) -> F
            where
                Self: $crate::bit::Composite<F>,
            {
                <Self as $crate::bit::Composite<F>>::get(self)
            }

            pub fn set<F: $crate::bit::Field>(&mut self, field: F)
            where
                Self: $crate::bit::Composite<F>,
            {
                <Self as $crate::bit::Composite<F>>::set(self, field)
            }
        }

        impl $crate::bit::Field for $name {
            const BITS: u32 = Self::BYTES as u32 * 8;
            const UNIT: $crate::bit::Unit = $crate::bit::Unit::Byte;
            const MASK: &'static [u8] = &Self::MASK;

            fn read(bytes: &[u8], position: u32) -> Self {
                Self::from_le_bytes($crate::bit::read_bytes(bytes, position))
            }

            fn write(self, bytes: &mut [u8], position: u32) {
                $crate::bit::write_bytes(bytes, position, self.0)
            }
        }

        impl $crate::Output for $name {
            fn encode(&self, output: &mut ::alloc::vec::Vec<u8>) {
                output.extend_from_slice(&self.0);
            }
        }

        impl $crate::bit::Zero for $name {
            const ZERO: Self = Self([0; Self::BYTES]);
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.0.iter().rev().cmp(other.0.iter().rev())
            }
        }

        $crate::bit::composite!(@position $name [] $($field),+);
    };
}
pub(crate) use composite;

#[cfg(test)]
mod tests {
    use core::{cmp::Ordering, primitive::u128};

    use arbitrary_int::{i3, i10, u1, u3, u4, u5, u7, u9, u13, u57};

    use super::*;
    use crate::Rng;

    field! {
        Low(u4),
        Middle(u4),
        High(u8),
        Flag(u1),
        Wide(u57),
        Tail(i3),
        P(u9),
        Q(i10),
        R(u1),
        S(u7),
        T(i3),
        U(u13),
        Tag(u3),
        Uid([u8; 2]),
        Count(u5),
    }

    composite! { struct Small { Low, Middle, High } }
    composite! { struct Straddle { Flag, Wide, Tail } }
    composite! { struct Mixed { P, Q, R, S, T, U } }
    composite! { struct Primitive { Flag, i16, u8, Tail, u32 } }
    composite! { struct Padded { Tag, Uid, Count } }
    composite! { struct Outer { Count, Padded, Tag } }

    fn random_mixed(rng: &mut Rng) -> Mixed {
        let mut mixed = Mixed::ZERO;
        mixed.set(P(u9::masked_new(rng.next_u64())));
        mixed.set(Q(i10::masked_new(rng.next_u64())));
        mixed.set(R(u1::masked_new(rng.next_u64())));
        mixed.set(S(u7::masked_new(rng.next_u64())));
        mixed.set(T(i3::masked_new(rng.next_u64())));
        mixed.set(U(u13::masked_new(rng.next_u64())));
        mixed
    }

    fn integer(mixed: &Mixed) -> u128 {
        let mut word = [0u8; 16];
        word[..mixed.0.len()].copy_from_slice(&mixed.0);
        u128::from_le_bytes(word)
    }

    #[test]
    fn fields_follow_the_declaration_order_from_the_lowest_bit() {
        assert_eq!(
            (
                <Small as Composite<Low>>::POSITION,
                <Small as Composite<Middle>>::POSITION,
                <Small as Composite<High>>::POSITION,
            ),
            (0, 4, 8)
        );
        let mut small = Small::ZERO;
        small.set(Low(u4::new(1)));
        small.set(Middle(u4::new(2)));
        small.set(High(0x34));
        assert_eq!(small.0, [0x21, 0x34]);
        assert_eq!(small.get::<Middle>(), Middle(u4::new(2)));
    }

    #[test]
    fn primitive_fields_round_trip_between_arbitrary_ones() {
        let mut primitive = Primitive::ZERO;
        primitive.set(Flag(u1::new(1)));
        primitive.set(-12345i16);
        primitive.set(0xA5u8);
        primitive.set(Tail(i3::new(-3)));
        primitive.set(u32::MAX - 7);
        assert_eq!(Primitive::BITS, 1 + 16 + 8 + 3 + 32);
        assert_eq!(primitive.get::<Flag>(), Flag(u1::new(1)));
        assert_eq!(primitive.get::<i16>(), -12345);
        assert_eq!(primitive.get::<u8>(), 0xA5);
        assert_eq!(primitive.get::<Tail>(), Tail(i3::new(-3)));
        assert_eq!(primitive.get::<u32>(), u32::MAX - 7);
        assert_eq!(<u8 as Zero>::ZERO, 0);
        assert_eq!(<i32 as Zero>::ZERO, 0);
    }

    #[test]
    fn a_byte_field_is_padded_to_the_next_byte_boundary() {
        assert_eq!(
            (
                <Padded as Composite<Tag>>::RANGE,
                <Padded as Composite<Uid>>::RANGE,
                <Padded as Composite<Count>>::RANGE,
            ),
            (0..3, 8..24, 24..29)
        );
        assert_eq!((Padded::BITS, Padded::BYTES), (29, 4));
        assert_eq!(Padded::MASK, [0b0000_0111, 0xFF, 0xFF, 0b0001_1111]);
        let mut padded = Padded::ZERO;
        padded.set(Tag(u3::new(5)));
        padded.set(Uid([0xAB, 0xCD]));
        padded.set(Count(u5::new(17)));
        assert_eq!(padded.0, [0x05, 0xAB, 0xCD, 17]);
        assert_eq!(padded.get::<Tag>(), Tag(u3::new(5)));
        assert_eq!(padded.get::<Uid>(), Uid([0xAB, 0xCD]));
        assert_eq!(padded.get::<Count>(), Count(u5::new(17)));
        let full = Padded::from_le_bytes([0xFF; 4]);
        assert_eq!(full.0, Padded::MASK);
        assert_eq!(full, {
            let mut same = Padded::ZERO;
            same.set(Tag(u3::MAX));
            same.set(Uid([0xFF, 0xFF]));
            same.set(Count(u5::MAX));
            same
        });
    }

    #[test]
    fn a_composite_nests_as_a_byte_field() {
        assert_eq!(<Padded as Field>::BITS, 32);
        assert_eq!(
            (
                <Outer as Composite<Count>>::RANGE,
                <Outer as Composite<Padded>>::RANGE,
                <Outer as Composite<Tag>>::RANGE,
            ),
            (0..5, 8..40, 40..43)
        );
        assert_eq!((Outer::BITS, Outer::BYTES), (43, 6));
        let mut inner = Padded::ZERO;
        inner.set(Tag(u3::new(2)));
        inner.set(Uid([0x12, 0x34]));
        inner.set(Count(u5::new(9)));
        let mut outer = Outer::ZERO;
        outer.set(Count(u5::new(31)));
        outer.set(inner);
        outer.set(Tag(u3::new(7)));
        assert_eq!(outer.0, [31, 0x02, 0x12, 0x34, 9, 7]);
        assert_eq!(outer.get::<Padded>(), inner);
        assert_eq!(outer.get::<Padded>().get::<Uid>(), Uid([0x12, 0x34]));
        assert_eq!(outer.get::<Count>(), Count(u5::new(31)));
        assert_eq!(outer.get::<Tag>(), Tag(u3::new(7)));
        let mut raw = outer.0;
        raw[1] = 0xFF;
        let outer = Outer::from_le_bytes(raw);
        assert_eq!(outer.get::<Padded>().get::<Tag>(), Tag(u3::MAX));
        assert_eq!(outer.0[1], 0x07);
    }

    #[test]
    fn storage_size_follows_the_total_bits() {
        assert_eq!((Small::BITS, size_of::<Small>()), (16, 2));
        assert_eq!((Straddle::BITS, size_of::<Straddle>()), (61, 8));
        assert_eq!((Mixed::BITS, size_of::<Mixed>()), (43, 6));
    }

    #[test]
    fn a_full_width_field_straddling_eight_bytes_round_trips() {
        for seed in 0..1000 {
            let mut rng = Rng::new(seed);
            let (flag, wide, tail) = (
                u1::masked_new(rng.next_u64()),
                u57::masked_new(rng.next_u64()),
                i3::masked_new(rng.next_u64()),
            );
            let mut straddle = Straddle::ZERO;
            straddle.set(Flag(flag));
            straddle.set(Wide(wide));
            straddle.set(Tail(tail));
            assert_eq!(straddle.get::<Flag>(), Flag(flag), "seed {seed}");
            assert_eq!(straddle.get::<Wide>(), Wide(wide), "seed {seed}");
            assert_eq!(straddle.get::<Tail>(), Tail(tail), "seed {seed}");
            assert_eq!(straddle.0[7] >> 5, 0, "seed {seed}");
        }
    }

    #[test]
    fn setting_one_field_leaves_the_others_and_the_spare_bits_untouched() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let before = random_mixed(&mut rng);
            let source = random_mixed(&mut rng);
            let mut after = before;
            let index = rng.below(6);
            match index {
                0 => after.set(source.get::<P>()),
                1 => after.set(source.get::<Q>()),
                2 => after.set(source.get::<R>()),
                3 => after.set(source.get::<S>()),
                4 => after.set(source.get::<T>()),
                _ => after.set(source.get::<U>()),
            }
            let pick = |i: usize| if i == index { &source } else { &before };
            assert_eq!(after.get::<P>(), pick(0).get::<P>(), "seed {seed}");
            assert_eq!(after.get::<Q>(), pick(1).get::<Q>(), "seed {seed}");
            assert_eq!(after.get::<R>(), pick(2).get::<R>(), "seed {seed}");
            assert_eq!(after.get::<S>(), pick(3).get::<S>(), "seed {seed}");
            assert_eq!(after.get::<T>(), pick(4).get::<T>(), "seed {seed}");
            assert_eq!(after.get::<U>(), pick(5).get::<U>(), "seed {seed}");
            assert_eq!(integer(&after) >> Mixed::BITS, 0, "seed {seed}");
        }
    }

    #[test]
    fn bytes_round_trip_and_spare_bits_are_cleared_on_the_way_in() {
        for seed in 0..1000 {
            let mut rng = Rng::new(seed);
            let mixed = random_mixed(&mut rng);
            assert_eq!(Mixed::from_le_bytes(mixed.to_le_bytes()), mixed, "seed {seed}");
        }
        let full = Mixed::from_le_bytes([0xFF; 6]);
        assert_eq!(integer(&full), (1 << Mixed::BITS) - 1);
        assert_eq!(full.get::<U>(), U(u13::MAX));
    }

    #[test]
    fn order_matches_the_whole_value_as_an_integer() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let (a, b) = (random_mixed(&mut rng), random_mixed(&mut rng));
            assert_eq!(a.cmp(&b), integer(&a).cmp(&integer(&b)), "seed {seed}");
            assert_eq!(a.cmp(&a), Ordering::Equal, "seed {seed}");
        }
    }
}
