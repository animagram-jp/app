// Arena
//

use alloc::{format, string::String, vec::Vec};
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};
use core::{
    assert,
    cell::UnsafeCell,
    debug_assert,
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
pub const EVENT_SLOT: usize = 4096;
pub const EVENT_SLOT_COUNT: u32 = 64;

pub const COMMAND_CONTROL: usize = EVENT_PAYLOAD + EVENT_SLOT * EVENT_SLOT_COUNT as usize;
pub const COMMAND_PAYLOAD: usize = COMMAND_CONTROL + CONTROL_SIZE;
pub const COMMAND_SLOT: usize = 4096;
pub const COMMAND_SLOT_COUNT: u32 = 64;

pub const ARENA_SIZE: usize = COMMAND_PAYLOAD + COMMAND_SLOT * COMMAND_SLOT_COUNT as usize;

pub const CONTROL_WRITE_OFFSET: usize = 0;
///
pub const CONTROL_READ_OFFSET: usize = 64;
pub const CONTROL_SIZE: usize = 2 * CONTROL_READ_OFFSET;
pub const LENGTH_PREFIX: usize = 4;

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

    ///
    pub(crate) fn ring_push(
        &self,
        control: usize,
        payload: usize,
        slot: usize,
        slot_count: u32,
        source: &[u8],
    ) -> bool {
        debug_assert!(source.len() + LENGTH_PREFIX <= slot);
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

        write_atomic.store(write.wrapping_add(1), Ordering::Release);
        true
    }

    ///
    pub(crate) fn ring_peek(
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
        let length = length.min(slot - LENGTH_PREFIX);
        Some(unsafe { slice::from_raw_parts((offset + LENGTH_PREFIX) as *const u8, length) })
    }

    pub(crate) fn ring_commit_pop(&self, control: usize) {
        let read_atomic = self.control_at(control, CONTROL_READ_OFFSET);
        let read = read_atomic.load(Ordering::Relaxed);
        read_atomic.store(read.wrapping_add(1), Ordering::Release);
    }

    pub fn event_peek(&self) -> Option<&[u8]> {
        self.ring_peek(EVENT_CONTROL, EVENT_PAYLOAD, EVENT_SLOT, EVENT_SLOT_COUNT)
    }

    pub fn event_commit_pop(&self) {
        self.ring_commit_pop(EVENT_CONTROL);
    }

    pub fn event_write_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_WRITE_OFFSET).load(Ordering::Acquire)
    }

    pub fn event_read_seq(&self) -> u32 {
        self.control_at(EVENT_CONTROL, CONTROL_READ_OFFSET).load(Ordering::Relaxed)
    }

    pub fn command_push(&self, frame: &[u8]) -> bool {
        assert!(
            frame.len() <= COMMAND_SLOT - LENGTH_PREFIX,
            "command frame too large: {} > {}",
            frame.len(),
            COMMAND_SLOT - LENGTH_PREFIX,
        );
        self.ring_push(COMMAND_CONTROL, COMMAND_PAYLOAD, COMMAND_SLOT, COMMAND_SLOT_COUNT, frame)
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
    let limit = COMMAND_SLOT - overhead;

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
mod command_ring_tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn command_push_accepts_the_largest_frame_and_reports_a_full_ring() {
        ARENA.initialize();
        let largest = COMMAND_SLOT - LENGTH_PREFIX;

        assert!(ARENA.command_push(&vec![7; largest]));
        let frame = ARENA
            .ring_peek(COMMAND_CONTROL, COMMAND_PAYLOAD, COMMAND_SLOT, COMMAND_SLOT_COUNT)
            .unwrap();
        assert_eq!(frame.len(), largest);
        assert!(frame.iter().all(|byte| *byte == 7));
        ARENA.ring_commit_pop(COMMAND_CONTROL);

        for _ in 0..COMMAND_SLOT_COUNT {
            assert!(ARENA.command_push(&[1]));
        }
        assert!(!ARENA.command_push(&[1]));
        ARENA.initialize();
    }

    #[test]
    #[should_panic(expected = "command frame too large")]
    fn command_push_panics_over_the_slot_limit() {
        ARENA.command_push(&vec![0; COMMAND_SLOT - LENGTH_PREFIX + 1]);
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
