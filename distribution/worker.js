self.addEventListener("message", async (e) => {
    const { type, payload } = e.data;
    if (type !== "init") return;

    const { default: init, App, arena_offset, initialize, serve_event } =
        await import(payload.app_url);
    const wasm = await init({ module_or_path: payload.module, memory: payload.memory });

    initialize();
    self.postMessage({ type: "ready", arena_offset: arena_offset() });

    await App.init(
        payload.pointer_coarse,
        payload.viewport_width_in_px,
        payload.viewport_height_in_px,
        payload.rem_in_px,
        payload.now_in_ms,
        payload.timezone_offset_minutes,
        payload.color_scheme,
    );
    await serve_event();

    if (wasm.__stack_alloc.value !== 0) wasm.__wbindgen_thread_destroy();
    self.postMessage({ type: "done" });
    self.close();
});
