// thread = "worker" | "main":
//
// | Thread | Memory | Command |
// |-|-|-|
// | dedicated worker | WebAssembly.Memory(shared=true)  | `run_loop` |
// | main thread      | WebAssembly.Memory(shared=false) | `poll`     |
//
// worker は `atomics` target feature (共有メモリと `memory.atomic.wait32`) を
// 必要とする。main thread は同じアリーナ配置を非共有メモリ上で使い、
// main thread から `poll` を同期呼び出しする。
//
// `arena.rs` だけは app repository に対応先が無い。共有アリーナのレイアウト、
// リングバッファ、トリプルバッファ、`Encoder` / `Decoder`、および
// `arena_pointer` / `initialize` / `poll` / `run_loop` を持つ。
//
// 以下は `no_std` 化に伴い、上記に加えてそのまま持ち込むファイル。
// 内容は app repository から変更していない (alloc の prelude 補いのみ)。
//
// | file | app repository での対応 |
// |-|-|
// | `list.rs`        | `src/list.rs` |
// | `timestamp.rs`   | `src/timestamp.rs` |
// | `file_store.rs`  | `src/file_store.rs` |
// | `data_struct.rs` | `src/data_struct.rs` |
// | `object.rs`      | `src/object.rs` |
// | `roll.rs`        | `src/roll.rs` |
//
// ============================================================
// no_std (core + alloc)
// ============================================================
//
// wasm-bindgen 0.2.127 は `#![no_std]` が基底で、`std` は加算的な feature
// (`default = ["std"]`) である。js-sys / web-sys / wasm-bindgen-futures /
// getrandom も同様に `default-features = false` で `no_std` になる。
// したがって Cargo.toml 側で default feature を落とすだけで、依存側は
// `core` + `alloc` に収まる。
//
// 障壁は wasm-bindgen の外に 2 つだけあった。
//
// 1. `serde-wasm-bindgen`
//    `[features]` 節を持たず `default-features = false` で無効化できない。
//    `std::` を直接使い `HashMap` に依存するため、これ 1 つが `std` を
//    crate graph 全体へ伝播させていた。共有アリーナがこの依存自体を
//    置き換えるので、arena 移行と `no_std` 化は同時に達成される。
//
// 2. `rand::rng()`
//    `#[cfg(feature = "thread_rng")]` であり `thread_rng = ["std", ..]`。
//    `sys_rng` feature の `SysRng` で seed を取り `SmallRng` を回す形に
//    替える。`SysRng` 自身は fallible で `RngExt` が付かないため、
//    直接は使えない。詳しくは `object.rs` / `app_macros_lib.rs` を参照。
//
// このほか `f64::sqrt` / `abs` は core に無いため `libm::sqrt` / `libm::fabs`
// を使う (`js_client.rs` の `detect_gesture` に 3 箇所)。app repository が
// `fract` を使う `get_js_u32` / `get_js_i32` は byte protocol が置き換える。`String` / `Vec` / `format!` などは
// 各 module の冒頭で `alloc` から明示的に `use` する。

#![no_std]
#![feature(adt_const_params)]
#![feature(const_param_ty_trait)]
// `memory_atomic_wait32` / `memory_atomic_notify` は未安定 (rust#77839)。
// worker 構成 (`-Ctarget-feature=+atomics`) の `run_loop` が使う。
// main thread 構成では `arena.rs` 側が cfg で落とすため参照されない。
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

// ============================================================
// Global Allocator
// ============================================================

// dlmalloc は wasm32 の `acquire_global_lock` / `release_global_lock` が
// `assert!(!cfg!(target_feature = "atomics"))` を持つだけでロックを
// 実装しておらず、`worker` feature (atomics 有効) では確保のたびに
// panic する。panic ハンドラの `format!` がまた確保を要求するため、
// panic -> alloc -> panic の無限再帰 ("too much recursion") になる。
// talc はスレッド構成向けに spinlock ベースのロックを公式サポートする
// ため、こちらを使う。
#[cfg(target_arch = "wasm32")]
use talc::{sync::TalcLock, wasm::*};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: TalcLock<spinning_top::RawSpinlock, WasmGrowAndClaim, WasmBinning> =
    TalcLock::new(WasmGrowAndClaim);

// ============================================================
// Lang, En
// ============================================================

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

// ============================================================
// panic handler
// ============================================================

// `std::panic::set_hook` は `std` 専用であり `no_std` では使えない。
// hook を実行時に保持する仕組みと unwind ランタイムが `std` 側にあるため。
// 代わりに `#[panic_handler]` を書く。arena.rs の `install_panic_hook`
// はこれに置き換わり、`initialize` からの呼び出しも不要になる。
//
// set_hook との差は 3 点。
//
// 1. 登録が compile 時になり、crate graph 全体で 1 つだけになる。
//    二重登録の考慮が要らなくなる。
// 2. この関数が終端であり `!` を返す。既定の panic 処理は続かない。
// 3. `info.payload()` の downcast が無い。`info.message()` を使う。
//    arena.rs は `&str` / `String` しか見ていないので等価である。
//
// `info.location()` は `no_std` でも取れるため、発生位置は失われない。
//
// `cargo test` はホスト側で `std` をリンクし `#[panic_handler]` が
// 衝突するので、`not(test)` で外す。
//
// wasm32 で `wasm-bindgen-test` を走らせる場合も同じ衝突が起こりうる。
// `--lib --tests` で対象を絞れば `cfg(test)` が立つので問題ないが、
// doctest の target は lib を `cfg(test)` 無しでコンパイルしたうえで
// `std` とリンクするため E0152 になる。doctest は wasm32 では実行
// できないので、ホスト側で `cargo test --doc` として走らせる。
// 詳細は `Cargo.toml` の該当箇所を参照。

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use alloc::format;

    use crate::{arena::report_error, js_client::ERROR_PANIC};

    let location = match info.location() {
        Some(location) => format!("{}:{}", location.file(), location.line()),
        None => alloc::string::String::from("unknown"),
    };

    report_error(ERROR_PANIC, &format!("panic at {location}: {}", info.message()));

    core::arch::wasm32::unreachable()
}

// ============================================================
// log
// ============================================================

// macro_rules! debug_log {
//     ($($arg:tt)*) => {{
//         web_sys::console::error_1(
//             &wasm_bindgen::JsValue::from_str(&format!($($arg)*))
//         );
//     }};
// }
