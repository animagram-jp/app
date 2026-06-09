// #![no_std]
extern crate core;
extern crate alloc;

pub mod list;
pub mod timestamp;
pub mod temporal;
pub mod js_client;
pub mod store;
pub mod data_struct;
// pub mod app;
pub mod character;
pub mod roll;
pub mod event;

macro_rules! debug_log {
    ($($arg:tt)*) => {{
        web_sys::console::error_1(
            &wasm_bindgen::JsValue::from_str(&format!($($arg)*))
        );
    }};
}

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
