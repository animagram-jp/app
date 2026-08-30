// 共有アリーナ。app repository に対応するファイルは無く、新規である。
//
// 持つのは 4 つである。
//
// 1. アリーナのレイアウト定数。`./init.js` と一対一で対応する。
// 2. `Arena` 本体。イベントリング、コマンドリング、トリプルバッファ。
// 3. entry point。`arena_pointer` / `initialize` / `poll` / `run_loop`。
// 4. `Encoder` / `Decoder`。バイト列の読み書き。
// 5. 異常報告。`report_error` と panic hook。
//
// レイアウトと同期はすべて `impl Arena` に閉じており、`crate::app` などの
// 利用者は添字やオフセットを知らない。

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    cell::{Cell, UnsafeCell},
    convert::TryInto,
    debug_assert,
    marker::Sync,
    option::Option::{self, None, Some},
    primitive::{bool, f32, f64, i32, str, u8, u16, u32, usize},
    ptr, slice,
    sync::atomic::{AtomicU32, Ordering},
};

// atomics を持つ構成 (worker) でのみ使う。main thread 向けの
// 非共有メモリ構成では待機も通知も行わない。
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::app::App;
use crate::js_client::{Command, ERROR_COMMAND_OVERFLOW, ErrorCode, dom, encode_command};

// ============================================================
// arena layout
// ============================================================
//
// 全て 64 byte 境界に整列させる。値は `./init.js` と一対一で対応する。
// 一方だけを変更してはならない。

/// イベントリング (JavaScript -> Wasm) の制御ブロック位置。
pub const EVENT_CONTROL: usize = 0;
/// イベントリングのスロット領域の開始位置。
pub const EVENT_PAYLOAD: usize = 128;
/// イベントリングの 1 スロットの byte 数。
pub const EVENT_SLOT: usize = 4096;
/// イベントリングのスロット数。2 の冪であること。
pub const EVENT_SLOT_COUNT: u32 = 64;

/// コマンドリング (Wasm -> JavaScript) の制御ブロック位置。
pub const COMMAND_CONTROL: usize = 262_272;
/// コマンドリングのスロット領域の開始位置。
pub const COMMAND_PAYLOAD: usize = 262_400;
/// コマンドリングの 1 スロットの byte 数。
pub const COMMAND_SLOT: usize = 4096;
/// コマンドリングのスロット数。2 の冪であること。
pub const COMMAND_SLOT_COUNT: u32 = 64;

/// トリプルバッファの共有状態語の位置。
pub const CELL_STATE: usize = 524_544;
/// トリプルバッファの本体領域の開始位置。
pub const CELL_PAYLOAD: usize = 524_608;
/// 1 バッファの byte 数。320 * 240 * 4 (RGBA)。
pub const CELL_SIZE: usize = 307_200;
/// バッファ数。書き手 1 枚、読み手 1 枚、受け渡し用 1 枚。
///
/// `CELL_INDEX_MASK` が下位 2 bit で添字を持つため、4 枚が上限である。
pub const CELL_COUNT: u32 = 3;

/// 共有アリーナ全体の byte 数。
pub const ARENA_SIZE: usize = 1_446_208;

/// 書き込みシーケンスの制御ブロック内オフセット。
pub const CONTROL_WRITE_OFFSET: usize = 0;
/// 読み出しシーケンスの制御ブロック内オフセット。
///
/// 書き込み側と 64 byte 離すことで、両者が別の cache line に載り
/// false sharing を避ける。
pub const CONTROL_READ_OFFSET: usize = 64;
/// スロット先頭に置く長さ前置語の byte 数。
pub const LENGTH_PREFIX: usize = 4;

/// 受け渡し中のバッファ添字を取り出すマスク。
pub const CELL_INDEX_MASK: u32 = 0b11;
/// 未読の新しいフレームが存在することを示すビット。
pub const CELL_DIRTY: u32 = 0b100;

/// イベントキューの初期容量。
pub const EVENT_CAPACITY: usize = 64;
/// コマンド出力バッファの初期容量。
pub const COMMAND_CAPACITY: usize = 16 * 1024;

// ============================================================
// arena state
// ============================================================

/// 共有アリーナ本体。64 byte 境界に整列させる。
///
/// 固定長のキューを 3 つ載せる。イベントリング、コマンドリング、
/// そしてトリプルバッファである。レイアウトと同期はすべて
/// `impl Arena` に閉じており、利用者は添字やオフセットを知らない。
#[repr(C, align(64))]
pub struct Arena {
    bytes: UnsafeCell<[u8; ARENA_SIZE]>,
    /// トリプルバッファのうち、書き手が専有しているバッファの添字。
    ///
    /// 共有する必要がないため、アリーナ本体ではなくこちらに持つ。
    cell_back: Cell<u32>,
}

// 単一の書き手と単一の読み手を前提とし、同期はアリーナ内の AtomicU32 が担う。
unsafe impl Sync for Arena {}

pub static ARENA: Arena = Arena {
    bytes: UnsafeCell::new([0; ARENA_SIZE]),
    cell_back: Cell::new(0),
};

/// `poll` / `run_loop` が駆動する App。
///
/// `App::init` は `wasm_bindgen` 経由でも呼べるが、worker では
/// `run_loop` が自走するため、そこから届く場所に置く必要がある。
pub static mut APP: Option<App> = None;

/// `run_loop` の継続条件。
pub static mut RUNNING: bool = true;

// ============================================================
// arena function
// ============================================================

impl Arena {
    /// 共有アリーナ先頭への生ポインタ。
    #[inline]
    pub fn base(&self) -> *mut u8 {
        self.bytes.get() as *mut u8
    }

    /// リング制御ブロック内の指定オフセットを `AtomicU32` として参照する。
    #[inline]
    fn control_at(&self, control: usize, offset: usize) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr((self.base() as usize + control + offset) as *mut u32) }
    }

    /// トリプルバッファの共有状態語。
    #[inline]
    fn cell_state(&self) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr((self.base() as usize + CELL_STATE) as *mut u32) }
    }

    /// 全リングの制御ブロックとトリプルバッファの状態語を初期化する。
    pub fn initialize(&self) {
        self.control_at(EVENT_CONTROL, CONTROL_WRITE_OFFSET)
            .store(0, Ordering::Relaxed);
        self.control_at(EVENT_CONTROL, CONTROL_READ_OFFSET)
            .store(0, Ordering::Relaxed);
        self.control_at(COMMAND_CONTROL, CONTROL_WRITE_OFFSET)
            .store(0, Ordering::Relaxed);
        self.control_at(COMMAND_CONTROL, CONTROL_READ_OFFSET)
            .store(0, Ordering::Relaxed);
        // 書き手が back=0 を専有し、読み手が front=1、共有枠が 2、dirty は未設定。
        self.cell_state().store(2, Ordering::Release);
    }

    /// 単一書き手・単一読み手のリングへ 1 フレーム追加する。満杯なら false。
    ///
    /// payload の書き込みは非アトミックで良い。書き込みシーケンスの
    /// Release store が、それ以前の書き込みの可視性を読み手に対して保証する。
    fn ring_push(
        &self,
        control: usize,
        payload: usize,
        slot: usize,
        slot_count: u32,
        source: &[u8],
    ) -> bool {
        debug_assert!(source.len() + LENGTH_PREFIX <= slot);
        // debug_assert! は release では消える。溢れたまま書くとスロットを
        // 越えて隣を壊すため、ここで弾く。呼び出し側は false を受ける。
        if source.len() + LENGTH_PREFIX > slot {
            return false;
        }
        let write_atomic = self.control_at(control, CONTROL_WRITE_OFFSET);
        let read_atomic = self.control_at(control, CONTROL_READ_OFFSET);

        let write = write_atomic.load(Ordering::Relaxed);
        let read = read_atomic.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= slot_count {
            return false;
        }

        let offset = self.base() as usize + payload + (write & (slot_count - 1)) as usize * slot;
        unsafe {
            (offset as *mut u32).write_unaligned(source.len() as u32);
            ptr::copy_nonoverlapping(
                source.as_ptr(),
                (offset + LENGTH_PREFIX) as *mut u8,
                source.len(),
            );
        }

        // コミット。ここで初めて読み手にスロットが見える。
        write_atomic.store(write.wrapping_add(1), Ordering::Release);
        true
    }

    /// リング先頭のフレームを参照する。空なら None。
    ///
    /// 返るスライスは `ring_commit_pop` を呼ぶまでのみ有効である。
    fn ring_peek(
        &self,
        control: usize,
        payload: usize,
        slot: usize,
        slot_count: u32,
    ) -> Option<&[u8]> {
        let write_atomic = self.control_at(control, CONTROL_WRITE_OFFSET);
        let read_atomic = self.control_at(control, CONTROL_READ_OFFSET);

        let read = read_atomic.load(Ordering::Relaxed);
        let write = write_atomic.load(Ordering::Acquire);
        if read == write {
            return None;
        }

        let offset = self.base() as usize + payload + (read & (slot_count - 1)) as usize * slot;
        let length = unsafe { (offset as *const u32).read_unaligned() } as usize;
        // 長さ前置語が壊れていてもスロット外へは出ない。
        let length = length.min(slot - LENGTH_PREFIX);
        Some(unsafe { slice::from_raw_parts((offset + LENGTH_PREFIX) as *const u8, length) })
    }

    /// `ring_peek` で参照したスロットを解放する。
    fn ring_commit_pop(&self, control: usize) {
        let read_atomic = self.control_at(control, CONTROL_READ_OFFSET);
        let read = read_atomic.load(Ordering::Relaxed);
        read_atomic.store(read.wrapping_add(1), Ordering::Release);
    }

    /// イベントリングの先頭フレームを参照する。空なら None。
    pub fn event_peek(&self) -> Option<&[u8]> {
        self.ring_peek(EVENT_CONTROL, EVENT_PAYLOAD, EVENT_SLOT, EVENT_SLOT_COUNT)
    }

    /// `event_peek` で参照したスロットを解放する。
    pub fn event_commit_pop(&self) {
        self.ring_commit_pop(EVENT_CONTROL);
    }

    /// イベントリングの書き込みシーケンス。`run_loop` の待機条件に用いる。
    pub fn event_write_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_WRITE_OFFSET)
            .load(Ordering::Acquire)
    }

    /// イベントリングの読み出しシーケンス。
    pub fn event_read_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_READ_OFFSET)
            .load(Ordering::Relaxed)
    }

    /// コマンドリングへ 1 フレーム追加する。満杯なら false。
    pub fn command_push(&self, frame: &[u8]) -> bool {
        self.ring_push(
            COMMAND_CONTROL,
            COMMAND_PAYLOAD,
            COMMAND_SLOT,
            COMMAND_SLOT_COUNT,
            frame,
        )
    }

    /// 書き手が専有しているバッファを可変スライスとして取り出す。
    ///
    /// 呼び出し側は書き終えたら `frame_commit` を呼ぶ。
    ///
    /// # Safety
    ///
    /// `ARENA` は `static` であり `&self` しか取れないため、`&mut` を
    /// 返せるのはトリプルバッファの規約に拠る。書き手が専有している
    /// バッファ (`cell_back`) は、`frame_commit` で公開するまで他の
    /// thread から触れられない。呼び出し側は次の 2 つを守ること。
    ///
    /// - 返るスライスは `frame_commit` を呼ぶまでのみ有効である。
    /// - 同時に 2 つ以上を生存させない。
    // 上記の規約により `&self` から `&mut` を返す。clippy::mut_from_ref は
    // その規約を追えないため、ここでのみ抑止する。
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn frame_back_mut(&self) -> &mut [u8] {
        let index = self.cell_back.get();
        let pointer = (self.base() as usize + CELL_PAYLOAD + index as usize * CELL_SIZE) as *mut u8;
        unsafe { slice::from_raw_parts_mut(pointer, CELL_SIZE) }
    }

    /// 書き終えたバッファを公開し、次に書くバッファを専有する。
    ///
    /// `swap` 1 回のみでリトライを持たない。読み手が受け取らないまま
    /// 次の公開が来た場合、未読のフレームは破棄される。
    pub fn frame_commit(&self) {
        let back = self.cell_back.get();
        let previous = self.cell_state().swap(back | CELL_DIRTY, Ordering::AcqRel);
        self.cell_back.set(previous & CELL_INDEX_MASK);
    }

    /// 公開されたバッファを受け取り、読み手の添字を進める。
    ///
    /// Wasm 側が読み手になる場合に用いる。`init.js` の `cellAcquire` と
    /// 対称である。未読フレームが無ければ現在の添字を維持する。
    #[allow(dead_code)]
    pub fn frame_acquire(&self, front: u32) -> u32 {
        if self.cell_state().load(Ordering::Acquire) & CELL_DIRTY == 0 {
            return front;
        }
        self.cell_state().swap(front, Ordering::AcqRel) & CELL_INDEX_MASK
    }
}

// ============================================================
// arena entry point
// ============================================================
//
// worker / main thread 共通。JavaScript 側は `arena_pointer` の直後に
// `initialize` を呼ぶ。

/// 共有アリーナ先頭の線形メモリ内オフセットを返す。
///
/// JavaScript 側は各種 typed array view の基点としてこれを用いる。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn arena_pointer() -> u32 {
    ARENA.base() as u32
}

/// 共有アリーナを初期化する。JavaScript 側は最初にこれを呼ぶ。
///
/// worker を作り直した際も再度呼ぶ。リングのシーケンスと
/// トリプルバッファの状態語が初期値に戻り、JavaScript 側が持つ
/// `cellFront` と揃う。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn initialize() {
    ARENA.initialize();
    unsafe { RUNNING = true };
}

/// main thread 用。同期呼び出しし、溜まっているイベントだけ処理して返る。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn poll() {
    // `&raw mut` を挟むのは `static mut` への参照を作らないためである。
    // clippy::deref_addrof は `APP.as_mut()` を勧めるが、それでは
    // `static mut` への参照が生じるため従わない。
    #[allow(clippy::deref_addrof)]
    let Some(app) = (unsafe { (*(&raw mut APP)).as_mut() }) else {
        return;
    };
    while let Some(frame) = ARENA.event_peek() {
        app.clear();
        app.process(frame);
        // 処理の前にスロットを返却する。emit による再入を避ける。
        ARENA.event_commit_pop();
        let commands = app.commands();
        if !commands.is_empty() && !emit(commands) {
            // リングが満杯でコマンドを落とした。画面が実際の状態から
            // ずれるため、黙って続けず JavaScript へ報告する。
            report_error(ERROR_COMMAND_OVERFLOW, "command ring is full");
        }
    }
}

/// worker 用。worker に常駐し、リングが空なら atomic wait でブロックする。
///
/// この関数は `memory_atomic_wait32` で無期限にブロックし、
/// `Event::Shutdown` を受けるまで戻らない。呼ぶのは dedicated worker 上に
/// 限る。main thread から呼ぶと thread ごと停止する。
///
/// 引数と戻り値を持たないため、`wasm_bindgen` が生成するグルーは
/// `wasm.run_loop()` を呼ぶだけの関数 1 枚であり、wasm 側の export は
/// `no_mangle` の場合と同一である。worker.js は他の export と同じく
/// 名前で import する。
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
#[wasm_bindgen]
pub fn run_loop() {
    while unsafe { RUNNING } {
        poll();
        let write = ARENA.event_write_seq();
        if ARENA.event_read_seq() == write {
            unsafe {
                let pointer = (ARENA.base() as usize + EVENT_CONTROL) as *mut i32;
                memory_atomic_wait32(pointer, write as i32, -1);
            }
        }
    }
}

/// コマンドリングへ 1 フレーム送る。満杯なら false。
///
/// 満杯の場合は破棄する。通常のコマンドは最新のものが優先であり、
/// 取りこぼしても後続のフレームが状態を上書きするためである。
/// 破棄が起きたことは戻り値で呼び出し側へ伝える。
pub fn emit(frame: &[u8]) -> bool {
    let pushed = ARENA.command_push(frame);

    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    unsafe {
        let pointer = (ARENA.base() as usize + COMMAND_CONTROL) as *mut i32;
        memory_atomic_notify(pointer, 1);
    }

    pushed
}

// ============================================================
// error report
// ============================================================

/// 異常をコマンドリング経由で JavaScript へ報告する。
///
/// `Handler` を経由せず直接リングへ積む。panic hook や `run_loop` からは
/// コマンド列を返す相手が居ないためである。
///
/// リングが満杯の場合は何もしない。ここで再帰的に報告しても同じ理由で
/// 失敗するだけである。JavaScript 側は worker の `error` イベントでも
/// 再起動できるため、報告の取りこぼしは復旧を妨げない。
pub fn report_error(code: ErrorCode, message: &str) {
    // 1 スロットに収める。panic のメッセージは長さに上限が無く、
    // 溢れると `ring_push` に弾かれて報告そのものが消える。
    // `[operation:u8][code:u8][length:u32]` と長さ前置語の分を引く。
    const OVERHEAD: usize = LENGTH_PREFIX + 1 + 1 + 4;
    let limit = COMMAND_SLOT - OVERHEAD;

    let message = if message.len() <= limit {
        message
    } else {
        // UTF-8 の境界で切る。`split_at` は境界以外で panic するため、
        // 手前の境界まで下がる。panic hook から呼ばれるので再帰は禁物。
        let mut end = limit;
        while end > 0 && !message.is_char_boundary(end) {
            end -= 1;
        }
        &message[..end]
    };

    let mut frame = Vec::with_capacity(message.len() + OVERHEAD);
    encode_command(
        &mut frame,
        &Command::Error {
            code,
            message: message.to_string(),
        },
    );
    let _ = emit(&frame);
}

// panic を `Command::Error` として JavaScript へ送る処理は `lib.rs` の
// `#[panic_handler]` にある。
//
// app repository の `src/lib.rs` はこれに相当するものを持たない
// (`debug_log!` と `#[panic_handler]` は両方コメントアウトされている)。
// そのため panic は `RuntimeError: unreachable` になり、メッセージも
// 発生位置も失われる。ここでは共有アリーナが既にあるため、panic の
// 内容をそのまま JavaScript へ運べる。
//
// `no_std` では `std::panic::set_hook` が使えないため、当初この位置に
// あった `install_panic_hook` は `#[panic_handler]` へ置き換えてある。
// 登録が compile 時になったので `initialize` からの呼び出しは無くなり、
// 二重登録の考慮も要らなくなった。`#[panic_handler]` は crate graph 全体で
// 1 つだけであり、`cdylib` 本体である `lib.rs` が持つのが適切である。
//
// handler が走った後 thread は停止する。停止した worker は JavaScript 側が
// `terminate` して作り直す。
//
// `RUNNING` は触らない。panic は `poll` の内側から巻き戻るため
// `run_loop` の `while` へ戻らず、停止条件を書いても読まれない。
// また JavaScript 側は丸ごと作り直す以外の判断をしないので、
// 停止したことを `message` と別に伝える必要もない。
//
// handler から呼ぶため `report_error` は `pub` のままにしてある。

// ============================================================
// encode, decode
// ============================================================

/// コマンドを書き出す際の追記先を保持する。
pub struct Encoder<'a>(&'a mut Vec<u8>);

impl<'a> Encoder<'a> {
    /// 追記先を指す Encoder を作る。
    pub fn new(commands: &'a mut Vec<u8>) -> Self {
        Self(commands)
    }

    /// `u8` 1 個を追記する。
    pub fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    /// `u16` を little endian で追記する。
    pub fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    /// `u32` を little endian で追記する。
    pub fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    /// `i32` を little endian で追記する。
    pub fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    /// `f32` を little endian で追記する。
    pub fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    /// 長さ前置付きで byte 列を追記する。
    pub fn bytes(&mut self, value: &[u8]) {
        self.u32(value.len() as u32);
        self.0.extend_from_slice(value);
    }

    /// 長さ前置付きで文字列を UTF-8 として追記する。
    pub fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    /// `dom::Id` を `[count:u8]([tag:u8][number:u32])*` として追記する。
    ///
    /// `Segment::n` が `None` の場合は番号に `u32::MAX` を置く。
    /// app repository が `Id::encode` で組み立てる文字列より短くなる。
    pub fn id(&mut self, value: &dom::Id) {
        self.u8(value.0.len() as u8);
        for segment in &value.0 {
            self.u8(segment.tag.encode_u8());
            self.u32(segment.n.unwrap_or(u32::MAX));
        }
    }
}

/// イベントを読み出す際の位置を保持する。
pub struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    /// 先頭を指す Decoder を作る。
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    /// 現在位置から `count` byte 取り出して位置を進める。範囲外なら None。
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let slice = self.bytes.get(self.position..self.position + count)?;
        self.position += count;
        Some(slice)
    }

    /// `u8` 1 個を読む。
    pub fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    /// little endian の `u32` を読む。
    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    /// little endian の `f32` を読む。
    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    /// little endian の `f64` を読む。
    pub fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    /// 長さ前置付きの byte 列を読む。
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let length = self.u32()? as usize;
        self.take(length)
    }

    /// 長さ前置付きの文字列を UTF-8 として読む。
    pub fn string(&mut self) -> Option<String> {
        Some(str::from_utf8(self.bytes()?).ok()?.to_string())
    }

    /// `Encoder::id` が書いた形式から `dom::Id` を読む。
    pub fn id(&mut self) -> Option<dom::Id> {
        let count = self.u8()? as usize;
        let mut segments = Vec::with_capacity(count);
        for _ in 0..count {
            let tag = dom::Tag::decode_u8(self.u8()?);
            let number = self.u32()?;
            segments.push(dom::Segment {
                tag,
                n: if number == u32::MAX {
                    None
                } else {
                    Some(number)
                },
            });
        }
        Some(dom::Id(segments))
    }
}
