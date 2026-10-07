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
mod testing;

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
