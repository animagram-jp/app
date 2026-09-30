use arbitrary_int::traits::Integer;

pub struct Field {
    pub position: u32,
    mask:         u64,
}

impl Field {
    pub const fn new(position: u32, width: u32) -> Self {
        assert!(width >= 1 && width < 64 && position + width <= 64);
        Self { position, mask: (1u64 << width) - 1 }
    }
    #[inline(always)]
    pub fn get<T: Integer>(&self, target: u64) -> T {
        ((target >> self.position) & self.mask).as_::<T>()
    }
    #[inline(always)]
    pub fn set<T: Integer>(&self, target: u64, value: T) -> u64 {
        (target & !(self.mask << self.position)) | ((value.as_u64() & self.mask) << self.position)
    }
}
