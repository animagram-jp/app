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

    #[test]
    fn get_set_round_trip_unsigned() {
        let f = Field::new(5, 9);
        let raw = f.set(0u64, 400u64);
        assert_eq!(f.get::<u64>(raw), 400);
        assert_eq!(f.get::<u16>(raw), 400);
        assert_eq!(raw, 400 << 5);
    }

    #[test]
    fn set_masks_value_and_preserves_neighbours() {
        let f = Field::new(4, 4);
        let raw = f.set(!0u64, 0u64);
        assert_eq!(raw, !(0xf << 4));
        let raw = f.set(0u64, 0x1ffu64);
        assert_eq!(raw, 0xf0);
    }

    #[test]
    fn signed_field_sign_extends() {
        let f = Field::new_signed(13, 10);
        for v in [-512i64, -400, -1, 0, 1, 400, 511] {
            let raw = f.set(0u64, v as u64);
            assert_eq!(f.get::<i16>(raw), v as i16, "v={v}");
            assert_eq!(f.get::<i64>(raw), v, "v={v}");
        }
    }

    #[test]
    fn unsigned_field_never_sign_extends() {
        let f = Field::new(13, 10);
        let raw = f.set(0u64, -1i64 as u64);
        assert_eq!(f.get::<u16>(raw), 1023);
        assert_eq!(f.get::<i16>(raw), 1023);
        let g = Field::new(32, 9);
        let raw = g.set(0u64, 511u64);
        assert_eq!(g.get::<i16>(raw), 511);
        assert_eq!(g.get::<i64>(raw), 511);
    }

    #[test]
    fn signed_field_does_not_touch_neighbours() {
        let f = Field::new_signed(4, 4);
        let raw = f.set(!0u64, 0u64);
        assert_eq!(raw, !(0xf << 4));
        let raw = f.set(0u64, -1i64 as u64);
        assert_eq!(raw, 0xf0);
    }

    #[test]
    fn widest_and_topmost_fields() {
        let f = Field::new(63, 1);
        assert_eq!(f.set(0u64, 1u64), 1 << 63);
        let g = Field::new(0, 63);
        assert_eq!(g.get::<u64>(!0u64), (1 << 63) - 1);
    }

    use super::Spec::{Signed, Unsigned};

    #[test]
    fn layout_assigns_positions_from_the_top() {
        const L: Layout<3> = Layout::new([Unsigned(13), Signed(10), Unsigned(1)]);
        assert_eq!(L.bits(), 24);
        assert_eq!(L.bytes(), 3);
        assert_eq!(L.field(0).position, 11);
        assert_eq!(L.field(1).position, 1);
        assert_eq!(L.field(2).position, 0);
    }

    #[test]
    fn layout_padding_sits_at_the_bottom() {
        const L: Layout<2> = Layout::with_padding([Unsigned(57), Unsigned(1)], 6);
        assert_eq!(L.bits(), 64);
        assert_eq!(L.field(0).position, 7);
        assert_eq!(L.field(1).position, 6);
        assert_eq!(L.bytes(), 8);
    }

    #[test]
    fn layout_bytes_round_up() {
        assert_eq!(Layout::new([Unsigned(8)]).bytes(), 1);
        assert_eq!(Layout::new([Unsigned(9)]).bytes(), 2);
        assert_eq!(Layout::new([Unsigned(9), Unsigned(9), Unsigned(10), Unsigned(10)]).bytes(), 5);
        assert_eq!(Layout::new([Unsigned(45)]).bytes(), 6);
        assert_eq!(Layout::new([Unsigned(63), Unsigned(1)]).bytes(), 8);
    }

    #[test]
    fn layout_pack_unpack_round_trip() {
        let l = Layout::new([Unsigned(9), Unsigned(9), Signed(10), Signed(10)]);
        let raw = l.pack([400, 511, -400i64 as u64, 511]);
        assert_eq!(l.unpack::<i64>(raw), [400, 511, -400, 511]);
        assert_eq!(l.get::<i16>(raw, 2), -400);
        assert_eq!(raw >> l.bits(), 0);
    }

    #[test]
    fn layout_pack_masks_overflow_without_touching_neighbours() {
        let l = Layout::new([Unsigned(4), Unsigned(4)]);
        assert_eq!(l.pack([0x1f, 0]), 0xf0);
        assert_eq!(l.pack([0, 0x1f]), 0x0f);
    }

    #[test]
    fn layout_set_keeps_other_fields() {
        let l = Layout::new([Unsigned(4), Signed(4), Unsigned(8)]);
        let raw = l.pack([3, -2i64 as u64, 200]);
        let raw = l.set(raw, 1, 5i64);
        assert_eq!(l.unpack::<i64>(raw), [3, 5, 200]);
    }

    #[test]
    fn layout_decode_encode_use_only_needed_bytes() {
        let l = Layout::new([Unsigned(9), Unsigned(9), Signed(10), Signed(10)]);
        let raw = l.pack([400, 300, -400i64 as u64, 400]);
        let (word, len) = l.encode(raw);
        assert_eq!(len, 5);
        assert_eq!(&word[5..], &[0, 0, 0]);
        assert_eq!(l.decode(&word[..len]), Some(raw));
        assert_eq!(l.decode(&word[..len - 1]), None);
        assert_eq!(l.decode(&[]), None);
    }

    #[test]
    fn layout_full_width_has_no_spare_bits() {
        let l = Layout::new([Unsigned(32), Unsigned(32)]);
        assert_eq!(l.pack([u32::MAX as u64, u32::MAX as u64]), !0u64);
    }
}
