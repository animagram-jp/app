// init.js
//
// start()
// drain() iter.execute()
// send(Event)
// bind()
//
// MUST sync CONSTANTS with arena.rs
// MUST sync OPERATION with js_client.rs
// MUST sync Event with event.rs

const params = new URLSearchParams(location.search);
if (params.has("eruda")) {
    const s = document.createElement("script");
    s.src = "https://cdn.jsdelivr.net/npm/eruda";
    s.onload = () => eruda.init();
    document.body.appendChild(s);
}

// === arena layout ===

const EVENT_CONTROL = 0;
const EVENT_PAYLOAD = 128; // range start
const EVENT_SLOT = 4096; // bytes per slot
const EVENT_SLOT_COUNT = 64;

const COMMAND_CONTROL = 262272;
const COMMAND_PAYLOAD = 262400; // range start
const COMMAND_SLOT = 4096; // bytes per slot
const COMMAND_SLOT_COUNT = 64;

const ARENA_SIZE = 524544; // bytes per slot

const CONTROL_WRITE_OFFSET = 0;
const CONTROL_READ_OFFSET = 64;
const LENGTH_PREFIX = 4;

// JavaScript -> Wasm
const EVENT_RING = {
    control: EVENT_CONTROL, payload: EVENT_PAYLOAD, slot: EVENT_SLOT, slot_count: EVENT_SLOT_COUNT,
};
// Wasm -> JavaScript
const COMMAND_RING = {
    control:   COMMAND_CONTROL,
    payload:   COMMAND_PAYLOAD,
    slot:      COMMAND_SLOT,
    slot_count: COMMAND_SLOT_COUNT,
};

const THREAD = crossOriginIsolated ? "worker" : "main";

// === arena state ===

/**
 *  MUST Sync with talc allocator -Clink-arg=--max-memory=134217728, 128MiB = 2048 pages
 */
const MEMORY_MAXIMUM_PAGES = 2048;

/**
 * Common state all over the module
 *
 * typed array view must be regenerated when memory.buffer changes.
 */
const S = {
    memory: new WebAssembly.Memory({
        initial: Math.ceil(ARENA_SIZE / 65536) + 256,
        maximum: MEMORY_MAXIMUM_PAGES,
        shared: THREAD === "worker",
    }),
    exports: null,
    base: 0,
    buffer: null,
    int32: null,
    uint8: null,
    data_view: null,
    event_scratch: new Uint8Array(EVENT_SLOT),
    command_scratch: new Uint8Array(COMMAND_SLOT),
    kick: () => {},
};

let worker = null;
let bound = false;
start();

function start() {
    if (THREAD === "main") {
        try_recover_to_worker_thread().then((reloading) => {
            if (!reloading) attach();
        });
        return;
    }

    const w = new Worker("./worker.js", { type: "module" });
    worker = w;

    w.addEventListener("message", (e) => {
        if (e.data.type === "error") { restart(); }
        if (e.data.type === "ready") {
            S.base = e.data.base;
            sessionStorage.removeItem(MAIN_RELOAD_KEY);
            pump();
        }
    });

    w.addEventListener("error", (e) => {
        console.error("[worker] restart:", e.message);
        restart();
    });

    w.postMessage({
        type: "init",
        payload: {
            memory: S.memory,
            pointer_coarse: window.matchMedia("(pointer: coarse)").matches,
            viewport_width: window.innerWidth,
            viewport_height: window.innerHeight,
        },
    });

    bind();
}

/**
 * recreate arena
 * - worker: worker.initialize
 * - main: initialize, not attach
 */
let restarting = false;
function restart() {
    if (restarting) return;
    restarting = true;

    worker?.terminate();
    worker = null;
    S.buffer = null;

    if (THREAD === "main") {
        S.exports?.initialize();
        S.base = S.exports?.arena_pointer() ?? S.base;
        bind();
        S.kick();
    } else {
        start();
    }

    restarting = false;
}

// === main thread ===

// flag of retry of loading when fallback to THREAD === "main"
const MAIN_RELOAD_KEY = "app:main-thread-reload-attempted";

// one time retry
async function try_recover_to_worker_thread() {
    if (sessionStorage.getItem(MAIN_RELOAD_KEY)) return false;
    if (!("serviceWorker" in navigator)) return false;

    sessionStorage.setItem(MAIN_RELOAD_KEY, "1");
    await navigator.serviceWorker.ready.catch(() => {});
    location.reload();
    return true;
}

/**
 * Boots Wasm right here when thread is "main".
 *
 * worker.js follows the same steps for the worker case.
 *
 * `App.init` awaits `FileStore::new`, which requires a dedicated worker
 * (`FileSystemSyncAccessHandle` is only obtainable in a worker). So this
 * path only works for a configuration without persistence. THREAD ===
 * "main" is for when you only want to verify the arena layout and the
 * command / event round trip.
 */
async function attach() {
    const { default: init, App, arena_pointer, initialize, poll } =
        await import("./app/app.js");
    await init({ memory: S.memory });

    S.exports = { arena_pointer, initialize, poll };
    S.buffer = null;
    initialize();
    S.base = arena_pointer();

    // On main thread nothing else drives Wasm, so run it on every send.
    S.kick = () => { poll(); drain(); };

    // `App.init` is async; without awaiting it, kick would run before
    // initial_draw's commands are queued.
    await App.init(
        window.matchMedia("(pointer: coarse)").matches,
        window.innerWidth,
        window.innerHeight,
    );
    bind();
    S.kick();
}

// === worker thread ===

async function pump() {
    for (;;) {
        drain();
        view();
        const index = (S.base + COMMAND_RING.control) >> 2;
        const write = Atomics.load(S.int32, index);
        const result = Atomics.waitAsync(S.int32, index, write);
        if (result.async) await result.value;
    }
}

// === Excute(commands) ===

/**
 *  Excute command (1 octets) recieved from app.
 *
 *  @param {number}  operation - js_client.rs:OPERATION_*
 *  @param {Decoder} d         - payload
 */
function execute(operation, d) {
    switch (operation) {
        case 18: {
            const serious = d.u8() !== 0;
            const code = d.u8();
            const message = d.string() ?? "";
            console.error(`[wasm] ${ERROR_NAMES[code] ?? code}:`, message);
            if (serious) restart();
            return;
        }
    }

    const el = document.getElementById(decode_id(d));
    if (!el) return;
    switch (operation) {
        case  1: el.textContent = d.string() ?? ""; break;
        case  2: el.value = d.string() ?? ""; break;
        case  3: el.setAttribute(ATTRIBUTES[d.u16()], d.string() ?? ""); break;
        case  4: el.removeAttribute(ATTRIBUTES[d.u16()]); break;
        case  5: el.classList.add(CLASS_NAMES[d.u16()]); break;
        case  6: el.classList.remove(CLASS_NAMES[d.u16()]); break;
        case  7: el.style.width = d.u32() + "px"; break;
        case  8: el.style.height = d.u32() + "px"; break;
        case  9: el.style.zIndex = d.i32(); break;
        case 10: el.style.background = d.string(); break;
        case 11: el.style.translate = `${d.f32()}px ${d.f32()}px`; break;
        case 12: el.style.cursor = CURSOR_VALUES[d.u16()] ?? ""; break;
        case 13: el.showModal(); break;
        case 14: el.close(); break;
        case 15: el.focus(); break;
        case 16: js_fn[FN_NAMES[d.u16()]]?.(el); break;
    }
}

function drain() {
    view();
    for (;;) {
        const length = ring_pop(COMMAND_RING, S.command_scratch);
        if (length === 0) return;
        const d = new Decoder(S.command_scratch, 1, length);
        execute(S.command_scratch[0], d);
    }
}

// === toast ===

const toast_cycles = new WeakMap();

const cancel_toast_cycle = (el) => {
    const cycle = toast_cycles.get(el);
    if (!cycle) return;
    clearTimeout(cycle.timer);
    cycle.controller.abort();
};

const js_fn = {
    show_toast: (el) => {
        cancel_toast_cycle(el);
        el.classList.remove("hidden", "hide");
        requestAnimationFrame(() => requestAnimationFrame(() => {
            el.classList.add("show");
            const timer = setTimeout(() => js_fn.hide_toast(el), 3000);
            toast_cycles.set(el, { timer, controller: new AbortController() });
        }));
    },
    hide_toast: (el) => {
        cancel_toast_cycle(el);
        const controller = new AbortController();
        const finish = () => {
            clearTimeout(fallback);
            el.classList.replace("hide", "hidden");
        };
        el.classList.replace("show", "hide");
        el.addEventListener("transitionend", finish, { once: true, signal: controller.signal });
        const fallback = setTimeout(finish, 250);
        toast_cycles.set(el, { timer: fallback, controller });
    },
};

const ROOTS = ["header", "main", "modal", "form", "output", "section"]
    .map(id => document.getElementById(id));

/**
 * Send Event to app
 *
 * @param {*} e - Web APIs Event
 * @returns
 */
function send(e) {
    if (!ROOTS.some(r => r && r.contains(e.target))) return;

    const encoder = new Encoder(S.event_scratch);
    encoder.u8(EVENT_CANVAS);
    encoder.u8(Math.max(EVENT_TYPES.indexOf(e.type), 0));
    encoder.id(e.target.id ?? "");
    encoder.u8(Math.max(KEY_NAMES.indexOf(e.key), 0));
    encoder.str(e.target.value ?? "");
    encoder.f32(e.clientX ?? 0);
    encoder.f32(e.clientY ?? 0);
    encoder.f64(e.timeStamp ?? 0);
    encoder.u32(e.pointerId ?? 0);

    push(encoder.frame());
}

/**
 * Write 1 event and kick App.
 *
 * @param {Uint8Array} - frame
 * @returns {boolean} - result
 */
function push(frame) {
    view();
    if (!ring_push(EVENT_RING, frame)) return false;

    Atomics.notify(S.int32, (S.base + EVENT_RING.control) >> 2);
    S.kick();
    return true;
}

/** Start listening Event to send. */
function bind() {
    if (bound) return;
    bound = true;

    const EVENTS = [
        "click", "keydown", "input", "change", "submit", "focusout",
        "pointerdown", "pointerup", "pointermove", "pointercancel"
    ];
    for (const type of EVENTS) {
        document.addEventListener(type, send);
    }

    let resize_timer;
    window.addEventListener("resize", () => {
        clearTimeout(resize_timer);
        resize_timer = setTimeout(() => {
            const encoder = new Encoder(S.event_scratch);
            encoder.u8(EVENT_RESIZE);
            encoder.f32(window.innerWidth);
            encoder.f32(window.innerHeight);
            push(encoder.frame());
        }, 100);
    });

    window.addEventListener("scroll", (e) => {
        const encoder = new Encoder(S.event_scratch);
        encoder.u8(EVENT_SCROLL);
        encoder.id(e.target?.id ?? "");
        encoder.f32(window.scrollX);
        encoder.f32(window.scrollY);
        push(encoder.frame());
    }, { passive: true });

    window.addEventListener("pagehide", (e) => {
        if (e.persisted) return;
        const encoder = new Encoder(S.event_scratch);
        encoder.u8(EVENT_SHUTDOWN);
        push(encoder.frame());
    });
}

// === event ===

const EVENT_CANVAS = 1;
const EVENT_RESIZE = 2;
const EVENT_SCROLL = 3;
const EVENT_SHUTDOWN = 8;

/**
 *  DOM event type. index == js_client.rs:EventType::decode_u8
 */
const EVENT_TYPES = [
    null,
    "change",
    "click",
    "contextmenu",
    "drop",
    "focusin",
    "focusout",
    "input",
    "keydown",
    "pointercancel",
    "pointerdown",
    "pointermove",
    "pointerup",
    "resize",
    "scroll",
    "submit",
];

/**
 *  Key name. index == js_client.rs:KeyName::decode_u8
 */
const KEY_NAMES = [
    null,
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "Backspace",
    "Enter",
    "Escape",
    "Tab",
];

/**
 *  Tag name. index == js_client.rs:dom::Tag::encode_u8
 */
const TAGS = [
    "",
    "article",
    "body",
    "button",
    "dd",
    "dl",
    "drawer",
    "dt",
    "fieldset",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "header",
    "input",
    "li",
    "main",
    "modal",
    "ol",
    "output",
    "p",
    "section",
    "select",
    "span",
    "table",
    "tbody",
    "td",
    "textarea",
    "th",
    "thead",
    "tr",
    "ul",
];

/**
 *  HTML attribute name. index == js_client.rs:Attribute
 */
const ATTRIBUTES = [
    "disabled",
    "hidden",
];

/**
 *  CSS class name. index == js_client.rs:ClassName
 */
const CLASS_NAMES = [
    "hide",
    "show",
    "hidden",
];

/**
 *  CSS `cursor` value. index == js_client.rs:CursorValue
 */
const CURSOR_VALUES = [
    "default",
    "grab",
];

/**
 *  js_fn key. index == js_client.rs:FnName
 */
const FN_NAMES = [
    "hide_toast",
    "show_toast",
];

/**
 *  Error kind (for log). key == js_client.rs:CommandError::wire_code
 *
 *  Whether a restart is required is not decided by this code; wasm sends
 *  it explicitly as the `serious` byte in `Command::Error`. 0 is unused.
 */
const ERROR_NAMES = {
    1: "decode",
    2: "command-overflow",
    3: "panic",
    4: "file-store",
};

/**
 * Holds the destination and position while writing out a command.
 *
 * Mirrors `Encoder` in `arena.rs`.
 */
class Encoder {
    /** @param {Uint8Array} scratch - write destination */
    constructor(scratch) {
        this.scratch = scratch;
        this.data_view = new DataView(scratch.buffer, scratch.byteOffset);
        this.position = 0;
    }

    /** Returns the range written so far. */
    frame() { return this.scratch.subarray(0, this.position); }

    u8(value) { this.scratch[this.position++] = value; }
    u16(value) { this.data_view.setUint16(this.position, value, true); this.position += 2; }
    u32(value) { this.data_view.setUint32(this.position, value, true); this.position += 4; }
    i32(value) { this.data_view.setInt32(this.position, value, true); this.position += 4; }
    f32(value) { this.data_view.setFloat32(this.position, value, true); this.position += 4; }
    f64(value) { this.data_view.setFloat64(this.position, value, true); this.position += 8; }

    /** Appends a byte sequence, length-prefixed. */
    bytes(value) {
        this.u32(value.length);
        this.scratch.set(value, this.position);
        this.position += value.length;
    }

    /** Appends a string as UTF-8, length-prefixed. */
    str(value) { this.bytes(TEXT_ENCODER.encode(value)); }

    /**
     * Appends an element id as `[count:u8]([tag:u8][number:u32])*`.
     *
     * Same format as `Encoder::id` / `Decoder::id` in `arena.rs`. A
     * segment with no sequence number uses 0xFFFFFFFF.
     */
    id(value) {
        if (!value) { this.u8(0); return; }
        const segments = value.split("_");
        this.u8(segments.length);
        for (const segment of segments) {
            const dash = segment.lastIndexOf("-");
            const number = dash < 0 ? NaN : Number(segment.slice(dash + 1));
            const tag = Number.isInteger(number) ? segment.slice(0, dash) : segment;
            this.u8(Math.max(0, TAGS.indexOf(tag)));
            this.u32(Number.isInteger(number) ? number : 0xFFFFFFFF);
        }
    }
}

/**
 * Holds the position while reading out an event.
 *
 * Mirrors `Decoder` in `arena.rs`. Reading past the end returns undefined.
 */
class Decoder {
    /**
     * @param {Uint8Array} scratch - read source
     * @param {number}     start   - position to start reading from
     * @param {number}     end     - position to stop reading at
     */
    constructor(scratch, start, end) {
        this.scratch = scratch;
        this.data_view = new DataView(scratch.buffer, scratch.byteOffset);
        this.position = start;
        this.end = end;
    }

    /** Checks whether `count` bytes can be advanced from the current position. */
    take(count) {
        if (this.position + count > this.end) return false;
        this.position += count;
        return true;
    }

    u8() { return this.take(1) ? this.scratch[this.position - 1] : undefined; }
    u16() { return this.take(2) ? this.data_view.getUint16(this.position - 2, true) : undefined; }
    u32() { return this.take(4) ? this.data_view.getUint32(this.position - 4, true) : undefined; }
    i32() { return this.take(4) ? this.data_view.getInt32(this.position - 4, true) : undefined; }
    f32() { return this.take(4) ? this.data_view.getFloat32(this.position - 4, true) : undefined; }
    f64() { return this.take(8) ? this.data_view.getFloat64(this.position - 8, true) : undefined; }

    /** Reads a length-prefixed byte sequence. */
    bytes() {
        const length = this.u32();
        if (length === undefined || !this.take(length)) return undefined;
        return this.scratch.subarray(this.position - length, this.position);
    }

    /** Reads a length-prefixed string as UTF-8. */
    string() {
        const bytes = this.bytes();
        return bytes === undefined ? undefined : TEXT_DECODER.decode(bytes);
    }
}

const TEXT_ENCODER = new TextEncoder();
const TEXT_DECODER = new TextDecoder();

// === arena function ===

/**
 * Rebuilds the typed array views and returns S if the buffer changed.
 *
 * Non-shared memory detaches `buffer` on `memory.grow`, so identity is
 * checked on every reference. The comparison itself is a single check.
 *
 * @returns {object} S
 */
function view() {
    const buffer = S.memory.buffer;
    if (buffer !== S.buffer) {
        S.buffer = buffer;
        S.int32 = new Int32Array(buffer);
        S.uint8 = new Uint8Array(buffer);
        S.data_view = new DataView(buffer);
    }
    return S;
}

/**
 * Rebuilds an element id from the format written by `Encoder::id`.
 *
 * Returns the same string as `dom::Id::encode` in `js_client.rs`.
 *
 * @param {Decoder} d
 * @returns {string} element id
 */
function decode_id(d) {
    const count = d.u8();
    if (count === undefined) return "";
    const segments = [];
    for (let i = 0; i < count; i++) {
        const tag = TAGS[d.u8()] ?? "";
        const number = d.u32();
        segments.push(number === 0xFFFFFFFF ? tag : `${tag}-${number}`);
    }
    return segments.join("_");
}

/**
 * Appends 1 frame to a single-writer, single-reader ring. False if full.
 *
 * Writing the payload need not be atomic; the `Atomics.store` of the
 * write sequence guarantees visibility of the prior writes to the reader.
 *
 * @param {{control: number, payload: number, slot: number, slot_count: number}} ring
 * @param {Uint8Array} source - frame to write
 * @returns {boolean} whether it was appended
 */
function ring_push(ring, source) {
    const { slot, slot_count } = ring;
    if (source.length + LENGTH_PREFIX > slot) throw new RangeError("frame too large");

    const control = S.base + ring.control;
    const payload = S.base + ring.payload;
    const write_index = control >> 2;
    const read_index = (control + CONTROL_READ_OFFSET) >> 2;

    const write = Atomics.load(S.int32, write_index) >>> 0;
    const read = Atomics.load(S.int32, read_index) >>> 0;
    if (((write - read) >>> 0) >= slot_count) return false;

    const offset = payload + (write & (slot_count - 1)) * slot;
    S.data_view.setUint32(offset, source.length, true);
    S.uint8.set(source, offset + LENGTH_PREFIX);

    // Commit. Only now does the slot become visible to the reader.
    Atomics.store(S.int32, write_index, (write + 1) | 0);
    return true;
}

/**
 * Copies the front frame of the ring into destination and returns its
 * length. 0 if empty.
 *
 * @param {{control: number, payload: number, slot: number, slot_count: number}} ring
 * @param {Uint8Array} destination - copy destination
 * @returns {number} bytes copied
 */
function ring_pop(ring, destination) {
    const { slot, slot_count } = ring;
    const control = S.base + ring.control;
    const payload = S.base + ring.payload;
    const write_index = control >> 2;
    const read_index = (control + CONTROL_READ_OFFSET) >> 2;

    const read = Atomics.load(S.int32, read_index) >>> 0;
    const write = Atomics.load(S.int32, write_index) >>> 0;
    if (read === write) return 0;

    const offset = payload + (read & (slot_count - 1)) * slot;
    // Even if the length prefix is corrupt, this stays inside the slot.
    const length = Math.min(S.data_view.getUint32(offset, true), slot - LENGTH_PREFIX);
    destination.set(S.uint8.subarray(offset + LENGTH_PREFIX, offset + LENGTH_PREFIX + length));

    Atomics.store(S.int32, read_index, (read + 1) | 0);
    // Wakes a writer that is waiting on a full ring.
    Atomics.notify(S.int32, read_index);
    return length;
}

