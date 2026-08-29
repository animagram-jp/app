// dedicated worker のエントリポイント。
//
// init.js の `attach` と同じ手順を踏む。相違点は 2 つだけである。
//
// 1. `memory` は init.js から共有メモリ (SharedArrayBuffer) で渡される。
// 2. 駆動は `poll` の呼び直しではなく、`run_loop` に入りきりで行う。
//    `run_loop` は `Event::Shutdown` を受けるまで戻らない。
//
// コマンド / イベント本体はリング経由であり、postMessage は "init" 受信、
// `S.base` を伝える "ready" 送信、復旧のための "error" 送信にしか使わない。

self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;
    if (type !== "init") return;

    const { default: init, App, arena_pointer, initialize, run_loop } =
        await import("./app/app.js");
    await init({ memory: payload.memory });

    initialize();
    const base = arena_pointer();
    // `ARENA` (static) の線形メモリ内オフセットは wasm モジュールの
    // レイアウトに拠り、0 とは限らない。init.js 側のリング操作は全て
    // これを基点にする必要があるため、確定し次第伝える。
    self.postMessage({ type: "ready", base });

    // `App.init` は async である。await しないと initial_draw の
    // コマンドが積まれる前に run_loop が回り出す。
    await App.init(
        payload.pointer_coarse,
        payload.viewport_width,
        payload.viewport_height,
    );

    // ここから先は戻らない。`Event::Shutdown` で `RUNNING` が false に
    // なるまで、リングを介してイベント / コマンドが往復する。
    run_loop();
});

self.addEventListener("error", (e) => {
    self.postMessage({ type: "error", message: e.message });
});
