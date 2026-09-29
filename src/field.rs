use arbitrary_int::traits::Integer;

pub struct Field {
    pub position: u32,
    pub mask:     u64,
}

impl Field {
    #[inline(always)]
    pub fn get<T: Integer>(&self, target: u64) -> T {
        u64::masked_new((target >> self.position) & self.mask).as_::<T>()
    }
    #[inline(always)]
    pub fn set<T: Integer>(&self, target: u64, value: T) -> u64 {
        (target & !(self.mask << self.position)) | ((value.as_u64() & self.mask) << self.position)
    }
}
