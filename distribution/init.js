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
const EVENT_PAYLOAD = 128; /** range start */
const EVENT_SLOT = 4096; /* bytes per slot */
const EVENT_SLOT_COUNT = 64;

const COMMAND_CONTROL = 262272;
const COMMAND_PAYLOAD = 262400; /** range start */
const COMMAND_SLOT = 4096; /* bytes per slot */
const COMMAND_SLOT_COUNT = 64;

const ARENA_SIZE = 524544; /* bytes per slot */

const CONTROL_WRITE_OFFSET = 0;
const CONTROL_READ_OFFSET = 64;
const LENGTH_PREFIX = 4;

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
    dataView: null,
    eventScratch: new Uint8Array(EVENT_SLOT),
    commandScratch: new Uint8Array(COMMAND_SLOT),
    kick: () => {},
};

let worker = null;
let bound = false;
start();

// flag of retry of loading when fallback to THREAD === "main"
const MAIN_RELOAD_KEY = "app:main-thread-reload-attempted";

// one time retry
async function tryRecoverToWorkerThread() {
    if (sessionStorage.getItem(MAIN_RELOAD_KEY)) return false;
    if (!("serviceWorker" in navigator)) return false;

    sessionStorage.setItem(MAIN_RELOAD_KEY, "1");
    await navigator.serviceWorker.ready.catch(() => {});
    location.reload();
    return true;
}

function start() {
    if (THREAD === "main") {
        tryRecoverToWorkerThread().then((reloading) => {
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

    const el = document.getElementById(decodeId(d));
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
        case 16: jsFn[FN_NAMES[d.u16()]]?.(el); break;
    }
}

function drain() {
    view();
    for (;;) {
        const length = ringPop(
            S.int32, S.uint8, S.dataView,
            S.base + COMMAND_CONTROL, S.base + COMMAND_PAYLOAD, COMMAND_SLOT, COMMAND_SLOT_COUNT,
            S.commandScratch,
        );
        if (length === 0) return;
        const d = new Decoder(S.commandScratch, 1, length);
        execute(S.commandScratch[0], d);
    }
}

const toastCycles = new WeakMap();

const cancelToastCycle = (el) => {
    const cycle = toastCycles.get(el);
    if (!cycle) return;
    clearTimeout(cycle.timer);
    cycle.controller.abort();
};

const jsFn = {
    show: (el) => {
        cancelToastCycle(el);
        el.classList.remove("hidden", "hide");
        requestAnimationFrame(() => requestAnimationFrame(() => {
            el.classList.add("show");
            const timer = setTimeout(() => jsFn.hide(el), 3000);
            toastCycles.set(el, { timer, controller: new AbortController() });
        }));
    },
    hide: (el) => {
        cancelToastCycle(el);
        const controller = new AbortController();
        const finish = () => {
            clearTimeout(fallback);
            el.classList.replace("hide", "hidden");
        };
        el.classList.replace("show", "hide");
        el.addEventListener("transitionend", finish, { once: true, signal: controller.signal });
        const fallback = setTimeout(finish, 250);
        toastCycles.set(el, { timer: fallback, controller });
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

    const encoder = new Encoder(S.eventScratch);
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
    const pushed = ringPush(
        S.int32, S.uint8, S.dataView,
        S.base + EVENT_CONTROL, S.base + EVENT_PAYLOAD, EVENT_SLOT, EVENT_SLOT_COUNT,
        frame,
    );
    if (!pushed) return false;

    Atomics.notify(S.int32, (S.base + EVENT_CONTROL) >> 2);
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

    let resizeTimer;
    window.addEventListener("resize", () => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
            const encoder = new Encoder(S.eventScratch);
            encoder.u8(EVENT_RESIZE);
            encoder.f32(window.innerWidth);
            encoder.f32(window.innerHeight);
            push(encoder.frame());
        }, 100);
    });

    window.addEventListener("scroll", (e) => {
        const encoder = new Encoder(S.eventScratch);
        encoder.u8(EVENT_SCROLL);
        encoder.id(e.target?.id ?? "");
        encoder.f32(window.scrollX);
        encoder.f32(window.scrollY);
        push(encoder.frame());
    }, { passive: true });

    window.addEventListener("pagehide", (e) => {
        if (e.persisted) return;
        const encoder = new Encoder(S.eventScratch);
        encoder.u8(EVENT_SHUTDOWN);
        push(encoder.frame());
    });
}

// listen function case thread == "worker"
async function pump() {
    for (;;) {
        drain();
        view();
        const index = (S.base + COMMAND_CONTROL) >> 2;
        const write = Atomics.load(S.int32, index);
        const result = Atomics.waitAsync(S.int32, index, write);
        if (result.async) await result.value;
    }
}

// === event ===

const EVENT_CANVAS = 1;
const EVENT_RESIZE = 2;
const EVENT_SCROLL = 3;
/** `run_loop` を終了させる。 */
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
 *  jsFn key. index == js_client.rs:FnName
 */
const FN_NAMES = [
    "hide",
    "show",
];

/**
 *  Error kind (for log). key == js_client.rs:CommandError::wire_code
 *
 *  再起動を要するかどうかはこの番号からは判定しない — wasm 側が
 *  `Command::Error` の `serious` バイトとして明示的に送る。0 は未使用。
 */
const ERROR_NAMES = {
    1: "decode",
    2: "command-overflow",
    3: "panic",
    4: "file-store",
};




/**
 * thread が "main" の場合に Wasm をこの場で立ち上げる。
 *
 * worker では worker.js が同じ手順を踏む。
 *
 * `App.init` は `FileStore::new` を await するため dedicated worker を要する
 * (`FileSystemSyncAccessHandle` が worker でしか取得できない)。したがって
 * この経路は永続化を伴わない構成でしか成立しない。THREAD === "main" を
 * 選べるのは、アリーナのレイアウトと command / event の往復だけを
 * 確かめたい場合である。
 */
async function attach() {
    const { default: init, App, arena_pointer, initialize, poll } =
        await import("./app/app.js");
    await init({ memory: S.memory });

    S.exports = { arena_pointer, initialize, poll };
    S.buffer = null;
    initialize();
    S.base = arena_pointer();

    // main thread では Wasm を駆動する主体が居ないため、送信ごとに回す。
    S.kick = () => { poll(); drain(); };

    // `App.init` は async である。await しないと initial_draw の
    // コマンドが積まれる前に kick が走る。
    await App.init(
        window.matchMedia("(pointer: coarse)").matches,
        window.innerWidth,
        window.innerHeight,
    );
    bind();
    S.kick();
}

/**
 * コマンドを書き出す際の追記先と位置を保持する。
 *
 * `arena.rs` の `Encoder` と対称である。
 */
class Encoder {
    /** @param {Uint8Array} scratch - 書き込み先 */
    constructor(scratch) {
        this.scratch = scratch;
        this.dataView = new DataView(scratch.buffer, scratch.byteOffset);
        this.position = 0;
    }

    /** 書き終えた範囲を返す。 */
    frame() { return this.scratch.subarray(0, this.position); }

    u8(value) { this.scratch[this.position++] = value; }
    u16(value) { this.dataView.setUint16(this.position, value, true); this.position += 2; }
    u32(value) { this.dataView.setUint32(this.position, value, true); this.position += 4; }
    i32(value) { this.dataView.setInt32(this.position, value, true); this.position += 4; }
    f32(value) { this.dataView.setFloat32(this.position, value, true); this.position += 4; }
    f64(value) { this.dataView.setFloat64(this.position, value, true); this.position += 8; }

    /** 長さ前置付きで byte 列を追記する。 */
    bytes(value) {
        this.u32(value.length);
        this.scratch.set(value, this.position);
        this.position += value.length;
    }

    /** 長さ前置付きで文字列を UTF-8 として追記する。 */
    str(value) { this.bytes(TEXT_ENCODER.encode(value)); }

    /**
     * element id を `[count:u8]([tag:u8][number:u32])*` として追記する。
     *
     * `arena.rs` の `Encoder::id` / `Decoder::id` と同じ形式である。
     * 連番が無いセグメントは番号に 0xFFFFFFFF を置く。
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
 * イベントを読み出す際の位置を保持する。
 *
 * `arena.rs` の `Decoder` と対称である。範囲外を読んだ場合は undefined を返す。
 */
class Decoder {
    /**
     * @param {Uint8Array} scratch - 読み込み元
     * @param {number}     start   - 読み始める位置
     * @param {number}     end     - 読み終わる位置
     */
    constructor(scratch, start, end) {
        this.scratch = scratch;
        this.dataView = new DataView(scratch.buffer, scratch.byteOffset);
        this.position = start;
        this.end = end;
    }

    /** 現在位置から count byte 進められるか確かめる。 */
    take(count) {
        if (this.position + count > this.end) return false;
        this.position += count;
        return true;
    }

    u8() { return this.take(1) ? this.scratch[this.position - 1] : undefined; }
    u16() { return this.take(2) ? this.dataView.getUint16(this.position - 2, true) : undefined; }
    u32() { return this.take(4) ? this.dataView.getUint32(this.position - 4, true) : undefined; }
    i32() { return this.take(4) ? this.dataView.getInt32(this.position - 4, true) : undefined; }
    f32() { return this.take(4) ? this.dataView.getFloat32(this.position - 4, true) : undefined; }
    f64() { return this.take(8) ? this.dataView.getFloat64(this.position - 8, true) : undefined; }

    /** 長さ前置付きの byte 列を読む。 */
    bytes() {
        const length = this.u32();
        if (length === undefined || !this.take(length)) return undefined;
        return this.scratch.subarray(this.position - length, this.position);
    }

    /** 長さ前置付きの文字列を UTF-8 として読む。 */
    string() {
        const bytes = this.bytes();
        return bytes === undefined ? undefined : TEXT_DECODER.decode(bytes);
    }
}

const TEXT_ENCODER = new TextEncoder();
const TEXT_DECODER = new TextDecoder();

// ============================================================
// arena function
// ============================================================

/**
 * buffer が差し替わっていれば typed array view を作り直して S を返す。
 *
 * 非共有メモリは `memory.grow` で buffer が detach されるため、
 * 参照のたびに同一性を確認する。比較は 1 回のみである。
 *
 * @returns {object} S
 */
function view() {
    const buffer = S.memory.buffer;
    if (buffer !== S.buffer) {
        S.buffer = buffer;
        S.int32 = new Int32Array(buffer);
        S.uint8 = new Uint8Array(buffer);
        S.dataView = new DataView(buffer);
    }
    return S;
}

/**
 * `Encoder::id` が書いた形式から element id を組み立てる。
 *
 * `js_client.rs` の `dom::Id::encode` と同じ文字列を返す。
 *
 * @param {Decoder} d
 * @returns {string} element id
 */
function decodeId(d) {
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
 * 単一書き手・単一読み手のリングへ 1 フレーム追加する。満杯なら false。
 *
 * payload の書き込みは非アトミックで良い。書き込みシーケンスの
 * `Atomics.store` が、それ以前の書き込みの可視性を読み手に対して保証する。
 *
 * @param {Int32Array} int32
 * @param {Uint8Array} uint8
 * @param {DataView}   dataView
 * @param {number}     control    - 制御ブロック位置
 * @param {number}     payload    - スロット領域の開始位置
 * @param {number}     slot       - 1 スロットの byte 数
 * @param {number}     slotCount  - スロット数
 * @param {Uint8Array} source     - 書き込むフレーム
 * @returns {boolean} 追加できたかどうか
 */
function ringPush(int32, uint8, dataView, control, payload, slot, slotCount, source) {
    if (source.length + LENGTH_PREFIX > slot) throw new RangeError("frame too large");

    const writeIndex = control >> 2;
    const readIndex = (control + CONTROL_READ_OFFSET) >> 2;

    const write = Atomics.load(int32, writeIndex) >>> 0;
    const read = Atomics.load(int32, readIndex) >>> 0;
    if (((write - read) >>> 0) >= slotCount) return false;

    const offset = payload + (write & (slotCount - 1)) * slot;
    dataView.setUint32(offset, source.length, true);
    uint8.set(source, offset + LENGTH_PREFIX);

    // コミット。ここで初めて読み手にスロットが見える。
    Atomics.store(int32, writeIndex, (write + 1) | 0);
    return true;
}

/**
 * リング先頭のフレームを destination へ写して長さを返す。空なら 0。
 *
 * @param {Int32Array} int32
 * @param {Uint8Array} uint8
 * @param {DataView}   dataView
 * @param {number}     control     - 制御ブロック位置
 * @param {number}     payload     - スロット領域の開始位置
 * @param {number}     slot        - 1 スロットの byte 数
 * @param {number}     slotCount   - スロット数
 * @param {Uint8Array} destination - 写し先
 * @returns {number} 写した byte 数
 */
function ringPop(int32, uint8, dataView, control, payload, slot, slotCount, destination) {
    const writeIndex = control >> 2;
    const readIndex = (control + CONTROL_READ_OFFSET) >> 2;

    const read = Atomics.load(int32, readIndex) >>> 0;
    const write = Atomics.load(int32, writeIndex) >>> 0;
    if (read === write) return 0;

    const offset = payload + (read & (slotCount - 1)) * slot;
    // 長さ前置語が壊れていてもスロット外へは出ない。
    const length = Math.min(dataView.getUint32(offset, true), slot - LENGTH_PREFIX);
    destination.set(uint8.subarray(offset + LENGTH_PREFIX, offset + LENGTH_PREFIX + length));

    Atomics.store(int32, readIndex, (read + 1) | 0);
    // 満杯で待っている書き手を起こす。
    Atomics.notify(int32, readIndex);
    return length;
}

