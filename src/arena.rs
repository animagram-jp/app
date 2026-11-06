use alloc::{format, string::String, vec::Vec};
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};
use core::{
    assert,
    cell::UnsafeCell,
    clone::Clone,
    fmt::{self, Debug, Display, Formatter},
    marker::{Copy, Sync},
    mem::size_of,
    option::Option::{self, None, Some},
    primitive::{bool, u8, u32, usize},
    ptr, slice,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Error, Output, WireError, app::App, host::Command};
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use crate::{
    app::{Store, now_in_ms},
    file_store::{FileStoreError, Mount, StoreId},
};

// === ring ===

pub const COUNTER_WRITE_OFFSET: usize = 0;
pub const COUNTER_READ_OFFSET: usize = 64;
pub const DATA_OFFSET: usize = 128;
pub const LENGTH_PREFIX: usize = 4;
pub const PADDING_MARK: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub struct Ring {
    pub start:     usize,
    pub data_size: usize,
    pub frame_max: usize,
}

impl Ring {
    /// ```
    /// # use app::arena::COMMAND_RING;
    /// assert_eq!(COMMAND_RING.record_size(0), 4);
    /// assert_eq!(COMMAND_RING.record_size(5), 12);
    /// ```
    pub const fn record_size(&self, length: usize) -> usize {
        LENGTH_PREFIX + length.next_multiple_of(size_of::<u32>())
    }
}

pub const EVENT_RING: Ring = Ring { start: 0, data_size: 1024 * 256, frame_max: 1024 * 4 };

pub const COMMAND_RING: Ring = Ring {
    start:     EVENT_RING.start + DATA_OFFSET + EVENT_RING.data_size,
    data_size: 1024 * 1024,
    frame_max: 1024 * 64,
};

const _: () = {
    assert!(COUNTER_WRITE_OFFSET == 0);
    assert!(COUNTER_READ_OFFSET >= COUNTER_WRITE_OFFSET + size_of::<u32>());
    assert!(DATA_OFFSET >= COUNTER_READ_OFFSET + size_of::<u32>());
    assert!(COUNTER_WRITE_OFFSET % 64 == 0);
    assert!(COUNTER_READ_OFFSET % 64 == 0);
    assert!(DATA_OFFSET % 64 == 0);
    assert!(EVENT_RING.data_size.is_power_of_two());
    assert!(COMMAND_RING.data_size.is_power_of_two());
    assert!(EVENT_RING.record_size(EVENT_RING.frame_max) * 2 <= EVENT_RING.data_size);
    assert!(COMMAND_RING.record_size(COMMAND_RING.frame_max) * 2 <= COMMAND_RING.data_size);
};

// === arena ===

pub const ARENA_SIZE: usize = COMMAND_RING.start + DATA_OFFSET + COMMAND_RING.data_size;

#[repr(C, align(64))]
pub struct Arena {
    bytes: UnsafeCell<[u8; ARENA_SIZE]>,
}

// SAFETY: the app runs on exactly one thread (main thread or dedicated worker, see
// CONTRIBUTING.md). The only other party is JS, which touches the arena through
// `Atomics` on the counters and the ring protocol below, never concurrently with Rust
// on the same bytes.
unsafe impl Sync for Arena {}

pub static ARENA: Arena = Arena { bytes: UnsafeCell::new([0; ARENA_SIZE]) };
pub static mut APP: Option<App> = None;
pub static RUNNING: AtomicBool = AtomicBool::new(true);

impl Arena {
    #[inline]
    fn counter_at(&self, start: usize, counter_offset: usize) -> &AtomicU32 {
        // SAFETY: `bytes` is `align(64)` and every `start` / `counter_offset` is a multiple
        // of 64 inside `ARENA_SIZE` (const asserts above), so the pointer is in bounds and
        // 4-aligned. Counters are only ever accessed atomically, by Rust here and by JS
        // through `Atomics`, and the reference lives as long as `ARENA`.
        unsafe {
            AtomicU32::from_ptr((self.bytes.get() as usize + start + counter_offset) as *mut u32)
        }
    }

    fn get(&self, start: usize, end: usize) -> Option<&[u8]> {
        if start > end || end > ARENA_SIZE {
            return None;
        }
        let pointer = (self.bytes.get() as *const u8).wrapping_add(start);
        // SAFETY: `start <= end <= ARENA_SIZE` was checked, the static is zero-initialized,
        // and the ring protocol keeps the writer past the write counter while the reader
        // holds `[read, write)`, so the slice is not mutated during its lifetime.
        Some(unsafe { slice::from_raw_parts(pointer, end - start) })
    }

    fn get_u32(&self, start: usize) -> Option<u32> {
        let bytes = self.get(start, start + size_of::<u32>())?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn set(&self, start: usize, bytes: &[u8]) -> bool {
        let Some(end) = start.checked_add(bytes.len()) else {
            return false;
        };
        if end > ARENA_SIZE {
            return false;
        }
        let pointer = (self.bytes.get() as *mut u8).wrapping_add(start);
        // SAFETY: `start + bytes.len() <= ARENA_SIZE` was checked, `bytes` is a Rust slice
        // outside the arena, and the ring protocol gives the writer exclusive use of
        // `[write, read)` until the write counter is published.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len()) };
        true
    }

    pub fn initialize(&self) {
        self.counter_at(EVENT_RING.start, COUNTER_WRITE_OFFSET).store(0, Ordering::Relaxed);
        self.counter_at(EVENT_RING.start, COUNTER_READ_OFFSET).store(0, Ordering::Relaxed);
        self.counter_at(COMMAND_RING.start, COUNTER_WRITE_OFFSET).store(0, Ordering::Relaxed);
        self.counter_at(COMMAND_RING.start, COUNTER_READ_OFFSET).store(0, Ordering::Relaxed);
    }

    fn frame_length(&self, ring: Ring, position: usize) -> usize {
        let length = self.get_u32(ring.start + DATA_OFFSET + position).unwrap_or(0) as usize;
        length.min(ring.frame_max).min(ring.data_size - position - LENGTH_PREFIX)
    }

    pub(crate) fn write_ring(&self, ring: Ring, source: &[u8]) -> bool {
        if source.len() > ring.frame_max {
            return false;
        }
        let write_counter = self.counter_at(ring.start, COUNTER_WRITE_OFFSET);
        let read_counter = self.counter_at(ring.start, COUNTER_READ_OFFSET);

        let size = ring.record_size(source.len());
        let mut write = write_counter.load(Ordering::Relaxed);
        let read = read_counter.load(Ordering::Acquire);
        let mut used = write.wrapping_sub(read) as usize;
        let mut position = write as usize & (ring.data_size - 1);

        let tail = ring.data_size - position;
        if size > tail {
            if used + tail > ring.data_size {
                return false;
            }
            if !self.set(ring.start + DATA_OFFSET + position, &PADDING_MARK.to_le_bytes()) {
                return false;
            }
            write = write.wrapping_add(tail as u32);
            write_counter.store(write, Ordering::Release);
            used += tail;
            position = 0;
        }
        if used + size > ring.data_size {
            return false;
        }

        let record = ring.start + DATA_OFFSET + position;
        if !self.set(record, &(source.len() as u32).to_le_bytes())
            || !self.set(record + LENGTH_PREFIX, source)
        {
            return false;
        }

        write_counter.store(write.wrapping_add(size as u32), Ordering::Release);
        true
    }

    pub(crate) fn read_ring(&self, ring: Ring) -> Option<&[u8]> {
        let write_counter = self.counter_at(ring.start, COUNTER_WRITE_OFFSET);
        let read_counter = self.counter_at(ring.start, COUNTER_READ_OFFSET);

        let mut read = read_counter.load(Ordering::Relaxed);
        loop {
            let write = write_counter.load(Ordering::Acquire);
            if read == write {
                return None;
            }
            let position = read as usize & (ring.data_size - 1);
            let record = ring.start + DATA_OFFSET + position;
            if self.get_u32(record) == Some(PADDING_MARK) {
                read = read.wrapping_add((ring.data_size - position) as u32);
                read_counter.store(read, Ordering::Release);
                continue;
            }
            let length = self.frame_length(ring, position);
            return self.get(record + LENGTH_PREFIX, record + LENGTH_PREFIX + length);
        }
    }

    pub(crate) fn advance_ring(&self, ring: Ring) {
        let read_counter = self.counter_at(ring.start, COUNTER_READ_OFFSET);
        let read = read_counter.load(Ordering::Relaxed);
        let position = read as usize & (ring.data_size - 1);
        let size = ring.record_size(self.frame_length(ring, position));
        read_counter.store(read.wrapping_add(size as u32), Ordering::Release);
    }

    pub fn get_write_count(&self, ring: Ring) -> u32 {
        self.counter_at(ring.start, COUNTER_WRITE_OFFSET).load(Ordering::Acquire)
    }

    pub fn get_read_count(&self, ring: Ring) -> u32 {
        self.counter_at(ring.start, COUNTER_READ_OFFSET).load(Ordering::Relaxed)
    }
}

// === public api ===

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn arena_offset() -> u32 {
    ARENA.bytes.get() as u32
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn initialize() {
    ARENA.initialize();
    RUNNING.store(true, Ordering::Relaxed);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn process_event() {
    let slot: *mut Option<App> = &raw mut APP;
    // SAFETY: `APP` is only touched from the single app thread, and the JS event loop
    // calls the exported functions one at a time, so no other reference exists while
    // this one is alive.
    let Some(app) = (unsafe { (*slot).as_mut() }) else {
        return;
    };
    let Some(frame) = ARENA.read_ring(EVENT_RING) else {
        return;
    };
    app.clear();
    app.process(frame);
    ARENA.advance_ring(EVENT_RING);
    flush(app);
}

pub(crate) fn flush(app: &App) {
    let mut frame = Vec::new();
    let emitted = app.commands().iter().all(|command| {
        frame.clear();
        command.encode(&mut frame);
        emit(&frame)
    });
    if !emitted {
        report_error(Error::Arena(ArenaError::CommandOverflow));
    }
}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub const REOPEN_INTERVAL_IN_MS: u64 = 250;
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub const REOPEN_ATTEMPTS: u32 = 40;

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
#[wasm_bindgen]
pub async fn serve_event() {
    let mut retry: Option<(StoreId, u32, u64)> = None;
    while RUNNING.load(Ordering::Relaxed) {
        process_event();
        {
            let slot: *mut Option<App> = &raw mut APP;
            // SAFETY: same as `process_event`; the reference is dropped before the `await`
            // and taken again after it.
            let wanted = unsafe { (*slot).as_mut() }.and_then(App::reopen_wanted);
            if let Some(id) = wanted {
                retry = Some((id, 0, 0));
            }
            if let Some((id, attempts, due_in_ms)) = retry
                && now_in_ms() >= due_in_ms
            {
                match Store::open(id, true).await {
                    Err(FileStoreError::NoModificationAllowed(_)) if attempts < REOPEN_ATTEMPTS => {
                        retry = Some((id, attempts + 1, now_in_ms() + REOPEN_INTERVAL_IN_MS));
                    }
                    opened => {
                        retry = None;
                        // SAFETY: same as above; nothing else borrows `APP` across the `await`.
                        if let Some(app) = unsafe { (*slot).as_mut() } {
                            app.clear();
                            app.opened(opened);
                            flush(app);
                        }
                    }
                }
                continue;
            }
        }
        let write = ARENA.get_write_count(EVENT_RING);
        if ARENA.get_read_count(EVENT_RING) == write {
            let timeout_in_ns = retry.map_or(-1, |(_, _, due_in_ms)| {
                (due_in_ms.saturating_sub(now_in_ms()) * 1_000_000) as i64
            });
            // SAFETY: the counter pointer is 4-aligned shared memory (see `counter_at`), and
            // `serve_event` only runs on the dedicated worker, where `memory.atomic.wait32`
            // is permitted (the main thread would trap).
            unsafe {
                let pointer =
                    ARENA.counter_at(EVENT_RING.start, COUNTER_WRITE_OFFSET).as_ptr() as *mut i32;
                memory_atomic_wait32(pointer, write as i32, timeout_in_ns);
            }
        }
    }
}

pub fn emit(frame: &[u8]) -> bool {
    assert!(
        frame.len() <= COMMAND_RING.frame_max,
        "command frame too large: {} > {}",
        frame.len(),
        COMMAND_RING.frame_max,
    );
    let written = ARENA.write_ring(COMMAND_RING, frame);

    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    // SAFETY: the counter pointer is 4-aligned shared memory (see `counter_at`);
    // `memory.atomic.notify` has no thread restriction.
    unsafe {
        let pointer =
            ARENA.counter_at(COMMAND_RING.start, COUNTER_WRITE_OFFSET).as_ptr() as *mut i32;
        memory_atomic_notify(pointer, 1);
    }

    written
}

// === error ===

#[derive(Debug)]
pub enum ArenaError {
    CommandOverflow,
}

impl Display for ArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for ArenaError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            ArenaError::CommandOverflow => path.push(1),
        }
    }

    fn detail(&self) -> String {
        String::new()
    }
}

#[derive(Debug)]
pub struct PanicError {
    pub location: String,
    pub message:  String,
}

impl Display for PanicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for PanicError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        path.push(1);
    }

    fn detail(&self) -> String {
        format!("{}: {}", self.location, self.message)
    }
}

// === error report ===

pub fn report_error(error: Error) {
    let mut frame = Vec::new();
    Command::Error { error }.encode(&mut frame);
    let _ = emit(&frame);
}

#[cfg(test)]
mod ring_tests {
    use alloc::{collections::VecDeque, vec, vec::Vec};
    use core::cell::UnsafeCell;
    use std::{
        format,
        sync::{Mutex, MutexGuard},
    };

    use super::*;
    use crate::Rng;

    const TEST_RING: Ring = Ring { start: 0, data_size: 256, frame_max: 100 };

    static TEST_ARENA: Arena = Arena { bytes: UnsafeCell::new([0; ARENA_SIZE]) };
    static GUARD: Mutex<()> = Mutex::new(());

    fn fresh() -> MutexGuard<'static, ()> {
        let guard = GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        TEST_ARENA.counter_at(TEST_RING.start, COUNTER_WRITE_OFFSET).store(0, Ordering::Relaxed);
        TEST_ARENA.counter_at(TEST_RING.start, COUNTER_READ_OFFSET).store(0, Ordering::Relaxed);
        guard
    }

    fn read_and_advance(ring: Ring) -> Option<Vec<u8>> {
        let frame = TEST_ARENA.read_ring(ring)?.to_vec();
        TEST_ARENA.advance_ring(ring);
        Some(frame)
    }

    fn frame_of(seed: u32, length: usize) -> Vec<u8> {
        (0..length).map(|index| (seed as usize * 31 + index) as u8).collect()
    }

    fn js_number(name: &str) -> usize {
        let init_js = include_str!("../distribution/init.js");
        let head = format!("const {name} = ");
        let start = init_js.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        let end = start + init_js[start..].find(';').unwrap();
        init_js[start..end].parse().unwrap()
    }

    fn js_ring_number(ring: &str, field: &str) -> usize {
        let init_js = include_str!("../distribution/init.js");
        let head = format!("const {ring} = {{");
        let body_start =
            init_js.find(&head).unwrap_or_else(|| panic!("{ring} not found")) + head.len();
        let body = &init_js[body_start..body_start + init_js[body_start..].find("};").unwrap()];
        let key = format!("{field}: ");
        let start =
            body.find(&key).unwrap_or_else(|| panic!("{ring}.{field} not found")) + key.len();
        let end = start + body[start..].find(',').unwrap();
        body[start..end].parse().unwrap()
    }

    #[test]
    fn layout_constants_match_init_js() {
        assert_eq!(js_number("COUNTER_WRITE_OFFSET"), COUNTER_WRITE_OFFSET);
        assert_eq!(js_number("COUNTER_READ_OFFSET"), COUNTER_READ_OFFSET);
        assert_eq!(js_number("DATA_OFFSET"), DATA_OFFSET);
        assert_eq!(js_number("LENGTH_PREFIX"), LENGTH_PREFIX);
        assert_eq!(js_number("ALIGNMENT"), size_of::<u32>());
        assert_eq!(js_ring_number("EVENT_RING", "start"), EVENT_RING.start);
        assert_eq!(js_ring_number("EVENT_RING", "data_size"), EVENT_RING.data_size);
        assert_eq!(js_ring_number("EVENT_RING", "frame_max"), EVENT_RING.frame_max);
        assert_eq!(js_ring_number("COMMAND_RING", "data_size"), COMMAND_RING.data_size);
        assert_eq!(js_ring_number("COMMAND_RING", "frame_max"), COMMAND_RING.frame_max);
        let init_js = include_str!("../distribution/init.js");
        assert!(init_js.contains(&format!("const PADDING_MARK = {:#X};", PADDING_MARK)));
        assert!(init_js.contains("start: EVENT_RING.start + DATA_OFFSET + EVENT_RING.data_size,"));
        assert_eq!(COMMAND_RING.start, EVENT_RING.start + DATA_OFFSET + EVENT_RING.data_size);
    }

    #[test]
    fn the_largest_frame_fits_at_every_position_of_an_empty_ring() {
        for step in 0..(TEST_RING.data_size / size_of::<u32>()) {
            let _guard = fresh();
            for _ in 0..step {
                assert!(TEST_ARENA.write_ring(TEST_RING, &[]));
                assert!(read_and_advance(TEST_RING).is_some());
            }
            let largest = frame_of(5, TEST_RING.frame_max);
            let mut attempts = 0;
            while !TEST_ARENA.write_ring(TEST_RING, &largest) {
                attempts += 1;
                assert!(attempts <= 2, "stuck at step {step}");
                assert!(read_and_advance(TEST_RING).is_none());
            }
            assert_eq!(read_and_advance(TEST_RING).unwrap(), largest);
        }
    }

    #[test]
    fn a_random_interleaving_of_writes_and_reads_matches_a_fifo_model() {
        for seed in 0..40 {
            let _guard = fresh();
            let mut rng = Rng::new(seed);
            let mut model: VecDeque<Vec<u8>> = VecDeque::new();
            for step in 0..3000 {
                if rng.chance(55) {
                    let length = rng.below(TEST_RING.frame_max + 20);
                    let frame = rng.bytes(length);
                    let accepted = TEST_ARENA.write_ring(TEST_RING, &frame);
                    if length > TEST_RING.frame_max {
                        assert!(!accepted, "seed {seed} step {step}: oversized frame accepted");
                    } else if accepted {
                        model.push_back(frame);
                    } else {
                        assert!(!model.is_empty(), "seed {seed} step {step}: empty ring refused");
                    }
                } else {
                    assert_eq!(
                        read_and_advance(TEST_RING),
                        model.pop_front(),
                        "seed {seed} step {step}"
                    );
                }
            }
            while let Some(expected) = model.pop_front() {
                assert_eq!(read_and_advance(TEST_RING), Some(expected), "seed {seed} drain");
            }
            assert!(read_and_advance(TEST_RING).is_none());
        }
    }

    #[test]
    #[should_panic(expected = "command frame too large")]
    fn emit_panics_over_the_frame_limit() {
        emit(&vec![0; COMMAND_RING.frame_max + 1]);
    }
}
