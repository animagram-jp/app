self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;
    if (type !== "init") return;

    const { default: init, App, arena_offset, initialize, serve_event } =
        await import("./app/app.js?v={version}");
    const wasm = await init({ module_or_path: "./app/app_bg.wasm?v={version}", memory: payload.memory });

    initialize();
    self.postMessage({ type: "ready", arena_offset: arena_offset() });

    await App.init(
        payload.pointer_coarse,
        payload.viewport_width_in_px,
        payload.viewport_height_in_px,
        payload.rem_in_px,
        payload.now_in_ms,
        payload.timezone_offset_minutes,
    );
    await serve_event();

    wasm.__wbindgen_thread_destroy();
    self.postMessage({ type: "done" });
    self.close();
});
