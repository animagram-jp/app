use alloc::{string::String, vec::Vec};
use core::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

use crate::js_client::dom;

pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() >> 11) as usize % bound
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    pub fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    pub fn string(&mut self) -> String {
        let letters = ['a', 'Z', 'é', '日', '😀', ' ', '\0'];
        let length = self.below(10);
        (0..length).map(|_| letters[self.below(letters.len())]).collect()
    }

    pub fn id(&mut self) -> dom::Id {
        let depth = self.below(5);
        let segments: Vec<(dom::Tag, Option<u32>)> = (0..depth)
            .map(|_| {
                let tag = dom::Tag::decode_u8(self.below(24) as u8);
                let number =
                    self.chance(50).then(|| (self.next_u64() % u64::from(u32::MAX)) as u32);
                (tag, number)
            })
            .collect();
        dom::Id::new(&segments)
    }
}
