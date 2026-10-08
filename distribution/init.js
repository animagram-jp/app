// === constants ===

const COUNTER_WRITE_OFFSET = 0;
const COUNTER_READ_OFFSET = 64;
const DATA_OFFSET = 128;
const LENGTH_PREFIX = 4;
const ALIGNMENT = 4;
const PADDING_MARK = 0xFFFFFFFF;
const FETCH_HEADER = 1 + 4 + 2 + 1 + LENGTH_PREFIX;
const API_BASE = document.querySelector("meta[name='api-base']")?.content ?? "./api/{version}";
const SW_URL = document.querySelector("meta[name='sw-url']")?.content ?? "./sw.js";

const EVENT_RING = {
    start: 0,
    data_size: 262144,
    frame_max: 4096,
};

const COMMAND_RING = {
    start: EVENT_RING.start + DATA_OFFSET + EVENT_RING.data_size,
    data_size: 1048576,
    frame_max: 65536,
};

const ARENA_SIZE = COMMAND_RING.start + DATA_OFFSET + COMMAND_RING.data_size;

const EVENT_CANVAS = 1;
const EVENT_RESIZE = 2;
const EVENT_SCROLL = 3;
const EVENT_VISIBILITY = 4;
const EVENT_FETCH = 5;
const EVENT_FULLSCREEN = 6;
const EVENT_SHUTDOWN = 8;

const THREAD = crossOriginIsolated ? "worker" : "main";

const RELOAD_LOG_KEY = "app:reload-log";
const RELOAD_WINDOW_MS = 60000;
const RELOAD_LIMIT = 3;

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
    arena_offset: 0,
    buffer: null,
    int32: null,
    uint8: null,
    data_view: null,
    event_frame: new Uint8Array(EVENT_RING.frame_max),
    command_frame: new Uint8Array(COMMAND_RING.frame_max),
    call_app: () => {},
};

let composing_element = null;

const sw_registration = "serviceWorker" in navigator
    ? navigator.serviceWorker.register(SW_URL).catch((err) => {
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

    const rem_in_px = parseFloat(getComputedStyle(document.documentElement).fontSize);
    const now = Date.now();
    const timezone_offset_minutes = -new Date().getTimezoneOffset();

    if (THREAD === "main") {
        try_recover_to_worker_thread().then(async (reloading) => {
            if (reloading) return;

            // `App.init` awaits `FileStore::new`, which requires a
            // dedicated worker (`FileSystemSyncAccessHandle` is only
            // obtainable in a worker). So this path only works for a
            // configuration without persistence. THREAD === "main" is
            // for when you only want to verify the arena layout and the
            // command / event round trip.
            const { default: init, App, arena_offset, initialize, process_event } =
                await import("./app/app.js?v={version}");
            await init({ module_or_path: "./app/app_bg.wasm?v={version}", memory: S.memory });

            S.buffer = null;
            initialize();
            S.arena_offset = arena_offset();

            S.call_app = () => { process_event(); drain(); };

            await App.init(
                window.matchMedia("(pointer: coarse)").matches,
                document.documentElement.clientWidth,
                window.innerHeight,
                rem_in_px,
                now,
                timezone_offset_minutes,
            );
            bind();
            S.call_app();
        });
        return;
    }

    const w = new Worker("./worker.js?v={version}", { type: "module" });

    w.addEventListener("message", async (e) => {
        if (e.data.type === "ready") {
            S.arena_offset = e.data.arena_offset;

            for (;;) {
                drain();
                view();
                const index = (S.arena_offset + COMMAND_RING.start) >> 2;
                const write = Atomics.load(S.int32, index);
                const result = Atomics.waitAsync(S.int32, index, write);
                if (result.async) await result.value;
            }
        }
    });

    w.addEventListener("error", (e) => {
        console.error("[worker] reload:", e.message);
        execute(Uint8Array.of(15));
    });

    w.postMessage({
        type: "init",
        payload: {
            memory: S.memory,
            pointer_coarse: window.matchMedia("(pointer: coarse)").matches,
            viewport_width: document.documentElement.clientWidth,
            viewport_height: window.innerHeight,
            rem_in_px,
            now,
            timezone_offset_minutes,
        },
    });

    bind();
}

async function try_recover_to_worker_thread() {
    if (!(await sw_registration)) return false;

    await navigator.serviceWorker.ready.catch(() => {});
    return execute(Uint8Array.of(15)) === true;
}

// === execute command ===

function drain() {
    view();
    for (;;) {
        const length = copy_and_advance_ring(COMMAND_RING, S.command_frame);
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
        const [depth, first] = get_u8(frame, 1);
        const identifiers = [];
        let next = first;
        for (let i = 0; i < (depth ?? 0); i++) {
            let identifier;
            [identifier, next] = get_u16(frame, next);
            identifiers.push(identifier);
        }
        const [detail] = get_str(frame, next);
        console.error(`[wasm] error ${identifiers.join(".")}:`, detail ?? "");
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

    if (operation === 15) {
        try {
            const now = Date.now();
            const log = JSON.parse(sessionStorage.getItem(RELOAD_LOG_KEY) ?? "[]")
                .filter((time) => now - time < RELOAD_WINDOW_MS);
            if (log.length >= RELOAD_LIMIT) {
                console.error("[reload] suppressed:", log.length);
                return false;
            }
            log.push(now);
            sessionStorage.setItem(RELOAD_LOG_KEY, JSON.stringify(log));
        } catch (err) {
            console.error("[reload] suppressed:", err);
            return false;
        }
        location.reload();
        return true;
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
            // checked: the attribute is only the default. Once the user has toggled, only the property counts.
            if (ATTRIBUTES[attribute] === "checked") el.checked = true;
            else el.setAttribute(ATTRIBUTES[attribute], get_str(frame, after)[0] ?? "");
            break;
        }
        case  4: {
            const name = ATTRIBUTES[get_u16(frame, offset)[0]];
            if (name === "checked") el.checked = false;
            else el.removeAttribute(name);
            break;
        }
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
        const response = await fetch(`${API_BASE}${path}`, {
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
    await write_fetched(request, status, bytes);
}

async function write_fetched(request, status, bytes) {
    const chunk_size = S.event_frame.length - FETCH_HEADER;
    let offset = 0;
    do {
        const chunk = bytes.subarray(offset, offset + chunk_size);
        offset += chunk.length;
        const last = offset >= bytes.length;
        while (!write_event(encode_fetch_event(S.event_frame, request, status, last, chunk))) {
            await new Promise((resolve) => setTimeout(resolve, 0));
        }
    } while (offset < bytes.length);
}

const js_fn = {
    copy_text: (el) => {
        navigator.clipboard.writeText(el.value ?? el.textContent).catch((err) => {
            console.warn("clipboard write failed:", err);
        });
    },
    enter_fullscreen: () => {
        document.documentElement.requestFullscreen?.()?.catch(() => {});
    },
    exit_fullscreen: () => {
        if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    },
    show_toast: (el) => {
        cancel_toast_cycle(el);
        el.classList.remove("hide");
        el.hidden = false;
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
            el.classList.remove("hide");
            el.hidden = true;
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
    if (!root && !(e.type.startsWith("key") && [document.body, document.documentElement].includes(e.target))) return;
    if (e.type === "submit") e.preventDefault();

    const x = e.clientX ?? 0;
    const y = e.clientY ?? 0;
    const rect = e.clientX === undefined ? null : root.getBoundingClientRect();
    write_event(encode_canvas_event(S.event_frame, e, x, y, rect ? x - rect.left : 0, rect ? y - rect.top : 0));
}

function send_key(e) {
    const target = e.target;
    if (e.isComposing || e.keyCode === 229 || target === composing_element) return;
    if (e.type === "keydown" && e.key === "Enter" && !e.repeat && (e.ctrlKey || e.metaKey)
        && target instanceof HTMLTextAreaElement && target.form && !target.disabled && !target.readOnly) {
        e.preventDefault();
        target.form.requestSubmit();
        return;
    }
    send(e);
}

function send_scroll(e) {
    if (e.target === document) {
        write_event(encode_scroll_event(S.event_frame, window.scrollX, window.scrollY));
        return;
    }
    if (!root_of(e.target)) return;

    write_event(encode_canvas_event(S.event_frame, e, e.target.scrollLeft, e.target.scrollTop, 0, 0));
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

function event_value(e) {
    if (e.type === "paste") return e.clipboardData?.getData("text/plain") ?? "";
    if (e.target instanceof HTMLFormElement) {
        return new URLSearchParams(new FormData(e.target)).toString();
    }
    return e.target.value ?? "";
}

function encode_canvas_event(frame, e, x, y, local_x, local_y) {
    let offset = put_u8(frame, 0, EVENT_CANVAS);
    offset = put_u8(frame, offset, Math.max(EVENT_TYPES.indexOf(e.type), 0));
    offset = put_id(frame, offset, e.target.id ?? "");
    offset = put_u8(frame, offset, key_index(e));
    offset = put_u8(frame, offset, key_flags(e));
    offset = put_str(frame, offset, event_value(e));
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

function encode_fullscreen_event(frame, type) {
    let offset = put_u8(frame, 0, EVENT_FULLSCREEN);
    offset = put_u8(frame, offset, Math.max(FULLSCREEN_EVENTS.indexOf(type), 0));
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
function write_event(frame) {
    view();
    if (!write_ring(EVENT_RING, frame)) return false;

    Atomics.notify(S.int32, (S.arena_offset + EVENT_RING.start) >> 2);
    S.call_app();
    return true;
}

function bind() {
    const EVENTS = [
        "change", "click", "contextmenu", "copy", "cut", "focusin", "focusout", "input",
        "paste", "pointercancel", "pointerdown", "pointerup", "submit"
    ];
    for (const type of EVENTS) {
        document.addEventListener(type, send);
    }
    for (const type of ["cancel", "close"]) {
        document.addEventListener(type, send, true);
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
            write_event(encode_resize_event(S.event_frame, document.documentElement.clientWidth, window.innerHeight));
        }, 100);
    });

    document.addEventListener("scroll", send_scroll, { capture: true, passive: true });

    window.addEventListener("pagehide", (e) => {
        if (e.persisted) return;
        write_event(encode_shutdown_event(S.event_frame));
    });

    for (const type of FULLSCREEN_EVENTS.slice(1)) {
        document.addEventListener(type, (e) => {
            write_event(encode_fullscreen_event(S.event_frame, e.type));
        });
    }

    document.addEventListener("visibilitychange", () => {
        write_event(encode_visibility_event(S.event_frame, document.visibilityState));
    });
}

const VISIBILITY_STATES = [
    null,
    "hidden",
    "visible",
];

const FULLSCREEN_EVENTS = [
    null,
    "fullscreenchange",
    "fullscreenerror",
];

const ROOTS = ["header", "main", "modal", "form", "toast"]
    .map(id => document.getElementById(id));

/**
 *  DOM event type. index == js_client.rs:EventType::decode_u8
 */
const EVENT_TYPES = [
    null,
    "cancel",
    "change",
    "click",
    "close",
    "contextmenu",
    "copy",
    "cut",
    "drop",
    "focusin",
    "focusout",
    "input",
    "keydown",
    "keyup",
    "paste",
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
    "div",
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
    "nav",
    "ol",
    "option",
    "output",
    "p",
    "section",
    "select",
    "span",
    "strong",
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
    "aria-current",
    "checked",
    "data-surround",
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
    "color",
    "cursor",
    "grid-template-columns",
    "grid-template-rows",
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
    "copy_text",
    "enter_fullscreen",
    "exit_fullscreen",
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
 *
 * @param {{start: number, data_size: number, frame_max: number}} ring
 * @param {Uint8Array} source - frame to write
 * @returns {boolean} whether it was appended
 */
function write_ring(ring, source) {
    const { data_size, frame_max } = ring;
    if (source.length > frame_max) throw new RangeError("frame too large");

    const start = S.arena_offset + ring.start;
    const data = S.arena_offset + ring.start + DATA_OFFSET;
    const write_index = start >> 2;
    const read_index = (start + COUNTER_READ_OFFSET) >> 2;

    const size = LENGTH_PREFIX + Math.ceil(source.length / ALIGNMENT) * ALIGNMENT;
    let write = Atomics.load(S.int32, write_index) >>> 0;
    const read = Atomics.load(S.int32, read_index) >>> 0;
    let used = (write - read) >>> 0;
    let position = write & (data_size - 1);

    const tail = data_size - position;
    if (size > tail) {
        if (used + tail > data_size) return false;
        S.data_view.setUint32(data + position, PADDING_MARK, true);
        write = (write + tail) >>> 0;
        Atomics.store(S.int32, write_index, write | 0);
        used += tail;
        position = 0;
    }
    if (used + size > data_size) return false;

    const offset = data + position;
    S.data_view.setUint32(offset, source.length, true);
    S.uint8.set(source, offset + LENGTH_PREFIX);

    Atomics.store(S.int32, write_index, ((write + size) >>> 0) | 0);
    return true;
}

/**
 *
 * @param {{start: number, data_size: number, frame_max: number}} ring
 * @param {Uint8Array} destination - copy destination
 * @returns {number} bytes copied
 */
function copy_and_advance_ring(ring, destination) {
    const { data_size, frame_max } = ring;
    const start = S.arena_offset + ring.start;
    const data = S.arena_offset + ring.start + DATA_OFFSET;
    const write_index = start >> 2;
    const read_index = (start + COUNTER_READ_OFFSET) >> 2;

    let read = Atomics.load(S.int32, read_index) >>> 0;
    for (;;) {
        const write = Atomics.load(S.int32, write_index) >>> 0;
        if (read === write) return 0;

        const position = read & (data_size - 1);
        const offset = data + position;
        const header = S.data_view.getUint32(offset, true);
        if (header === PADDING_MARK) {
            read = (read + data_size - position) >>> 0;
            Atomics.store(S.int32, read_index, read | 0);
            continue;
        }

        // Even if the length prefix is corrupt, this stays inside the region.
        const length = Math.min(header, frame_max, data_size - position - LENGTH_PREFIX);
        destination.set(S.uint8.subarray(offset + LENGTH_PREFIX, offset + LENGTH_PREFIX + length));

        const size = LENGTH_PREFIX + Math.ceil(length / ALIGNMENT) * ALIGNMENT;
        Atomics.store(S.int32, read_index, ((read + size) >>> 0) | 0);
        Atomics.notify(S.int32, read_index);
        return length;
    }
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

