extern crate core;
extern crate alloc;
extern crate std;

pub mod list;
pub mod timestamp;
pub mod js_client;
pub mod store;
pub mod data_struct;
pub mod app;
pub mod character;
pub mod roll;
pub mod event;

// ============================================================
// Global Allocator
// ============================================================

#[cfg(target_arch = "wasm32")]
use dlmalloc::GlobalDlmalloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: GlobalDlmalloc = GlobalDlmalloc;

// ============================================================
// Lang
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ja }

impl Lang {
    fn label(self) -> &'static str {
        match self {
            Self::En => "en-US",
            Self::Ja => "ja",
        }
    }
}

// ============================================================
// log
// ============================================================

macro_rules! debug_log {
    ($($arg:tt)*) => {{
        web_sys::console::error_1(
            &wasm_bindgen::JsValue::from_str(&format!($($arg)*))
        );
    }};
}

// ============================================================
// no_std (note)
// ============================================================

// #![no_std]
// use core::{
//     panic::Panicinfo,
//     arch::wasm32::unreachable
// };
//
// #[panic_handler]
// fn panic(info: &PanicInfo) -> ! {
//     debug_log!("panic: {}", info);
//     unreachable()
// }