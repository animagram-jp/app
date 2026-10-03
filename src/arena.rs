// Arena
//

use alloc::{format, string::String, vec::Vec};
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};
use core::{
    assert,
    cell::UnsafeCell,
    fmt::{self, Debug, Display, Formatter},
    marker::Sync,
    option::Option::{self, None, Some},
    primitive::{bool, u8, u32, usize},
    ptr, slice,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    Error,
    app::App,
    js_client::{WireError, encode_command, encode_error},
};

// === arena layout ===

pub const EVENT_CONTROL: usize = 0;
pub const EVENT_PAYLOAD: usize = EVENT_CONTROL + CONTROL_SIZE;
pub const EVENT_CAPACITY: usize = 262144;
pub const EVENT_FRAME_MAX: usize = 4096;

pub const COMMAND_CONTROL: usize = EVENT_PAYLOAD + EVENT_CAPACITY;
pub const COMMAND_PAYLOAD: usize = COMMAND_CONTROL + CONTROL_SIZE;
pub const COMMAND_CAPACITY: usize = 1048576;
pub const COMMAND_FRAME_MAX: usize = 65536;

pub const ARENA_SIZE: usize = COMMAND_PAYLOAD + COMMAND_CAPACITY;

pub const CONTROL_WRITE_OFFSET: usize = 0;
pub const CONTROL_READ_OFFSET: usize = 64;
pub const CONTROL_SIZE: usize = 2 * CONTROL_READ_OFFSET;
pub const LENGTH_PREFIX: usize = 4;
pub const ALIGNMENT: usize = 4;
pub const PADDING_MARK: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub struct Ring {
    pub control:   usize,
    pub payload:   usize,
    pub capacity:  usize,
    pub frame_max: usize,
}

impl Ring {
    pub const fn record_size(&self, length: usize) -> usize {
        LENGTH_PREFIX + length.div_ceil(ALIGNMENT) * ALIGNMENT
    }
}

pub const EVENT_RING: Ring = Ring {
    control:   EVENT_CONTROL,
    payload:   EVENT_PAYLOAD,
    capacity:  EVENT_CAPACITY,
    frame_max: EVENT_FRAME_MAX,
};

pub const COMMAND_RING: Ring = Ring {
    control:   COMMAND_CONTROL,
    payload:   COMMAND_PAYLOAD,
    capacity:  COMMAND_CAPACITY,
    frame_max: COMMAND_FRAME_MAX,
};

// === arena state ===

#[repr(C, align(64))]
pub struct Arena {
    bytes: UnsafeCell<[u8; ARENA_SIZE]>,
}

unsafe impl Sync for Arena {}

pub static ARENA: Arena = Arena { bytes: UnsafeCell::new([0; ARENA_SIZE]) };

///
pub static mut APP: Option<App> = None;

pub static mut RUNNING: bool = true;

// === arena function ===

impl Arena {
    #[inline]
    pub fn base(&self) -> *mut u8 {
        self.bytes.get() as *mut u8
    }

    #[inline]
    fn control_at(&self, control: usize, offset: usize) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr((self.base() as usize + control + offset) as *mut u32) }
    }

    pub fn initialize(&self) {
        self.control_at(EVENT_CONTROL, CONTROL_WRITE_OFFSET).store(0, Ordering::Relaxed);
        self.control_at(EVENT_CONTROL, CONTROL_READ_OFFSET).store(0, Ordering::Relaxed);
        self.control_at(COMMAND_CONTROL, CONTROL_WRITE_OFFSET).store(0, Ordering::Relaxed);
        self.control_at(COMMAND_CONTROL, CONTROL_READ_OFFSET).store(0, Ordering::Relaxed);
    }

    fn word_at(&self, ring: Ring, position: usize) -> *mut u32 {
        (self.base() as usize + ring.payload + position) as *mut u32
    }

    fn frame_length(&self, ring: Ring, position: usize) -> usize {
        let length = unsafe { self.word_at(ring, position).read_unaligned() } as usize;
        length.min(ring.frame_max).min(ring.capacity - position - LENGTH_PREFIX)
    }

    pub(crate) fn ring_push(&self, ring: Ring, source: &[u8]) -> bool {
        if source.len() > ring.frame_max {
            return false;
        }
        let write_atomic = self.control_at(ring.control, CONTROL_WRITE_OFFSET);
        let read_atomic = self.control_at(ring.control, CONTROL_READ_OFFSET);

        let size = ring.record_size(source.len());
        let mut write = write_atomic.load(Ordering::Relaxed);
        let read = read_atomic.load(Ordering::Acquire);
        let mut used = write.wrapping_sub(read) as usize;
        let mut position = write as usize & (ring.capacity - 1);

        let tail = ring.capacity - position;
        if size > tail {
            if used + tail > ring.capacity {
                return false;
            }
            unsafe { self.word_at(ring, position).write_unaligned(PADDING_MARK) };
            write = write.wrapping_add(tail as u32);
            write_atomic.store(write, Ordering::Release);
            used += tail;
            position = 0;
        }
        if used + size > ring.capacity {
            return false;
        }

        unsafe {
            self.word_at(ring, position).write_unaligned(source.len() as u32);
            ptr::copy_nonoverlapping(
                source.as_ptr(),
                (self.word_at(ring, position) as usize + LENGTH_PREFIX) as *mut u8,
                source.len(),
            );
        }

        write_atomic.store(write.wrapping_add(size as u32), Ordering::Release);
        true
    }

    pub(crate) fn ring_peek(&self, ring: Ring) -> Option<&[u8]> {
        let write_atomic = self.control_at(ring.control, CONTROL_WRITE_OFFSET);
        let read_atomic = self.control_at(ring.control, CONTROL_READ_OFFSET);

        let mut read = read_atomic.load(Ordering::Relaxed);
        loop {
            let write = write_atomic.load(Ordering::Acquire);
            if read == write {
                return None;
            }
            let position = read as usize & (ring.capacity - 1);
            if unsafe { self.word_at(ring, position).read_unaligned() } == PADDING_MARK {
                read = read.wrapping_add((ring.capacity - position) as u32);
                read_atomic.store(read, Ordering::Release);
                continue;
            }
            let length = self.frame_length(ring, position);
            let start = self.word_at(ring, position) as usize + LENGTH_PREFIX;
            return Some(unsafe { slice::from_raw_parts(start as *const u8, length) });
        }
    }

    pub(crate) fn ring_commit_pop(&self, ring: Ring) {
        let read_atomic = self.control_at(ring.control, CONTROL_READ_OFFSET);
        let read = read_atomic.load(Ordering::Relaxed);
        let position = read as usize & (ring.capacity - 1);
        let size = ring.record_size(self.frame_length(ring, position));
        read_atomic.store(read.wrapping_add(size as u32), Ordering::Release);
    }

    pub fn event_peek(&self) -> Option<&[u8]> {
        self.ring_peek(EVENT_RING)
    }

    pub fn event_commit_pop(&self) {
        self.ring_commit_pop(EVENT_RING);
    }

    pub fn event_write_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_WRITE_OFFSET).load(Ordering::Acquire)
    }

    pub fn event_read_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_READ_OFFSET).load(Ordering::Relaxed)
    }

    pub fn command_push(&self, frame: &[u8]) -> bool {
        assert!(
            frame.len() <= COMMAND_FRAME_MAX,
            "command frame too large: {} > {}",
            frame.len(),
            COMMAND_FRAME_MAX,
        );
        self.ring_push(COMMAND_RING, frame)
    }
}

// === arena entry point ===
//

///
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn arena_pointer() -> u32 {
    ARENA.base() as u32
}

///
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn initialize() {
    ARENA.initialize();
    unsafe { RUNNING = true };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn process_event() {
    #[allow(clippy::deref_addrof)]
    let Some(app) = (unsafe { (*(&raw mut APP)).as_mut() }) else {
        return;
    };
    let Some(frame) = ARENA.event_peek() else {
        return;
    };
    app.clear();
    app.process(frame);
    ARENA.event_commit_pop();
    let mut frame = Vec::new();
    let emitted = app.commands().iter().all(|command| {
        frame.clear();
        encode_command(&mut frame, command);
        emit(&frame)
    });
    if !emitted {
        report_error(Error::Arena(ArenaError::CommandOverflow));
    }
}

///
///
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
#[wasm_bindgen]
pub fn serve_event() {
    while unsafe { RUNNING } {
        process_event();
        let write = ARENA.event_write_seq();
        if ARENA.event_read_seq() == write {
            unsafe {
                let pointer = (ARENA.base() as usize + EVENT_CONTROL) as *mut i32;
                memory_atomic_wait32(pointer, write as i32, -1);
            }
        }
    }
}

///
pub fn emit(frame: &[u8]) -> bool {
    let pushed = ARENA.command_push(frame);

    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    unsafe {
        let pointer = (ARENA.base() as usize + COMMAND_CONTROL) as *mut i32;
        memory_atomic_notify(pointer, 1);
    }

    pushed
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

    fn is_serious(&self) -> bool {
        false
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

    fn is_serious(&self) -> bool {
        true
    }
}

// === error report ===

pub fn report_error(error: Error) {
    let mut path = Vec::new();
    error.identifiers(&mut path);
    let overhead = LENGTH_PREFIX + 1 + 1 + 1 + 2 * path.len() + 4;
    let limit = COMMAND_FRAME_MAX - overhead;

    let full_detail = error.detail();
    let detail = if full_detail.len() <= limit {
        &full_detail[..]
    } else {
        let mut end = limit;
        while end > 0 && !full_detail.is_char_boundary(end) {
            end -= 1;
        }
        &full_detail[..end]
    };

    let mut frame = Vec::with_capacity(detail.len() + overhead);
    encode_error(&mut frame, &error, detail);
    let _ = emit(&frame);
}

#[cfg(test)]
mod ring_tests {
    use alloc::{vec, vec::Vec};
    use core::cell::UnsafeCell;

    use super::*;

    const TEST_RING: Ring =
        Ring { control: 0, payload: CONTROL_SIZE, capacity: 256, frame_max: 100 };

    static TEST_ARENA: Arena = Arena { bytes: UnsafeCell::new([0; ARENA_SIZE]) };
    static GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fresh() -> std::sync::MutexGuard<'static, ()> {
        let guard = GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        TEST_ARENA.control_at(TEST_RING.control, CONTROL_WRITE_OFFSET).store(0, Ordering::Relaxed);
        TEST_ARENA.control_at(TEST_RING.control, CONTROL_READ_OFFSET).store(0, Ordering::Relaxed);
        guard
    }

    fn pop(ring: Ring) -> Option<Vec<u8>> {
        let frame = TEST_ARENA.ring_peek(ring)?.to_vec();
        TEST_ARENA.ring_commit_pop(ring);
        Some(frame)
    }

    fn frame_of(seed: u32, length: usize) -> Vec<u8> {
        (0..length).map(|index| (seed as usize * 31 + index) as u8).collect()
    }

    fn js_number(name: &str) -> usize {
        let init_js = include_str!("../distribution/init.js");
        let head = std::format!("const {name} = ");
        let start = init_js.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        let end = start + init_js[start..].find(';').unwrap();
        init_js[start..end].parse().unwrap()
    }

    #[test]
    fn layout_constants_match_init_js() {
        assert_eq!(js_number("CONTROL_WRITE_OFFSET"), CONTROL_WRITE_OFFSET);
        assert_eq!(js_number("CONTROL_READ_OFFSET"), CONTROL_READ_OFFSET);
        assert_eq!(js_number("LENGTH_PREFIX"), LENGTH_PREFIX);
        assert_eq!(js_number("ALIGNMENT"), ALIGNMENT);
        assert_eq!(js_number("EVENT_CAPACITY"), EVENT_CAPACITY);
        assert_eq!(js_number("EVENT_FRAME_MAX"), EVENT_FRAME_MAX);
        assert_eq!(js_number("COMMAND_CAPACITY"), COMMAND_CAPACITY);
        assert_eq!(js_number("COMMAND_FRAME_MAX"), COMMAND_FRAME_MAX);
        assert!(
            include_str!("../distribution/init.js")
                .contains(&std::format!("const PADDING_MARK = {:#X};", PADDING_MARK))
        );
    }

    #[test]
    fn capacities_are_powers_of_two_that_hold_a_frame_and_its_padding() {
        for ring in [EVENT_RING, COMMAND_RING] {
            assert!(ring.capacity.is_power_of_two());
            assert!(ring.record_size(ring.frame_max) * 2 <= ring.capacity);
        }
    }

    #[test]
    fn record_size_is_the_prefix_plus_the_length_rounded_up() {
        assert_eq!(TEST_RING.record_size(0), 4);
        assert_eq!(TEST_RING.record_size(1), 8);
        assert_eq!(TEST_RING.record_size(4), 8);
        assert_eq!(TEST_RING.record_size(5), 12);
    }

    #[test]
    fn frames_come_out_in_order_with_their_own_lengths() {
        let _guard = fresh();
        assert!(TEST_ARENA.ring_peek(TEST_RING).is_none());
        for (seed, length) in [(1, 0), (2, 1), (3, 7), (4, 33)] {
            assert!(TEST_ARENA.ring_push(TEST_RING, &frame_of(seed, length)));
        }
        for (seed, length) in [(1, 0), (2, 1), (3, 7), (4, 33)] {
            assert_eq!(pop(TEST_RING).unwrap(), frame_of(seed, length));
        }
        assert!(pop(TEST_RING).is_none());
    }

    #[test]
    fn a_full_ring_refuses_and_recovers_after_a_pop() {
        let _guard = fresh();
        let mut pushed = 0;
        while TEST_ARENA.ring_push(TEST_RING, &[9; 20]) {
            pushed += 1;
        }
        assert_eq!(pushed, TEST_RING.capacity / TEST_RING.record_size(20));
        assert!(!TEST_ARENA.ring_push(TEST_RING, &[1]) || pushed > 0);
        assert_eq!(pop(TEST_RING).unwrap(), vec![9; 20]);
        assert!(TEST_ARENA.ring_push(TEST_RING, &[9; 20]));
    }

    #[test]
    fn the_largest_frame_fits_at_every_position_of_an_empty_ring() {
        for step in 0..(TEST_RING.capacity / ALIGNMENT) {
            let _guard = fresh();
            for _ in 0..step {
                assert!(TEST_ARENA.ring_push(TEST_RING, &[]));
                assert!(pop(TEST_RING).is_some());
            }
            let largest = frame_of(5, TEST_RING.frame_max);
            let mut attempts = 0;
            while !TEST_ARENA.ring_push(TEST_RING, &largest) {
                attempts += 1;
                assert!(attempts <= 2, "stuck at step {step}");
                assert!(pop(TEST_RING).is_none());
            }
            assert_eq!(pop(TEST_RING).unwrap(), largest);
        }
    }

    #[test]
    fn varied_frames_survive_many_wraps_in_order() {
        let _guard = fresh();
        let mut state = 12345u32;
        let mut next_length = || {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state >> 16) as usize % (TEST_RING.frame_max + 1)
        };
        let mut pending: Vec<(u32, usize)> = Vec::new();
        let mut produced = 0u32;
        let mut consumed = 0u32;
        while consumed < 3000 {
            let length = next_length();
            if TEST_ARENA.ring_push(TEST_RING, &frame_of(produced, length)) {
                pending.push((produced, length));
                produced += 1;
            } else {
                let (seed, length) = pending.remove(0);
                assert_eq!(pop(TEST_RING).unwrap(), frame_of(seed, length));
                consumed += 1;
            }
            if produced % 7 == 0 && !pending.is_empty() {
                let (seed, length) = pending.remove(0);
                assert_eq!(pop(TEST_RING).unwrap(), frame_of(seed, length));
                consumed += 1;
            }
        }
        for (seed, length) in pending {
            assert_eq!(pop(TEST_RING).unwrap(), frame_of(seed, length));
        }
        assert!(pop(TEST_RING).is_none());
    }

    #[test]
    fn a_frame_over_the_limit_is_refused() {
        let _guard = fresh();
        assert!(!TEST_ARENA.ring_push(TEST_RING, &vec![0; TEST_RING.frame_max + 1]));
    }

    #[test]
    #[should_panic(expected = "command frame too large")]
    fn command_push_panics_over_the_frame_limit() {
        ARENA.command_push(&vec![0; COMMAND_FRAME_MAX + 1]);
    }
}

#[cfg(test)]
mod error_tests {
    use alloc::{format, string::String, vec::Vec};

    use super::*;

    fn identifiers(error: &impl WireError) -> Vec<u16> {
        let mut path = Vec::new();
        error.identifiers(&mut path);
        path
    }

    #[test]
    fn arena_error_command_overflow_is_recoverable_and_has_no_detail() {
        assert_eq!(identifiers(&ArenaError::CommandOverflow), [1]);
        assert_eq!(ArenaError::CommandOverflow.detail(), "");
        assert!(!ArenaError::CommandOverflow.is_serious());
    }

    #[test]
    fn panic_error_is_serious_and_reports_location_and_message() {
        let error = PanicError { location: String::from("a.rs:1"), message: String::from("boom") };
        assert_eq!(identifiers(&error), [1]);
        assert_eq!(error.detail(), "a.rs:1: boom");
        assert!(error.is_serious());
        assert_eq!(format!("{error}"), format!("{error:?}"));
    }
}
