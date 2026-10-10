#![no_std]
#![feature(adt_const_params)]
#![feature(const_param_ty_trait)]
#![feature(variant_count)]
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

extern crate alloc;
extern crate core;
#[cfg(any(test, not(target_arch = "wasm32")))]
extern crate std;

#[cfg(feature = "calendar")]
use crate::calendar::data::DataError;
use crate::{
    arena::{ArenaError, PanicError},
    event::EventError,
    file_store::FileStoreError,
    js_client::wire_error,
};

pub mod app;
pub mod arena;
#[cfg(feature = "calendar")]
pub mod calendar;
pub mod data_struct;
pub mod event;
pub mod field;
pub mod file_store;
pub mod handler;
pub mod js_client;
pub mod list;
pub mod object;
pub mod roll;
pub mod timestamp;

#[cfg(test)]
fn block_on<F: core::future::Future>(future: F) -> F::Output {
    use core::{
        pin::pin,
        task::{Context, Poll, Waker},
    };

    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

#[cfg(test)]
struct Rng(u64);

#[cfg(test)]
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() >> 11) as usize % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    fn bytes(&mut self, length: usize) -> alloc::vec::Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    fn string(&mut self) -> alloc::string::String {
        let letters = ['a', 'Z', 'é', '日', '😀', ' ', '\0'];
        let length = self.below(10);
        (0..length).map(|_| letters[self.below(letters.len())]).collect()
    }

    fn id(&mut self) -> js_client::dom::Id {
        use js_client::dom;

        let depth = self.below(5);
        let segments: alloc::vec::Vec<(dom::Tag, Option<u32>)> = (0..depth)
            .map(|_| {
                let tag = dom::Tag::from_u8(self.below(24) as u8);
                let number =
                    self.chance(50).then(|| (self.next_u64() % u64::from(u32::MAX)) as u32);
                (tag, number)
            })
            .collect();
        dom::Id::new(&segments)
    }
}

// === Error ===

wire_error! {
    Error {
        Arena(ArenaError) = 1,
        Event(EventError) = 2,
        Panic(PanicError) = 3,
        FileStore(FileStoreError) = 4,
        #[cfg(feature = "calendar")]
        Data(DataError) = 5,
    }
}

// === Global allocator ===

#[cfg(target_arch = "wasm32")]
use talc::{sync::TalcLock, wasm::*};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: TalcLock<spinning_top::RawSpinlock, WasmGrowAndClaim, WasmBinning> =
    TalcLock::new(WasmGrowAndClaim);

// === Lang ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En(En),
    Ja,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum En {
    Us,
}

impl Lang {
    fn display(self) -> &'static str {
        match self {
            Self::En(En::Us) => "en-US",
            Self::En(_) => "En",
            Self::Ja => "ja",
        }
    }
}

// === Panic handler ===

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use alloc::format;

    use crate::arena::{PanicError, report_error};

    let location = match info.location() {
        Some(location) => format!("{}:{}", location.file(), location.line()),
        None => alloc::string::String::from("unknown"),
    };

    report_error(Error::Panic(PanicError { location, message: format!("{}", info.message()) }));
    let _ = crate::arena::emit(&[crate::js_client::OPERATION_RELOAD]);

    core::arch::wasm32::unreachable()
}

#[cfg(test)]
mod error_tests {
    use alloc::{string::String, vec, vec::Vec};

    use super::*;
    use crate::js_client::WireError;

    #[test]
    fn every_error_has_a_stable_wire_path_and_detail() {
        let message = || String::from("m");
        #[cfg_attr(not(feature = "calendar"), allow(unused_mut))]
        let mut cases: Vec<(Error, Vec<u16>, &str)> = vec![
            (Error::Arena(ArenaError::CommandOverflow), vec![1, 1], ""),
            (Error::Event(EventError::Decode), vec![2, 1], ""),
            (
                Error::Panic(PanicError { location: String::from("a.rs:1"), message: message() }),
                vec![3, 1],
                "a.rs:1: m",
            ),
            (Error::FileStore(FileStoreError::InvalidState(message())), vec![4, 1], "m"),
            (Error::FileStore(FileStoreError::QuotaExceeded(message())), vec![4, 2], "m"),
            (Error::FileStore(FileStoreError::UnsupportedOp(message())), vec![4, 3], "m"),
            (Error::FileStore(FileStoreError::InvalidName(message())), vec![4, 4], "m"),
            (Error::FileStore(FileStoreError::Unknown(message())), vec![4, 5], "m"),
            (Error::FileStore(FileStoreError::NotFound(message())), vec![4, 6], "m"),
        ];
        #[cfg(feature = "calendar")]
        cases.extend([
            (Error::Data(DataError::Status(404)), vec![5, 1], "404"),
            (Error::Data(DataError::Parse(message())), vec![5, 2], "m"),
            (Error::Data(DataError::Format(message())), vec![5, 3], "m"),
        ]);
        for (error, path, detail) in cases {
            let mut actual = Vec::new();
            error.identifiers(&mut actual);
            assert_eq!(actual, path, "{error}");
            assert_eq!(error.detail(), detail, "{error}");
        }
    }
}
