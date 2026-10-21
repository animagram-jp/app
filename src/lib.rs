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
#[cfg(all(
    target_arch = "wasm32",
    feature = "file_store",
    not(target_feature = "atomics"),
    not(test)
))]
compile_error!("feature `file_store` on wasm32 requires -Ctarget-feature=+atomics");
#[cfg(any(test, not(target_arch = "wasm32")))]
extern crate std;

use alloc::{string::String, vec::Vec};
#[cfg(test)]
use core::future::Future;
#[cfg(all(target_arch = "wasm32", not(test)))]
use core::panic::PanicInfo;

#[cfg(feature = "calendar")]
use crate::calendar::data::DataError;
#[cfg(feature = "file_store")]
use crate::file_store::FileStoreError;
use crate::{
    arena::{ArenaError, PanicError},
    event::EventError,
};

pub mod app;
pub mod arena;
#[cfg(feature = "calendar")]
pub mod calendar;
pub mod coc7th;
pub mod data_struct;
pub mod event;
pub mod field;
#[cfg(feature = "file_store")]
pub mod file_store;
pub mod host;
pub mod list;
pub mod object;
pub mod timestamp;

// === Error ===

pub trait WireError {
    fn identifiers(&self, path: &mut Vec<u16>);
    fn detail(&self) -> String;
}

macro_rules! wire_error {
    ($name:ident { $($(#[$meta:meta])* $variant:ident($inner:ty) = $identifier:expr),+ $(,)? }) => {
        #[derive(Debug)]
        pub enum $name {
            $($(#[$meta])* $variant($inner)),+
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::write!(f, "{:?}", self)
            }
        }

        impl $crate::WireError for $name {
            fn identifiers(&self, path: &mut ::alloc::vec::Vec<u16>) {
                match self {
                    $($(#[$meta])* $name::$variant(error) => {
                        path.push($identifier);
                        $crate::WireError::identifiers(error, path);
                    })+
                }
            }

            fn detail(&self) -> ::alloc::string::String {
                match self {
                    $($(#[$meta])* $name::$variant(error) => $crate::WireError::detail(error)),+
                }
            }
        }
    };
}

wire_error! {
    Error {
        Arena(ArenaError) = 1,
        Event(EventError) = 2,
        Panic(PanicError) = 3,
        #[cfg(feature = "file_store")]
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

// === Language ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En(En),
    Ja,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum En {
    Gb,
    Us,
}

impl Language {
    pub fn list() -> &'static [Self] {
        &[Self::En(En::Gb), Self::Ja]
    }

    pub fn number(self) -> u32 {
        match self {
            Self::En(_) => 1,
            Self::Ja => 2,
        }
    }

    pub fn display(self) -> (&'static str, &'static str) {
        match self {
            Self::En(En::Gb) => ("en-GB", "English"),
            Self::En(En::Us) => ("en-US", "English"),
            Self::Ja => ("ja", "日本語"),
        }
    }
}

// === Panic handler ===

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use alloc::format;
    use core::arch::wasm32::unreachable;

    use crate::{
        arena::{PanicError, emit, report_error},
        host::OPERATION_RELOAD,
    };

    let location = match info.location() {
        Some(location) => format!("{}:{}", location.file(), location.line()),
        None => String::from("unknown"),
    };

    report_error(Error::Panic(PanicError { location, message: format!("{}", info.message()) }));
    let _ = emit(&[OPERATION_RELOAD]);

    unreachable()
}

#[cfg(test)]
mod error_tests {
    use alloc::{string::String, vec, vec::Vec};

    use super::*;

    #[test]
    fn every_error_has_a_stable_wire_path_and_detail() {
        let message = || String::from("m");
        #[cfg_attr(not(feature = "file_store"), allow(unused_mut))]
        let mut cases: Vec<(Error, Vec<u16>, &str)> = vec![
            (Error::Arena(ArenaError::CommandOverflow), vec![1, 1], ""),
            (Error::Event(EventError::Decode), vec![2, 1], ""),
            (
                Error::Panic(PanicError { location: String::from("a.rs:1"), message: message() }),
                vec![3, 1],
                "a.rs:1: m",
            ),
        ];
        #[cfg(feature = "file_store")]
        cases.extend([
            (Error::FileStore(FileStoreError::InvalidState(message())), vec![4, 1], "m"),
            (Error::FileStore(FileStoreError::QuotaExceeded(message())), vec![4, 2], "m"),
            (Error::FileStore(FileStoreError::UnsupportedOp(message())), vec![4, 3], "m"),
            (Error::FileStore(FileStoreError::InvalidName(message())), vec![4, 4], "m"),
            (Error::FileStore(FileStoreError::Unknown(message())), vec![4, 5], "m"),
            (Error::FileStore(FileStoreError::NotFound(message())), vec![4, 6], "m"),
            (Error::FileStore(FileStoreError::Locked(message())), vec![4, 7], "m"),
        ]);
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

// === For test ===

#[cfg(test)]
fn block_on<F: Future>(future: F) -> F::Output {
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

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    fn string(&mut self) -> String {
        let letters = ['a', 'Z', 'é', '日', '😀', ' ', '\0'];
        let length = self.below(10);
        (0..length).map(|_| letters[self.below(letters.len())]).collect()
    }

    fn id(&mut self) -> host::dom::Id {
        use host::dom;

        let depth = self.below(5);
        let segments: Vec<(dom::Tag, Option<u32>)> = (0..depth)
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
