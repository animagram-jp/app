// Arena
//

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};
use core::{
    cell::UnsafeCell,
    convert::TryInto,
    debug_assert,
    marker::Sync,
    option::Option::{self, None, Some},
    primitive::{bool, f32, f64, i32, str, u8, u16, u32, usize},
    ptr, slice,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    app::App,
    js_client::{CommandError, dom, encode_error},
};

// === arena layout ===

pub const EVENT_CONTROL: usize = 0;
pub const EVENT_PAYLOAD: usize = 128;
pub const EVENT_SLOT: usize = 4096;
pub const EVENT_SLOT_COUNT: u32 = 64;

pub const COMMAND_CONTROL: usize = 262_272;
pub const COMMAND_PAYLOAD: usize = 262_400;
pub const COMMAND_SLOT: usize = 4096;
pub const COMMAND_SLOT_COUNT: u32 = 64;

pub const ARENA_SIZE: usize = 524_544;

pub const CONTROL_WRITE_OFFSET: usize = 0;
///
pub const CONTROL_READ_OFFSET: usize = 64;
pub const LENGTH_PREFIX: usize = 4;

pub const EVENT_CAPACITY: usize = 64;
pub const COMMAND_CAPACITY: usize = 16 * 1024;

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
    fn ring_push(
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
        let length = length.min(slot - LENGTH_PREFIX);
        Some(unsafe { slice::from_raw_parts((offset + LENGTH_PREFIX) as *const u8, length) })
    }

    fn ring_commit_pop(&self, control: usize) {
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
    let commands = app.commands();
    if !commands.is_empty() && !emit(commands) {
        report_error(CommandError::CommandOverflow);
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

// === error report ===

///
///
pub fn report_error(error: CommandError) {
    const OVERHEAD: usize = LENGTH_PREFIX + 1 + 1 + 1 + 4;
    let limit = COMMAND_SLOT - OVERHEAD;

    let full_message = error.to_string();
    let message = if full_message.len() <= limit {
        &full_message[..]
    } else {
        let mut end = limit;
        while end > 0 && !full_message.is_char_boundary(end) {
            end -= 1;
        }
        &full_message[..end]
    };

    let mut frame = Vec::with_capacity(message.len() + OVERHEAD);
    let mut encoder = Encoder::new(&mut frame);
    encode_error(&mut encoder, &error, message);
    let _ = emit(&frame);
}

//
//
//
//

// === encode, decode ===

pub struct Encoder<'a>(&'a mut Vec<u8>);

impl<'a> Encoder<'a> {
    pub fn new(commands: &'a mut Vec<u8>) -> Self {
        Self(commands)
    }

    pub fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    pub fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn bytes(&mut self, value: &[u8]) {
        self.u32(value.len() as u32);
        self.0.extend_from_slice(value);
    }

    pub fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    ///
    pub fn id(&mut self, value: &dom::Id) {
        self.u8(value.0.len() as u8);
        for segment in &value.0 {
            self.u8(segment.tag.encode_u8());
            self.u32(segment.n.unwrap_or(u32::MAX));
        }
    }
}

pub struct Decoder<'a> {
    bytes:    &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let slice = self.bytes.get(self.position..self.position + count)?;
        self.position += count;
        Some(slice)
    }

    pub fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let length = self.u32()? as usize;
        self.take(length)
    }

    pub fn string(&mut self) -> Option<String> {
        Some(str::from_utf8(self.bytes()?).ok()?.to_string())
    }

    pub fn id(&mut self) -> Option<dom::Id> {
        let count = self.u8()? as usize;
        let mut segments = Vec::with_capacity(count);
        for _ in 0..count {
            let tag = dom::Tag::decode_u8(self.u8()?);
            let number = self.u32()?;
            segments.push(dom::Segment {
                tag,
                n: if number == u32::MAX { None } else { Some(number) },
            });
        }
        Some(dom::Id(segments))
    }
}
