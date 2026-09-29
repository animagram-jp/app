// === constants ===

const CONTROL_WRITE_OFFSET = 0;
const CONTROL_READ_OFFSET = 64;
const CONTROL_SIZE = 2 * CONTROL_READ_OFFSET;
const LENGTH_PREFIX = 4;
const FETCH_HEADER = 1 + 4 + 2 + 1 + LENGTH_PREFIX;

const EVENT_CONTROL = 0;
const EVENT_PAYLOAD = EVENT_CONTROL + CONTROL_SIZE; // range start
const EVENT_SLOT = 4096; // bytes per slot
const EVENT_SLOT_COUNT = 64;

const COMMAND_CONTROL = EVENT_PAYLOAD + EVENT_SLOT * EVENT_SLOT_COUNT;
const COMMAND_PAYLOAD = COMMAND_CONTROL + CONTROL_SIZE; // range start
const COMMAND_SLOT = 4096; // bytes per slot
const COMMAND_SLOT_COUNT = 64;

const ARENA_SIZE = COMMAND_PAYLOAD + COMMAND_SLOT * COMMAND_SLOT_COUNT;

const EVENT_RING = {
    control: EVENT_CONTROL, 
    payload: EVENT_PAYLOAD, 
    slot: EVENT_SLOT, 
    slot_count: EVENT_SLOT_COUNT,
};

const COMMAND_RING = {
    control: COMMAND_CONTROL,
    payload: COMMAND_PAYLOAD,
    slot: COMMAND_SLOT,
    slot_count: COMMAND_SLOT_COUNT,
};

const EVENT_CANVAS = 1;
const EVENT_RESIZE = 2;
const EVENT_SCROLL = 3;
const EVENT_VISIBILITY = 4;
const EVENT_FETCH = 5;
const EVENT_SHUTDOWN = 8;

const THREAD = crossOriginIsolated ? "worker" : "main";

// flag of retry of loading when fallback to THREAD === "main"
const MAIN_RELOAD_KEY = "app:main-thread-reload-attempted";

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
    event_frame: new Uint8Array(EVENT_SLOT - LENGTH_PREFIX),
    command_frame: new Uint8Array(COMMAND_SLOT - LENGTH_PREFIX),
    call_app: () => {},
};

let worker = null;
let bound = false;
let restarting = false;
let composing_element = null;

const sw_registration = "serviceWorker" in navigator
    ? navigator.serviceWorker.register("./sw.js").catch((err) => {
        console.warn("SW registration failed:", err);
        return null;
    })
    : Promise.resolve(null);

start();

// === start ===

function start() {
    const params = new URLSearchParams(location.search);
    if (params.has("eruda")) {
        const s = document.createElement("script");
        s.src = "https://cdn.jsdelivr.net/npm/eruda";
        s.onload = () => eruda.init();
        document.body.appendChild(s);
    }

    if (THREAD === "main") {
        try_recover_to_worker_thread().then(async (reloading) => {
            if (reloading) return;

            // `App.init` awaits `FileStore::new`, which requires a
            // dedicated worker (`FileSystemSyncAccessHandle` is only
            // obtainable in a worker). So this path only works for a
            // configuration without persistence. THREAD === "main" is
            // for when you only want to verify the arena layout and the
            // command / event round trip.
            const { default: init, App, arena_pointer, initialize, process_event } =
                await import("./app/app.js?v={version}");
            await init({ module_or_path: "./app/app_bg.wasm?v={version}", memory: S.memory });

            S.exports = { arena_pointer, initialize, process_event };
            S.buffer = null;
            initialize();
            S.base = arena_pointer();

            S.call_app = () => { process_event(); drain(); };

            await App.init(
                window.matchMedia("(pointer: coarse)").matches,
                window.innerWidth,
                window.innerHeight,
            );
            bind();
            S.call_app();
        });
        return;
    }

    const w = new Worker("./worker.js?v={version}", { type: "module" });
    worker = w;

    w.addEventListener("message", async (e) => {
        if (e.data.type === "error") { restart(); }
        if (e.data.type === "ready") {
            S.base = e.data.base;
            sessionStorage.removeItem(MAIN_RELOAD_KEY);

            for (;;) {
                drain();
                view();
                const index = (S.base + COMMAND_RING.control) >> 2;
                const write = Atomics.load(S.int32, index);
                const result = Atomics.waitAsync(S.int32, index, write);
                if (result.async) await result.value;
            }
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
        S.call_app();
    } else {
        start();
    }

    restarting = false;
}

async function try_recover_to_worker_thread() {
    if (sessionStorage.getItem(MAIN_RELOAD_KEY)) return false;
    if (!(await sw_registration)) return false;

    sessionStorage.setItem(MAIN_RELOAD_KEY, "1");
    await navigator.serviceWorker.ready.catch(() => {});
    location.reload();
    return true;
}

// === execute command ===

function drain() {
    view();
    for (;;) {
        const length = ring_pop(COMMAND_RING, S.command_frame);
        if (length === 0) return;
        execute(S.command_frame.subarray(0, length));
    }
}

/**
 *  Execute command (1 octets) recieved from app.
 */
function execute(frame) {
    const operation = frame[0];
    if (operation === 13) {
        const [serious, after_serious] = get_u8(frame, 1);
        const [depth, first] = get_u8(frame, after_serious);
        const identifiers = [];
        let next = first;
        for (let i = 0; i < (depth ?? 0); i++) {
            let identifier;
            [identifier, next] = get_u16(frame, next);
            identifiers.push(identifier);
        }
        const [detail] = get_str(frame, next);
        console.error(`[wasm] error ${identifiers.join(".")}:`, detail ?? "");
        if (serious !== 0) restart();
        return;
    }
    if (operation === 14) {
        const [request, after_request] = get_u32(frame, 1);
        const [method, after_method] = get_u8(frame, after_request);
        const [path, after_path] = get_str(frame, after_method);
        const [body] = get_bytes(frame, after_path);
        if (request === undefined || !METHODS[method] || path === undefined || body === undefined) return;
        fetch_request(request, METHODS[method], path, body.slice());
        return;
    }

    const [id, offset] = get_id(frame, 1);
    const el = document.getElementById(id);
    if (!el) return;
    switch (operation) {
        case  1: el.textContent = get_str(frame, offset)[0] ?? ""; break;
        case  2:
            if (el !== composing_element) el.value = get_str(frame, offset)[0] ?? "";
            break;
        case  3: {
            const [attribute, after] = get_u16(frame, offset);
            el.setAttribute(ATTRIBUTES[attribute], get_str(frame, after)[0] ?? "");
            break;
        }
        case  4: el.removeAttribute(ATTRIBUTES[get_u16(frame, offset)[0]]); break;
        case  5: el.classList.add(CLASS_NAMES[get_u16(frame, offset)[0]]); break;
        case  6: el.classList.remove(CLASS_NAMES[get_u16(frame, offset)[0]]); break;
        case  7: {
            const [property, after] = get_u16(frame, offset);
            const [value] = get_style_value(frame, after);
            if (STYLE_PROPERTIES[property] && value !== undefined) {
                el.style.setProperty(STYLE_PROPERTIES[property], value);
            }
            break;
        }
        case  8: {
            const name = STYLE_PROPERTIES[get_u16(frame, offset)[0]];
            if (name) el.style.removeProperty(name);
            break;
        }
        case  9: el.showModal(); break;
        case 10: el.close(); break;
        case 11: el.focus(); break;
        case 12: js_fn[FN_NAMES[get_u16(frame, offset)[0]]]?.(el); break;
    }
}

async function fetch_request(request, method, path, body) {
    let status;
    let bytes;
    try {
        const response = await fetch(`./api/{version}${path}`, {
            method,
            credentials: "same-origin",
            ...(body.length > 0
                ? { headers: { "Content-Type": "application/octet-stream" }, body }
                : {}),
        });
        status = response.status;
        bytes = new Uint8Array(await response.arrayBuffer());
    } catch (err) {
        status = 0;
        bytes = TEXT_ENCODER.encode(String(err?.message ?? err));
    }
    await push_fetched(request, status, bytes);
}

async function push_fetched(request, status, bytes) {
    const chunk_size = S.event_frame.length - FETCH_HEADER;
    let offset = 0;
    do {
        const chunk = bytes.subarray(offset, offset + chunk_size);
        offset += chunk.length;
        const last = offset >= bytes.length;
        while (!push(encode_fetch_event(S.event_frame, request, status, last, chunk))) {
            await new Promise((resolve) => setTimeout(resolve, 0));
        }
    } while (offset < bytes.length);
}

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

const cancel_toast_cycle = (el) => {
    const cycle = toast_cycles.get(el);
    if (!cycle) return;
    clearTimeout(cycle.timer);
    cycle.controller.abort();
};

const toast_cycles = new WeakMap();

// === send event ===

/**
 * Send Event to app
 *
 * @param {*} e - Web APIs Event
 * @returns
 */
function send(e) {
    const root = root_of(e.target);
    if (!root) return;

    const x = e.clientX ?? 0;
    const y = e.clientY ?? 0;
    const rect = e.clientX === undefined ? null : root.getBoundingClientRect();
    push(encode_canvas_event(S.event_frame, e, x, y, rect ? x - rect.left : 0, rect ? y - rect.top : 0));
}

function send_key(e) {
    if (e.isComposing || e.keyCode === 229 || e.target === composing_element) return;
    send(e);
}

function send_scroll(e) {
    if (e.target === document) {
        push(encode_scroll_event(S.event_frame, window.scrollX, window.scrollY));
        return;
    }
    if (!root_of(e.target)) return;

    push(encode_canvas_event(S.event_frame, e, e.target.scrollLeft, e.target.scrollTop, 0, 0));
}

function key_index(e) {
    const key = e.key?.length === 1 ? e.key.toLowerCase() : e.key;
    return Math.max(KEY_NAMES.indexOf(key), 0);
}

function key_flags(e) {
    return (e.altKey ? 1 << 1 : 0)
        | (e.ctrlKey ? 1 << 2 : 0)
        | (e.metaKey ? 1 << 3 : 0)
        | (e.repeat ? 1 << 4 : 0)
        | (e.shiftKey ? 1 << 5 : 0);
}

function root_of(target) {
    return ROOTS.find(r => r && r.contains(target));
}

function encode_canvas_event(frame, e, x, y, local_x, local_y) {
    let offset = put_u8(frame, 0, EVENT_CANVAS);
    offset = put_u8(frame, offset, Math.max(EVENT_TYPES.indexOf(e.type), 0));
    offset = put_id(frame, offset, e.target.id ?? "");
    offset = put_u8(frame, offset, key_index(e));
    offset = put_u8(frame, offset, key_flags(e));
    offset = put_str(frame, offset, e.target.value ?? "");
    offset = put_f32(frame, offset, x);
    offset = put_f32(frame, offset, y);
    offset = put_f32(frame, offset, local_x);
    offset = put_f32(frame, offset, local_y);
    offset = put_f64(frame, offset, e.timeStamp ?? 0);
    offset = put_u32(frame, offset, e.pointerId ?? 0);
    return frame.subarray(0, offset);
}

function encode_resize_event(frame, width, height) {
    let offset = put_u8(frame, 0, EVENT_RESIZE);
    offset = put_f32(frame, offset, width);
    offset = put_f32(frame, offset, height);
    return frame.subarray(0, offset);
}

function encode_scroll_event(frame, x, y) {
    let offset = put_u8(frame, 0, EVENT_SCROLL);
    offset = put_f32(frame, offset, x);
    offset = put_f32(frame, offset, y);
    return frame.subarray(0, offset);
}

function encode_visibility_event(frame, state) {
    let offset = put_u8(frame, 0, EVENT_VISIBILITY);
    offset = put_u8(frame, offset, Math.max(VISIBILITY_STATES.indexOf(state), 0));
    return frame.subarray(0, offset);
}

function encode_fetch_event(frame, request, status, last, bytes) {
    let offset = put_u8(frame, 0, EVENT_FETCH);
    offset = put_u32(frame, offset, request);
    offset = put_u16(frame, offset, status);
    offset = put_u8(frame, offset, last ? 1 : 0);
    offset = put_bytes(frame, offset, bytes);
    return frame.subarray(0, offset);
}

function encode_shutdown_event(frame) {
    const offset = put_u8(frame, 0, EVENT_SHUTDOWN);
    return frame.subarray(0, offset);
}

/**
 * Write 1 event and call_app.
 *
 * @param {Uint8Array} - frame
 * @returns {boolean} - result
 */
function push(frame) {
    view();
    if (!ring_push(EVENT_RING, frame)) return false;

    Atomics.notify(S.int32, (S.base + EVENT_RING.control) >> 2);
    S.call_app();
    return true;
}

function bind() {
    if (bound) return;
    bound = true;

    const EVENTS = [
        "change", "click", "contextmenu", "focusin", "focusout", "input",
        "pointercancel", "pointerdown", "pointerup", "submit"
    ];
    for (const type of EVENTS) {
        document.addEventListener(type, send);
    }
    document.addEventListener("pointermove", send, { passive: true });

    document.addEventListener("compositionstart", (e) => { composing_element = e.target; });
    document.addEventListener("compositionend", () => { composing_element = null; });
    document.addEventListener("focusout", () => { composing_element = null; });
    for (const type of ["keydown", "keyup"]) {
        document.addEventListener(type, send_key);
    }

    let resize_timer;
    window.addEventListener("resize", () => {
        clearTimeout(resize_timer);
        resize_timer = setTimeout(() => {
            push(encode_resize_event(S.event_frame, window.innerWidth, window.innerHeight));
        }, 100);
    });

    document.addEventListener("scroll", send_scroll, { capture: true, passive: true });

    window.addEventListener("pagehide", (e) => {
        if (e.persisted) return;
        push(encode_shutdown_event(S.event_frame));
    });

    document.addEventListener("visibilitychange", () => {
        push(encode_visibility_event(S.event_frame, document.visibilityState));
    });
}

const VISIBILITY_STATES = [
    null,
    "hidden",
    "visible",
];

const ROOTS = ["header", "main", "form", "toast"]
    .map(id => document.getElementById(id));

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
    "scroll",
    "submit",
];

/**
 *  Key name. index == js_client.rs:KeyName::decode_u8
 */
const KEY_NAMES = [
    null,
    "Alt",
    "AltGraph",
    "&",
    "'",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "*",
    "@",
    "\\",
    "Backspace",
    "`",
    "CapsLock",
    "^",
    "}",
    "]",
    ")",
    ":",
    ",",
    "ContextMenu",
    "Control",
    "Delete",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "$",
    "\"",
    "End",
    "Enter",
    "=",
    "Escape",
    "!",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    ">",
    "#",
    "Home",
    "Insert",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "<",
    "Meta",
    "-",
    "{",
    "[",
    "(",
    "PageDown",
    "PageUp",
    "%",
    ".",
    "|",
    "+",
    "?",
    ";",
    "Shift",
    "/",
    " ",
    "Tab",
    "~",
    "_",
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
    "hgroup",
    "input",
    "label",
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
    "toast",
    "tr",
    "ul",
];

/**
 *  HTML attribute name. index == js_client.rs:Attribute
 */
const ATTRIBUTES = [
    null,
    "disabled",
    "hidden",
];

/**
 *  CSS class name. index == js_client.rs:ClassName
 */
const CLASS_NAMES = [
    null,
    "hide",
    "show",
    "hidden",
];

const METHODS = [
    null,
    "DELETE",
    "GET",
    "POST",
    "PUT",
];

const STYLE_KEYWORDS = [
    null,
    "default",
    "ew-resize",
    "grab",
    "nesw-resize",
    "ns-resize",
    "nwse-resize",
];

const STYLE_PROPERTIES = [
    null,
    "background",
    "cursor",
    "height",
    "translate",
    "width",
    "z-index",
];

const STYLE_UNITS = [
    null,
    "em",
    "%",
    "px",
    "rem",
    "vh",
    "vmax",
    "vmin",
    "vw",
];

/**
 *  js_fn key. index == js_client.rs:FnName
 */
const FN_NAMES = [
    null,
    "hide_toast",
    "show_toast",
];

// === arena ===

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

function view_of(frame) {
    return new DataView(frame.buffer, frame.byteOffset, frame.length);
}

function reserve(frame, offset, count) {
    if (offset + count > frame.length) {
        throw new RangeError(`frame too large: ${offset + count} > ${frame.length}`);
    }
}

function put_u8(frame, offset, value) {
    reserve(frame, offset, 1);
    frame[offset] = value;
    return offset + 1;
}

function put_u16(frame, offset, value) {
    reserve(frame, offset, 2);
    view_of(frame).setUint16(offset, value, true);
    return offset + 2;
}

function put_u32(frame, offset, value) {
    reserve(frame, offset, 4);
    view_of(frame).setUint32(offset, value, true);
    return offset + 4;
}

function put_f32(frame, offset, value) {
    reserve(frame, offset, 4);
    view_of(frame).setFloat32(offset, value, true);
    return offset + 4;
}

function put_f64(frame, offset, value) {
    reserve(frame, offset, 8);
    view_of(frame).setFloat64(offset, value, true);
    return offset + 8;
}

function put_bytes(frame, offset, bytes) {
    reserve(frame, offset, 4 + bytes.length);
    offset = put_u32(frame, offset, bytes.length);
    frame.set(bytes, offset);
    return offset + bytes.length;
}

function put_str(frame, offset, value) {
    return put_bytes(frame, offset, TEXT_ENCODER.encode(value));
}

function put_id(frame, offset, value) {
    if (!value) return put_u8(frame, offset, 0);
    const segments = value.split("_");
    offset = put_u8(frame, offset, segments.length);
    for (const segment of segments) {
        const dash = segment.lastIndexOf("-");
        const number = dash < 0 ? NaN : Number(segment.slice(dash + 1));
        const tag = Number.isInteger(number) ? segment.slice(0, dash) : segment;
        offset = put_u8(frame, offset, Math.max(0, TAGS.indexOf(tag)));
        offset = put_u32(frame, offset, Number.isInteger(number) ? number : 0xFFFFFFFF);
    }
    return offset;
}

function get_u8(frame, offset) {
    if (offset + 1 > frame.length) return [undefined, offset];
    return [frame[offset], offset + 1];
}

function get_u16(frame, offset) {
    if (offset + 2 > frame.length) return [undefined, offset];
    return [view_of(frame).getUint16(offset, true), offset + 2];
}

function get_u32(frame, offset) {
    if (offset + 4 > frame.length) return [undefined, offset];
    return [view_of(frame).getUint32(offset, true), offset + 4];
}

function get_i32(frame, offset) {
    if (offset + 4 > frame.length) return [undefined, offset];
    return [view_of(frame).getInt32(offset, true), offset + 4];
}

function get_f32(frame, offset) {
    if (offset + 4 > frame.length) return [undefined, offset];
    return [view_of(frame).getFloat32(offset, true), offset + 4];
}

function get_bytes(frame, offset) {
    const [length, start] = get_u32(frame, offset);
    if (length === undefined || start + length > frame.length) return [undefined, offset];
    return [frame.subarray(start, start + length), start + length];
}

function get_str(frame, offset) {
    const [bytes, next] = get_bytes(frame, offset);
    return bytes === undefined ? [undefined, offset] : [TEXT_DECODER.decode(bytes), next];
}

function get_style_value(frame, offset) {
    const [tag, start] = get_u8(frame, offset);
    switch (tag) {
        case 1: {
            const [value, next] = get_i32(frame, start);
            return value === undefined ? [undefined, offset] : [String(value), next];
        }
        case 2: {
            const [keyword, next] = get_u16(frame, start);
            const name = STYLE_KEYWORDS[keyword];
            return name ? [name, next] : [undefined, offset];
        }
        case 3: {
            const [value, after] = get_f32(frame, start);
            const [unit, next] = get_u8(frame, after);
            const name = STYLE_UNITS[unit];
            return value === undefined || !name ? [undefined, offset] : [`${value}${name}`, next];
        }
        case 4: {
            const [count, first] = get_u8(frame, start);
            if (count === undefined) return [undefined, offset];
            const parts = [];
            let next = first;
            for (let i = 0; i < count; i++) {
                let part;
                [part, next] = get_style_value(frame, next);
                if (part === undefined) return [undefined, offset];
                parts.push(part);
            }
            return [parts.join(" "), next];
        }
        case 5: {
            const [value, next] = get_f32(frame, start);
            return value === undefined ? [undefined, offset] : [String(value), next];
        }
        case 6: {
            const [text, next] = get_str(frame, start);
            return text === undefined ? [undefined, offset] : [text, next];
        }
        default:
            return [undefined, offset];
    }
}

function get_id(frame, offset) {
    const [count, start] = get_u8(frame, offset);
    if (count === undefined) return ["", offset];
    const segments = [];
    let next = start;
    for (let i = 0; i < count; i++) {
        let tag, number;
        [tag, next] = get_u8(frame, next);
        [number, next] = get_u32(frame, next);
        if (number === undefined) return ["", offset];
        const name = TAGS[tag] ?? "";
        segments.push(number === 0xFFFFFFFF ? name : `${name}-${number}`);
    }
    return [segments.join("_"), next];
}

const TEXT_ENCODER = new TextEncoder();
const TEXT_DECODER = new TextDecoder();

