// thread = "worker" | "main":
//
// | Thread | Memory | Command |
// |-|-|-|
// | dedicated worker | WebAssembly.Memory(shared=true)  | `run_loop` |
// | main thread      | WebAssembly.Memory(shared=false) | `poll`     |

#![no_std]
#![feature(adt_const_params)]
#![feature(const_param_ty_trait)]
// `memory_atomic_wait32` / `memory_atomic_notify`
// worker (`-Ctarget-feature=+atomics`) `run_loop`
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

extern crate alloc;
extern crate core;
#[cfg(test)]
extern crate std;

pub mod app;
pub mod arena;
pub mod data_struct;
pub mod event;
pub mod file_store;
pub mod js_client;
pub mod list;
pub mod object;
pub mod roll;
pub mod timestamp;

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

    use crate::{arena::report_error, js_client::CommandError};

    let location = match info.location() {
        Some(location) => format!("{}:{}", location.file(), location.line()),
        None => alloc::string::String::from("unknown"),
    };

    report_error(CommandError::Panic { location, message: format!("{}", info.message()) });

    core::arch::wasm32::unreachable()
}

// === log ===

// macro_rules! debug_log {
//     ($($arg:tt)*) => {{
//         web_sys::console::error_1(
//             &wasm_bindgen::JsValue::from_str(&format!($($arg)*))
//         );
//     }};
// }
