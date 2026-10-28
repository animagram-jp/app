use arbitrary_int::traits::Integer;

pub trait Field: Copy + From<Self::Value> + Into<Self::Value> {
    type Value: Integer;
}

pub trait Composite<F: Field> {
    const POSITION: u32;
    fn get(&self) -> F;
    fn set(&mut self, field: F);
}

pub const fn width<F: Field>() -> u32 {
    <F::Value as Integer>::BITS as u32
}

pub const fn bytes(bits: u32) -> usize {
    bits.div_ceil(8) as usize
}

const fn mask(width: u32) -> u64 {
    if width == 64 { u64::MAX } else { (1 << width) - 1 }
}

const fn window(position: u32, width: u32) -> (usize, usize) {
    ((position / 8) as usize, (position + width).div_ceil(8) as usize)
}

pub fn read<F: Field>(bytes: &[u8], position: u32) -> F {
    let width = width::<F>();
    let (start, end) = window(position, width);
    let mut word = [0u8; 16];
    word[..end - start].copy_from_slice(&bytes[start..end]);
    let value = (u128::from_le_bytes(word) >> (position % 8)) as u64 & mask(width);
    F::from(if <F::Value as Integer>::IS_SIGNED {
        let shift = 64 - width;
        ((value << shift) as i64 >> shift).as_::<F::Value>()
    } else {
        value.as_::<F::Value>()
    })
}

pub fn write<F: Field>(bytes: &mut [u8], position: u32, field: F) {
    let width = width::<F>();
    let (start, end) = window(position, width);
    let offset = position % 8;
    let mut word = [0u8; 16];
    word[..end - start].copy_from_slice(&bytes[start..end]);
    let cleared = u128::from_le_bytes(word) & !((mask(width) as u128) << offset);
    let value = ((Into::<F::Value>::into(field).as_u64() & mask(width)) as u128) << offset;
    bytes[start..end].copy_from_slice(&(cleared | value).to_le_bytes()[..end - start]);
}

macro_rules! field {
    ($($(#[$meta:meta])* $vis:vis $name:ident($value:ty)),+ $(,)?) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
        $vis struct $name(pub $value);

        impl $crate::bit::Field for $name {
            type Value = $value;
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
    )+};
}
pub(crate) use field;

macro_rules! composite {
    (@position $name:ident [$($previous:ty),*]) => {};
    (@position $name:ident [$($previous:ty),*] $field:ty $(, $rest:ty)*) => {
        impl $crate::bit::Composite<$field> for $name {
            const POSITION: u32 = 0 $(+ $crate::bit::width::<$previous>())*;

            fn get(&self) -> $field {
                $crate::bit::read(&self.0, <Self as $crate::bit::Composite<$field>>::POSITION)
            }

            fn set(&mut self, field: $field) {
                $crate::bit::write(
                    &mut self.0,
                    <Self as $crate::bit::Composite<$field>>::POSITION,
                    field,
                )
            }
        }

        $crate::bit::composite!(@position $name [$($previous,)* $field] $($rest),*);
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident { $($field:ty),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis struct $name([u8; $crate::bit::bytes(0 $(+ $crate::bit::width::<$field>())+)]);

        #[allow(dead_code)]
        impl $name {
            pub const BITS: u32 = 0 $(+ $crate::bit::width::<$field>())+;
            pub const BYTES: usize = $crate::bit::bytes(Self::BITS);

            pub const ZERO: Self = Self([0; Self::BYTES]);

            pub const fn from_le_bytes(mut bytes: [u8; Self::BYTES]) -> Self {
                if Self::BITS % 8 != 0 {
                    bytes[Self::BYTES - 1] &= (1 << (Self::BITS % 8)) - 1;
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

        impl $crate::Output for $name {
            fn encode(&self, output: &mut ::alloc::vec::Vec<u8>) {
                output.extend_from_slice(&self.0);
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.0.iter().rev().cmp(other.0.iter().rev())
            }
        }

        $crate::bit::composite!(@position $name [] $($field),+);
    };
}
pub(crate) use composite;

#[cfg(test)]
mod tests {
    use arbitrary_int::{i3, i10, u1, u4, u7, u9, u13};

    use super::*;
    use crate::Rng;

    field! {
        Low(u4),
        Middle(u4),
        High(u8),
        Flag(u1),
        Wide(u64),
        Tail(i3),
        P(u9),
        Q(i10),
        R(u1),
        S(u7),
        T(i3),
        U(u13),
    }

    composite! { struct Small { Low, Middle, High } }
    composite! { struct Straddle { Flag, Wide, Tail } }
    composite! { struct Mixed { P, Q, R, S, T, U } }

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
    fn storage_size_follows_the_total_width() {
        assert_eq!((Small::BITS, size_of::<Small>()), (16, 2));
        assert_eq!((Straddle::BITS, size_of::<Straddle>()), (68, 9));
        assert_eq!((Mixed::BITS, size_of::<Mixed>()), (43, 6));
    }

    #[test]
    fn a_full_width_field_straddling_nine_bytes_round_trips() {
        for seed in 0..1000 {
            let mut rng = Rng::new(seed);
            let (flag, wide, tail) =
                (u1::masked_new(rng.next_u64()), rng.next_u64(), i3::masked_new(rng.next_u64()));
            let mut straddle = Straddle::ZERO;
            straddle.set(Flag(flag));
            straddle.set(Wide(wide));
            straddle.set(Tail(tail));
            assert_eq!(straddle.get::<Flag>(), Flag(flag), "seed {seed}");
            assert_eq!(straddle.get::<Wide>(), Wide(wide), "seed {seed}");
            assert_eq!(straddle.get::<Tail>(), Tail(tail), "seed {seed}");
            assert_eq!(straddle.0[8] >> 4, 0, "seed {seed}");
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
            assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal, "seed {seed}");
        }
    }
}
