self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;
    if (type !== "init") return;

    const { default: init, App, arena_pointer, initialize, serve_event } =
        await import("./app/app.js?v={version}");
    await init({ module_or_path: "./app/app_bg.wasm?v={version}", memory: payload.memory });

    initialize();
    const base = arena_pointer();
    self.postMessage({ type: "ready", base });

    await App.init(
        payload.pointer_coarse,
        payload.viewport_width,
        payload.viewport_height,
        payload.rem_in_px,
        payload.now,
        payload.timezone_offset_minutes,
    );
    serve_event();
});

self.addEventListener("error", (e) => {
    self.postMessage({ type: "error", message: e.message });
});
