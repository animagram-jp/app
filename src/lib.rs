// thread = "worker" | "main":
//
// | Thread | Memory |
// |-|-|
// | dedicated worker | WebAssembly.Memory(shared=true)  |
// | main thread      | WebAssembly.Memory(shared=false) |

#![no_std]
#![feature(adt_const_params)]
#![feature(const_param_ty_trait)]
#![feature(variant_count)]
// `memory_atomic_wait32` / `memory_atomic_notify`
// worker (`-Ctarget-feature=+atomics`) `serve_event`
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

extern crate alloc;
extern crate core;
#[cfg(any(test, not(target_arch = "wasm32")))]
extern crate std;

use crate::{
    arena::{ArenaError, PanicError},
    event::EventError,
    file_store::FileStoreError,
    js_client::wire_error,
};

pub mod app;
pub mod arena;
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

// === Error ===

wire_error! {
    Error {
        Arena(ArenaError) = 1,
        Event(EventError) = 2,
        FileStore(FileStoreError) = 3,
        Panic(PanicError) = 4,
    }
}

// === Global allocator ===

#[cfg(target_arch = "wasm32")]
use talc::{sync::TalcLock, wasm::*};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: TalcLock<spinning_top::RawSpinlock, WasmGrowAndClaim, WasmBinning> =
    TalcLock::new(WasmGrowAndClaim);

// === Lang, En ===

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

    core::arch::wasm32::unreachable()
}

#[cfg(test)]
mod error_tests {
    use alloc::{format, string::String, vec::Vec};

    use super::*;
    use crate::js_client::WireError;

    fn identifiers(error: &Error) -> Vec<u16> {
        let mut path = Vec::new();
        error.identifiers(&mut path);
        path
    }

    #[test]
    fn identifiers_are_the_composed_variant_then_the_module_variant() {
        assert_eq!(identifiers(&Error::Arena(ArenaError::CommandOverflow)), [1, 1]);
        assert_eq!(identifiers(&Error::Event(EventError::Decode)), [2, 1]);
        assert_eq!(identifiers(&Error::FileStore(FileStoreError::Unknown(String::new()))), [3, 5]);
        let panic = PanicError { location: String::new(), message: String::new() };
        assert_eq!(identifiers(&Error::Panic(panic)), [4, 1]);
    }

    #[test]
    fn detail_and_seriousness_come_from_the_module() {
        let quota = Error::FileStore(FileStoreError::QuotaExceeded(String::from("full")));
        assert_eq!(quota.detail(), "full");
        assert!(quota.is_serious());
        assert!(!Error::Event(EventError::Decode).is_serious());
    }

    #[test]
    fn display_is_the_debug_representation() {
        let error = Error::Event(EventError::Decode);
        assert_eq!(format!("{error}"), format!("{error:?}"));
        assert_eq!(format!("{error}"), "Event(Decode)");
    }
}
